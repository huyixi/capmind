use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerAction {
    None,
    Submit,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Insert,
    Normal,
}

#[derive(Debug, Clone)]
pub struct Composer {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_y: u16,
    mode: VimMode,
    pending_delete_line: bool,
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
            mode: VimMode::Insert,
            pending_delete_line: false,
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.lines.push(String::new());
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_y = 0;
        self.mode = VimMode::Insert;
        self.pending_delete_line = false;
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
        self.mode = VimMode::Insert;
        self.pending_delete_line = false;
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    #[cfg(test)]
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    pub fn cursor_display_col(&self) -> usize {
        let line = &self.lines[self.cursor_row];
        display_col_for_char_idx(line, self.cursor_col)
    }

    pub fn scroll_y(&self) -> u16 {
        self.scroll_y
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn vim_mode(&self) -> VimMode {
        self.mode
    }

    pub fn is_insert_mode(&self) -> bool {
        self.mode == VimMode::Insert
    }

    pub fn switch_to_insert_mode(&mut self) {
        self.mode = VimMode::Insert;
        self.pending_delete_line = false;
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

        if is_submit_shortcut(key_event) {
            return ComposerAction::Submit;
        }

        match self.mode {
            VimMode::Insert => self.handle_insert_mode_key(key_event),
            VimMode::Normal => self.handle_normal_mode_key(key_event),
        }
    }

    fn handle_insert_mode_key(&mut self, key_event: KeyEvent) -> ComposerAction {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                self.exit_insert_mode();
                ComposerAction::None
            }
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

    fn handle_normal_mode_key(&mut self, key_event: KeyEvent) -> ComposerAction {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                self.pending_delete_line = false;
                ComposerAction::Cancel
            }
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => {
                self.pending_delete_line = false;
                self.move_left();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => {
                self.pending_delete_line = false;
                self.move_right();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Up, ..
            } => {
                self.pending_delete_line = false;
                self.move_up();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                self.pending_delete_line = false;
                self.move_down();
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Home,
                ..
            } => {
                self.pending_delete_line = false;
                self.cursor_col = 0;
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::End, ..
            } => {
                self.pending_delete_line = false;
                self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
                ComposerAction::None
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if is_normal_command_modifier(modifiers) => {
                self.handle_normal_mode_char(c);
                ComposerAction::None
            }
            _ => {
                self.pending_delete_line = false;
                ComposerAction::None
            }
        }
    }

    fn handle_normal_mode_char(&mut self, c: char) {
        if self.pending_delete_line {
            self.pending_delete_line = false;
            if c == 'd' {
                self.delete_current_line();
                return;
            }
        }

        match c {
            'h' | 'H' => self.move_left(),
            'j' | 'J' => self.move_down(),
            'k' | 'K' => self.move_up(),
            'l' | 'L' => self.move_right(),
            'd' => self.pending_delete_line = true,
            '0' => self.cursor_col = 0,
            '$' => self.cursor_col = line_len_chars(&self.lines[self.cursor_row]),
            'w' | 'W' => self.move_word_forward(),
            'b' | 'B' => self.move_word_backward(),
            'x' | 'X' => self.delete_char_under_cursor(),
            'i' => self.switch_to_insert(),
            'a' => {
                self.append_after_cursor();
                self.switch_to_insert();
            }
            'I' => {
                self.cursor_col = 0;
                self.switch_to_insert();
            }
            'A' => {
                self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
                self.switch_to_insert();
            }
            'o' => {
                self.open_line_below();
                self.switch_to_insert();
            }
            'O' => {
                self.open_line_above();
                self.switch_to_insert();
            }
            _ => {}
        }
    }

    fn switch_to_insert(&mut self) {
        self.mode = VimMode::Insert;
        self.pending_delete_line = false;
    }

    fn exit_insert_mode(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
        self.mode = VimMode::Normal;
        self.pending_delete_line = false;
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

    fn append_after_cursor(&mut self) {
        let line_len = line_len_chars(&self.lines[self.cursor_row]);
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        }
    }

    fn move_word_forward(&mut self) {
        loop {
            let line = &self.lines[self.cursor_row];
            let chars: Vec<char> = line.chars().collect();
            let len = chars.len();

            if self.cursor_col >= len {
                if self.cursor_row + 1 >= self.lines.len() {
                    self.cursor_col = len;
                    return;
                }
                self.cursor_row += 1;
                self.cursor_col = 0;
                continue;
            }

            let mut idx = self.cursor_col;
            if idx < len && !chars[idx].is_whitespace() {
                while idx < len && !chars[idx].is_whitespace() {
                    idx += 1;
                }
            }
            while idx < len && chars[idx].is_whitespace() {
                idx += 1;
            }

            if idx < len {
                self.cursor_col = idx;
                return;
            }

            if self.cursor_row + 1 >= self.lines.len() {
                self.cursor_col = len;
                return;
            }

            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_word_backward(&mut self) {
        if self.cursor_col == 0 {
            if self.cursor_row == 0 {
                return;
            }
            self.cursor_row -= 1;
            self.cursor_col = line_len_chars(&self.lines[self.cursor_row]);
        }

        let line = &self.lines[self.cursor_row];
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            self.cursor_col = 0;
            return;
        }

        let mut idx = self.cursor_col.min(chars.len()).saturating_sub(1);
        while idx > 0 && chars[idx].is_whitespace() {
            idx -= 1;
        }
        while idx > 0 && !chars[idx - 1].is_whitespace() {
            idx -= 1;
        }
        self.cursor_col = idx;
    }

    fn delete_char_under_cursor(&mut self) {
        let line = &mut self.lines[self.cursor_row];
        let line_len = line_len_chars(line);
        if self.cursor_col >= line_len {
            return;
        }
        let start = byte_idx_for_char(line, self.cursor_col);
        let end = byte_idx_for_char(line, self.cursor_col + 1);
        line.replace_range(start..end, "");
        if self.cursor_col > 0 {
            let new_len = line_len_chars(line);
            self.cursor_col = self.cursor_col.min(new_len.saturating_sub(1));
        }
    }

    fn open_line_below(&mut self) {
        self.lines.insert(self.cursor_row + 1, String::new());
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn open_line_above(&mut self) {
        self.lines.insert(self.cursor_row, String::new());
        self.cursor_col = 0;
    }

    fn delete_current_line(&mut self) {
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor_row = 0;
            self.cursor_col = 0;
            return;
        }

        self.lines.remove(self.cursor_row);
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len().saturating_sub(1);
        }
        let line_len = line_len_chars(&self.lines[self.cursor_row]);
        self.cursor_col = self.cursor_col.min(line_len);
    }
}

fn is_typing_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn is_normal_command_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn is_submit_shortcut(key_event: KeyEvent) -> bool {
    matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL) && c.eq_ignore_ascii_case(&'s')
    ) || matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Enter,
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL)
    ) || matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Enter,
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::ALT)
    ) || matches!(
        key_event,
        KeyEvent {
            code: KeyCode::Enter,
            modifiers,
            ..
        } if modifiers == KeyModifiers::SHIFT
    )
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

fn display_col_for_char_idx(input: &str, char_idx: usize) -> usize {
    let byte_idx = byte_idx_for_char(input, char_idx);
    UnicodeWidthStr::width(&input[..byte_idx])
}

#[cfg(test)]
mod tests {
    use super::{Composer, ComposerAction, VimMode, display_col_for_char_idx};
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
    fn esc_in_insert_switches_to_normal() {
        let mut composer = Composer::new();
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        let action = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::None);
        assert_eq!(composer.vim_mode(), VimMode::Normal);
        assert_eq!(composer.cursor_col(), 0);
    }

    #[test]
    fn esc_in_normal_cancels() {
        let mut composer = Composer::new();
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let action = composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(action, ComposerAction::Cancel);
    }

    #[test]
    fn display_col_handles_wide_chars() {
        assert_eq!(display_col_for_char_idx("ab中c", 0), 0);
        assert_eq!(display_col_for_char_idx("ab中c", 1), 1);
        assert_eq!(display_col_for_char_idx("ab中c", 2), 2);
        assert_eq!(display_col_for_char_idx("ab中c", 3), 4);
        assert_eq!(display_col_for_char_idx("ab中c", 4), 5);
    }

    #[test]
    fn cursor_display_col_tracks_wide_chars() {
        let mut composer = Composer::new();
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('中'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(composer.cursor_col(), 3);
        assert_eq!(composer.cursor_display_col(), 4);
    }

    #[test]
    fn normal_mode_char_does_not_insert_text() {
        let mut composer = Composer::new();
        composer.set_text("abc");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(composer.text(), "abc");
    }

    #[test]
    fn normal_mode_navigation_keys_move_cursor() {
        let mut composer = Composer::new();
        composer.set_text("abc def");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(composer.cursor_col(), 6);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE));
        assert_eq!(composer.cursor_col(), 0);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        assert_eq!(composer.cursor_col(), 4);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(composer.cursor_col(), 0);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::SHIFT));
        assert_eq!(composer.cursor_col(), 7);
    }

    #[test]
    fn normal_mode_insert_commands_switch_back_to_insert() {
        let mut composer = Composer::new();
        composer.set_text("abc");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert!(composer.is_insert_mode());
        assert_eq!(composer.cursor_col(), 3);
    }

    #[test]
    fn normal_mode_x_deletes_char_under_cursor() {
        let mut composer = Composer::new();
        composer.set_text("abc");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(composer.text(), "bc");
    }

    #[test]
    fn submit_shortcuts_work_in_normal_mode() {
        let mut composer = Composer::new();
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let action =
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(action, ComposerAction::Submit);
    }

    #[test]
    fn dd_deletes_current_line() {
        let mut composer = Composer::new();
        composer.set_text("one\ntwo\nthree");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(composer.text(), "one\nthree");
    }

    #[test]
    fn dd_on_single_line_clears_text() {
        let mut composer = Composer::new();
        composer.set_text("abc");
        composer.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(composer.text(), "");
        assert_eq!(composer.cursor_row(), 0);
        assert_eq!(composer.cursor_col(), 0);
    }
}
