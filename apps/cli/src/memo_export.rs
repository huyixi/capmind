use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Local, Utc};

use crate::error::AppError;
use crate::supabase::RecentMemo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportRangePreset {
    Last3Days,
    Week,
    Month,
    All,
}

impl ExportRangePreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Last3Days => "last 3 days",
            Self::Week => "last week",
            Self::Month => "last month",
            Self::All => "all memos",
        }
    }
}

#[derive(Debug)]
pub struct ExportDateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub struct MemoExportPayload {
    pub text: String,
    pub memo_count: usize,
}

pub fn date_range_for_preset(preset: ExportRangePreset, now_utc: DateTime<Utc>) -> ExportDateRange {
    match preset {
        ExportRangePreset::Last3Days => ExportDateRange {
            from: Some(now_utc - Duration::days(3)),
            to: Some(now_utc),
        },
        ExportRangePreset::Week => ExportDateRange {
            from: Some(now_utc - Duration::days(7)),
            to: Some(now_utc),
        },
        ExportRangePreset::Month => ExportDateRange {
            from: Some(now_utc - Duration::days(30)),
            to: Some(now_utc),
        },
        ExportRangePreset::All => ExportDateRange {
            from: None,
            to: None,
        },
    }
}

pub fn build_export_payload(
    memos: &[RecentMemo],
    date_range: &ExportDateRange,
) -> MemoExportPayload {
    let mut texts = Vec::new();

    for memo in memos {
        if memo.deleted_at.is_some() {
            continue;
        }
        if memo.text.trim().is_empty() {
            continue;
        }
        if !matches_date_range(memo, date_range) {
            continue;
        }
        texts.push(memo.text.clone());
    }

    MemoExportPayload {
        text: texts.join("\n\n"),
        memo_count: texts.len(),
    }
}

pub fn next_export_file_path(cwd: &Path, now_local: DateTime<Local>) -> Result<PathBuf, AppError> {
    if !cwd.is_dir() {
        return Err(AppError::Api(format!(
            "Current directory is not a folder: {}",
            display(cwd)
        )));
    }

    let base_name = format!("capmind-{}", now_local.format("%Y%m%d-%H%M"));
    for suffix in 0.. {
        let file_name = if suffix == 0 {
            format!("{base_name}.txt")
        } else {
            format!("{base_name}-{suffix}.txt")
        };
        let candidate = cwd.join(file_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::Api(
        "Failed to generate export filename".to_string(),
    ))
}

pub fn write_export_file(text: &str, output: &Path) -> Result<(), AppError> {
    fs::write(output, text).map_err(|err| {
        AppError::Api(format!(
            "Failed to write export file {}: {err}",
            display(output)
        ))
    })
}

fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn matches_date_range(memo: &RecentMemo, range: &ExportDateRange) -> bool {
    let Ok(created_at) = DateTime::parse_from_rfc3339(&memo.created_at) else {
        return false;
    };
    let created_at = created_at.with_timezone(&Utc);

    if let Some(from) = range.from.as_ref()
        && created_at < *from
    {
        return false;
    }
    if let Some(to) = range.to.as_ref()
        && created_at > *to
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{
        ExportRangePreset, build_export_payload, date_range_for_preset, next_export_file_path,
    };
    use crate::supabase::RecentMemo;
    use chrono::{DateTime, Local, Utc};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn export_payload_joins_non_empty_memos_with_blank_lines() {
        let memos = vec![
            memo("1", "first", "2026-01-01T00:00:00Z", None),
            memo("2", "   ", "2026-01-02T00:00:00Z", None),
            memo("3", "third", "2026-01-03T00:00:00Z", None),
        ];
        let range = date_range_for_preset(ExportRangePreset::All, utc("2026-02-01T00:00:00Z"));

        let payload = build_export_payload(&memos, &range);

        assert_eq!(payload.text, "first\n\nthird");
        assert_eq!(payload.memo_count, 2);
    }

    #[test]
    fn export_payload_skips_soft_deleted_memos() {
        let memos = vec![
            memo("1", "first", "2026-01-01T00:00:00Z", None),
            memo(
                "2",
                "second",
                "2026-01-02T00:00:00Z",
                Some("2026-01-03T00:00:00Z"),
            ),
        ];
        let range = date_range_for_preset(ExportRangePreset::All, utc("2026-02-01T00:00:00Z"));

        let payload = build_export_payload(&memos, &range);

        assert_eq!(payload.text, "first");
        assert_eq!(payload.memo_count, 1);
    }

    #[test]
    fn export_payload_filters_by_last_3_days() {
        let memos = vec![
            memo("1", "old", "2026-01-10T00:00:00Z", None),
            memo("2", "inside", "2026-01-31T01:00:00Z", None),
        ];
        let range =
            date_range_for_preset(ExportRangePreset::Last3Days, utc("2026-02-01T00:00:00Z"));

        let payload = build_export_payload(&memos, &range);

        assert_eq!(payload.text, "inside");
        assert_eq!(payload.memo_count, 1);
    }

    #[test]
    fn export_payload_filters_by_week_and_month() {
        let memos = vec![
            memo("1", "ten-days-old", "2026-01-22T00:00:00Z", None),
            memo("2", "five-days-old", "2026-01-27T00:00:00Z", None),
            memo("3", "thirty-five-days-old", "2025-12-28T00:00:00Z", None),
        ];

        let week = date_range_for_preset(ExportRangePreset::Week, utc("2026-02-01T00:00:00Z"));
        let month = date_range_for_preset(ExportRangePreset::Month, utc("2026-02-01T00:00:00Z"));

        let week_payload = build_export_payload(&memos, &week);
        let month_payload = build_export_payload(&memos, &month);

        assert_eq!(week_payload.text, "five-days-old");
        assert_eq!(week_payload.memo_count, 1);
        assert_eq!(month_payload.text, "ten-days-old\n\nfive-days-old");
        assert_eq!(month_payload.memo_count, 2);
    }

    #[test]
    fn next_export_file_path_appends_suffix_on_collision() {
        let temp_dir = temp_dir("collision");
        let now_local = Local::now();
        let base = format!("capmind-{}", now_local.format("%Y%m%d-%H%M"));
        let first = next_export_file_path(&temp_dir, now_local).expect("first path");
        fs::write(&first, "one").expect("write first");

        let second = next_export_file_path(&temp_dir, now_local).expect("second path");
        let second_name = second
            .file_name()
            .and_then(|value| value.to_str())
            .expect("second filename");
        assert_eq!(second_name, format!("{base}-1.txt"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn next_export_file_path_uses_base_name_when_available() {
        let temp_dir = temp_dir("basename");
        let now_local = Local::now();
        let base = format!("capmind-{}", now_local.format("%Y%m%d-%H%M"));
        let path = next_export_file_path(&temp_dir, now_local).expect("path");

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("filename");
        assert_eq!(file_name, format!("{base}.txt"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    fn memo(id: &str, text: &str, created_at: &str, deleted_at: Option<&str>) -> RecentMemo {
        RecentMemo {
            id: id.to_string(),
            text: text.to_string(),
            created_at: created_at.to_string(),
            version: "1".to_string(),
            deleted_at: deleted_at.map(str::to_string),
        }
    }

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid datetime")
            .with_timezone(&Utc)
    }

    fn temp_dir(suffix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("capmind-export-test-{suffix}-{stamp}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
