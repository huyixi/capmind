use std::fs;
use std::path::{Path, PathBuf};

const CAPMIND_DIR_NAME: &str = ".capmind";
const SESSION_FILE_NAME: &str = "auth.json";
const REFRESH_TOKEN_KEY: &str = "refresh_token";
const USER_ID_KEY: &str = "user_id";

pub fn load_refresh_token() -> Result<Option<String>, String> {
    load_session_field(REFRESH_TOKEN_KEY)
}

pub fn load_cached_user_id() -> Result<Option<String>, String> {
    load_session_field(USER_ID_KEY)
}

pub fn save_session(refresh_token: &str, user_id: Option<&str>) -> Result<(), String> {
    let path = session_file_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|err| format!("Failed to create {}: {err}", display(dir)))?;
    } else {
        return Err(format!("Invalid session path: {}", display(&path)));
    }

    let mut data = if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read {}: {err}", display(&path)))?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(serde_json::Value::Object(map)) => map,
            Ok(_) => serde_json::Map::new(),
            Err(_) => serde_json::Map::new(),
        }
    } else {
        serde_json::Map::new()
    };

    data.insert(
        REFRESH_TOKEN_KEY.to_string(),
        serde_json::Value::String(refresh_token.to_string()),
    );
    if let Some(user_id) = user_id.map(str::trim).filter(|value| !value.is_empty()) {
        data.insert(
            USER_ID_KEY.to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
    } else {
        data.remove(USER_ID_KEY);
    }

    let output = serde_json::to_string_pretty(&serde_json::Value::Object(data))
        .map_err(|err| format!("Failed to serialize session data: {err}"))?;
    fs::write(&path, output).map_err(|err| format!("Failed to write {}: {err}", display(&path)))?;
    set_file_permissions_600(&path)?;
    Ok(())
}

fn load_session_field(key: &str) -> Result<Option<String>, String> {
    let path = session_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", display(&path)))?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|err| format!("Invalid session file {}: {err}", display(&path)))?;
    let value = data
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(value))
}

pub fn clear_saved_session() -> Result<bool, String> {
    let path = session_file_path()?;
    if !path.exists() {
        return Ok(false);
    }

    fs::remove_file(&path).map_err(|err| format!("Failed to remove {}: {err}", display(&path)))?;
    Ok(true)
}

fn session_file_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(CAPMIND_DIR_NAME)
        .join(SESSION_FILE_NAME))
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
