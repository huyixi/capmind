use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::supabase::RecentMemo;

const CAPMIND_DIR_NAME: &str = ".capmind";
const MEMO_LIST_CACHE_FILE_NAME: &str = "memo-list-cache.json";

#[derive(Debug, Serialize, Deserialize)]
struct MemoListCachePayload {
    user_id: String,
    updated_at: String,
    memos: Vec<RecentMemo>,
}

pub fn load_for_user(user_id: &str) -> Result<Option<Vec<RecentMemo>>, String> {
    let path = cache_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", display(&path)))?;
    let payload: MemoListCachePayload = serde_json::from_str(&content)
        .map_err(|err| format!("Invalid cache file {}: {err}", display(&path)))?;

    if payload.user_id != user_id {
        return Ok(None);
    }

    Ok(Some(payload.memos))
}

pub fn save_for_user(user_id: &str, memos: &[RecentMemo]) -> Result<(), String> {
    let path = cache_file_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|err| format!("Failed to create {}: {err}", display(dir)))?;
    } else {
        return Err(format!("Invalid cache path: {}", display(&path)));
    }

    let payload = MemoListCachePayload {
        user_id: user_id.to_string(),
        updated_at: Utc::now().to_rfc3339(),
        memos: memos.to_vec(),
    };

    let output = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Failed to serialize memo cache data: {err}"))?;
    fs::write(&path, output).map_err(|err| format!("Failed to write {}: {err}", display(&path)))?;
    set_file_permissions_600(&path)?;
    Ok(())
}

fn cache_file_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(CAPMIND_DIR_NAME)
        .join(MEMO_LIST_CACHE_FILE_NAME))
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
