use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;

use crate::auth::{authenticate_with_stored_token, login_interactive};
use crate::cli::resolve_text;
use crate::error::AppError;
use crate::pending_submit_cache_store::{
    PendingSubmitCacheItem, append_dedup as append_pending_submit_cache,
    load as load_pending_submit_cache, save as save_pending_submit_cache,
};
use crate::session_store::load_cached_user_id;
use crate::submission::submit_memo;
use crate::supabase::{
    DeleteMemoOutcome, InsertedMemo, RecentMemo, Session, SupabaseClient, UpdateMemoOutcome,
    extract_user_id_from_access_token,
};

use super::chat_widget::{ChatWidget, PendingSubmitAction, WidgetAction};
use super::memo_list_cache_store::{load_for_user, save_for_user};
use super::render;

const WQ_MAX_ATTEMPTS: usize = 3;

enum CopyCommandError {
    NotFound,
    Failed(String),
}

pub struct ComposeApp<'a> {
    client: &'a SupabaseClient,
    widget: ChatWidget,
    memo_list_cache_enabled: bool,
    cached_session: Option<Session>,
}

enum BackgroundEvent {
    InitialHistoryLoaded {
        memos: Vec<RecentMemo>,
        from_cache: bool,
    },
    InitialHistoryLoadFailed {
        message: String,
    },
    RefreshHistoryLoaded(Vec<RecentMemo>),
    RefreshHistoryFailed(String),
    SubmitCreateSuccess {
        normalized_text: String,
        inserted: InsertedMemo,
    },
    SubmitEditSuccess(EditSubmitSuccess),
    SubmitFailed {
        submit: PendingSubmitAction,
        message: String,
        is_auth_error: bool,
    },
    WqStatus(String),
    WqCreateSuccess {
        normalized_text: String,
        inserted: InsertedMemo,
    },
    WqEditSuccess(EditSubmitSuccess),
    WqFailed {
        message: String,
        text: String,
    },
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
            memo_list_cache_enabled: false,
            cached_session: None,
        }
    }

    pub fn new_list(client: &'a SupabaseClient) -> Self {
        let mut app = Self::new(client);
        app.widget.open_memo_list_page();
        app.memo_list_cache_enabled = true;
        app
    }

    fn cached_access_token(&self) -> Option<&str> {
        self.cached_session
            .as_ref()
            .map(|session| session.access_token.as_str())
    }

    fn cache_session(&mut self, session: Session) {
        self.cached_session = Some(session);
    }

    fn invalidate_cached_session(&mut self) {
        self.cached_session = None;
    }

    async fn resolve_submit_access_token(&mut self) -> Result<String, AppError> {
        if let Some(access_token) = self.cached_access_token() {
            return Ok(access_token.to_string());
        }

        let session = authenticate_with_stored_token(self.client).await?;
        let access_token = session.access_token.clone();
        self.cache_session(session);
        Ok(access_token)
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        let mut terminal = TerminalSession::new()?;
        let (background_tx, mut background_rx) = mpsc::unbounded_channel();
        self.start_pending_submit_replay();
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

            let action = match event {
                Event::Key(key_event) => self.widget.handle_key_event(key_event),
                Event::Paste(text) => self.widget.handle_paste_event(&text),
                _ => WidgetAction::None,
            };

            match action {
                WidgetAction::None => {}
                WidgetAction::Quit => break,
                WidgetAction::RefreshHistory => {
                    self.handle_refresh_history(background_tx.clone());
                }
                WidgetAction::SubmitCreate(text) => {
                    self.handle_submit_create(background_tx.clone(), text).await;
                }
                WidgetAction::SubmitEdit {
                    memo_id,
                    expected_version,
                    text,
                } => {
                    self.handle_submit_edit(background_tx.clone(), memo_id, expected_version, text)
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
                WidgetAction::CopySelectedMemo => {
                    self.handle_copy_selected_memo();
                }
                WidgetAction::LoginForPendingSubmit(submit) => {
                    self.handle_login_for_pending_submit(
                        background_tx.clone(),
                        &mut terminal,
                        submit,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    fn start_history_loader(&mut self, tx: UnboundedSender<BackgroundEvent>) {
        self.widget.set_memo_list_loading(true);
        let client = (*self.client).clone();
        let cache_enabled = self.memo_list_cache_enabled;
        tokio::spawn(async move {
            if cache_enabled {
                match load_cached_user_id() {
                    Ok(Some(user_id)) => match load_for_user(&user_id) {
                        Ok(Some(cached_memos)) => {
                            let _ = tx.send(BackgroundEvent::InitialHistoryLoaded {
                                memos: cached_memos,
                                from_cache: true,
                            });
                        }
                        Ok(None) => {}
                        Err(err) => {
                            eprintln!("Warning: failed to read memo list cache: {err}");
                        }
                    },
                    Ok(None) => {}
                    Err(err) => {
                        eprintln!("Warning: failed to read cached user id: {err}");
                    }
                }
            }

            let session = match authenticate_with_stored_token(&client).await {
                Ok(session) => session,
                Err(err) => {
                    if cache_enabled {
                        let _ = tx.send(BackgroundEvent::RefreshHistoryFailed(format!(
                            "Refresh memo list failed: {err}"
                        )));
                    } else {
                        let _ = tx.send(BackgroundEvent::InitialHistoryLoadFailed {
                            message: format_history_load_error(&err),
                        });
                    }
                    return;
                }
            };

            let cache_user_id = if cache_enabled {
                match extract_user_id_from_access_token(&session.access_token) {
                    Ok(user_id) => Some(user_id),
                    Err(err) => {
                        eprintln!("Warning: failed to parse user id from access token: {err}");
                        None
                    }
                }
            } else {
                None
            };

            let event = match client.list_recent_memos(&session.access_token).await {
                Ok(memos) => {
                    if cache_enabled
                        && let Some(user_id) = cache_user_id.as_deref()
                        && let Err(err) = save_for_user(user_id, &memos)
                    {
                        eprintln!("Warning: failed to persist memo list cache: {err}");
                    }

                    if cache_enabled {
                        BackgroundEvent::RefreshHistoryLoaded(memos)
                    } else {
                        BackgroundEvent::InitialHistoryLoaded {
                            memos,
                            from_cache: false,
                        }
                    }
                }
                Err(err) => {
                    if cache_enabled {
                        BackgroundEvent::RefreshHistoryFailed(format!(
                            "Refresh memo list failed: {err}"
                        ))
                    } else {
                        BackgroundEvent::InitialHistoryLoadFailed {
                            message: format_history_load_error(&err),
                        }
                    }
                }
            };

            let _ = tx.send(event);
        });
    }

    fn start_pending_submit_replay(&self) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            run_pending_submit_replay(client).await;
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
                BackgroundEvent::InitialHistoryLoaded { memos, from_cache } => {
                    if !from_cache {
                        self.widget.set_memo_list_loading(false);
                    }
                    self.widget.hydrate_history_from_memos(memos);
                }
                BackgroundEvent::InitialHistoryLoadFailed { message } => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.on_wq_submission_status(&message);
                }
                BackgroundEvent::RefreshHistoryLoaded(memos) => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.refresh_history_from_memos(memos);
                }
                BackgroundEvent::RefreshHistoryFailed(message) => {
                    self.widget.set_memo_list_loading(false);
                    self.widget.on_wq_submission_status(&message);
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
                    submit,
                    message,
                    is_auth_error,
                } => {
                    if is_auth_error {
                        self.invalidate_cached_session();
                        self.widget.show_auth_required_submit_prompt(submit);
                    } else {
                        self.widget
                            .on_submit_error(pending_submit_text(&submit), &message);
                    }
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
                BackgroundEvent::WqFailed { message, text } => {
                    self.widget.on_wq_submission_failed(&message);
                    if text.is_empty() {
                        self.widget.on_wq_submission_status(
                            "Save failed before cache step. Continue editing.",
                        );
                        continue;
                    }
                    match append_pending_submit_cache(&text) {
                        Ok(()) => {
                            self.widget
                                .on_wq_submission_status("Save failed; cached for next launch.");
                            should_quit = true;
                        }
                        Err(err) => {
                            self.widget.on_wq_submission_status(&format!(
                                "Save failed and cache write failed: {err}"
                            ));
                        }
                    }
                }
            }
        }

        should_quit
    }

    async fn handle_submit_create(&mut self, tx: UnboundedSender<BackgroundEvent>, text: String) {
        let normalized = match resolve_text(Some(text)) {
            Ok(value) => value,
            Err(err) => {
                self.widget.on_validation_error(&err.to_string());
                return;
            }
        };

        let access_token = match self.resolve_submit_access_token().await {
            Ok(access_token) => access_token,
            Err(_) => {
                self.widget
                    .show_auth_required_submit_prompt(PendingSubmitAction::Create {
                        text: normalized,
                    });
                return;
            }
        };

        self.widget.on_submit_started();
        self.start_submit_create_submission(tx, normalized, access_token);
    }

    async fn handle_submit_edit(
        &mut self,
        tx: UnboundedSender<BackgroundEvent>,
        memo_id: String,
        expected_version: String,
        text: String,
    ) {
        let normalized = match resolve_text(Some(text)) {
            Ok(value) => value,
            Err(err) => {
                self.widget.on_validation_error(&err.to_string());
                return;
            }
        };

        let access_token = match self.resolve_submit_access_token().await {
            Ok(access_token) => access_token,
            Err(_) => {
                self.widget
                    .show_auth_required_submit_prompt(PendingSubmitAction::Edit {
                        memo_id,
                        expected_version,
                        text: normalized,
                    });
                return;
            }
        };

        self.widget.on_submit_started();
        self.start_submit_edit_submission(tx, memo_id, expected_version, normalized, access_token);
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
        let cache_enabled = self.memo_list_cache_enabled;
        tokio::spawn(async move {
            let event = match authenticate_with_stored_token(&client).await {
                Ok(session) => match client.list_recent_memos(&session.access_token).await {
                    Ok(memos) => {
                        if cache_enabled {
                            match extract_user_id_from_access_token(&session.access_token) {
                                Ok(user_id) => {
                                    if let Err(err) = save_for_user(&user_id, &memos) {
                                        eprintln!(
                                            "Warning: failed to persist memo list cache: {err}"
                                        );
                                    }
                                }
                                Err(err) => {
                                    eprintln!(
                                        "Warning: failed to parse user id from access token: {err}"
                                    );
                                }
                            }
                        }
                        BackgroundEvent::RefreshHistoryLoaded(memos)
                    }
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

    fn start_submit_create_submission(
        &self,
        tx: UnboundedSender<BackgroundEvent>,
        normalized: String,
        access_token: String,
    ) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event = match client.insert_memo(&access_token, &normalized).await {
                Ok(inserted) => BackgroundEvent::SubmitCreateSuccess {
                    normalized_text: normalized,
                    inserted,
                },
                Err(err) => {
                    let is_auth_error = is_auth_error(&err);
                    BackgroundEvent::SubmitFailed {
                        submit: PendingSubmitAction::Create { text: normalized },
                        message: err.to_string(),
                        is_auth_error,
                    }
                }
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
        access_token: String,
    ) {
        let client = (*self.client).clone();
        tokio::spawn(async move {
            let event = match submit_edit_with_access_token(
                &client,
                &access_token,
                &memo_id,
                &expected_version,
                &normalized,
            )
            .await
            {
                Ok(success) => BackgroundEvent::SubmitEditSuccess(success),
                Err(err) => {
                    let is_auth_error = is_auth_error(&err);
                    BackgroundEvent::SubmitFailed {
                        submit: PendingSubmitAction::Edit {
                            memo_id,
                            expected_version,
                            text: normalized,
                        },
                        message: err.to_string(),
                        is_auth_error,
                    }
                }
            };
            let _ = tx.send(event);
        });
    }

    async fn handle_login_for_pending_submit(
        &mut self,
        tx: UnboundedSender<BackgroundEvent>,
        terminal: &mut TerminalSession,
        submit: PendingSubmitAction,
    ) -> Result<(), AppError> {
        terminal.suspend()?;
        let login_result = login_interactive(self.client).await;
        terminal.resume()?;

        match login_result {
            Ok(session) => {
                let access_token = session.access_token.clone();
                self.cache_session(session);
                self.widget.on_submit_started();
                match submit {
                    PendingSubmitAction::Create { text } => {
                        self.start_submit_create_submission(tx, text, access_token);
                    }
                    PendingSubmitAction::Edit {
                        memo_id,
                        expected_version,
                        text,
                    } => {
                        self.start_submit_edit_submission(
                            tx,
                            memo_id,
                            expected_version,
                            text,
                            access_token,
                        );
                    }
                }
            }
            Err(err) => {
                self.widget
                    .on_wq_submission_status(&format!("Login failed: {err}"));
            }
        }
        Ok(())
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
            let _ = tx.send(BackgroundEvent::WqFailed {
                message: err.to_string(),
                text: String::new(),
            });
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
                    let _ = tx.send(BackgroundEvent::WqFailed {
                        message: err.to_string(),
                        text: normalized,
                    });
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
            let _ = tx.send(BackgroundEvent::WqFailed {
                message: err.to_string(),
                text: String::new(),
            });
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
                    let _ = tx.send(BackgroundEvent::WqFailed {
                        message: err.to_string(),
                        text: normalized,
                    });
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

async fn run_pending_submit_replay(client: SupabaseClient) {
    let cached_items = match load_pending_submit_cache() {
        Ok(items) => items,
        Err(err) => {
            eprintln!("Warning: failed to read pending submit cache: {err}");
            return;
        }
    };

    if cached_items.is_empty() {
        return;
    }

    let session = match authenticate_with_stored_token(&client).await {
        Ok(session) => session,
        Err(_) => {
            return;
        }
    };

    let mut failed_items: Vec<PendingSubmitCacheItem> = Vec::new();
    for item in cached_items {
        let normalized = match resolve_text(Some(item.text.clone())) {
            Ok(value) => value,
            Err(err) => {
                eprintln!(
                    "Warning: dropping invalid pending submit item {}: {err}",
                    item.id
                );
                continue;
            }
        };

        if let Err(err) = client.insert_memo(&session.access_token, &normalized).await {
            eprintln!(
                "Warning: replay submit failed for cached item {}: {err}",
                item.id
            );
            failed_items.push(item);
        }
    }

    if let Err(err) = save_pending_submit_cache(&failed_items) {
        eprintln!("Warning: failed to persist pending submit cache replay result: {err}");
    }
}

async fn submit_edit_once(
    client: &SupabaseClient,
    memo_id: &str,
    expected_version: &str,
    normalized: &str,
) -> Result<EditSubmitSuccess, AppError> {
    let session = authenticate_with_stored_token(client).await?;

    submit_edit_with_access_token(
        client,
        &session.access_token,
        memo_id,
        expected_version,
        normalized,
    )
    .await
}

async fn submit_edit_with_access_token(
    client: &SupabaseClient,
    access_token: &str,
    memo_id: &str,
    expected_version: &str,
    normalized: &str,
) -> Result<EditSubmitSuccess, AppError> {
    match client
        .update_memo(access_token, memo_id, normalized, expected_version)
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

fn format_history_load_error(err: &AppError) -> String {
    format!("Load memo list failed: {err}")
}

fn pending_submit_text(submit: &PendingSubmitAction) -> &str {
    match submit {
        PendingSubmitAction::Create { text } => text,
        PendingSubmitAction::Edit { text, .. } => text,
    }
}

fn is_auth_error(err: &AppError) -> bool {
    matches!(err, AppError::Auth(_))
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        copy_with_command("pbcopy", &[], text).map_err(|err| match err {
            CopyCommandError::NotFound => "`pbcopy` not found.".to_string(),
            CopyCommandError::Failed(message) => message,
        })
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
        Err("No clipboard tool found (install `wl-copy`, `xclip`, or `xsel`).".to_string())
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
    active: bool,
}

impl TerminalSession {
    fn new() -> Result<Self, AppError> {
        enable_raw_mode()
            .map_err(|err| AppError::InvalidInput(format!("Failed enabling raw mode: {err}")))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste).map_err(|err| {
            AppError::InvalidInput(format!("Failed entering alternate screen: {err}"))
        })?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .map_err(|err| AppError::InvalidInput(format!("Failed creating terminal: {err}")))?;
        terminal
            .hide_cursor()
            .map_err(|err| AppError::InvalidInput(format!("Failed hiding cursor: {err}")))?;

        Ok(Self {
            terminal,
            active: true,
        })
    }

    fn draw(&mut self, widget: &mut ChatWidget) -> Result<(), AppError> {
        self.terminal
            .draw(|frame| render::draw(frame, widget))
            .map_err(|err| AppError::InvalidInput(format!("TUI draw failed: {err}")))?;
        Ok(())
    }

    fn suspend(&mut self) -> Result<(), AppError> {
        if !self.active {
            return Ok(());
        }

        disable_raw_mode()
            .map_err(|err| AppError::InvalidInput(format!("Failed disabling raw mode: {err}")))?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableBracketedPaste
        )
        .map_err(|err| AppError::InvalidInput(format!("Failed leaving alternate screen: {err}")))?;
        self.terminal
            .show_cursor()
            .map_err(|err| AppError::InvalidInput(format!("Failed showing cursor: {err}")))?;
        self.active = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), AppError> {
        if self.active {
            return Ok(());
        }

        enable_raw_mode()
            .map_err(|err| AppError::InvalidInput(format!("Failed enabling raw mode: {err}")))?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableBracketedPaste
        )
        .map_err(|err| {
            AppError::InvalidInput(format!("Failed entering alternate screen: {err}"))
        })?;
        self.terminal
            .hide_cursor()
            .map_err(|err| AppError::InvalidInput(format!("Failed hiding cursor: {err}")))?;
        self.active = true;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let _ = execute!(
                self.terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableBracketedPaste
            );
            self.active = false;
        }
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::{format_history_load_error, is_auth_error, pending_submit_text};
    use crate::error::AppError;
    use crate::tui::chat_widget::PendingSubmitAction;

    #[test]
    fn history_load_error_prefixes_message() {
        let message = format_history_load_error(&AppError::Auth("You are not logged in".into()));
        assert_eq!(message, "Load memo list failed: You are not logged in");
    }

    #[test]
    fn pending_submit_text_uses_create_text() {
        let submit = PendingSubmitAction::Create {
            text: "draft".to_string(),
        };
        assert_eq!(pending_submit_text(&submit), "draft");
    }

    #[test]
    fn pending_submit_text_uses_edit_text() {
        let submit = PendingSubmitAction::Edit {
            memo_id: "memo-1".to_string(),
            expected_version: "7".to_string(),
            text: "edited".to_string(),
        };
        assert_eq!(pending_submit_text(&submit), "edited");
    }

    #[test]
    fn is_auth_error_detects_auth_variant() {
        assert!(is_auth_error(&AppError::Auth("not logged in".to_string())));
        assert!(!is_auth_error(&AppError::Api("api failure".to_string())));
    }
}
