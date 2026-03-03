use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::AppError;

const RELEASES_URL: &str = "https://github.com/huyixi/capmind/releases";
const USER_AGENT: &str = "capmind-self-update";
const CHECKSUM_ASSET_NAME: &str = "SHA256SUMS";
const TAG_PREFIX: &str = "capmind-v";

struct UpdateConfig<'a> {
    releases_url: &'a str,
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
struct GitHubLatestReleaseResponse {
    tag_name: String,
}

pub async fn run_self_update(
    requested_version: Option<&str>,
) -> Result<SelfUpdateOutcome, AppError> {
    let config = UpdateConfig {
        releases_url: RELEASES_URL,
        current_exe_path: None,
        current_version: env!("CARGO_PKG_VERSION"),
    };
    run_self_update_with_config(requested_version, &config).await
}

pub async fn latest_release_tag() -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| AppError::Network(format!("Failed to build HTTP client: {err}")))?;
    resolve_latest_release_tag(&client, RELEASES_URL).await
}

async fn run_self_update_with_config(
    requested_version: Option<&str>,
    config: &UpdateConfig<'_>,
) -> Result<SelfUpdateOutcome, AppError> {
    let platform_asset = platform_asset_name()?;
    let requested_tag = requested_version.map(normalize_target_tag);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|err| AppError::Network(format!("Failed to build HTTP client: {err}")))?;
    let release_tag = match requested_tag {
        Some(tag) => tag,
        None => resolve_latest_release_tag(&client, config.releases_url).await?,
    };

    let binary_url = release_asset_url(config.releases_url, Some(&release_tag), platform_asset);
    let checksums_url =
        release_asset_url(config.releases_url, Some(&release_tag), CHECKSUM_ASSET_NAME);

    let checksums = download_asset_text(&client, &checksums_url).await?;
    let binary_bytes = download_asset_bytes(&client, &binary_url).await?;

    apply_release_update(
        config.current_version,
        &release_tag,
        platform_asset,
        &checksums,
        &binary_bytes,
        config.current_exe_path,
    )
}

fn platform_asset_name() -> Result<&'static str, AppError> {
    match std::env::consts::OS {
        "linux" => Ok("capmind-Linux"),
        "macos" => Ok("capmind-macOS"),
        "windows" => Ok("capmind-Windows.exe"),
        other => Err(AppError::Api(format!(
            "Update is not supported on this platform: {other}"
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

pub fn release_version_from_tag(tag: &str) -> &str {
    tag.strip_prefix(TAG_PREFIX).unwrap_or(tag)
}

fn release_asset_url(releases_url: &str, target_tag: Option<&str>, asset_name: &str) -> String {
    match target_tag {
        Some(tag) => format!("{releases_url}/download/{tag}/{asset_name}"),
        None => format!("{releases_url}/latest/download/{asset_name}"),
    }
}

fn latest_release_api_url(releases_url: &str) -> Result<String, AppError> {
    let parsed = reqwest::Url::parse(releases_url)
        .map_err(|err| AppError::Api(format!("Invalid releases URL `{releases_url}`: {err}")))?;
    let segments: Vec<_> = parsed
        .path_segments()
        .map(|v| v.collect())
        .unwrap_or_default();
    if parsed.domain() != Some("github.com") || segments.len() < 3 || segments[2] != "releases" {
        return Err(AppError::Api(format!(
            "Unable to derive repository owner/name from releases URL `{releases_url}`"
        )));
    }

    Ok(format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        segments[0], segments[1]
    ))
}

async fn resolve_latest_release_tag(
    client: &reqwest::Client,
    releases_url: &str,
) -> Result<String, AppError> {
    let api_url = latest_release_api_url(releases_url)?;
    let response = client
        .get(&api_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|err| AppError::Network(format!("Failed to fetch release metadata: {err}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Api(format!(
            "Failed to fetch release metadata `{api_url}` ({status}): {body}"
        )));
    }

    let release: GitHubLatestReleaseResponse = response
        .json()
        .await
        .map_err(|err| AppError::Api(format!("Invalid release metadata JSON: {err}")))?;
    let tag = release.tag_name.trim();
    if tag.is_empty() {
        return Err(AppError::Api(
            "Release metadata does not include `tag_name`".to_string(),
        ));
    }
    Ok(tag.to_string())
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

    let bytes = response
        .bytes()
        .await
        .map(|v| v.to_vec())
        .map_err(|err| AppError::Network(format!("Failed to read asset `{url}`: {err}")))?;

    Ok(bytes)
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
    release_tag: &str,
    platform_asset: &str,
    checksums: &str,
    binary_bytes: &[u8],
    current_exe_override: Option<&Path>,
) -> Result<SelfUpdateOutcome, AppError> {
    let release_version = release_version_from_tag(release_tag).to_string();
    if release_version == current_version {
        return Ok(SelfUpdateOutcome::UpToDate {
            version: current_version.to_string(),
        });
    }

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
        tag: release_tag.to_string(),
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
            "Failed to move current binary to backup. Update aborted without replacing executable: {err}"
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
        SelfUpdateOutcome, apply_release_update, compute_sha256_hex, latest_release_api_url,
        normalize_target_tag, parse_expected_sha256, platform_asset_name, release_asset_url,
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

    fn assert_temp_update_files_absent(exe_path: &Path) {
        let parent = exe_path.parent().expect("exe parent");
        let backup_path = super::staged_file_path(parent, exe_path, "bak").expect("backup path");
        let staged_path = super::staged_file_path(parent, exe_path, "new").expect("staged path");
        assert!(!backup_path.exists());
        assert!(!staged_path.exists());
    }

    #[test]
    fn normalize_target_tag_adds_prefix() {
        assert_eq!(normalize_target_tag("0.2.1"), "capmind-v0.2.1");
        assert_eq!(normalize_target_tag(" capmind-v0.2.2 "), "capmind-v0.2.2");
    }

    #[test]
    fn release_version_from_tag_strips_prefix() {
        assert_eq!(release_version_from_tag("capmind-v0.2.1"), "0.2.1");
        assert_eq!(release_version_from_tag("v0.2.1"), "v0.2.1");
    }

    #[test]
    fn parse_expected_sha256_supports_common_formats() {
        let checksums = "\
1111111111111111111111111111111111111111111111111111111111111111  capmind-Linux\n\
2222222222222222222222222222222222222222222222222222222222222222  ./capmind-macOS\n\
3333333333333333333333333333333333333333333333333333333333333333 *capmind-Windows.exe\n\
";

        assert_eq!(
            parse_expected_sha256(checksums, "capmind-Linux").as_deref(),
            Some("1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(
            parse_expected_sha256(checksums, "capmind-macOS").as_deref(),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert_eq!(
            parse_expected_sha256(checksums, "capmind-Windows.exe").as_deref(),
            Some("3333333333333333333333333333333333333333333333333333333333333333")
        );
    }

    #[test]
    fn release_asset_url_uses_latest_or_tag_paths() {
        let base = "https://github.com/huyixi/capmind/releases";
        assert_eq!(
            release_asset_url(base, None, "capmind-macOS"),
            "https://github.com/huyixi/capmind/releases/latest/download/capmind-macOS"
        );
        assert_eq!(
            release_asset_url(base, Some("capmind-v1.2.3"), "capmind-macOS"),
            "https://github.com/huyixi/capmind/releases/download/capmind-v1.2.3/capmind-macOS"
        );
    }

    #[test]
    fn latest_release_api_url_derives_repo_endpoint() {
        let api_url = latest_release_api_url("https://github.com/huyixi/capmind/releases")
            .expect("valid releases url");
        assert_eq!(
            api_url,
            "https://api.github.com/repos/huyixi/capmind/releases/latest"
        );
    }

    #[test]
    fn latest_release_api_url_rejects_non_github_release_urls() {
        let err = latest_release_api_url("https://example.com/releases")
            .expect_err("url should be rejected");
        assert!(
            err.to_string()
                .contains("Unable to derive repository owner/name")
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
        let outcome = apply_release_update(
            "0.1.0",
            "capmind-v9.9.9",
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
                assert_eq!(tag, "capmind-v9.9.9");
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
        let err = apply_release_update(
            "0.1.0",
            "capmind-v9.9.9",
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
