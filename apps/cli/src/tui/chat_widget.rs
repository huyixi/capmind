use std::collections::VecDeque;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::supabase::{InsertedMemo, RecentMemo};

use super::bottom_pane::{BottomPane, InputResult};
use super::composer::VimMode;
use super::types::{FocusArea, HistoryCell};

const SUBMIT_SUCCESS_TIP_DURATION: Duration = Duration::from_secs(1);
const MEMO_LIST_PAGE_STEP: isize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetAction {
    None,
    SubmitCreate(String),
    SubmitEdit {
        memo_id: String,
        expected_version: String,
        text: String,
    },
    SubmitCreateBeforeQuit(String),
    SubmitEditBeforeQuit {
        memo_id: String,
        expected_version: String,
        text: String,
    },
    RefreshHistory,
    DeleteMemo {
        memo_id: String,
        expected_version: String,
    },
    CopySelectedMemo,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    Composer,
    MemoList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpOverlayContext {
    ComposerNormal,
    MemoList,
}

#[derive(Debug, Clone)]
struct PendingDelete {
    memo_id: String,
    expected_version: String,
    preview_text: String,
}

#[derive(Debug, Clone)]
enum ComposerMode {
    Create,
    Edit {
        memo_id: String,
        expected_version: String,
    },
}

#[derive(Debug, Clone)]
pub struct ChatWidget {
    bottom_pane: BottomPane,
    history: VecDeque<HistoryCell>,
    selected_history: usize,
    filtered_history_indices: Vec<usize>,
    memo_list_search_mode: bool,
    memo_list_query: String,
    focus: FocusArea,
    pending_delete: Option<PendingDelete>,
    composer_mode: ComposerMode,
    quit_confirmation_pending: bool,
    page_mode: PageMode,
    split_list_open: bool,
    memo_list_loading: bool,
    help_overlay: Option<HelpOverlayContext>,
    status_message: Option<String>,
    status_message_expires_at: Option<Instant>,
    wq_submission_in_progress: bool,
    wq_failure_prompt: Option<String>,
    clean_composer_text: String,
    composer_dirty: bool,
}

impl Default for ChatWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatWidget {
    pub fn new() -> Self {
        Self {
            bottom_pane: BottomPane::default(),
            history: VecDeque::new(),
            selected_history: 0,
            filtered_history_indices: Vec::new(),
            memo_list_search_mode: false,
            memo_list_query: String::new(),
            focus: FocusArea::Composer,
            pending_delete: None,
            composer_mode: ComposerMode::Create,
            quit_confirmation_pending: false,
            page_mode: PageMode::Composer,
            split_list_open: false,
            memo_list_loading: false,
            help_overlay: None,
            status_message: None,
            status_message_expires_at: None,
            wq_submission_in_progress: false,
            wq_failure_prompt: None,
            clean_composer_text: String::new(),
            composer_dirty: false,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> WidgetAction {
        if key_event.kind != KeyEventKind::Press {
            return WidgetAction::None;
        }

        if is_ctrl_c(key_event) {
            return WidgetAction::Quit;
        }

        if self.wq_failure_prompt.is_some() {
            return self.handle_wq_failure_prompt_key(key_event);
        }

        if self.wq_submission_in_progress {
            return WidgetAction::None;
        }

        if self.help_overlay.is_some() {
            return self.handle_help_overlay_key(key_event);
        }

        if self.pending_delete.is_some() {
            self.quit_confirmation_pending = false;
            return self.handle_delete_confirmation_key(key_event);
        }

        if self.page_mode == PageMode::MemoList {
            return self.handle_memo_list_key(key_event);
        }

        if self.split_list_open && self.focus == FocusArea::History && is_quit_key(key_event) {
            return WidgetAction::Quit;
        }

        if self.focus == FocusArea::Composer
            && key_event.code == KeyCode::Esc
            && self.bottom_pane.composer().is_insert_mode()
        {
            self.quit_confirmation_pending = true;
            return self.handle_composer_key(key_event);
        }

        if key_event.code == KeyCode::Esc {
            if self.quit_confirmation_pending {
                return WidgetAction::Quit;
            }
            self.quit_confirmation_pending = true;
            return WidgetAction::None;
        }

        self.quit_confirmation_pending = false;

        if let Some(action) = self.handle_normal_mode_command_key(key_event) {
            return action;
        }

        match self.focus {
            FocusArea::History => self.handle_history_key(key_event),
            FocusArea::Composer => self.handle_composer_key(key_event),
        }
    }

    pub fn on_wq_submission_status(&mut self, message: &str) {
        self.set_status_message(message.to_string());
    }

    pub fn on_wq_submission_failed(&mut self, message: &str) {
        self.wq_submission_in_progress = false;
        self.wq_failure_prompt = Some(message.to_string());
        self.set_status_message(format!("Save failed: {message}"));
    }

    pub fn on_wq_submission_succeeded(&mut self) {
        self.wq_submission_in_progress = false;
        self.wq_failure_prompt = None;
        self.clear_status_message();
    }

    pub fn open_memo_list_page(&mut self) {
        self.page_mode = PageMode::MemoList;
        self.focus = FocusArea::History;
        self.pending_delete = None;
    }

    pub fn bottom_pane_mut(&mut self) -> &mut BottomPane {
        &mut self.bottom_pane
    }

    pub fn history(&self) -> &VecDeque<HistoryCell> {
        &self.history
    }

    pub fn selected_history(&self) -> Option<usize> {
        if self.history.is_empty() {
            None
        } else {
            Some(
                self.selected_history
                    .min(self.history.len().saturating_sub(1)),
            )
        }
    }

    pub fn focus(&self) -> FocusArea {
        self.focus
    }

    pub fn selected_memo_text(&self) -> Option<&str> {
        let index = self.selected_history_for_current_list()?;
        self.history.get(index).map(|cell| cell.full_text.as_str())
    }

    pub fn memo_list_visible_indices(&self) -> Vec<usize> {
        if self.memo_list_filter_active() {
            return self.filtered_history_indices.clone();
        }
        (0..self.history.len()).collect()
    }

    pub fn memo_list_selected_visible_index(&self) -> Option<usize> {
        let selected = self.selected_history_index_for_memo_list()?;
        self.memo_list_visible_indices()
            .iter()
            .position(|index| *index == selected)
    }

    pub fn memo_list_selected_cell(&self) -> Option<&HistoryCell> {
        let selected = self.selected_history_index_for_memo_list()?;
        self.history.get(selected)
    }

    pub fn memo_list_search_mode(&self) -> bool {
        self.memo_list_search_mode
    }

    pub fn memo_list_query(&self) -> &str {
        &self.memo_list_query
    }

    pub fn page_mode(&self) -> PageMode {
        self.page_mode
    }

    pub fn split_list_open(&self) -> bool {
        self.split_list_open
    }

    pub fn memo_list_loading(&self) -> bool {
        self.memo_list_loading
    }

    pub fn set_memo_list_loading(&mut self, loading: bool) {
        self.memo_list_loading = loading;
    }

    pub fn help_overlay(&self) -> Option<HelpOverlayContext> {
        self.help_overlay
    }

    pub fn delete_confirmation_text(&self) -> Option<&str> {
        self.pending_delete
            .as_ref()
            .map(|pending| pending.preview_text.as_str())
    }

    pub fn quit_confirmation_pending(&self) -> bool {
        self.quit_confirmation_pending
    }

    pub fn is_editing_memo(&self) -> bool {
        matches!(self.composer_mode, ComposerMode::Edit { .. })
    }

    pub fn status_message(&self) -> Option<&str> {
        if let Some(expires_at) = self.status_message_expires_at
            && Instant::now() >= expires_at
        {
            return None;
        }
        self.status_message.as_deref()
    }

    pub fn wq_failure_prompt(&self) -> Option<&str> {
        self.wq_failure_prompt.as_deref()
    }

    #[cfg(test)]
    pub fn wq_submission_in_progress(&self) -> bool {
        self.wq_submission_in_progress
    }

    pub fn on_submit_success(
        &mut self,
        text: &str,
        memo_id: &str,
        created_at: &str,
        version: &str,
    ) {
        self.on_wq_submission_succeeded();
        self.push_history_memo(
            text.to_string(),
            parse_timestamp(created_at).unwrap_or_else(Utc::now),
            memo_id.to_string(),
            version.to_string(),
        );
        self.maybe_recompute_memo_list_filter();
        self.reset_composer_state();
        self.show_submit_success_tip();
    }

    pub fn on_submit_started(&mut self) {
        self.reset_composer_state();
        self.set_status_message("Submitting...".to_string());
    }

    pub fn on_edit_success(&mut self, updated_memo: &RecentMemo) {
        self.on_wq_submission_succeeded();
        self.upsert_history_memo(updated_memo);
        self.reset_composer_state();
        self.show_submit_success_tip();
    }

    pub fn on_edit_conflict(
        &mut self,
        server_memo: &RecentMemo,
        submitted_text: &str,
        forked: &InsertedMemo,
    ) {
        self.on_wq_submission_succeeded();
        self.upsert_history_memo(server_memo);
        self.push_history_memo(
            submitted_text.to_string(),
            parse_timestamp(&forked.created_at).unwrap_or_else(Utc::now),
            forked.id.clone(),
            forked.version.clone(),
        );
        self.maybe_recompute_memo_list_filter();
        self.reset_composer_state();
        self.show_submit_success_tip();
    }

    pub fn on_submit_error(&mut self, text: &str, message: &str) {
        let full_text = format!("{message}\n\n{text}");
        self.push_history(full_text);
        self.maybe_recompute_memo_list_filter();
        self.set_status_message(message.to_string());
    }

    pub fn on_validation_error(&mut self, message: &str) {
        self.push_history(message.to_string());
        self.maybe_recompute_memo_list_filter();
        self.set_status_message(message.to_string());
    }

    pub fn on_delete_success(&mut self, memo_id: &str) {
        self.pending_delete = None;

        let Some(position) = self
            .history
            .iter()
            .position(|cell| cell.memo_id.as_deref() == Some(memo_id))
        else {
            return;
        };

        self.history.remove(position);
        if let ComposerMode::Edit {
            memo_id: editing_id,
            ..
        } = &self.composer_mode
            && editing_id == memo_id
        {
            self.composer_mode = ComposerMode::Create;
            self.set_clean_composer_text("");
        }

        if self.history.is_empty() {
            self.selected_history = 0;
            self.maybe_recompute_memo_list_filter();
            self.focus = FocusArea::Composer;
            return;
        }

        if position < self.selected_history {
            self.selected_history = self.selected_history.saturating_sub(1);
        }
        if self.selected_history >= self.history.len() {
            self.selected_history = self.history.len().saturating_sub(1);
        }
        self.maybe_recompute_memo_list_filter();
    }

    pub fn on_delete_error(&mut self, message: &str) {
        self.pending_delete = None;
        self.on_validation_error(&format!("Delete memo failed: {message}"));
    }

    pub fn on_delete_conflict(&mut self, server_memo: &RecentMemo) {
        self.pending_delete = None;
        if server_memo.deleted_at.is_some() {
            self.on_delete_success(&server_memo.id);
            return;
        }
        self.upsert_history_memo(server_memo);
    }

    pub fn on_copy_success(&mut self) {
        self.set_temporary_status_message("Copied memo.".to_string(), SUBMIT_SUCCESS_TIP_DURATION);
    }

    pub fn on_copy_error(&mut self, message: &str) {
        self.set_status_message(format!("Copy failed: {message}"));
    }

    pub fn hydrate_history_from_memos(&mut self, memos: Vec<RecentMemo>) {
        if !self.history.is_empty() {
            return;
        }

        for memo in memos.into_iter().rev() {
            if memo.deleted_at.is_some() {
                continue;
            }
            self.push_history_memo(
                memo.text,
                parse_timestamp(&memo.created_at).unwrap_or_else(Utc::now),
                memo.id,
                memo.version,
            );
        }
        self.maybe_recompute_memo_list_filter();
    }

    pub fn refresh_history_from_memos(&mut self, memos: Vec<RecentMemo>) {
        let selected_memo_id = self
            .selected_history_for_current_list()
            .and_then(|index| self.history.get(index))
            .and_then(|cell| cell.memo_id.clone());

        self.history.clear();
        self.selected_history = 0;

        for memo in memos.into_iter().rev() {
            if memo.deleted_at.is_some() {
                continue;
            }
            self.push_history_memo(
                memo.text,
                parse_timestamp(&memo.created_at).unwrap_or_else(Utc::now),
                memo.id,
                memo.version,
            );
        }

        if self.history.is_empty() {
            self.focus = FocusArea::Composer;
            return;
        }

        if let Some(selected_memo_id) = selected_memo_id
            && let Some(position) = self
                .history
                .iter()
                .position(|cell| cell.memo_id.as_deref() == Some(selected_memo_id.as_str()))
        {
            self.selected_history = position;
        }
        self.maybe_recompute_memo_list_filter();
    }

    fn handle_composer_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        let before_text = self.bottom_pane.composer().text();
        let result = self.bottom_pane.handle_key_event(key_event);
        self.refresh_composer_dirty_state();

        let action = match result {
            InputResult::None => WidgetAction::None,
            InputResult::Submitted(text) => match &self.composer_mode {
                ComposerMode::Create => WidgetAction::SubmitCreate(text),
                ComposerMode::Edit {
                    memo_id,
                    expected_version,
                } => WidgetAction::SubmitEdit {
                    memo_id: memo_id.clone(),
                    expected_version: expected_version.clone(),
                    text,
                },
            },
            InputResult::Cancelled => WidgetAction::None,
            InputResult::SwitchFocusToHistory => {
                if self.split_list_open {
                    self.focus = FocusArea::History;
                }
                WidgetAction::None
            }
        };

        if before_text != self.bottom_pane.composer().text() {
            self.clear_status_message();
        }

        action
    }

    fn handle_history_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match key_event.code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = FocusArea::Composer;
                WidgetAction::None
            }
            KeyCode::Up => {
                self.move_history_selection(-1);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'k') =>
            {
                self.move_history_selection(-1);
                WidgetAction::None
            }
            KeyCode::Down => {
                self.move_history_selection(1);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'j') =>
            {
                self.move_history_selection(1);
                WidgetAction::None
            }
            KeyCode::Enter => {
                self.load_selected_history_to_composer();
                WidgetAction::None
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'d') => {
                self.start_delete_confirmation();
                WidgetAction::None
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'r') => WidgetAction::RefreshHistory,
            KeyCode::Esc => WidgetAction::None,
            _ => WidgetAction::None,
        }
    }

    fn handle_memo_list_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        if self.memo_list_search_mode && self.handle_memo_list_search_input_key(key_event) {
            return WidgetAction::None;
        }

        if is_help_key(key_event) {
            self.help_overlay = Some(HelpOverlayContext::MemoList);
            return WidgetAction::None;
        }

        match key_event.code {
            KeyCode::Char('/') if is_plain_or_shift(key_event.modifiers) => {
                self.memo_list_search_mode = true;
                self.recompute_memo_list_filter();
                WidgetAction::None
            }
            KeyCode::PageUp => {
                self.move_memo_list_selection(-MEMO_LIST_PAGE_STEP);
                WidgetAction::None
            }
            KeyCode::PageDown => {
                self.move_memo_list_selection(MEMO_LIST_PAGE_STEP);
                WidgetAction::None
            }
            KeyCode::Up => {
                self.move_memo_list_selection(-1);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && c.eq_ignore_ascii_case(&'b') =>
            {
                self.move_memo_list_selection(-MEMO_LIST_PAGE_STEP);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'k') =>
            {
                self.move_memo_list_selection(-1);
                WidgetAction::None
            }
            KeyCode::Down => {
                self.move_memo_list_selection(1);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && c.eq_ignore_ascii_case(&'f') =>
            {
                self.move_memo_list_selection(MEMO_LIST_PAGE_STEP);
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'j') =>
            {
                self.move_memo_list_selection(1);
                WidgetAction::None
            }
            KeyCode::Enter => {
                self.load_selected_history_to_composer();
                self.return_to_composer_insert_mode();
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'q') =>
            {
                self.return_to_composer_insert_mode();
                WidgetAction::None
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'r') =>
            {
                WidgetAction::RefreshHistory
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'y') =>
            {
                WidgetAction::CopySelectedMemo
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'d') => {
                self.start_delete_confirmation();
                WidgetAction::None
            }
            KeyCode::Esc => {
                self.return_to_composer_insert_mode();
                WidgetAction::None
            }
            _ => WidgetAction::None,
        }
    }

    fn handle_memo_list_search_input_key(&mut self, key_event: KeyEvent) -> bool {
        match key_event.code {
            KeyCode::Backspace => {
                self.memo_list_query.pop();
                self.recompute_memo_list_filter();
                true
            }
            KeyCode::Enter => {
                self.memo_list_search_mode = false;
                true
            }
            KeyCode::Esc => {
                self.memo_list_search_mode = false;
                if !self.memo_list_query.is_empty() {
                    self.clear_memo_list_filter();
                }
                true
            }
            KeyCode::Char(c) if is_plain_or_shift(key_event.modifiers) => {
                self.memo_list_query.push(c);
                self.recompute_memo_list_filter();
                true
            }
            _ => false,
        }
    }

    fn handle_delete_confirmation_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match key_event.code {
            KeyCode::Enter => self.confirm_pending_delete(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'y') => self.confirm_pending_delete(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'d') => self.confirm_pending_delete(),
            KeyCode::Esc => {
                self.pending_delete = None;
                WidgetAction::None
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'n') => {
                self.pending_delete = None;
                WidgetAction::None
            }
            _ => {
                self.pending_delete = None;
                WidgetAction::None
            }
        }
    }

    fn handle_wq_failure_prompt_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match key_event.code {
            KeyCode::Enter => {
                self.wq_failure_prompt = None;
                WidgetAction::Quit
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') || c.eq_ignore_ascii_case(&'y') => {
                self.wq_failure_prompt = None;
                WidgetAction::Quit
            }
            KeyCode::Esc => {
                self.wq_failure_prompt = None;
                self.set_status_message("Continue editing. Use w/s to retry.".to_string());
                WidgetAction::None
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'c') || c.eq_ignore_ascii_case(&'n') => {
                self.wq_failure_prompt = None;
                self.set_status_message("Continue editing. Use w/s to retry.".to_string());
                WidgetAction::None
            }
            _ => WidgetAction::None,
        }
    }

    fn handle_help_overlay_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match key_event.code {
            KeyCode::Esc => {
                self.help_overlay = None;
            }
            KeyCode::Char('?') if is_plain_or_shift(key_event.modifiers) => {
                self.help_overlay = None;
            }
            KeyCode::Char(c)
                if is_plain_or_shift(key_event.modifiers) && c.eq_ignore_ascii_case(&'q') =>
            {
                self.help_overlay = None;
            }
            _ => {}
        }
        WidgetAction::None
    }

    fn handle_normal_mode_command_key(&mut self, key_event: KeyEvent) -> Option<WidgetAction> {
        if self.focus != FocusArea::Composer {
            return None;
        }
        if self.bottom_pane.composer().vim_mode() != VimMode::Normal {
            return None;
        }
        if !is_plain_or_shift(key_event.modifiers) {
            return None;
        }

        let KeyCode::Char(c) = key_event.code else {
            return None;
        };

        let action = match c {
            '?' => {
                self.help_overlay = Some(HelpOverlayContext::ComposerNormal);
                WidgetAction::None
            }
            'w' | 's' => self.build_submit_action(false),
            'W' => self.build_submit_action(true),
            'q' => {
                if self.composer_dirty {
                    self.set_status_message("Unsaved changes. Use w/s, W, or Q.".to_string());
                    WidgetAction::None
                } else {
                    WidgetAction::Quit
                }
            }
            'Q' => WidgetAction::Quit,
            'l' => {
                self.page_mode = PageMode::MemoList;
                self.focus = FocusArea::History;
                WidgetAction::None
            }
            'p' => {
                self.split_list_open = !self.split_list_open;
                if !self.split_list_open && self.focus == FocusArea::History {
                    self.focus = FocusArea::Composer;
                }
                self.set_status_message(if self.split_list_open {
                    "Split list opened. Tab switches to list pane.".to_string()
                } else {
                    "Split list hidden.".to_string()
                });
                WidgetAction::None
            }
            _ => return None,
        };

        Some(action)
    }

    fn build_submit_action(&mut self, before_quit: bool) -> WidgetAction {
        let text = self.bottom_pane.composer().text();
        if before_quit {
            self.wq_submission_in_progress = true;
            self.wq_failure_prompt = None;
            self.set_status_message("W submitting in background...".to_string());
        } else {
            self.set_status_message("w submitting...".to_string());
        }

        match &self.composer_mode {
            ComposerMode::Create => {
                if before_quit {
                    WidgetAction::SubmitCreateBeforeQuit(text)
                } else {
                    WidgetAction::SubmitCreate(text)
                }
            }
            ComposerMode::Edit {
                memo_id,
                expected_version,
            } => {
                if before_quit {
                    WidgetAction::SubmitEditBeforeQuit {
                        memo_id: memo_id.clone(),
                        expected_version: expected_version.clone(),
                        text,
                    }
                } else {
                    WidgetAction::SubmitEdit {
                        memo_id: memo_id.clone(),
                        expected_version: expected_version.clone(),
                        text,
                    }
                }
            }
        }
    }

    fn confirm_pending_delete(&mut self) -> WidgetAction {
        let Some(pending) = self.pending_delete.take() else {
            return WidgetAction::None;
        };
        WidgetAction::DeleteMemo {
            memo_id: pending.memo_id,
            expected_version: pending.expected_version,
        }
    }

    fn start_delete_confirmation(&mut self) {
        let index = match self.selected_history_for_current_list() {
            Some(v) => v,
            None => return,
        };
        let Some(cell) = self.history.get(index) else {
            return;
        };
        let (Some(memo_id), Some(expected_version)) =
            (cell.memo_id.clone(), cell.memo_version.clone())
        else {
            return;
        };
        self.pending_delete = Some(PendingDelete {
            memo_id,
            expected_version,
            preview_text: cell.full_text.clone(),
        });
    }

    fn move_history_selection(&mut self, delta: isize) {
        if self.history.is_empty() {
            return;
        }
        let current = self
            .selected_history
            .min(self.history.len().saturating_sub(1)) as isize;
        let max = self.history.len().saturating_sub(1) as isize;
        let next = (current + delta).clamp(0, max) as usize;
        self.selected_history = next;
    }

    fn move_memo_list_selection(&mut self, delta: isize) {
        if !self.memo_list_filter_active() {
            self.move_history_selection(delta);
            return;
        }

        if self.filtered_history_indices.is_empty() {
            return;
        }

        let current_index = self
            .selected_history_index_for_memo_list()
            .unwrap_or(self.filtered_history_indices[0]);
        let current_position = self
            .filtered_history_indices
            .iter()
            .position(|index| *index == current_index)
            .unwrap_or(0) as isize;
        let max = self.filtered_history_indices.len().saturating_sub(1) as isize;
        let next_position = (current_position + delta).clamp(0, max) as usize;
        self.selected_history = self.filtered_history_indices[next_position];
    }

    fn load_selected_history_to_composer(&mut self) {
        let index = match self.selected_history_for_current_list() {
            Some(v) => v,
            None => return,
        };
        let Some(cell) = self.history.get(index).cloned() else {
            return;
        };
        self.bottom_pane.composer_mut().set_text(&cell.full_text);
        self.focus = FocusArea::Composer;
        self.page_mode = PageMode::Composer;
        self.composer_mode = match (cell.memo_id, cell.memo_version) {
            (Some(memo_id), Some(expected_version)) => ComposerMode::Edit {
                memo_id,
                expected_version,
            },
            _ => ComposerMode::Create,
        };
        self.set_clean_composer_text(&cell.full_text);
    }

    fn push_history(&mut self, full_text: String) {
        self.push_history_cell(HistoryCell::new(full_text));
    }

    fn push_history_memo(
        &mut self,
        full_text: String,
        created_at: DateTime<Utc>,
        memo_id: String,
        memo_version: String,
    ) {
        self.push_history_cell(HistoryCell::with_memo(
            full_text,
            created_at,
            memo_id,
            memo_version,
        ));
    }

    fn push_history_cell(&mut self, cell: HistoryCell) {
        self.history.push_back(cell);
        self.selected_history = self.history.len().saturating_sub(1);
    }

    fn upsert_history_memo(&mut self, memo: &RecentMemo) {
        let parsed_created_at = parse_timestamp(&memo.created_at).unwrap_or_else(Utc::now);
        if let Some(position) = self
            .history
            .iter()
            .position(|cell| cell.memo_id.as_deref() == Some(memo.id.as_str()))
        {
            if let Some(cell) = self.history.get_mut(position) {
                cell.full_text = memo.text.clone();
                cell.created_at = parsed_created_at;
                cell.memo_version = Some(memo.version.clone());
            }
            self.selected_history = position;
            self.maybe_recompute_memo_list_filter();
            return;
        }

        self.push_history_memo(
            memo.text.clone(),
            parsed_created_at,
            memo.id.clone(),
            memo.version.clone(),
        );
        self.maybe_recompute_memo_list_filter();
    }

    fn reset_composer_state(&mut self) {
        self.bottom_pane.composer_mut().clear();
        self.focus = FocusArea::Composer;
        self.page_mode = PageMode::Composer;
        self.composer_mode = ComposerMode::Create;
        self.set_clean_composer_text("");
    }

    fn set_clean_composer_text(&mut self, text: &str) {
        self.clean_composer_text = text.to_string();
        self.refresh_composer_dirty_state();
    }

    fn return_to_composer_insert_mode(&mut self) {
        self.page_mode = PageMode::Composer;
        self.focus = FocusArea::Composer;
        self.memo_list_search_mode = false;
        self.bottom_pane.composer_mut().switch_to_insert_mode();
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_message_expires_at = None;
    }

    fn set_temporary_status_message(&mut self, message: String, duration: Duration) {
        self.status_message = Some(message);
        self.status_message_expires_at = Some(Instant::now() + duration);
    }

    fn clear_status_message(&mut self) {
        self.status_message = None;
        self.status_message_expires_at = None;
    }

    fn show_submit_success_tip(&mut self) {
        self.set_temporary_status_message("Submitted.".to_string(), SUBMIT_SUCCESS_TIP_DURATION);
    }

    fn refresh_composer_dirty_state(&mut self) {
        self.composer_dirty = self.bottom_pane.composer().text() != self.clean_composer_text;
    }

    fn clear_memo_list_filter(&mut self) {
        self.memo_list_query.clear();
        self.filtered_history_indices.clear();
    }

    fn maybe_recompute_memo_list_filter(&mut self) {
        if self.memo_list_filter_active() {
            self.recompute_memo_list_filter();
        }
    }

    fn recompute_memo_list_filter(&mut self) {
        let selected_before = self.selected_history();
        if self.memo_list_query.is_empty() {
            self.filtered_history_indices = (0..self.history.len()).collect();
        } else {
            let query = self.memo_list_query.to_lowercase();
            self.filtered_history_indices = self
                .history
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| {
                    if cell.full_text.to_lowercase().contains(&query) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect();
        }

        let Some(selected_before) = selected_before else {
            return;
        };
        if self
            .filtered_history_indices
            .iter()
            .any(|index| *index == selected_before)
        {
            return;
        }
        if let Some(first) = self.filtered_history_indices.first() {
            self.selected_history = *first;
        }
    }

    fn memo_list_filter_active(&self) -> bool {
        self.memo_list_search_mode || !self.memo_list_query.is_empty()
    }

    fn selected_history_for_current_list(&self) -> Option<usize> {
        if self.page_mode == PageMode::MemoList {
            return self.selected_history_index_for_memo_list();
        }
        self.selected_history()
    }

    fn selected_history_index_for_memo_list(&self) -> Option<usize> {
        if !self.memo_list_filter_active() {
            return self.selected_history();
        }
        if self.filtered_history_indices.is_empty() {
            return None;
        }

        let selected = self.selected_history();
        if let Some(selected) = selected
            && self
                .filtered_history_indices
                .iter()
                .any(|index| *index == selected)
        {
            return Some(selected);
        }
        self.filtered_history_indices.first().copied()
    }
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn is_ctrl_c(key_event: KeyEvent) -> bool {
    matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'c')
    )
}

fn is_quit_key(key_event: KeyEvent) -> bool {
    matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers,
            ..
        } if is_plain_or_shift(modifiers) && c.eq_ignore_ascii_case(&'q')
    )
}

fn is_plain_or_shift(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn is_help_key(key_event: KeyEvent) -> bool {
    matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Char('?'),
            modifiers,
            ..
        } if is_plain_or_shift(modifiers)
    )
}

#[cfg(test)]
mod tests {
    use super::{ChatWidget, HelpOverlayContext, PageMode, WidgetAction};
    use crate::supabase::RecentMemo;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn push_history_keeps_insert_order() {
        let mut widget = ChatWidget::new();
        widget.on_validation_error("one");
        widget.on_submit_success("two", "memo_2", "2026-02-23T00:00:00Z", "1");
        widget.on_submit_error("three", "err-3");
        widget.on_validation_error("four");

        assert_eq!(widget.history().len(), 4);
        assert_eq!(
            history_texts(&widget),
            vec![
                "one".to_string(),
                "two".to_string(),
                "err-3\n\nthree".to_string(),
                "four".to_string()
            ]
        );
    }

    #[test]
    fn history_keeps_all_entries_without_eviction() {
        let mut widget = ChatWidget::new();
        for idx in 0..102 {
            widget.on_validation_error(&format!("entry-{idx}"));
        }

        assert_eq!(widget.history().len(), 102);
        assert_eq!(
            widget.history().front().map(|cell| cell.full_text.as_str()),
            Some("entry-0")
        );
        assert_eq!(
            widget.history().back().map(|cell| cell.full_text.as_str()),
            Some("entry-101")
        );
    }

    #[test]
    fn hydrate_history_keeps_image_only_memos() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "", "7")]);

        assert_eq!(widget.history().len(), 1);
        assert_eq!(
            widget
                .history()
                .front()
                .and_then(|cell| cell.memo_id.as_deref()),
            Some("memo-1")
        );
        assert_eq!(
            widget.history().front().map(|cell| cell.full_text.as_str()),
            Some("")
        );
    }

    #[test]
    fn refresh_history_preserves_selection_with_image_only_memo() {
        let mut widget = ChatWidget::new();
        let memos = vec![memo("memo-2", "", "2"), memo("memo-1", "memo-1", "1")];
        widget.hydrate_history_from_memos(memos.clone());

        let selected_memo_id_before = widget
            .selected_history()
            .and_then(|index| widget.history().get(index))
            .and_then(|cell| cell.memo_id.clone());
        widget.refresh_history_from_memos(memos);
        let selected_memo_id_after = widget
            .selected_history()
            .and_then(|index| widget.history().get(index))
            .and_then(|cell| cell.memo_id.clone());

        assert_eq!(widget.history().len(), 2);
        assert_eq!(selected_memo_id_before, selected_memo_id_after);
    }

    #[test]
    fn history_enter_loads_edit_mode_without_duplicating_entry() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "7")]);

        open_split_list(&mut widget);
        widget.handle_key_event(key(KeyCode::Tab));
        let action = widget.handle_key_event(key(KeyCode::Enter));

        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.history().len(), 1);
        assert!(widget.is_editing_memo());
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "memo-1");
    }

    #[test]
    fn submit_in_edit_mode_emits_submit_edit_action() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "7")]);
        open_split_list(&mut widget);
        widget.handle_key_event(key(KeyCode::Tab));
        widget.handle_key_event(key(KeyCode::Enter));

        let action = widget.handle_key_event(key(KeyCode::Char('x')));
        assert_eq!(action, WidgetAction::None);

        let submit = widget.handle_key_event(ctrl(KeyCode::Char('s')));
        assert_eq!(
            submit,
            WidgetAction::SubmitEdit {
                memo_id: "memo-1".to_string(),
                expected_version: "7".to_string(),
                text: "memo-1x".to_string(),
            }
        );
    }

    #[test]
    fn q_quits_in_history() {
        let mut widget = ChatWidget::new();
        open_split_list(&mut widget);
        widget.handle_key_event(key(KeyCode::Tab));

        let action = widget.handle_key_event(key(KeyCode::Char('q')));
        assert_eq!(action, WidgetAction::Quit);
    }

    #[test]
    fn tab_does_not_switch_focus_when_split_list_closed() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Tab));
        assert_eq!(widget.focus(), crate::tui::types::FocusArea::Composer);
    }

    #[test]
    fn colon_still_inserts_text_in_insert_mode() {
        let mut widget = ChatWidget::new();

        let action = widget.handle_key_event(key(KeyCode::Char(':')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), ":");
    }

    #[test]
    fn normal_mode_unmapped_keys_still_reach_composer() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('a')));
        widget.handle_key_event(key(KeyCode::Char('b')));
        widget.handle_key_event(key(KeyCode::Char('c')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('x')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "ab");
    }

    #[test]
    fn command_w_submits_in_create_mode() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('w')));
        assert_eq!(action, WidgetAction::SubmitCreate("h".to_string()));
    }

    #[test]
    fn command_s_submits_in_create_mode() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('s')));
        assert_eq!(action, WidgetAction::SubmitCreate("h".to_string()));
    }

    #[test]
    fn command_s_submits_in_edit_mode() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "7")]);
        open_split_list(&mut widget);
        widget.handle_key_event(key(KeyCode::Tab));
        widget.handle_key_event(key(KeyCode::Enter));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('s')));
        assert_eq!(
            action,
            WidgetAction::SubmitEdit {
                memo_id: "memo-1".to_string(),
                expected_version: "7".to_string(),
                text: "memo-1".to_string(),
            }
        );
    }

    #[test]
    fn command_shift_s_is_not_submit_alias() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(shift_char('S'));
        assert_eq!(action, WidgetAction::None);
        assert!(!widget.wq_submission_in_progress());
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "h");
    }

    #[test]
    fn command_q_blocks_when_dirty() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('q')));

        assert_eq!(action, WidgetAction::None);
        assert_eq!(
            widget.status_message(),
            Some("Unsaved changes. Use w/s, W, or Q.")
        );
    }

    #[test]
    fn command_shift_q_force_quits_even_when_dirty() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(shift_char('Q'));

        assert_eq!(action, WidgetAction::Quit);
    }

    #[test]
    fn command_list_opens_list_page_and_enter_loads() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-2", "memo-2", "2"),
            memo("memo-1", "memo-1", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('l')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.page_mode(), PageMode::MemoList);

        widget.handle_key_event(key(KeyCode::Up));
        widget.handle_key_event(key(KeyCode::Enter));

        assert_eq!(widget.page_mode(), PageMode::Composer);
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "memo-1");
        assert!(widget.bottom_pane_mut().composer_mut().is_insert_mode());
    }

    #[test]
    fn memo_list_loading_flag_can_toggle() {
        let mut widget = ChatWidget::new();
        assert!(!widget.memo_list_loading());

        widget.set_memo_list_loading(true);
        assert!(widget.memo_list_loading());

        widget.set_memo_list_loading(false);
        assert!(!widget.memo_list_loading());
    }

    #[test]
    fn help_opens_in_composer_normal_mode() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(key(KeyCode::Char('?')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(
            widget.help_overlay(),
            Some(HelpOverlayContext::ComposerNormal)
        );
    }

    #[test]
    fn help_opens_in_memo_list_page() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        let action = widget.handle_key_event(key(KeyCode::Char('?')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.help_overlay(), Some(HelpOverlayContext::MemoList));
    }

    #[test]
    fn help_overlay_closes_on_question_esc_or_q() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        widget.handle_key_event(key(KeyCode::Char('?')));
        assert!(widget.help_overlay().is_some());

        widget.handle_key_event(key(KeyCode::Char('?')));
        assert_eq!(widget.help_overlay(), None);

        widget.handle_key_event(key(KeyCode::Char('?')));
        assert!(widget.help_overlay().is_some());
        widget.handle_key_event(key(KeyCode::Esc));
        assert_eq!(widget.help_overlay(), None);

        widget.handle_key_event(key(KeyCode::Char('?')));
        assert!(widget.help_overlay().is_some());
        widget.handle_key_event(key(KeyCode::Char('q')));
        assert_eq!(widget.help_overlay(), None);
    }

    #[test]
    fn help_overlay_blocks_commands_until_closed() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        widget.handle_key_event(key(KeyCode::Char('?')));
        assert_eq!(
            widget.help_overlay(),
            Some(HelpOverlayContext::ComposerNormal)
        );

        let blocked = widget.handle_key_event(key(KeyCode::Char('w')));
        assert_eq!(blocked, WidgetAction::None);
        assert!(!widget.wq_submission_in_progress());
        assert_eq!(
            widget.help_overlay(),
            Some(HelpOverlayContext::ComposerNormal)
        );

        widget.handle_key_event(key(KeyCode::Char('?')));
        let submit = widget.handle_key_event(key(KeyCode::Char('w')));
        assert_eq!(submit, WidgetAction::SubmitCreate("h".to_string()));
    }

    #[test]
    fn q_with_memo_list_help_open_closes_help_only() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);
        widget.handle_key_event(key(KeyCode::Char('?')));
        assert_eq!(widget.help_overlay(), Some(HelpOverlayContext::MemoList));

        let action = widget.handle_key_event(key(KeyCode::Char('q')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.help_overlay(), None);
        assert_eq!(widget.page_mode(), PageMode::MemoList);
    }

    #[test]
    fn enter_in_memo_list_without_selection_returns_to_insert_mode() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        assert!(!widget.bottom_pane_mut().composer_mut().is_insert_mode());

        open_memo_list(&mut widget);
        let action = widget.handle_key_event(key(KeyCode::Enter));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.page_mode(), PageMode::Composer);
        assert!(widget.bottom_pane_mut().composer_mut().is_insert_mode());
    }

    #[test]
    fn q_in_memo_list_returns_to_composer_page() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "1")]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);
        assert_eq!(widget.page_mode(), PageMode::MemoList);
        assert!(!widget.bottom_pane_mut().composer_mut().is_insert_mode());

        let action = widget.handle_key_event(key(KeyCode::Char('q')));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.page_mode(), PageMode::Composer);
        assert!(widget.bottom_pane_mut().composer_mut().is_insert_mode());
    }

    #[test]
    fn esc_in_memo_list_returns_to_composer_insert_mode() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);
        assert_eq!(widget.page_mode(), PageMode::MemoList);
        assert!(!widget.bottom_pane_mut().composer_mut().is_insert_mode());

        let action = widget.handle_key_event(key(KeyCode::Esc));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.page_mode(), PageMode::Composer);
        assert!(widget.bottom_pane_mut().composer_mut().is_insert_mode());
    }

    #[test]
    fn r_in_memo_list_emits_refresh_history() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        let action = widget.handle_key_event(key(KeyCode::Char('r')));

        assert_eq!(action, WidgetAction::RefreshHistory);
    }

    #[test]
    fn shift_r_in_memo_list_emits_refresh_history() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        let action = widget.handle_key_event(shift_char('R'));

        assert_eq!(action, WidgetAction::RefreshHistory);
    }

    #[test]
    fn y_in_memo_list_emits_copy_selected_memo() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "1")]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        let action = widget.handle_key_event(key(KeyCode::Char('y')));

        assert_eq!(action, WidgetAction::CopySelectedMemo);
    }

    #[test]
    fn slash_enters_memo_list_search_mode() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-1", "memo-1", "1"),
            memo("memo-2", "memo-2", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));

        assert!(widget.memo_list_search_mode());
        assert_eq!(widget.memo_list_query(), "");
        assert_eq!(
            widget.memo_list_visible_indices().len(),
            widget.history().len()
        );
    }

    #[test]
    fn memo_list_search_filters_case_insensitive_and_enter_keeps_filter() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-1", "Alpha", "1"),
            memo("memo-2", "beta", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));
        type_text(&mut widget, "ALP");

        assert_eq!(widget.memo_list_visible_indices().len(), 1);
        assert_eq!(
            widget
                .memo_list_selected_cell()
                .map(|cell| cell.full_text.as_str()),
            Some("Alpha")
        );

        widget.handle_key_event(key(KeyCode::Enter));

        assert!(!widget.memo_list_search_mode());
        assert_eq!(widget.memo_list_query(), "ALP");
        assert_eq!(widget.memo_list_visible_indices().len(), 1);
    }

    #[test]
    fn memo_list_search_backspace_and_esc_update_and_clear_filter() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-1", "dog", "1"),
            memo("memo-2", "dot", "1"),
            memo("memo-3", "cat", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));
        type_text(&mut widget, "dog");
        assert_eq!(widget.memo_list_visible_indices().len(), 1);

        widget.handle_key_event(key(KeyCode::Backspace));
        assert_eq!(widget.memo_list_query(), "do");
        assert_eq!(widget.memo_list_visible_indices().len(), 2);

        widget.handle_key_event(key(KeyCode::Esc));
        assert!(!widget.memo_list_search_mode());
        assert_eq!(widget.memo_list_query(), "");
        assert_eq!(
            widget.memo_list_visible_indices().len(),
            widget.history().len()
        );
    }

    #[test]
    fn memo_list_enter_loads_selected_from_active_filter() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-1", "dog", "1"),
            memo("memo-2", "cat", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));
        type_text(&mut widget, "dog");
        widget.handle_key_event(key(KeyCode::Enter));

        let action = widget.handle_key_event(key(KeyCode::Enter));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.page_mode(), PageMode::Composer);
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "dog");
    }

    #[test]
    fn memo_list_search_no_match_has_no_selected_cell() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "dog", "1")]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));
        type_text(&mut widget, "zzz");

        assert!(widget.memo_list_selected_cell().is_none());
        assert_eq!(widget.selected_memo_text(), None);
    }

    #[test]
    fn refresh_history_reapplies_active_memo_list_filter() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            memo("memo-1", "dog", "1"),
            memo("memo-2", "cat", "1"),
        ]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        widget.handle_key_event(key(KeyCode::Char('/')));
        type_text(&mut widget, "dog");
        widget.handle_key_event(key(KeyCode::Enter));
        assert_eq!(widget.memo_list_visible_indices().len(), 1);

        widget.refresh_history_from_memos(vec![
            memo("memo-1", "dog updated", "2"),
            memo("memo-3", "bird", "1"),
        ]);

        assert_eq!(widget.memo_list_query(), "dog");
        assert_eq!(widget.memo_list_visible_indices().len(), 1);
        assert_eq!(
            widget
                .memo_list_selected_cell()
                .map(|cell| cell.full_text.as_str()),
            Some("dog updated")
        );
    }

    #[test]
    fn ctrl_f_and_ctrl_b_in_memo_list_page_by_ten_rows() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(seed_memos(30));
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        for _ in 0..29 {
            widget.handle_key_event(key(KeyCode::Up));
        }
        assert_eq!(widget.selected_history(), Some(0));

        widget.handle_key_event(ctrl(KeyCode::Char('f')));
        assert_eq!(widget.selected_history(), Some(10));

        widget.handle_key_event(ctrl(KeyCode::Char('b')));
        assert_eq!(widget.selected_history(), Some(0));
    }

    #[test]
    fn pageup_and_pagedown_in_memo_list_page_by_ten_rows() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(seed_memos(30));
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        for _ in 0..29 {
            widget.handle_key_event(key(KeyCode::Up));
        }
        assert_eq!(widget.selected_history(), Some(0));

        widget.handle_key_event(key(KeyCode::PageDown));
        assert_eq!(widget.selected_history(), Some(10));

        widget.handle_key_event(key(KeyCode::PageUp));
        assert_eq!(widget.selected_history(), Some(0));
    }

    #[test]
    fn d_in_memo_list_starts_confirmation_and_enter_confirms_delete() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![memo("memo-1", "memo-1", "1")]);
        widget.handle_key_event(key(KeyCode::Esc));
        open_memo_list(&mut widget);

        let first = widget.handle_key_event(key(KeyCode::Char('d')));
        assert_eq!(first, WidgetAction::None);
        assert_eq!(widget.delete_confirmation_text(), Some("memo-1"));

        let second = widget.handle_key_event(key(KeyCode::Enter));
        assert_eq!(
            second,
            WidgetAction::DeleteMemo {
                memo_id: "memo-1".to_string(),
                expected_version: "1".to_string(),
            }
        );
    }

    #[test]
    fn command_splitlist_toggles_split_layout() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Esc));

        widget.handle_key_event(key(KeyCode::Char('p')));
        assert!(widget.split_list_open());

        widget.handle_key_event(key(KeyCode::Char('p')));
        assert!(!widget.split_list_open());
    }

    #[test]
    fn command_shift_w_emits_background_submit_action() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(key(KeyCode::Char('h')));
        widget.handle_key_event(key(KeyCode::Esc));

        let action = widget.handle_key_event(shift_char('W'));

        assert_eq!(
            action,
            WidgetAction::SubmitCreateBeforeQuit("h".to_string())
        );
        assert!(widget.wq_submission_in_progress());
    }

    #[test]
    fn wq_failure_prompt_can_cancel_or_quit() {
        let mut widget = ChatWidget::new();
        widget.on_wq_submission_failed("network error");
        assert!(widget.wq_failure_prompt().is_some());

        let cancel = widget.handle_key_event(key(KeyCode::Char('c')));
        assert_eq!(cancel, WidgetAction::None);
        assert!(widget.wq_failure_prompt().is_none());

        widget.on_wq_submission_failed("network error");
        let quit = widget.handle_key_event(key(KeyCode::Enter));
        assert_eq!(quit, WidgetAction::Quit);
    }

    #[test]
    fn esc_in_composer_insert_switches_mode_and_primes_quit_confirmation() {
        let mut widget = ChatWidget::new();

        let action = widget.handle_key_event(key(KeyCode::Esc));
        assert_eq!(action, WidgetAction::None);
        assert!(widget.quit_confirmation_pending());
        assert!(!widget.bottom_pane_mut().composer_mut().is_insert_mode());
    }

    #[test]
    fn esc_twice_from_composer_insert_quits() {
        let mut widget = ChatWidget::new();

        let first = widget.handle_key_event(key(KeyCode::Esc));
        assert_eq!(first, WidgetAction::None);
        assert!(widget.quit_confirmation_pending());

        let second = widget.handle_key_event(key(KeyCode::Esc));
        assert_eq!(second, WidgetAction::Quit);
    }

    fn history_texts(widget: &ChatWidget) -> Vec<String> {
        widget
            .history()
            .iter()
            .map(|cell| cell.full_text.clone())
            .collect()
    }

    fn memo(id: &str, text: &str, version: &str) -> RecentMemo {
        RecentMemo {
            id: id.to_string(),
            text: text.to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: version.to_string(),
            deleted_at: None,
        }
    }

    fn seed_memos(count: usize) -> Vec<RecentMemo> {
        (0..count)
            .map(|idx| RecentMemo {
                id: format!("memo-{idx}"),
                text: format!("memo-{idx}"),
                created_at: "2026-02-23T01:00:00Z".to_string(),
                version: "1".to_string(),
                deleted_at: None,
            })
            .collect()
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn shift_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    }

    fn open_memo_list(widget: &mut ChatWidget) {
        widget.handle_key_event(key(KeyCode::Char('l')));
    }

    fn open_split_list(widget: &mut ChatWidget) {
        widget.handle_key_event(key(KeyCode::Esc));
        widget.handle_key_event(key(KeyCode::Char('p')));
    }

    fn type_text(widget: &mut ChatWidget, text: &str) {
        for c in text.chars() {
            widget.handle_key_event(key(KeyCode::Char(c)));
        }
    }
}
