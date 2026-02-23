use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::cli::resolve_text;
use crate::error::AppError;
use crate::submission::submit_memo;
use crate::supabase::SupabaseClient;

use super::chat_widget::{ChatWidget, WidgetAction};
use super::render;
use super::theme::{UiTheme, build_ui_theme, detect_terminal_palette};

pub struct ComposeApp<'a> {
    client: &'a SupabaseClient,
    widget: ChatWidget,
    theme: UiTheme,
}

impl<'a> ComposeApp<'a> {
    pub fn new(client: &'a SupabaseClient) -> Self {
        let palette = detect_terminal_palette();
        let theme = build_ui_theme(palette);
        Self {
            client,
            widget: ChatWidget::new(),
            theme,
        }
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        let mut terminal = TerminalSession::new()?;

        loop {
            terminal.draw(&mut self.widget, &self.theme)?;

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
                WidgetAction::Submit(text) => {
                    self.widget.set_submitting();
                    terminal.draw(&mut self.widget, &self.theme)?;
                    self.handle_submit(text).await;
                }
            }
        }

        Ok(())
    }

    async fn handle_submit(&mut self, text: String) {
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
                );
            }
            Err(err) => {
                self.widget.on_submit_error(&normalized, &err.to_string());
            }
        }
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

    fn draw(&mut self, widget: &mut ChatWidget, theme: &UiTheme) -> Result<(), AppError> {
        self.terminal
            .draw(|frame| render::draw(frame, widget, theme))
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
