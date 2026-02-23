use chrono::{DateTime, Utc};

pub const MAX_HISTORY_ITEMS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    History,
    Composer,
}

#[derive(Debug, Clone)]
pub struct HistoryCell {
    pub created_at: DateTime<Utc>,
    pub full_text: String,
}

impl HistoryCell {
    pub fn new(full_text: String) -> Self {
        Self {
            created_at: Utc::now(),
            full_text,
        }
    }

    pub fn with_created_at(full_text: String, created_at: DateTime<Utc>) -> Self {
        Self {
            created_at,
            full_text,
        }
    }
}
