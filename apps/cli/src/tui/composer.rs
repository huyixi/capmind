use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerAction {
    None,
    Submit,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Composer {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_y: u16,
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}

impl Composer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_y: 0,
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push(String::new());
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_y = 0;
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(ToString::to_string).collect()
        };
        self.cursor_row = self.lines.len().saturating_sub(1);
        self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
        self.scroll_y = 0;
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn scroll_y(&self) -> u16 {
        self.scroll_y
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn ensure_cursor_visible(&mut self, viewport_height: u16) {
        if viewport_height == 0 {
            return;
        }
        let row = self.cursor_row as u16;
        if row < self.scroll_y {
            self.scroll_y = row;
            return;
        }
        let bottom = self
            .scroll_y
            .saturating_add(viewport_height.saturating_sub(1));
        if row > bottom {
            self.scroll_y = row.saturating_sub(viewport_height.saturating_sub(1));
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> ComposerAction {
        if key_event.kind != KeyEventKind::Press {
            return ComposerAction::None;
        }

        match key_event {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'s') => {
                ComposerAction::Submit
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => ComposerAction::Submit,
            KeyEvent {
                code: KeyCode::Enter,
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::ALT) => ComposerAction::Submit,
            KeyEvent {
                code: KeyCode::Enter,
                modifiers,
                ..
            } if modifiers == KeyModifiers::SHIFT => ComposerAction::Submit,
            KeyEvent {
                code: KeyCode::Esc, ..
            } => ComposerAction::Cancel,
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                self.insert_newline();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.backspace();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => {
                self.move_left();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => {
                self.move_right();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Up, ..
            } => {
                self.move_up();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                self.move_down();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Home,
                ..
            } => {
                self.cursor_col = 0;
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::End, ..
            } => {
                self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if is_typing_modifier(modifiers) => {
                self.insert_char(c);
                ComposerAction::None
            }
            _ => ComposerAction::None,
        }
    }

    fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        let idx = byte_idx_for_char(line, self.cursor_col);
        line.insert(idx, c);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let line = &mut self.lines[self.cursor_row];
        let idx = byte_idx_for_char(line, self.cursor_col);
        let tail = line[idx..].to_string();
        line.truncate(idx);
        self.lines.insert(self.cursor_row + 1, tail);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let start = byte_idx_for_char(line, self.cursor_col - 1);
            let end = byte_idx_for_char(line, self.cursor_col);
            line.replace_range(start..end, "");
            self.cursor_col -= 1;
            return;
        }

        if self.cursor_row == 0 {
            return;
        }

        let current = self.lines.remove(self.cursor_row);
        self.cursor_row -= 1;
        let prev = &mut self.lines[self.cursor_row];
        let prev_len = line_len_chars(prev);
        prev.push_str(&current);
        self.cursor_col = prev_len;
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            return;
        }
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
        }
    }

    fn move_right(&mut self) {
        let line_len = line_len_chars(&self.lines[self.cursor_row]);
        if self.cursor_col < line_len {
            self.cursor_col += 1;
            return;
        }
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row == 0 {
            return;
        }
        self.cursor_row -= 1;
        self.cursor_col = self
            .cursor_col
            .min(line_len_chars(&self.lines[self.cursor_row]));
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 >= self.lines.len() {
            return;
        }
        self.cursor_row += 1;
        self.cursor_col = self
            .cursor_col
            .min(line_len_chars(&self.lines[self.cursor_row]));
    }
}

fn is_typing_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn line_len_chars(input: &str) -> usize {
    input.chars().count()
}

fn byte_idx_for_char(input: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    input
        .char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(input.len())
}

#[cfg(test)]
mod tests {
    use super::{Composer, ComposerAction};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn enter_inserts_new_line() {
        let mut composer = Composer::new();
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(composer.text(), "a\nb");
    }

    #[test]
    fn shift_enter_submits() {
        let mut composer = Composer::new();
        let action = composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn ctrl_s_submits() {
        let mut composer = Composer::new();
        let action =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn ctrl_enter_submits() {
        let mut composer = Composer::new();
        let action =
            composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn alt_enter_submits() {
        let mut composer = Composer::new();
        let action = composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn esc_cancels() {
        let mut composer = Composer::new();
        let action = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::Cancel);
    }
}
