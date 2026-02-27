use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;

use crate::auth::authenticate_with_stored_token;
use crate::cli::resolve_text;
use crate::error::AppError;
use crate::submission::submit_memo;
use crate::supabase::{
    DeleteMemoOutcome, InsertedMemo, RecentMemo, SupabaseClient, UpdateMemoOutcome,
};

use super::chat_widget::{ChatWidget, WidgetAction};
use super::render;

const WQ_MAX_ATTEMPTS: usize = 3;

enum CopyCommandError {
    NotFound,
    Failed(String),
}

pub struct ComposeApp<'a> {
    client: &'a SupabaseClient,
    widget: ChatWidget,
}

enum BackgroundEvent {
    InitialHistoryLoaded(Vec<RecentMemo>),
    InitialHistoryLoadFailed,
    RefreshHistoryLoaded(Vec<RecentMemo>),
    RefreshHistoryFailed(String),
    SubmitCreateSuccess {
        normalized_text: String,
        inserted: InsertedMemo,
    },
    SubmitEditSuccess(EditSubmitSuccess),
    SubmitFailed {
        normalized_text: String,
        message: String,
    },
    WqStatus(String),
    WqCreateSuccess {
        normalized_text: String,
        inserted: InsertedMemo,
    },
    WqEditSuccess(EditSubmitSuccess),
    WqFailed(String),
}

enum EditSubmitSuccess {
    Updated(RecentMemo),
    Conflict {
        server_memo: RecentMemo,
        submitted_text: String,
        forked: InsertedMemo,
    },
}

impl<'a> ComposeApp<'a> {
    pub fn new(client: &'a SupabaseClient) -> Self {
        Self {
            client,
            widget: ChatWidget::new(),
        }
    }

    pub fn new_list(client: &'a SupabaseClient) -> Self {
        let mut app = Self::new(client);
        app.widget.open_memo_list_page();
        app
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        let mut terminal = TerminalSession::new()?;
        let (background_tx, mut background_rx) = mpsc::unbounded_channel();
        self.start_history_loader(background_tx.clone());

        loop {
            if self.drain_background_events(&mut background_rx) {
                break;
            }

            terminal.draw(&mut self.widget)?;

            let has_event = event::poll(Duration::from_millis(120))
                .map_err(|err| AppError::InvalidInput(format!("TUI poll failed: {err}")))?;
            if !has_event {
                continue;
            }

            let event = event::read()
                .map_err(|err| AppError::InvalidInput(format!("TUI read failed: {err}")))?;
            let Event::Key(key_event) = event else {
                continue;
            };

            match self.widget.handle_key_event(key_event) {
                WidgetAction::None => {}
                WidgetAction::Quit => break,
                WidgetAction::RefreshHistory => {
                    self.handle_refresh_history(background_tx.clone());
                }
                WidgetAction::SubmitCreate(text) => {
                    self.handle_submit_create(background_tx.clone(), text);
                }
                WidgetAction::SubmitEdit {
                    memo_id,
                    expected_version,
                    text,
                } => {
                    self.handle_submit_edit(background_tx.clone(), memo_id, expected_version, text);
                }
                WidgetAction::SubmitCreateBeforeQuit(text) => {
                    self.start_wq_create_submission(background_tx.clone(), text);
                }
                WidgetAction::SubmitEditBeforeQuit {
                    memo_id,
                    expected_version,
                    text,
                } => {
                    self.start_wq_edit_submission(
                        background_tx.clone(),
                        memo_id,
                        expected_version,
                        text,
                    );
                }
                WidgetAction::DeleteMemo {
                    memo_id,
                    expected_version,
                } => {
                    self.handle_delete_memo(memo_id, expected_version).await;
                }
                WidgetAction::CopySelectedMemo => {
                    self.handle_copy_selected_memo();
                }
            }
        }

        Ok(())
    }

    fn start_history_loader(&mut self, tx: UnboundedSender<BackgroundEvent>) {
        self.widget.set_memo_list_loading(true);
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event = match authenticate_with_stored_token(&client).await {
                Ok(session) => match client.list_recent_memos(&session.access_token).await {
                    Ok(memos) => BackgroundEvent::InitialHistoryLoaded(memos),
                    Err(_) => BackgroundEvent::InitialHistoryLoadFailed,
                },
                Err(_) => BackgroundEvent::InitialHistoryLoadFailed,
            };
            let _ = tx.send(event);
        });
    }

    fn start_wq_create_submission(&self, tx: UnboundedSender<BackgroundEvent>, text: String) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            run_wq_create_submission(client, text, tx).await;
        });
    }

    fn start_wq_edit_submission(
        &self,
        tx: UnboundedSender<BackgroundEvent>,
        memo_id: String,
        expected_version: String,
        text: String,
    ) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            run_wq_edit_submission(client, memo_id, expected_version, text, tx).await;
        });
    }

    fn drain_background_events(&mut self, rx: &mut UnboundedReceiver<BackgroundEvent>) -> bool {
        let mut should_quit = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                BackgroundEvent::InitialHistoryLoaded(memos) => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.hydrate_history_from_memos(memos);
                }
                BackgroundEvent::InitialHistoryLoadFailed => {
                    self.widget.set_memo_list_loading(false);
                }
                BackgroundEvent::RefreshHistoryLoaded(memos) => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.refresh_history_from_memos(memos);
                }
                BackgroundEvent::RefreshHistoryFailed(message) => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.on_validation_error(&message);
                }
                BackgroundEvent::SubmitCreateSuccess {
                    normalized_text,
                    inserted,
                } => {
                    self.widget.on_submit_success(
                        &normalized_text,
                        &inserted.id,
                        &inserted.created_at,
                        &inserted.version,
                    );
                }
                BackgroundEvent::SubmitEditSuccess(success) => match success {
                    EditSubmitSuccess::Updated(updated_memo) => {
                        self.widget.on_edit_success(&updated_memo);
                    }
                    EditSubmitSuccess::Conflict {
                        server_memo,
                        submitted_text,
                        forked,
                    } => {
                        self.widget
                            .on_edit_conflict(&server_memo, &submitted_text, &forked);
                    }
                },
                BackgroundEvent::SubmitFailed {
                    normalized_text,
                    message,
                } => {
                    self.widget.on_submit_error(&normalized_text, &message);
                }
                BackgroundEvent::WqStatus(message) => {
                    self.widget.on_wq_submission_status(&message);
                }
                BackgroundEvent::WqCreateSuccess {
                    normalized_text,
                    inserted,
                } => {
                    self.widget.on_submit_success(
                        &normalized_text,
                        &inserted.id,
                        &inserted.created_at,
                        &inserted.version,
                    );
                    should_quit = true;
                }
                BackgroundEvent::WqEditSuccess(success) => {
                    match success {
                        EditSubmitSuccess::Updated(updated_memo) => {
                            self.widget.on_edit_success(&updated_memo);
                        }
                        EditSubmitSuccess::Conflict {
                            server_memo,
                            submitted_text,
                            forked,
                        } => {
                            self.widget
                                .on_edit_conflict(&server_memo, &submitted_text, &forked);
                        }
                    }
                    should_quit = true;
                }
                BackgroundEvent::WqFailed(message) => {
                    self.widget.on_wq_submission_failed(&message);
                }
            }
        }

        should_quit
    }

    fn handle_submit_create(&mut self, tx: UnboundedSender<BackgroundEvent>, text: String) {
        let normalized = match resolve_text(Some(text.clone())) {
            Ok(value) => value,
            Err(err) => {
                self.widget.on_validation_error(&err.to_string());
                return;
            }
        };

        self.widget.on_submit_started();
        self.start_submit_create_submission(tx, normalized);
    }

    fn handle_submit_edit(
        &mut self,
        tx: UnboundedSender<BackgroundEvent>,
        memo_id: String,
        expected_version: String,
        text: String,
    ) {
        let normalized = match resolve_text(Some(text.clone())) {
            Ok(value) => value,
            Err(err) => {
                self.widget.on_validation_error(&err.to_string());
                return;
            }
        };

        self.widget.on_submit_started();
        self.start_submit_edit_submission(tx, memo_id, expected_version, normalized);
    }

    async fn handle_delete_memo(&mut self, memo_id: String, expected_version: String) {
        let session = match authenticate_with_stored_token(self.client).await {
            Ok(session) => session,
            Err(err) => {
                self.widget.on_delete_error(&err.to_string());
                return;
            }
        };

        match self
            .client
            .delete_memo(&session.access_token, &memo_id, &expected_version)
            .await
        {
            Ok(DeleteMemoOutcome::Deleted) => self.widget.on_delete_success(&memo_id),
            Ok(DeleteMemoOutcome::Conflict { server_memo }) => {
                self.widget.on_delete_conflict(&server_memo);
            }
            Err(err) => self.widget.on_delete_error(&err.to_string()),
        }
    }

    fn handle_refresh_history(&mut self, tx: UnboundedSender<BackgroundEvent>) {
        if self.widget.memo_list_loading() {
            return;
        }
        self.widget.set_memo_list_loading(true);

        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event = match authenticate_with_stored_token(&client).await {
                Ok(session) => match client.list_recent_memos(&session.access_token).await {
                    Ok(memos) => BackgroundEvent::RefreshHistoryLoaded(memos),
                    Err(err) => BackgroundEvent::RefreshHistoryFailed(format!(
                        "Refresh memo list failed: {err}"
                    )),
                },
                Err(err) => BackgroundEvent::RefreshHistoryFailed(format!(
                    "Refresh memo list failed: {err}"
                )),
            };

            let _ = tx.send(event);
        });
    }

    fn handle_copy_selected_memo(&mut self) {
        let Some(text) = self.widget.selected_memo_text().map(ToString::to_string) else {
            self.widget.on_copy_error("No memo selected.");
            return;
        };

        match copy_to_clipboard(&text) {
            Ok(()) => self.widget.on_copy_success(),
            Err(err) => self.widget.on_copy_error(&err),
        }
    }

    fn handle_copy_selected_memo(&mut self) {
        let Some(text) = self.widget.selected_memo_text().map(ToString::to_string) else {
            self.widget.on_copy_error("No memo selected.");
            return;
        };

        match copy_to_clipboard(&text) {
            Ok(()) => self.widget.on_copy_success(),
            Err(err) => self.widget.on_copy_error(&err),
        }
    }

    fn start_submit_create_submission(
        &self,
        tx: UnboundedSender<BackgroundEvent>,
        normalized: String,
    ) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event = match submit_memo(&client, &normalized).await {
                Ok(result) => BackgroundEvent::SubmitCreateSuccess {
                    normalized_text: normalized,
                    inserted: result.inserted,
                },
                Err(err) => BackgroundEvent::SubmitFailed {
                    normalized_text: normalized,
                    message: err.to_string(),
                },
            };
            // Local channel send cannot fail in normal run loop, ignore if UI already closed.
            let _ = tx.send(event);
        });
    }

    fn start_submit_edit_submission(
        &self,
        tx: UnboundedSender<BackgroundEvent>,
        memo_id: String,
        expected_version: String,
        normalized: String,
    ) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event =
                match submit_edit_once(&client, &memo_id, &expected_version, &normalized).await {
                    Ok(success) => BackgroundEvent::SubmitEditSuccess(success),
                    Err(err) => BackgroundEvent::SubmitFailed {
                        normalized_text: normalized,
                        message: err.to_string(),
                    },
                };
            let _ = tx.send(event);
        });
    }
}

async fn run_wq_create_submission(
    client: SupabaseClient,
    text: String,
    tx: UnboundedSender<BackgroundEvent>,
) {
    let normalized = match resolve_text(Some(text)) {
        Ok(value) => value,
        Err(err) => {
            let _ = tx.send(BackgroundEvent::WqFailed(err.to_string()));
            return;
        }
    };

    for attempt in 1..=WQ_MAX_ATTEMPTS {
        match submit_memo(&client, &normalized).await {
            Ok(outcome) => {
                let _ = tx.send(BackgroundEvent::WqCreateSuccess {
                    normalized_text: normalized,
                    inserted: outcome.inserted,
                });
                return;
            }
            Err(err) => {
                if attempt >= WQ_MAX_ATTEMPTS {
                    let _ = tx.send(BackgroundEvent::WqFailed(err.to_string()));
                    return;
                }

                let _ = tx.send(BackgroundEvent::WqStatus(format!(
                    "W attempt {attempt}/{WQ_MAX_ATTEMPTS} failed: {err}. Retrying..."
                )));
                sleep(wq_retry_delay(attempt)).await;
            }
        }
    }
}

async fn run_wq_edit_submission(
    client: SupabaseClient,
    memo_id: String,
    expected_version: String,
    text: String,
    tx: UnboundedSender<BackgroundEvent>,
) {
    let normalized = match resolve_text(Some(text)) {
        Ok(value) => value,
        Err(err) => {
            let _ = tx.send(BackgroundEvent::WqFailed(err.to_string()));
            return;
        }
    };

    for attempt in 1..=WQ_MAX_ATTEMPTS {
        match submit_edit_once(&client, &memo_id, &expected_version, &normalized).await {
            Ok(success) => {
                let _ = tx.send(BackgroundEvent::WqEditSuccess(success));
                return;
            }
            Err(err) => {
                if attempt >= WQ_MAX_ATTEMPTS {
                    let _ = tx.send(BackgroundEvent::WqFailed(err.to_string()));
                    return;
                }

                let _ = tx.send(BackgroundEvent::WqStatus(format!(
                    "W attempt {attempt}/{WQ_MAX_ATTEMPTS} failed: {err}. Retrying..."
                )));
                sleep(wq_retry_delay(attempt)).await;
            }
        }
    }
}

async fn submit_edit_once(
    client: &SupabaseClient,
    memo_id: &str,
    expected_version: &str,
    normalized: &str,
) -> Result<EditSubmitSuccess, AppError> {
    let session = authenticate_with_stored_token(client).await?;

    match client
        .update_memo(&session.access_token, memo_id, normalized, expected_version)
        .await?
    {
        UpdateMemoOutcome::Updated(updated_memo) => Ok(EditSubmitSuccess::Updated(updated_memo)),
        UpdateMemoOutcome::Conflict {
            server_memo,
            forked,
        } => Ok(EditSubmitSuccess::Conflict {
            server_memo,
            submitted_text: normalized.to_string(),
            forked,
        }),
    }
}

fn wq_retry_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_secs(1),
        2 => Duration::from_secs(3),
        _ => Duration::from_secs(0),
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return copy_with_command("pbcopy", &[], text).map_err(|err| match err {
            CopyCommandError::NotFound => "`pbcopy` not found.".to_string(),
            CopyCommandError::Failed(message) => message,
        });
    }

    #[cfg(target_os = "windows")]
    {
        return copy_with_command("clip", &[], text).map_err(|err| match err {
            CopyCommandError::NotFound => "`clip` not found.".to_string(),
            CopyCommandError::Failed(message) => message,
        });
    }

    #[cfg(target_os = "linux")]
    {
        for (program, args) in [
            ("wl-copy", Vec::new()),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ] {
            match copy_with_command(program, &args, text) {
                Ok(()) => return Ok(()),
                Err(CopyCommandError::NotFound) => continue,
                Err(CopyCommandError::Failed(message)) => return Err(message),
            }
        }
        return Err("No clipboard tool found (install `wl-copy`, `xclip`, or `xsel`).".to_string());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = text;
        Err("Clipboard copy is not supported on this platform.".to_string())
    }
}

fn copy_with_command(program: &str, args: &[&str], text: &str) -> Result<(), CopyCommandError> {
    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                return Err(CopyCommandError::NotFound);
            }
            return Err(CopyCommandError::Failed(format!(
                "Failed to start `{program}`: {err}"
            )));
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes()).map_err(|err| {
            CopyCommandError::Failed(format!("Failed to write clipboard data: {err}"))
        })?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| CopyCommandError::Failed(format!("Clipboard command failed: {err}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(CopyCommandError::Failed(format!(
            "`{program}` exited with status {}",
            output.status
        )))
    } else {
        Err(CopyCommandError::Failed(format!(
            "`{program}` failed: {stderr}"
        )))
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    fn new() -> Result<Self, AppError> {
        enable_raw_mode()
            .map_err(|err| AppError::InvalidInput(format!("Failed enabling raw mode: {err}")))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|err| {
            AppError::InvalidInput(format!("Failed entering alternate screen: {err}"))
        })?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|err| AppError::InvalidInput(format!("Failed creating terminal: {err}")))?;
        terminal
            .hide_cursor()
            .map_err(|err| AppError::InvalidInput(format!("Failed hiding cursor: {err}")))?;

        Ok(Self { terminal })
    }

    fn draw(&mut self, widget: &mut ChatWidget) -> Result<(), AppError> {
        self.terminal
            .draw(|frame| render::draw(frame, widget))
            .map_err(|err| AppError::InvalidInput(format!("TUI draw failed: {err}")))?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
