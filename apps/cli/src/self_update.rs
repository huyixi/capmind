use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use reqwest::StatusCode;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::AppError;

const RELEASES_API: &str = "https://api.github.com/repos/huyixi/cap-mind/releases";
const USER_AGENT: &str = "cap-cli-self-update";
const CHECKSUM_ASSET_NAME: &str = "SHA256SUMS";
const TAG_PREFIX: &str = "cli-v";

struct UpdateConfig<'a> {
    releases_api: &'a str,
    current_exe_path: Option<&'a Path>,
    current_version: &'a str,
}

#[derive(Debug)]
pub enum SelfUpdateOutcome {
    UpToDate {
        version: String,
    },
    Updated {
        from_version: String,
        to_version: String,
        tag: String,
    },
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub async fn run_self_update(
    requested_version: Option<&str>,
) -> Result<SelfUpdateOutcome, AppError> {
    let config = UpdateConfig {
        releases_api: RELEASES_API,
        current_exe_path: None,
        current_version: env!("CARGO_PKG_VERSION"),
    };
    run_self_update_with_config(requested_version, &config).await
}

async fn run_self_update_with_config(
    requested_version: Option<&str>,
    config: &UpdateConfig<'_>,
) -> Result<SelfUpdateOutcome, AppError> {
    let platform_asset = platform_asset_name()?;
    let current_version = config.current_version.to_string();
    let requested_tag = requested_version.map(normalize_target_tag);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| AppError::Network(format!("Failed to build HTTP client: {err}")))?;

    let release = fetch_release(&client, config.releases_api, requested_tag.as_deref()).await?;
    if release_version_from_tag(&release.tag_name) == current_version {
        return Ok(SelfUpdateOutcome::UpToDate {
            version: current_version,
        });
    }

    let binary_url = find_asset_url(&release, platform_asset)?;
    let checksums_url = find_asset_url(&release, CHECKSUM_ASSET_NAME)?;

    let checksums = download_asset_text(&client, &checksums_url).await?;
    let binary_bytes = download_asset_bytes(&client, &binary_url).await?;

    apply_release_update(
        config.current_version,
        release,
        platform_asset,
        &checksums,
        &binary_bytes,
        config.current_exe_path,
    )
}

fn platform_asset_name() -> Result<&'static str, AppError> {
    match std::env::consts::OS {
        "linux" => Ok("cap-Linux"),
        "macos" => Ok("cap-macOS"),
        "windows" => Ok("cap-Windows.exe"),
        other => Err(AppError::Api(format!(
            "Self-update is not supported on this platform: {other}"
        ))),
    }
}

fn normalize_target_tag(version: &str) -> String {
    let trimmed = version.trim();
    if trimmed.starts_with(TAG_PREFIX) {
        trimmed.to_string()
    } else {
        format!("{TAG_PREFIX}{trimmed}")
    }
}

fn release_version_from_tag(tag: &str) -> &str {
    tag.strip_prefix(TAG_PREFIX).unwrap_or(tag)
}

async fn fetch_release(
    client: &reqwest::Client,
    releases_api: &str,
    target_tag: Option<&str>,
) -> Result<GitHubRelease, AppError> {
    let endpoint = match target_tag {
        Some(tag) => format!("{releases_api}/tags/{tag}"),
        None => format!("{releases_api}/latest"),
    };

    let response = client
        .get(endpoint)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|err| AppError::Network(format!("Failed to fetch release metadata: {err}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::Network(format!("Failed to read release metadata: {err}")))?;

    if !status.is_success() {
        if status == StatusCode::NOT_FOUND {
            return Err(AppError::Api("Requested release was not found".to_string()));
        }
        return Err(AppError::Api(format!(
            "Failed to fetch release metadata ({status}): {body}"
        )));
    }

    serde_json::from_str(&body)
        .map_err(|err| AppError::Api(format!("Invalid release metadata JSON: {err}")))
}

fn find_asset_url(release: &GitHubRelease, asset_name: &str) -> Result<String, AppError> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .map(|asset| asset.browser_download_url.clone())
        .ok_or_else(|| {
            AppError::Api(format!(
                "Release `{}` is missing asset `{asset_name}`",
                release.tag_name
            ))
        })
}

async fn download_asset_text(client: &reqwest::Client, url: &str) -> Result<String, AppError> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|err| AppError::Network(format!("Failed to download asset `{url}`: {err}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "Failed to download asset `{url}` ({status}): {body}"
        )));
    }

    response
        .text()
        .await
        .map_err(|err| AppError::Network(format!("Failed to read asset `{url}`: {err}")))
}

async fn download_asset_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, AppError> {
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|err| AppError::Network(format!("Failed to download asset `{url}`: {err}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "Failed to download asset `{url}` ({status}): {body}"
        )));
    }

    response
        .bytes()
        .await
        .map(|v| v.to_vec())
        .map_err(|err| AppError::Network(format!("Failed to read asset `{url}`: {err}")))
}

fn parse_expected_sha256(checksums: &str, asset_name: &str) -> Option<String> {
    for line in checksums.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else {
            continue;
        };
        let Some(mut file) = parts.next() else {
            continue;
        };

        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }

        if let Some(stripped) = file.strip_prefix("./") {
            file = stripped;
        }
        if let Some(stripped) = file.strip_prefix('*') {
            file = stripped;
        }

        if file == asset_name || file.ends_with(&format!("/{asset_name}")) {
            return Some(hash.to_ascii_lowercase());
        }
    }

    None
}

fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn apply_release_update(
    current_version: &str,
    release: GitHubRelease,
    platform_asset: &str,
    checksums: &str,
    binary_bytes: &[u8],
    current_exe_override: Option<&Path>,
) -> Result<SelfUpdateOutcome, AppError> {
    let release_version = release_version_from_tag(&release.tag_name).to_string();
    if release_version == current_version {
        return Ok(SelfUpdateOutcome::UpToDate {
            version: current_version.to_string(),
        });
    }

    let _ = find_asset_url(&release, platform_asset)?;
    let _ = find_asset_url(&release, CHECKSUM_ASSET_NAME)?;

    let expected_hash = parse_expected_sha256(checksums, platform_asset).ok_or_else(|| {
        AppError::Api(format!(
            "{CHECKSUM_ASSET_NAME} does not contain a hash for asset `{platform_asset}`"
        ))
    })?;
    let actual_hash = compute_sha256_hex(binary_bytes);
    if !actual_hash.eq_ignore_ascii_case(&expected_hash) {
        return Err(AppError::Api(format!(
            "Checksum mismatch for `{platform_asset}`. Expected `{expected_hash}`, got `{actual_hash}`"
        )));
    }

    install_binary_with_rollback(binary_bytes, current_exe_override)?;

    Ok(SelfUpdateOutcome::Updated {
        from_version: current_version.to_string(),
        to_version: release_version,
        tag: release.tag_name,
    })
}

fn install_binary_with_rollback(
    binary_bytes: &[u8],
    current_exe_override: Option<&Path>,
) -> Result<(), AppError> {
    let current_exe = match current_exe_override {
        Some(path) => path.to_path_buf(),
        None => std::env::current_exe().map_err(|err| {
            AppError::Api(format!("Failed to resolve current executable path: {err}"))
        })?,
    };
    let parent = current_exe.parent().ok_or_else(|| {
        AppError::Api("Current executable path has no parent directory".to_string())
    })?;

    let staged_path = staged_file_path(parent, &current_exe, "new")?;
    let backup_path = staged_file_path(parent, &current_exe, "bak")?;

    remove_if_exists(&staged_path)?;
    remove_if_exists(&backup_path)?;

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)
            .map_err(|err| AppError::Api(format!("Failed to create staged binary: {err}")))?;
        file.write_all(binary_bytes)
            .map_err(|err| AppError::Api(format!("Failed to write staged binary: {err}")))?;
        file.sync_all()
            .map_err(|err| AppError::Api(format!("Failed to flush staged binary: {err}")))?;
    }

    let current_permissions = fs::metadata(&current_exe)
        .map_err(|err| AppError::Api(format!("Failed to read current binary permissions: {err}")))?
        .permissions();
    fs::set_permissions(&staged_path, current_permissions)
        .map_err(|err| AppError::Api(format!("Failed to set staged binary permissions: {err}")))?;

    if let Err(err) = fs::rename(&current_exe, &backup_path) {
        let _ = remove_if_exists(&staged_path);
        return Err(AppError::Api(format!(
            "Failed to move current binary to backup. Self-update aborted without replacing executable: {err}"
        )));
    }

    if let Err(err) = fs::rename(&staged_path, &current_exe) {
        let rollback_result = fs::rename(&backup_path, &current_exe);
        let _ = remove_if_exists(&staged_path);
        return match rollback_result {
            Ok(()) => Err(AppError::Api(format!(
                "Failed to install updated binary ({err}). Rolled back to previous version."
            ))),
            Err(rollback_err) => Err(AppError::Api(format!(
                "Failed to install updated binary ({err}), and rollback failed ({rollback_err}). Manual recovery is required."
            ))),
        };
    }

    if let Err(err) = remove_if_exists(&backup_path) {
        eprintln!("Warning: update succeeded but failed to remove backup file: {err}");
    }

    Ok(())
}

fn staged_file_path(parent: &Path, current_exe: &Path, suffix: &str) -> Result<PathBuf, AppError> {
    let file_name = current_exe
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| AppError::Api("Invalid executable filename".to_string()))?;
    let pid = std::process::id();
    Ok(parent.join(format!(".{file_name}.{pid}.{suffix}")))
}

fn remove_if_exists(path: &Path) -> Result<(), AppError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(AppError::Api(format!(
            "Failed to remove `{}`: {err}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::{
        CHECKSUM_ASSET_NAME, GitHubAsset, GitHubRelease, SelfUpdateOutcome, apply_release_update,
        compute_sha256_hex, normalize_target_tag, parse_expected_sha256, platform_asset_name,
        release_version_from_tag,
    };

    fn temp_file_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix timestamp")
                .as_nanos()
        ));
        path
    }

    fn mock_release(tag: &str, asset_name: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            assets: vec![
                GitHubAsset {
                    name: asset_name.to_string(),
                    browser_download_url: "https://example.invalid/cap-cli".to_string(),
                },
                GitHubAsset {
                    name: CHECKSUM_ASSET_NAME.to_string(),
                    browser_download_url: "https://example.invalid/SHA256SUMS".to_string(),
                },
            ],
        }
    }

    fn assert_temp_update_files_absent(exe_path: &Path) {
        let parent = exe_path.parent().expect("exe parent");
        let backup_path = super::staged_file_path(parent, exe_path, "bak").expect("backup path");
        let staged_path = super::staged_file_path(parent, exe_path, "new").expect("staged path");
        assert!(!backup_path.exists());
        assert!(!staged_path.exists());
    }

    #[test]
    fn normalize_target_tag_adds_prefix() {
        assert_eq!(normalize_target_tag("0.2.1"), "cli-v0.2.1");
        assert_eq!(normalize_target_tag(" cli-v0.2.2 "), "cli-v0.2.2");
    }

    #[test]
    fn release_version_from_tag_strips_prefix() {
        assert_eq!(release_version_from_tag("cli-v0.2.1"), "0.2.1");
        assert_eq!(release_version_from_tag("v0.2.1"), "v0.2.1");
    }

    #[test]
    fn parse_expected_sha256_supports_common_formats() {
        let checksums = "\
1111111111111111111111111111111111111111111111111111111111111111  cap-Linux\n\
2222222222222222222222222222222222222222222222222222222222222222  ./cap-macOS\n\
3333333333333333333333333333333333333333333333333333333333333333 *cap-Windows.exe\n\
";

        assert_eq!(
            parse_expected_sha256(checksums, "cap-Linux").as_deref(),
            Some("1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(
            parse_expected_sha256(checksums, "cap-macOS").as_deref(),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert_eq!(
            parse_expected_sha256(checksums, "cap-Windows.exe").as_deref(),
            Some("3333333333333333333333333333333333333333333333333333333333333333")
        );
    }

    #[test]
    fn self_update_apply_release_replaces_binary_when_checksum_matches() {
        let asset_name = platform_asset_name()
            .expect("supported platform")
            .to_string();
        let binary_body = b"new-binary-content".to_vec();
        let checksum = compute_sha256_hex(&binary_body);
        let checksums_body = format!("{checksum}  {asset_name}\n");

        let exe_path = temp_file_path("cap-self-update-success");
        std::fs::write(&exe_path, b"old-binary-content").expect("write old binary");
        let release = mock_release("cli-v9.9.9", &asset_name);
        let outcome = apply_release_update(
            "0.1.0",
            release,
            &asset_name,
            &checksums_body,
            &binary_body,
            Some(exe_path.as_path()),
        )
        .expect("self update should succeed");

        match outcome {
            SelfUpdateOutcome::Updated {
                from_version,
                to_version,
                tag,
            } => {
                assert_eq!(from_version, "0.1.0");
                assert_eq!(to_version, "9.9.9");
                assert_eq!(tag, "cli-v9.9.9");
            }
            other => panic!("unexpected outcome: {other:?}"),
        }

        let installed = std::fs::read(&exe_path).expect("read updated binary");
        assert_eq!(installed, binary_body);

        assert_temp_update_files_absent(&exe_path);

        std::fs::remove_file(&exe_path).expect("cleanup exe");
    }

    #[test]
    fn self_update_apply_release_rejects_mismatch_and_keeps_old_binary() {
        let asset_name = platform_asset_name()
            .expect("supported platform")
            .to_string();
        let binary_body = b"new-binary-content".to_vec();
        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";
        let checksums_body = format!("{wrong_checksum}  {asset_name}\n");

        let exe_path = temp_file_path("cap-self-update-mismatch");
        let old = b"old-binary-content";
        std::fs::write(&exe_path, old).expect("write old binary");
        let release = mock_release("cli-v9.9.9", &asset_name);
        let err = apply_release_update(
            "0.1.0",
            release,
            &asset_name,
            &checksums_body,
            &binary_body,
            Some(exe_path.as_path()),
        )
        .expect_err("self update should fail on checksum mismatch");
        assert!(
            err.to_string().contains("Checksum mismatch"),
            "unexpected error: {err}"
        );

        let still_old = std::fs::read(&exe_path).expect("read old binary");
        assert_eq!(still_old, old);
        assert_temp_update_files_absent(&exe_path);

        std::fs::remove_file(&exe_path).expect("cleanup exe");
    }
}
