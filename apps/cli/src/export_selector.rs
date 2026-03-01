use std::io::{self, IsTerminal, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::error::AppError;
use crate::memo_export::ExportRangePreset;

const OPTIONS: [(ExportRangePreset, &str); 4] = [
    (ExportRangePreset::Last3Days, "Last 3 days"),
    (ExportRangePreset::Week, "Last week"),
    (ExportRangePreset::Month, "Last month"),
    (ExportRangePreset::All, "All memos"),
];

pub fn prompt_export_range() -> Result<ExportRangePreset, AppError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(AppError::InvalidInput(
            "Interactive export selection requires a terminal.".to_string(),
        ));
    }

    let mut stdout = io::stdout();
    let _guard = TerminalGuard::enter(&mut stdout)?;
    let mut state = SelectorState::new();

    loop {
        render(&mut stdout, &state)?;

        let event = event::read()
            .map_err(|err| AppError::InvalidInput(format!("Failed reading key event: {err}")))?;
        let Event::Key(key) = event else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => state.move_up(),
            KeyCode::Down | KeyCode::Char('j') => state.move_down(),
            KeyCode::Enter => return Ok(state.selected()),
            KeyCode::Esc | KeyCode::Char('q') => {
                return Err(AppError::InvalidInput("Export canceled.".to_string()));
            }
            _ => {}
        }
    }
}

fn render(stdout: &mut io::Stdout, state: &SelectorState) -> Result<(), AppError> {
    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))
        .map_err(|err| AppError::InvalidInput(format!("Failed rendering selector: {err}")))?;
    writeln!(stdout, "Select export range")
        .map_err(|err| AppError::InvalidInput(format!("Failed writing selector: {err}")))?;
    writeln!(stdout, "")
        .map_err(|err| AppError::InvalidInput(format!("Failed writing selector: {err}")))?;

    for (idx, (_, label)) in OPTIONS.iter().enumerate() {
        let marker = if idx == state.index { ">" } else { " " };
        writeln!(stdout, "{marker} {label}")
            .map_err(|err| AppError::InvalidInput(format!("Failed writing selector: {err}")))?;
    }

    writeln!(stdout, "")
        .map_err(|err| AppError::InvalidInput(format!("Failed writing selector: {err}")))?;
    writeln!(
        stdout,
        "Use Up/Down to choose, Enter to confirm, q/Esc to cancel."
    )
    .map_err(|err| AppError::InvalidInput(format!("Failed writing selector: {err}")))?;
    stdout
        .flush()
        .map_err(|err| AppError::InvalidInput(format!("Failed flushing selector output: {err}")))
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut io::Stdout) -> Result<Self, AppError> {
        enable_raw_mode()
            .map_err(|err| AppError::InvalidInput(format!("Failed enabling raw mode: {err}")))?;
        execute!(stdout, EnterAlternateScreen, Hide).map_err(|err| {
            AppError::InvalidInput(format!("Failed entering alternate screen: {err}"))
        })?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    }
}

#[derive(Debug)]
struct SelectorState {
    index: usize,
}

impl SelectorState {
    fn new() -> Self {
        Self { index: 0 }
    }

    fn move_up(&mut self) {
        if self.index == 0 {
            self.index = OPTIONS.len() - 1;
        } else {
            self.index -= 1;
        }
    }

    fn move_down(&mut self) {
        self.index = (self.index + 1) % OPTIONS.len();
    }

    fn selected(&self) -> ExportRangePreset {
        OPTIONS[self.index].0
    }
}

#[cfg(test)]
mod tests {
    use super::SelectorState;
    use crate::memo_export::ExportRangePreset;

    #[test]
    fn selector_state_wraps_up() {
        let mut state = SelectorState::new();
        state.move_up();
        assert_eq!(state.selected(), ExportRangePreset::All);
    }

    #[test]
    fn selector_state_wraps_down() {
        let mut state = SelectorState::new();
        for _ in 0..4 {
            state.move_down();
        }
        assert_eq!(state.selected(), ExportRangePreset::Last3Days);
    }
}
