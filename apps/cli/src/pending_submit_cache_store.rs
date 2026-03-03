use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

const CAPMIND_DIR_NAME: &str = ".capmind";
const PENDING_SUBMIT_CACHE_FILE_NAME: &str = "pending-submit-cache.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingSubmitCacheItem {
    pub id: String,
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingSubmitCachePayload {
    updated_at: String,
    items: Vec<PendingSubmitCacheItem>,
}

pub fn load() -> Result<Vec<PendingSubmitCacheItem>, String> {
    let path = cache_file_path()?;
    load_from_path(&path)
}

pub fn append_dedup(text: &str) -> Result<(), String> {
    let path = cache_file_path()?;
    append_dedup_to_path(&path, text)
}

pub fn save(items: &[PendingSubmitCacheItem]) -> Result<(), String> {
    let path = cache_file_path()?;
    save_to_path(&path, items)
}

fn append_dedup_to_path(path: &Path, text: &str) -> Result<(), String> {
    let mut items = load_from_path(path)?;
    if items.iter().any(|item| item.text == text) {
        return Ok(());
    }

    items.push(PendingSubmitCacheItem {
        id: next_item_id(text),
        text: text.to_string(),
        created_at: Utc::now().to_rfc3339(),
    });
    save_to_path(path, &items)
}

fn load_from_path(path: &Path) -> Result<Vec<PendingSubmitCacheItem>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|err| format!("Failed to read {}: {err}", display(path)))?;
    let payload: PendingSubmitCachePayload = serde_json::from_str(&content)
        .map_err(|err| format!("Invalid cache file {}: {err}", display(path)))?;
    Ok(payload.items)
}

fn save_to_path(path: &Path, items: &[PendingSubmitCacheItem]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|err| format!("Failed to create {}: {err}", display(dir)))?;
    } else {
        return Err(format!("Invalid cache path: {}", display(path)));
    }

    let payload = PendingSubmitCachePayload {
        updated_at: Utc::now().to_rfc3339(),
        items: items.to_vec(),
    };
    let output = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Failed to serialize pending submit cache data: {err}"))?;
    fs::write(path, output).map_err(|err| format!("Failed to write {}: {err}", display(path)))?;
    set_file_permissions_600(path)?;
    Ok(())
}

fn cache_file_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(CAPMIND_DIR_NAME)
        .join(PENDING_SUBMIT_CACHE_FILE_NAME))
}

fn next_item_id(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(unix)]
fn set_file_permissions_600(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|err| format!("Failed to set permissions on {}: {err}", display(path)))
}

#[cfg(not(unix))]
fn set_file_permissions_600(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PendingSubmitCacheItem, append_dedup_to_path, load_from_path, save_to_path};
    use std::fs;
    use std::path::PathBuf;

    fn temp_cache_path(name: &str) -> PathBuf {
        let unique = format!(
            "capmind-pending-cache-{name}-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        std::env::temp_dir()
            .join(unique)
            .join("pending-submit-cache.json")
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let path = temp_cache_path("missing");
        let loaded = load_from_path(&path).expect("load should succeed");
        assert!(loaded.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_cache_path("roundtrip");
        let items = vec![PendingSubmitCacheItem {
            id: "id-1".to_string(),
            text: "hello".to_string(),
            created_at: "2026-03-03T00:00:00Z".to_string(),
        }];

        save_to_path(&path, &items).expect("save should succeed");
        let loaded = load_from_path(&path).expect("load should succeed");
        assert_eq!(loaded, items);

        let _ = fs::remove_dir_all(path.parent().expect("parent dir should exist"));
    }

    #[test]
    fn append_dedup_keeps_single_copy_for_same_text() {
        let path = temp_cache_path("dedup");
        append_dedup_to_path(&path, "same-text").expect("first append should succeed");
        append_dedup_to_path(&path, "same-text").expect("second append should succeed");

        let loaded = load_from_path(&path).expect("load should succeed");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "same-text");

        let _ = fs::remove_dir_all(path.parent().expect("parent dir should exist"));
    }
}
