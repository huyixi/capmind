use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::composer::{Composer, ComposerAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    None,
    Submitted(String),
    Cancelled,
    SwitchFocusToHistory,
}

#[derive(Debug, Default, Clone)]
pub struct BottomPane {
    composer: Composer,
}

impl BottomPane {
    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    pub fn composer_mut(&mut self) -> &mut Composer {
        &mut self.composer
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> InputResult {
        if key_event.kind != KeyEventKind::Press {
            return InputResult::None;
        }

        match key_event.code {
            KeyCode::Tab | KeyCode::BackTab => return InputResult::SwitchFocusToHistory,
            _ => {}
        }

        match self.composer.handle_key_event(key_event) {
            ComposerAction::None => InputResult::None,
            ComposerAction::Submit => InputResult::Submitted(self.composer.text()),
            ComposerAction::Cancel => InputResult::Cancelled,
        }
    }
}
