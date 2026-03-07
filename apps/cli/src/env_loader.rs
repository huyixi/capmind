use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static FILE_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn init() {
    let _ = FILE_ENV.get_or_init(load_file_env);
}

#[allow(dead_code)]
pub fn get_value(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    let file_env = FILE_ENV.get_or_init(load_file_env);
    for key in keys {
        if let Some(value) = file_env.get(*key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn load_file_env() -> HashMap<String, String> {
    let mut out = HashMap::new();
    for path in candidate_paths() {
        if !path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for line in content.lines() {
            if let Some((key, value)) = parse_line(line) {
                out.entry(key).or_insert(value);
            }
        }
    }
    out
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(workspace_root) = workspace_root_from_manifest_dir() {
        paths.push(workspace_root.join(".env.local"));
        paths.push(workspace_root.join(".env"));
        if let Some(shared_root) = workspace_root.parent() {
            paths.push(shared_root.join(".env.local"));
            paths.push(shared_root.join(".env"));
        }
    }

    paths.push(PathBuf::from(".env.local"));
    paths.push(PathBuf::from(".env"));

    if let Ok(home) = std::env::var("HOME") {
        let capmind = PathBuf::from(home).join(".capmind");
        paths.push(capmind.join(".env.local"));
        paths.push(capmind.join(".env"));
    }
    paths
}

fn workspace_root_from_manifest_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let apps_dir = manifest_dir.parent()?;
    let workspace_root = apps_dir.parent()?;
    Some(workspace_root.to_path_buf())
}

fn parse_line(raw: &str) -> Option<(String, String)> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let line = line.strip_prefix("export ").unwrap_or(line);
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    let mut value = value.trim().to_string();
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value = value[1..value.len() - 1].to_string();
    }

    Some((key.to_string(), value))
}
