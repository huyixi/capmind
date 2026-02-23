use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::supabase::{InsertedMemo, RecentMemo};

use super::bottom_pane::{BottomPane, InputResult};
use super::types::{FocusArea, HistoryCell, MAX_HISTORY_ITEMS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetAction {
    None,
    SubmitCreate(String),
    SubmitEdit {
        memo_id: String,
        expected_version: String,
        text: String,
    },
    RefreshHistory,
    DeleteMemo {
        memo_id: String,
        expected_version: String,
    },
    Quit,
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
    focus: FocusArea,
    pending_delete: Option<PendingDelete>,
    composer_mode: ComposerMode,
    quit_confirmation_pending: bool,
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
            focus: FocusArea::Composer,
            pending_delete: None,
            composer_mode: ComposerMode::Create,
            quit_confirmation_pending: false,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> WidgetAction {
        if key_event.kind != KeyEventKind::Press {
            return WidgetAction::None;
        }

        if is_ctrl_c(key_event) {
            return WidgetAction::Quit;
        }

        if self.focus == FocusArea::History && is_quit_key(key_event) {
            return WidgetAction::Quit;
        }

        if self.pending_delete.is_some() {
            self.quit_confirmation_pending = false;
            return self.handle_delete_confirmation_key(key_event);
        }

        if key_event.code == KeyCode::Esc {
            if self.quit_confirmation_pending {
                return WidgetAction::Quit;
            }
            self.quit_confirmation_pending = true;
            return WidgetAction::None;
        }

        self.quit_confirmation_pending = false;

        match self.focus {
            FocusArea::History => self.handle_history_key(key_event),
            FocusArea::Composer => self.handle_composer_key(key_event),
        }
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

    pub fn on_submit_success(
        &mut self,
        text: &str,
        memo_id: &str,
        created_at: &str,
        version: &str,
    ) {
        self.push_history_memo(
            text.to_string(),
            parse_timestamp(created_at).unwrap_or_else(Utc::now),
            memo_id.to_string(),
            version.to_string(),
        );
        self.reset_composer_state();
    }

    pub fn on_edit_success(&mut self, updated_memo: &RecentMemo) {
        self.upsert_history_memo(updated_memo);
        self.reset_composer_state();
    }

    pub fn on_edit_conflict(
        &mut self,
        server_memo: &RecentMemo,
        submitted_text: &str,
        forked: &InsertedMemo,
    ) {
        self.upsert_history_memo(server_memo);
        self.push_history_memo(
            submitted_text.to_string(),
            parse_timestamp(&forked.created_at).unwrap_or_else(Utc::now),
            forked.id.clone(),
            forked.version.clone(),
        );
        self.reset_composer_state();
    }

    pub fn on_submit_error(&mut self, text: &str, message: &str) {
        let full_text = format!("{message}\n\n{text}");
        self.push_history(full_text);
    }

    pub fn on_validation_error(&mut self, message: &str) {
        self.push_history(message.to_string());
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
        }

        if self.history.is_empty() {
            self.selected_history = 0;
            self.focus = FocusArea::Composer;
            return;
        }

        if position < self.selected_history {
            self.selected_history = self.selected_history.saturating_sub(1);
        }
        if self.selected_history >= self.history.len() {
            self.selected_history = self.history.len().saturating_sub(1);
        }
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

    pub fn hydrate_history_from_memos(&mut self, memos: Vec<RecentMemo>) {
        if !self.history.is_empty() {
            return;
        }

        for memo in memos.into_iter().rev() {
            if memo.deleted_at.is_some() {
                continue;
            }
            let text = memo.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            self.push_history_memo(
                text,
                parse_timestamp(&memo.created_at).unwrap_or_else(Utc::now),
                memo.id,
                memo.version,
            );
        }
    }

    pub fn refresh_history_from_memos(&mut self, memos: Vec<RecentMemo>) {
        let selected_memo_id = self
            .selected_history()
            .and_then(|index| self.history.get(index))
            .and_then(|cell| cell.memo_id.clone());

        self.history.clear();
        self.selected_history = 0;

        for memo in memos.into_iter().rev() {
            if memo.deleted_at.is_some() {
                continue;
            }
            let text = memo.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            self.push_history_memo(
                text,
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
    }

    fn handle_composer_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match self.bottom_pane.handle_key_event(key_event) {
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
                self.focus = FocusArea::History;
                WidgetAction::None
            }
        }
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
            KeyCode::Down => {
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
        let index = match self.selected_history() {
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

    fn load_selected_history_to_composer(&mut self) {
        let index = match self.selected_history() {
            Some(v) => v,
            None => return,
        };
        let Some(cell) = self.history.get(index).cloned() else {
            return;
        };
        self.bottom_pane.composer_mut().set_text(&cell.full_text);
        self.focus = FocusArea::Composer;
        self.composer_mode = match (cell.memo_id, cell.memo_version) {
            (Some(memo_id), Some(expected_version)) => ComposerMode::Edit {
                memo_id,
                expected_version,
            },
            _ => ComposerMode::Create,
        };
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
        if self.history.len() >= MAX_HISTORY_ITEMS {
            self.history.pop_front();
            self.selected_history = self.selected_history.saturating_sub(1);
        }
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
            return;
        }

        self.push_history_memo(
            memo.text.clone(),
            parsed_created_at,
            memo.id.clone(),
            memo.version.clone(),
        );
    }

    fn reset_composer_state(&mut self) {
        self.bottom_pane.composer_mut().clear();
        self.focus = FocusArea::Composer;
        self.composer_mode = ComposerMode::Create;
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
        } if (modifiers.is_empty() || modifiers == KeyModifiers::SHIFT) && c.eq_ignore_ascii_case(&'q')
    )
}

#[cfg(test)]
mod tests {
    use super::{ChatWidget, WidgetAction};
    use crate::supabase::{InsertedMemo, RecentMemo};
    use crate::tui::types::MAX_HISTORY_ITEMS;
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
    fn history_eviction_keeps_latest_entries_ordered() {
        let mut widget = ChatWidget::new();
        for idx in 0..(MAX_HISTORY_ITEMS + 2) {
            widget.on_validation_error(&format!("entry-{idx}"));
        }

        let expected_last = format!("entry-{}", MAX_HISTORY_ITEMS + 1);
        assert_eq!(widget.history().len(), MAX_HISTORY_ITEMS);
        assert_eq!(
            widget.history().front().map(|cell| cell.full_text.as_str()),
            Some("entry-2")
        );
        assert_eq!(
            widget.history().back().map(|cell| cell.full_text.as_str()),
            Some(expected_last.as_str())
        );
    }

    #[test]
    fn hydrate_history_from_memos_uses_only_memos_and_respects_order() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            RecentMemo {
                id: "memo-newest".to_string(),
                text: "memo-newest".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
                version: "3".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-middle".to_string(),
                text: "memo-middle".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
                version: "2".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-oldest".to_string(),
                text: "memo-oldest".to_string(),
                created_at: "2026-02-23T01:00:00Z".to_string(),
                version: "1".to_string(),
                deleted_at: None,
            },
        ]);

        assert_eq!(
            history_texts(&widget),
            vec![
                "memo-oldest".to_string(),
                "memo-middle".to_string(),
                "memo-newest".to_string()
            ]
        );
        assert_eq!(widget.selected_history(), Some(2));
    }

    #[test]
    fn hydrate_history_from_memos_does_not_override_existing_history() {
        let mut widget = ChatWidget::new();
        widget.on_validation_error("local-entry");
        widget.hydrate_history_from_memos(vec![
            RecentMemo {
                id: "memo-newest".to_string(),
                text: "memo-newest".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
                version: "3".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-middle".to_string(),
                text: "memo-middle".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
                version: "2".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-oldest".to_string(),
                text: "memo-oldest".to_string(),
                created_at: "2026-02-23T01:00:00Z".to_string(),
                version: "1".to_string(),
                deleted_at: None,
            },
        ]);

        assert_eq!(history_texts(&widget), vec!["local-entry".to_string()]);
    }

    #[test]
    fn hydrate_history_from_memos_skips_deleted_items() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            RecentMemo {
                id: "memo-newest-deleted".to_string(),
                text: "memo-newest-deleted".to_string(),
                created_at: "2026-02-23T04:00:00Z".to_string(),
                version: "4".to_string(),
                deleted_at: Some("2026-02-23T04:30:00Z".to_string()),
            },
            RecentMemo {
                id: "memo-middle".to_string(),
                text: "memo-middle".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
                version: "3".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-oldest".to_string(),
                text: "memo-oldest".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
                version: "2".to_string(),
                deleted_at: None,
            },
        ]);

        assert_eq!(
            history_texts(&widget),
            vec!["memo-oldest".to_string(), "memo-middle".to_string()]
        );
    }

    #[test]
    fn history_enter_loads_edit_mode_without_duplicating_entry() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "7".to_string(),
            deleted_at: None,
        }]);

        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.history().len(), 1);
        assert!(widget.is_editing_memo());
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "memo-1");
    }

    #[test]
    fn submit_in_edit_mode_emits_submit_edit_action() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "7".to_string(),
            deleted_at: None,
        }]);
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(action, WidgetAction::None);

        let submit =
            widget.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
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
    fn on_edit_success_updates_existing_history_and_resets_mode() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "7".to_string(),
            deleted_at: None,
        }]);
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        widget.on_edit_success(&RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1 edited".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "8".to_string(),
            deleted_at: None,
        });

        assert_eq!(widget.history().len(), 1);
        assert_eq!(history_texts(&widget), vec!["memo-1 edited".to_string()]);
        assert!(!widget.is_editing_memo());
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "");
    }

    #[test]
    fn on_edit_conflict_keeps_server_and_adds_fork() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "7".to_string(),
            deleted_at: None,
        }]);

        widget.on_edit_conflict(
            &RecentMemo {
                id: "memo-1".to_string(),
                text: "server-latest".to_string(),
                created_at: "2026-02-23T01:00:00Z".to_string(),
                version: "9".to_string(),
                deleted_at: None,
            },
            "my edited text",
            &InsertedMemo {
                id: "memo-fork".to_string(),
                created_at: "2026-02-23T10:00:00Z".to_string(),
                version: "1".to_string(),
            },
        );

        assert_eq!(widget.history().len(), 2);
        assert_eq!(
            history_texts(&widget),
            vec!["server-latest".to_string(), "my edited text".to_string()]
        );
        assert!(!widget.is_editing_memo());
    }

    #[test]
    fn esc_requires_confirmation_before_quit() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let first = widget.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(first, WidgetAction::None);
        assert!(widget.quit_confirmation_pending());

        let second = widget.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(second, WidgetAction::Quit);
    }

    #[test]
    fn esc_confirmation_resets_after_other_key() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let first = widget.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(first, WidgetAction::None);
        assert!(widget.quit_confirmation_pending());

        let down = widget.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(down, WidgetAction::None);
        assert!(!widget.quit_confirmation_pending());
    }

    #[test]
    fn q_quits_in_history() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(action, WidgetAction::Quit);
    }

    #[test]
    fn q_in_composer_keeps_typing() {
        let mut widget = ChatWidget::new();

        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.bottom_pane_mut().composer_mut().text(), "q");
    }

    #[test]
    fn r_in_history_emits_refresh_history_action() {
        let mut widget = ChatWidget::new();
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(action, WidgetAction::RefreshHistory);
    }

    #[test]
    fn d_starts_confirmation_and_enter_confirms_delete() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "1".to_string(),
            deleted_at: None,
        }]);
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        let first = widget.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(first, WidgetAction::None);
        assert_eq!(widget.delete_confirmation_text(), Some("memo-1"));

        let second = widget.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            second,
            WidgetAction::DeleteMemo {
                memo_id: "memo-1".to_string(),
                expected_version: "1".to_string(),
            }
        );
        assert_eq!(widget.delete_confirmation_text(), None);
    }

    #[test]
    fn delete_confirmation_esc_cancels() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "1".to_string(),
            deleted_at: None,
        }]);
        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        widget.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(widget.delete_confirmation_text(), Some("memo-1"));

        let action = widget.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, WidgetAction::None);
        assert_eq!(widget.delete_confirmation_text(), None);
        assert!(!widget.quit_confirmation_pending());
    }

    #[test]
    fn on_delete_conflict_updates_with_server_memo_if_not_deleted() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "1".to_string(),
            deleted_at: None,
        }]);

        widget.on_delete_conflict(&RecentMemo {
            id: "memo-1".to_string(),
            text: "server-updated".to_string(),
            created_at: "2026-02-23T02:00:00Z".to_string(),
            version: "2".to_string(),
            deleted_at: None,
        });

        assert_eq!(history_texts(&widget), vec!["server-updated".to_string()]);
    }

    #[test]
    fn on_delete_conflict_removes_when_server_already_deleted() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T01:00:00Z".to_string(),
            version: "1".to_string(),
            deleted_at: None,
        }]);

        widget.on_delete_conflict(&RecentMemo {
            id: "memo-1".to_string(),
            text: "memo-1".to_string(),
            created_at: "2026-02-23T02:00:00Z".to_string(),
            version: "2".to_string(),
            deleted_at: Some("2026-02-23T02:00:00Z".to_string()),
        });

        assert!(widget.history().is_empty());
    }

    #[test]
    fn refresh_history_from_memos_replaces_existing_entries() {
        let mut widget = ChatWidget::new();
        widget.on_submit_success("memo-1", "memo-1", "2026-02-23T01:00:00Z", "1");
        widget.on_validation_error("local-entry");

        widget.refresh_history_from_memos(vec![
            RecentMemo {
                id: "memo-3".to_string(),
                text: "memo-3".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
                version: "3".to_string(),
                deleted_at: None,
            },
            RecentMemo {
                id: "memo-2".to_string(),
                text: "memo-2".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
                version: "2".to_string(),
                deleted_at: None,
            },
        ]);

        assert_eq!(
            history_texts(&widget),
            vec!["memo-2".to_string(), "memo-3".to_string()]
        );
    }

    fn history_texts(widget: &ChatWidget) -> Vec<String> {
        widget
            .history()
            .iter()
            .map(|cell| cell.full_text.clone())
            .collect()
    }
}
