use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::supabase::RecentMemo;

use super::bottom_pane::{BottomPane, InputResult};
use super::types::{FocusArea, HistoryCell, MAX_HISTORY_ITEMS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetAction {
    None,
    Submit(String),
    Quit,
}

#[derive(Debug, Clone)]
pub struct ChatWidget {
    bottom_pane: BottomPane,
    history: VecDeque<HistoryCell>,
    selected_history: usize,
    focus: FocusArea,
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
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> WidgetAction {
        if key_event.kind != KeyEventKind::Press {
            return WidgetAction::None;
        }

        if is_ctrl_c(key_event) {
            return WidgetAction::Quit;
        }

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

    pub fn on_submit_success(&mut self, text: &str, _memo_id: &str, created_at: &str) {
        self.push_history_with_created_at(
            text.to_string(),
            parse_timestamp(created_at).unwrap_or_else(Utc::now),
        );
        self.bottom_pane.composer_mut().clear();
        self.focus = FocusArea::Composer;
    }

    pub fn on_submit_error(&mut self, text: &str, message: &str) {
        let full_text = format!("{message}\n\n{text}");
        self.push_history(full_text);
    }

    pub fn on_validation_error(&mut self, message: &str) {
        self.push_history(message.to_string());
    }

    pub fn hydrate_history_from_memos(&mut self, memos: Vec<RecentMemo>) {
        if !self.history.is_empty() {
            return;
        }

        for memo in memos.into_iter().rev() {
            let text = memo.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            self.push_history_with_created_at(
                text,
                parse_timestamp(&memo.created_at).unwrap_or_else(Utc::now),
            );
        }
    }

    fn handle_composer_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match self.bottom_pane.handle_key_event(key_event) {
            InputResult::None => WidgetAction::None,
            InputResult::Submitted(text) => WidgetAction::Submit(text),
            InputResult::Cancelled => WidgetAction::Quit,
            InputResult::SwitchFocusToHistory => {
                self.focus = FocusArea::History;
                WidgetAction::None
            }
        }
    }

    fn handle_history_key(&mut self, key_event: KeyEvent) -> WidgetAction {
        match key_event.code {
            KeyCode::Esc => WidgetAction::Quit,
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
            _ => WidgetAction::None,
        }
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
        self.push_history(cell.full_text);
    }

    fn push_history(&mut self, full_text: String) {
        self.push_history_cell(HistoryCell::new(full_text));
    }

    fn push_history_with_created_at(&mut self, full_text: String, created_at: DateTime<Utc>) {
        self.push_history_cell(HistoryCell::with_created_at(full_text, created_at));
    }

    fn push_history_cell(&mut self, cell: HistoryCell) {
        if self.history.len() >= MAX_HISTORY_ITEMS {
            self.history.pop_front();
            self.selected_history = self.selected_history.saturating_sub(1);
        }
        self.history.push_back(cell);
        self.selected_history = self.history.len().saturating_sub(1);
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

#[cfg(test)]
mod tests {
    use super::ChatWidget;
    use crate::supabase::RecentMemo;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn push_history_keeps_only_latest_three() {
        let mut widget = ChatWidget::new();
        widget.on_validation_error("one");
        widget.on_submit_success("two", "memo_2", "2026-02-23T00:00:00Z");
        widget.on_submit_error("three", "err-3");
        widget.on_validation_error("four");

        assert_eq!(widget.history().len(), 3);
        assert_eq!(
            history_texts(&widget),
            vec![
                "two".to_string(),
                "err-3\n\nthree".to_string(),
                "four".to_string()
            ]
        );
    }

    #[test]
    fn history_eviction_keeps_latest_entries_ordered() {
        let mut widget = ChatWidget::new();
        widget.on_validation_error("entry-1");
        widget.on_validation_error("entry-2");
        widget.on_validation_error("entry-3");
        widget.on_validation_error("entry-4");
        widget.on_validation_error("entry-5");

        assert_eq!(widget.history().len(), 3);
        assert_eq!(
            history_texts(&widget),
            vec![
                "entry-3".to_string(),
                "entry-4".to_string(),
                "entry-5".to_string()
            ]
        );
    }

    #[test]
    fn selected_history_remains_valid_after_eviction() {
        let mut widget = ChatWidget::new();
        widget.on_validation_error("a");
        widget.on_validation_error("b");
        widget.on_validation_error("c");

        widget.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        widget.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        widget.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(widget.selected_history(), Some(0));

        widget.on_validation_error("d");
        assert_eq!(widget.history().len(), 3);
        assert_eq!(widget.selected_history(), Some(2));
    }

    #[test]
    fn hydrate_history_from_memos_uses_only_memos_and_respects_order() {
        let mut widget = ChatWidget::new();
        widget.hydrate_history_from_memos(vec![
            RecentMemo {
                text: "memo-newest".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
            },
            RecentMemo {
                text: "memo-middle".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
            },
            RecentMemo {
                text: "memo-oldest".to_string(),
                created_at: "2026-02-23T01:00:00Z".to_string(),
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
                text: "memo-newest".to_string(),
                created_at: "2026-02-23T03:00:00Z".to_string(),
            },
            RecentMemo {
                text: "memo-middle".to_string(),
                created_at: "2026-02-23T02:00:00Z".to_string(),
            },
            RecentMemo {
                text: "memo-oldest".to_string(),
                created_at: "2026-02-23T01:00:00Z".to_string(),
            },
        ]);

        assert_eq!(history_texts(&widget), vec!["local-entry".to_string()]);
    }

    fn history_texts(widget: &ChatWidget) -> Vec<String> {
        widget
            .history()
            .iter()
            .map(|cell| cell.full_text.clone())
            .collect()
    }
}
