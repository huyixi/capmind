use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::bottom_pane::{BottomPane, InputResult};
use super::types::{FocusArea, HistoryCell, HistoryKind, MAX_HISTORY_ITEMS};

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
    status_line: Option<String>,
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
            status_line: None,
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

    pub fn bottom_pane(&self) -> &BottomPane {
        &self.bottom_pane
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

    pub fn status_line(&self) -> Option<&str> {
        self.status_line.as_deref()
    }

    pub fn on_submit_success(&mut self, text: &str, memo_id: &str, created_at: &str) {
        self.push_history(HistoryKind::Submitted, text.to_string());
        self.status_line = Some(format!(
            "submitted memo_id={memo_id} created_at={created_at}"
        ));
        self.bottom_pane.composer_mut().clear();
        self.focus = FocusArea::Composer;
    }

    pub fn on_submit_error(&mut self, text: &str, message: &str) {
        let full_text = format!("{message}\n\n{text}");
        self.push_history(HistoryKind::Error, full_text);
        self.status_line = Some(format!("submit failed: {message}"));
    }

    pub fn on_validation_error(&mut self, message: &str) {
        self.push_history(HistoryKind::Error, message.to_string());
        self.status_line = Some(message.to_string());
    }

    pub fn set_submitting(&mut self) {
        self.status_line = Some("submitting...".to_string());
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
        self.push_history(HistoryKind::DraftLoaded, cell.full_text);
        self.status_line = Some("loaded draft into composer".to_string());
    }

    fn push_history(&mut self, kind: HistoryKind, full_text: String) {
        if self.history.len() >= MAX_HISTORY_ITEMS {
            self.history.pop_front();
            self.selected_history = self.selected_history.saturating_sub(1);
        }
        self.history.push_back(HistoryCell::new(kind, full_text));
        self.selected_history = self.history.len().saturating_sub(1);
    }
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
