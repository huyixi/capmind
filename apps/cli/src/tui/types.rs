use chrono::{DateTime, Utc};

pub const MAX_HISTORY_ITEMS: usize = 200;
const TITLE_MAX_CHARS: usize = 48;
const PREVIEW_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    DraftLoaded,
    Submitted,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    History,
    Composer,
}

#[derive(Debug, Clone)]
pub struct HistoryCell {
    pub created_at: DateTime<Utc>,
    pub kind: HistoryKind,
    pub title: String,
    pub body_preview: String,
    pub full_text: String,
}

impl HistoryCell {
    pub fn new(kind: HistoryKind, full_text: String) -> Self {
        let title = first_non_empty_line(&full_text)
            .map(|line| truncate(line, TITLE_MAX_CHARS))
            .unwrap_or_else(|| "(empty)".to_string());
        let body_preview = truncate(&full_text.replace('\n', " "), PREVIEW_MAX_CHARS);

        Self {
            created_at: Utc::now(),
            kind,
            title,
            body_preview,
            full_text,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            HistoryKind::DraftLoaded => "loaded",
            HistoryKind::Submitted => "submitted",
            HistoryKind::Error => "error",
        }
    }
}

fn first_non_empty_line(input: &str) -> Option<&str> {
    input.lines().map(str::trim).find(|line| !line.is_empty())
}

fn truncate(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let out: String = input.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{out}...")
}
