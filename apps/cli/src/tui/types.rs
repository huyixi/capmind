use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    History,
    Composer,
}

#[derive(Debug, Clone)]
pub struct HistoryCell {
    pub created_at: DateTime<Utc>,
    pub full_text: String,
    pub memo_id: Option<String>,
    pub memo_version: Option<String>,
}

impl HistoryCell {
    pub fn new(full_text: String) -> Self {
        Self {
            created_at: Utc::now(),
            full_text,
            memo_id: None,
            memo_version: None,
        }
    }

    pub fn with_memo(
        full_text: String,
        created_at: DateTime<Utc>,
        memo_id: String,
        memo_version: String,
    ) -> Self {
        Self {
            created_at,
            full_text,
            memo_id: Some(memo_id),
            memo_version: Some(memo_version),
        }
    }
}
