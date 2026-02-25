use std::io;
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
use super::types::MAX_HISTORY_ITEMS;

const WQ_MAX_ATTEMPTS: usize = 3;

pub struct ComposeApp<'a> {
    client: &'a SupabaseClient,
    widget: ChatWidget,
}

enum BackgroundEvent {
    HistoryLoaded(Vec<RecentMemo>),
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
                    self.handle_refresh_history().await;
                }
                WidgetAction::SubmitCreate(text) => {
                    self.handle_submit_create(text).await;
                }
                WidgetAction::SubmitEdit {
                    memo_id,
                    expected_version,
                    text,
                } => {
                    self.handle_submit_edit(memo_id, expected_version, text)
                        .await;
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
            }
        }

        Ok(())
    }

    fn start_history_loader(&self, tx: UnboundedSender<BackgroundEvent>) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let Ok(session) = authenticate_with_stored_token(&client).await else {
                return;
            };
            let Ok(memos) = client
                .list_recent_memos(&session.access_token, MAX_HISTORY_ITEMS)
                .await
            else {
                return;
            };
            let _ = tx.send(BackgroundEvent::HistoryLoaded(memos));
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
                BackgroundEvent::HistoryLoaded(memos) => {
                    self.widget.hydrate_history_from_memos(memos);
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

    async fn handle_submit_create(&mut self, text: String) {
        let normalized = match resolve_text(Some(text.clone())) {
            Ok(value) => value,
            Err(err) => {
                self.widget.on_validation_error(&err.to_string());
                return;
            }
        };

        match submit_memo(self.client, &normalized).await {
            Ok(result) => {
                self.widget.on_submit_success(
                    &normalized,
                    &result.inserted.id,
                    &result.inserted.created_at,
                    &result.inserted.version,
                );
            }
            Err(err) => {
                self.widget.on_submit_error(&normalized, &err.to_string());
            }
        }
    }

    async fn handle_submit_edit(
        &mut self,
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

        match submit_edit_once(self.client, &memo_id, &expected_version, &normalized).await {
            Ok(EditSubmitSuccess::Updated(updated_memo)) => {
                self.widget.on_edit_success(&updated_memo);
            }
            Ok(EditSubmitSuccess::Conflict {
                server_memo,
                submitted_text,
                forked,
            }) => {
                self.widget
                    .on_edit_conflict(&server_memo, &submitted_text, &forked);
            }
            Err(err) => {
                self.widget.on_submit_error(&normalized, &err.to_string());
            }
        }
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
            Ok(DeleteMemoOutcome::Conflict) => {
                match self
                    .client
                    .get_memo_by_id(&session.access_token, &memo_id)
                    .await
                {
                    Ok(server_memo) => self.widget.on_delete_conflict(&server_memo),
                    Err(err) => self.widget.on_delete_error(&err.to_string()),
                }
            }
            Err(err) => self.widget.on_delete_error(&err.to_string()),
        }
    }

    async fn handle_refresh_history(&mut self) {
        let session = match authenticate_with_stored_token(self.client).await {
            Ok(session) => session,
            Err(err) => {
                self.widget
                    .on_validation_error(&format!("Refresh memo list failed: {err}"));
                return;
            }
        };

        match self
            .client
            .list_recent_memos(&session.access_token, MAX_HISTORY_ITEMS)
            .await
        {
            Ok(memos) => self.widget.refresh_history_from_memos(memos),
            Err(err) => self
                .widget
                .on_validation_error(&format!("Refresh memo list failed: {err}")),
        }
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
        UpdateMemoOutcome::Conflict => {
            let server_memo = client
                .get_memo_by_id(&session.access_token, memo_id)
                .await?;
            let forked = client
                .insert_memo(&session.access_token, normalized)
                .await?;
            Ok(EditSubmitSuccess::Conflict {
                server_memo,
                submitted_text: normalized.to_string(),
                forked,
            })
        }
    }
}

fn wq_retry_delay(attempt: usize) -> Duration {
    match attempt {
        1 => Duration::from_secs(1),
        2 => Duration::from_secs(3),
        _ => Duration::from_secs(0),
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
