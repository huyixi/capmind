use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::cli::DoctorArgs;
use crate::error::AppError;
use crate::self_update::{latest_release_tag, release_version_from_tag};
use crate::update::{
    InstallSource, detect_current_reported_version, detect_current_reported_version_opt,
    detect_install_source,
};

const FORMULA_NAME: &str = "capmind";
const REFRESH_AHEAD_SECONDS: i64 = 300;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorFinding {
    pub severity: FindingSeverity,
    pub code: String,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub generated_at: String,
    pub status_summary: String,
    pub install: InstallSection,
    pub release: ReleaseSection,
    pub homebrew: HomebrewSection,
    pub session: SessionSection,
    pub cache: CacheSection,
    pub findings: Vec<DoctorFinding>,
}

#[derive(Debug, Serialize)]
pub struct InstallSection {
    pub install_source: InstallSource,
    pub current_version: String,
    pub current_version_detected_from_cap: bool,
    pub cap_path: Option<String>,
    pub cap_symlink_target: Option<String>,
    pub current_exe: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReleaseSection {
    pub latest_tag: Option<String>,
    pub latest_version: Option<String>,
    pub check_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HomebrewSection {
    pub brew_in_path: bool,
    pub brew_version: Option<String>,
    pub brew_prefix: Option<String>,
    pub formula_installed: Option<bool>,
    pub formula_check_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionSection {
    pub auth_file_path: String,
    pub exists: bool,
    pub readable: bool,
    pub parse_ok: bool,
    pub perms_600: Option<bool>,
    pub refresh_token_present: bool,
    pub access_token_present: bool,
    pub access_token_expires_at: Option<String>,
    pub access_token_expires_at_valid: Option<bool>,
    pub access_token_expired: Option<bool>,
    pub refresh_recommended: Option<bool>,
    pub user_id_present: bool,
}

#[derive(Debug, Serialize)]
pub struct CacheSection {
    pub memo_list: CacheFileSection,
    pub pending_submit: CacheFileSection,
}

#[derive(Debug, Serialize)]
pub struct CacheFileSection {
    pub path: String,
    pub exists: bool,
    pub readable: bool,
    pub parse_ok: bool,
    pub perms_600: Option<bool>,
    pub updated_at: Option<String>,
    pub updated_at_valid: Option<bool>,
    pub item_count: Option<usize>,
    pub parse_error: Option<String>,
}

pub async fn run_doctor(_args: &DoctorArgs) -> Result<DoctorReport, AppError> {
    let mut findings: Vec<DoctorFinding> = Vec::new();

    let install_source = detect_install_source(FORMULA_NAME);
    let current_version = detect_current_reported_version();
    let current_version_detected_from_cap = detect_current_reported_version_opt().is_some();
    let cap_path = which_in_path("cap");
    let cap_symlink_target = cap_path
        .as_ref()
        .and_then(|path| read_link_absolute(path).ok())
        .map(|path| path_display(&path));
    let current_exe = std::env::current_exe().ok().map(|path| path_display(&path));

    let install = InstallSection {
        install_source,
        current_version: current_version.clone(),
        current_version_detected_from_cap,
        cap_path: cap_path.as_ref().map(|path| path_display(path)),
        cap_symlink_target,
        current_exe,
    };

    let release = match latest_release_tag().await {
        Ok(tag) => ReleaseSection {
            latest_version: Some(release_version_from_tag(&tag).to_string()),
            latest_tag: Some(tag),
            check_error: None,
        },
        Err(err) => {
            findings.push(DoctorFinding {
                severity: FindingSeverity::Warning,
                code: "release_check_failed".to_string(),
                message: format!("Failed to check latest release tag: {err}"),
                hint: Some("Verify network access and GitHub availability.".to_string()),
            });
            ReleaseSection {
                latest_tag: None,
                latest_version: None,
                check_error: Some(err.to_string()),
            }
        }
    };

    let homebrew = inspect_homebrew(FORMULA_NAME);
    if install.install_source == InstallSource::Homebrew && !homebrew.brew_in_path {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Error,
            code: "brew_not_found".to_string(),
            message: "Install source looks like Homebrew, but `brew` is not available in PATH."
                .to_string(),
            hint: Some("Add Homebrew to PATH, then run `brew upgrade capmind`.".to_string()),
        });
    }
    if install.install_source == InstallSource::Homebrew
        && homebrew.formula_installed == Some(false)
    {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "brew_formula_missing".to_string(),
            message: "Homebrew is available, but formula `capmind` is not installed.".to_string(),
            hint: Some(
                "Install via `brew install capmind` or check tap configuration.".to_string(),
            ),
        });
    }

    let session = inspect_session_file()?;
    add_session_findings(&session, &mut findings);

    let memo_cache_path = capmind_home_file("memo-list-cache.json");
    let pending_cache_path = capmind_home_file("pending-submit-cache.json");
    let cache = CacheSection {
        memo_list: inspect_cache_file(&memo_cache_path, "memos"),
        pending_submit: inspect_cache_file(&pending_cache_path, "items"),
    };
    add_cache_findings(&cache.memo_list, "memo_list_cache", &mut findings);
    add_cache_findings(&cache.pending_submit, "pending_submit_cache", &mut findings);

    if let (Some(latest), Some(_tag)) = (&release.latest_version, &release.latest_tag)
        && latest != &install.current_version
    {
        let hint = match install.install_source {
            InstallSource::Homebrew => "Run `brew upgrade capmind`.",
            InstallSource::Standalone | InstallSource::Unknown => "Run `cap update`.",
        };
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            code: "newer_release_available".to_string(),
            message: format!(
                "Current version `{}` is behind latest `{latest}`.",
                install.current_version
            ),
            hint: Some(hint.to_string()),
        });
    }

    findings.sort_by(|a, b| {
        severity_rank(b.severity)
            .cmp(&severity_rank(a.severity))
            .then_with(|| a.code.cmp(&b.code))
    });
    let status_summary = summary_from_findings(&findings).to_string();

    Ok(DoctorReport {
        generated_at: Utc::now().to_rfc3339(),
        status_summary,
        install,
        release,
        homebrew,
        session,
        cache,
        findings,
    })
}

pub fn render_doctor_text(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("Doctor report\n");
    out.push_str(&format!("generated_at: {}\n", report.generated_at));
    out.push_str(&format!("status: {}\n\n", report.status_summary));

    out.push_str("[Install]\n");
    out.push_str(&format!(
        "install_source: {:?}\n",
        report.install.install_source
    ));
    out.push_str(&format!(
        "current_version: {}\n",
        report.install.current_version
    ));
    out.push_str(&format!(
        "current_version_detected_from_cap: {}\n",
        report.install.current_version_detected_from_cap
    ));
    out.push_str(&format!(
        "cap_path: {}\n",
        report.install.cap_path.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "cap_symlink_target: {}\n",
        report
            .install
            .cap_symlink_target
            .as_deref()
            .unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "current_exe: {}\n\n",
        report.install.current_exe.as_deref().unwrap_or("unknown")
    ));

    out.push_str("[Release]\n");
    out.push_str(&format!(
        "latest_tag: {}\n",
        report.release.latest_tag.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "latest_version: {}\n",
        report
            .release
            .latest_version
            .as_deref()
            .unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "check_error: {}\n\n",
        report.release.check_error.as_deref().unwrap_or("none")
    ));

    out.push_str("[Homebrew]\n");
    out.push_str(&format!("brew_in_path: {}\n", report.homebrew.brew_in_path));
    out.push_str(&format!(
        "brew_version: {}\n",
        report.homebrew.brew_version.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "brew_prefix: {}\n",
        report.homebrew.brew_prefix.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "formula_installed: {}\n",
        report
            .homebrew
            .formula_installed
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "formula_check_error: {}\n\n",
        report
            .homebrew
            .formula_check_error
            .as_deref()
            .unwrap_or("none")
    ));

    out.push_str("[Session]\n");
    out.push_str(&format!(
        "auth_file_path: {}\n",
        report.session.auth_file_path
    ));
    out.push_str(&format!("exists: {}\n", report.session.exists));
    out.push_str(&format!("readable: {}\n", report.session.readable));
    out.push_str(&format!("parse_ok: {}\n", report.session.parse_ok));
    out.push_str(&format!(
        "perms_600: {}\n",
        report
            .session
            .perms_600
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "refresh_token_present: {}\n",
        report.session.refresh_token_present
    ));
    out.push_str(&format!(
        "access_token_present: {}\n",
        report.session.access_token_present
    ));
    out.push_str(&format!(
        "access_token_expires_at: {}\n",
        report
            .session
            .access_token_expires_at
            .as_deref()
            .unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "access_token_expires_at_valid: {}\n",
        report
            .session
            .access_token_expires_at_valid
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "access_token_expired: {}\n",
        report
            .session
            .access_token_expired
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "refresh_recommended: {}\n",
        report
            .session
            .refresh_recommended
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "user_id_present: {}\n\n",
        report.session.user_id_present
    ));

    out.push_str("[Cache]\n");
    render_cache_file_text(&mut out, "memo_list", &report.cache.memo_list);
    render_cache_file_text(&mut out, "pending_submit", &report.cache.pending_submit);
    out.push('\n');

    out.push_str("[Findings]\n");
    if report.findings.is_empty() {
        out.push_str("- none\n");
    } else {
        for finding in &report.findings {
            out.push_str(&format!(
                "- [{:?}] {}: {}\n",
                finding.severity, finding.code, finding.message
            ));
            if let Some(hint) = &finding.hint {
                out.push_str(&format!("  hint: {hint}\n"));
            }
        }
    }

    out
}

pub fn render_doctor_json(report: &DoctorReport) -> Result<String, AppError> {
    serde_json::to_string_pretty(report)
        .map_err(|err| AppError::Api(format!("Failed to serialize doctor report as JSON: {err}")))
}

fn render_cache_file_text(out: &mut String, label: &str, section: &CacheFileSection) {
    out.push_str(&format!("{label}.path: {}\n", section.path));
    out.push_str(&format!("{label}.exists: {}\n", section.exists));
    out.push_str(&format!("{label}.readable: {}\n", section.readable));
    out.push_str(&format!("{label}.parse_ok: {}\n", section.parse_ok));
    out.push_str(&format!(
        "{label}.perms_600: {}\n",
        section
            .perms_600
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "{label}.updated_at: {}\n",
        section.updated_at.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "{label}.updated_at_valid: {}\n",
        section
            .updated_at_valid
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "{label}.item_count: {}\n",
        section
            .item_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!(
        "{label}.parse_error: {}\n",
        section.parse_error.as_deref().unwrap_or("none")
    ));
}

fn inspect_homebrew(formula_name: &str) -> HomebrewSection {
    let brew_version_output = match run_command("brew", &["--version"]) {
        Ok(output) => output,
        Err(_) => {
            return HomebrewSection {
                brew_in_path: false,
                brew_version: None,
                brew_prefix: None,
                formula_installed: None,
                formula_check_error: Some("`brew` is unavailable in PATH.".to_string()),
            };
        }
    };

    let brew_version = output_text(&brew_version_output)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string);

    let brew_prefix = run_command("brew", &["--prefix"])
        .ok()
        .map(|out| output_text(&out))
        .and_then(|text| {
            text.lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToString::to_string)
        });

    match run_command("brew", &["list", "--formula"]) {
        Ok(output) if output.status.success() => {
            let installed = output_text(&output)
                .lines()
                .any(|line| line.trim() == formula_name);
            HomebrewSection {
                brew_in_path: true,
                brew_version,
                brew_prefix,
                formula_installed: Some(installed),
                formula_check_error: None,
            }
        }
        Ok(output) => HomebrewSection {
            brew_in_path: true,
            brew_version,
            brew_prefix,
            formula_installed: None,
            formula_check_error: Some(format!(
                "`brew list --formula` failed: {}",
                compact_command_output(&output_text(&output))
            )),
        },
        Err(err) => HomebrewSection {
            brew_in_path: true,
            brew_version,
            brew_prefix,
            formula_installed: None,
            formula_check_error: Some(format!("Failed to run `brew list --formula`: {err}")),
        },
    }
}

fn inspect_session_file() -> Result<SessionSection, AppError> {
    let path = capmind_home_file("auth.json");
    let auth_file_path = path_display(&path);
    let exists = path.exists();
    let perms_600 = file_permissions_600(&path);

    if !exists {
        return Ok(SessionSection {
            auth_file_path,
            exists,
            readable: false,
            parse_ok: false,
            perms_600,
            refresh_token_present: false,
            access_token_present: false,
            access_token_expires_at: None,
            access_token_expires_at_valid: None,
            access_token_expired: None,
            refresh_recommended: None,
            user_id_present: false,
        });
    }

    let raw = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(_) => {
            return Ok(SessionSection {
                auth_file_path,
                exists,
                readable: false,
                parse_ok: false,
                perms_600,
                refresh_token_present: false,
                access_token_present: false,
                access_token_expires_at: None,
                access_token_expires_at_valid: None,
                access_token_expired: None,
                refresh_recommended: None,
                user_id_present: false,
            });
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => {
            return Ok(SessionSection {
                auth_file_path,
                exists,
                readable: true,
                parse_ok: false,
                perms_600,
                refresh_token_present: false,
                access_token_present: false,
                access_token_expires_at: None,
                access_token_expires_at_valid: None,
                access_token_expired: None,
                refresh_recommended: None,
                user_id_present: false,
            });
        }
    };

    let refresh_token_present = json_string_field(&parsed, "refresh_token").is_some();
    let access_token_present = json_string_field(&parsed, "access_token").is_some();
    let user_id_present = json_string_field(&parsed, "user_id").is_some();
    let access_token_expires_at = json_string_field(&parsed, "access_token_expires_at");

    let (access_token_expires_at_valid, access_token_expired, refresh_recommended) =
        match access_token_expires_at.as_deref() {
            Some(value) => match DateTime::parse_from_rfc3339(value) {
                Ok(dt) => {
                    let expires_at = dt.with_timezone(&Utc);
                    let expired = expires_at <= Utc::now();
                    let refresh_recommended =
                        expires_at <= Utc::now() + Duration::seconds(REFRESH_AHEAD_SECONDS);
                    (Some(true), Some(expired), Some(refresh_recommended))
                }
                Err(_) => (Some(false), None, None),
            },
            None => (None, None, None),
        };

    Ok(SessionSection {
        auth_file_path,
        exists,
        readable: true,
        parse_ok: true,
        perms_600,
        refresh_token_present,
        access_token_present,
        access_token_expires_at,
        access_token_expires_at_valid,
        access_token_expired,
        refresh_recommended,
        user_id_present,
    })
}

fn inspect_cache_file(path: &Path, items_key: &str) -> CacheFileSection {
    let exists = path.exists();
    let perms_600 = file_permissions_600(path);

    if !exists {
        return CacheFileSection {
            path: path_display(path),
            exists,
            readable: false,
            parse_ok: false,
            perms_600,
            updated_at: None,
            updated_at_valid: None,
            item_count: None,
            parse_error: None,
        };
    }

    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return CacheFileSection {
                path: path_display(path),
                exists,
                readable: false,
                parse_ok: false,
                perms_600,
                updated_at: None,
                updated_at_valid: None,
                item_count: None,
                parse_error: Some(format!("Failed to read: {err}")),
            };
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            return CacheFileSection {
                path: path_display(path),
                exists,
                readable: true,
                parse_ok: false,
                perms_600,
                updated_at: None,
                updated_at_valid: None,
                item_count: None,
                parse_error: Some(format!("Invalid JSON: {err}")),
            };
        }
    };

    let updated_at = json_string_field(&parsed, "updated_at");
    let updated_at_valid = updated_at
        .as_deref()
        .map(|value| DateTime::parse_from_rfc3339(value).is_ok());
    let item_count = parsed
        .get(items_key)
        .and_then(|v| v.as_array())
        .map(|items| items.len());

    CacheFileSection {
        path: path_display(path),
        exists,
        readable: true,
        parse_ok: true,
        perms_600,
        updated_at,
        updated_at_valid,
        item_count,
        parse_error: None,
    }
}

fn add_session_findings(session: &SessionSection, findings: &mut Vec<DoctorFinding>) {
    if !session.exists {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "session_missing".to_string(),
            message: format!("Session file not found: {}", session.auth_file_path),
            hint: Some("Run `cap login` to create a session.".to_string()),
        });
        return;
    }

    if !session.readable {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Error,
            code: "session_unreadable".to_string(),
            message: "Session file exists but cannot be read.".to_string(),
            hint: Some("Check file permissions and ownership.".to_string()),
        });
        return;
    }

    if !session.parse_ok {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "session_invalid_json".to_string(),
            message: "Session file is not valid JSON.".to_string(),
            hint: Some("Backup then remove ~/.capmind/auth.json and run `cap login`.".to_string()),
        });
        return;
    }

    if session.perms_600 == Some(false) {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "session_permissions_weak".to_string(),
            message: "Session file permissions are not 0600.".to_string(),
            hint: Some("Run `chmod 600 ~/.capmind/auth.json`.".to_string()),
        });
    }
    if !session.refresh_token_present {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "refresh_token_missing".to_string(),
            message: "Session file is missing `refresh_token`.".to_string(),
            hint: Some("Run `cap login`.".to_string()),
        });
    }
    if !session.access_token_present {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            code: "access_token_missing".to_string(),
            message: "Session file is missing `access_token`.".to_string(),
            hint: Some("Re-login to refresh local session fields.".to_string()),
        });
    }
    if session.access_token_expires_at_valid == Some(false) {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: "access_token_expiry_invalid".to_string(),
            message: "Session has invalid `access_token_expires_at` format.".to_string(),
            hint: Some("Run `cap login` to rewrite session metadata.".to_string()),
        });
    }
}

fn add_cache_findings(
    section: &CacheFileSection,
    code_prefix: &str,
    findings: &mut Vec<DoctorFinding>,
) {
    if !section.exists || !section.readable {
        return;
    }
    if !section.parse_ok {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Warning,
            code: format!("{code_prefix}_invalid_json"),
            message: format!("Cache file is invalid JSON: {}", section.path),
            hint: Some(format!("Backup then remove {}.", section.path)),
        });
    }
    if section.perms_600 == Some(false) {
        findings.push(DoctorFinding {
            severity: FindingSeverity::Info,
            code: format!("{code_prefix}_permissions_weak"),
            message: format!("Cache file permissions are not 0600: {}", section.path),
            hint: Some(format!("Run `chmod 600 {}`.", section.path)),
        });
    }
}

fn summary_from_findings(findings: &[DoctorFinding]) -> &'static str {
    if findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Error)
    {
        return "error";
    }
    if findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Warning)
    {
        return "warn";
    }
    "ok"
}

fn severity_rank(severity: FindingSeverity) -> u8 {
    match severity {
        FindingSeverity::Error => 3,
        FindingSeverity::Warning => 2,
        FindingSeverity::Info => 1,
    }
}

fn capmind_home_file(name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".capmind").join(name)
}

fn path_display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

fn run_command(program: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|err| err.to_string())
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match (stdout.trim().is_empty(), stderr.trim().is_empty()) {
        (false, false) => format!("{}\n{}", stdout.trim(), stderr.trim()),
        (false, true) => stdout.trim().to_string(),
        (true, false) => stderr.trim().to_string(),
        (true, true) => String::new(),
    }
}

fn compact_command_output(body: &str) -> String {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return "No command output available.".to_string();
    }
    let start = lines.len().saturating_sub(6);
    lines[start..].join("\n")
}

fn which_in_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_var) {
        let candidate = entry.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn read_link_absolute(path: &Path) -> Result<PathBuf, std::io::Error> {
    let target = fs::read_link(path)?;
    if target.is_absolute() {
        return Ok(target);
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    Ok(parent.join(target))
}

fn file_permissions_600(path: &Path) -> Option<bool> {
    if !path.exists() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).ok()?;
        let mode = metadata.permissions().mode() & 0o777;
        Some(mode == 0o600)
    }
    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DoctorFinding, DoctorReport, FindingSeverity, InstallSection, ReleaseSection,
        summary_from_findings,
    };
    use crate::doctor::{
        CacheFileSection, CacheSection, HomebrewSection, SessionSection, render_doctor_json,
        render_doctor_text,
    };
    use crate::update::InstallSource;

    fn sample_report() -> DoctorReport {
        DoctorReport {
            generated_at: "2026-03-04T00:00:00Z".to_string(),
            status_summary: "warn".to_string(),
            install: InstallSection {
                install_source: InstallSource::Homebrew,
                current_version: "0.4.0".to_string(),
                current_version_detected_from_cap: true,
                cap_path: Some("/opt/homebrew/bin/cap".to_string()),
                cap_symlink_target: None,
                current_exe: Some("/opt/homebrew/bin/cap".to_string()),
            },
            release: ReleaseSection {
                latest_tag: Some("capmind-v0.5.0".to_string()),
                latest_version: Some("0.5.0".to_string()),
                check_error: None,
            },
            homebrew: HomebrewSection {
                brew_in_path: true,
                brew_version: Some("Homebrew 4.4.0".to_string()),
                brew_prefix: Some("/opt/homebrew".to_string()),
                formula_installed: Some(true),
                formula_check_error: None,
            },
            session: SessionSection {
                auth_file_path: "/Users/test/.capmind/auth.json".to_string(),
                exists: true,
                readable: true,
                parse_ok: true,
                perms_600: Some(true),
                refresh_token_present: true,
                access_token_present: true,
                access_token_expires_at: Some("2026-03-04T10:00:00Z".to_string()),
                access_token_expires_at_valid: Some(true),
                access_token_expired: Some(false),
                refresh_recommended: Some(false),
                user_id_present: true,
            },
            cache: CacheSection {
                memo_list: CacheFileSection {
                    path: "/Users/test/.capmind/memo-list-cache.json".to_string(),
                    exists: true,
                    readable: true,
                    parse_ok: true,
                    perms_600: Some(true),
                    updated_at: Some("2026-03-04T09:00:00Z".to_string()),
                    updated_at_valid: Some(true),
                    item_count: Some(10),
                    parse_error: None,
                },
                pending_submit: CacheFileSection {
                    path: "/Users/test/.capmind/pending-submit-cache.json".to_string(),
                    exists: false,
                    readable: false,
                    parse_ok: false,
                    perms_600: None,
                    updated_at: None,
                    updated_at_valid: None,
                    item_count: None,
                    parse_error: None,
                },
            },
            findings: vec![DoctorFinding {
                severity: FindingSeverity::Warning,
                code: "newer_release_available".to_string(),
                message: "Current version is behind latest.".to_string(),
                hint: Some("Run `brew upgrade capmind`.".to_string()),
            }],
        }
    }

    #[test]
    fn summary_is_error_when_error_finding_exists() {
        let findings = vec![
            DoctorFinding {
                severity: FindingSeverity::Info,
                code: "i".to_string(),
                message: "info".to_string(),
                hint: None,
            },
            DoctorFinding {
                severity: FindingSeverity::Error,
                code: "e".to_string(),
                message: "error".to_string(),
                hint: None,
            },
        ];
        assert_eq!(summary_from_findings(&findings), "error");
    }

    #[test]
    fn render_text_contains_sections() {
        let text = render_doctor_text(&sample_report());
        assert!(text.contains("[Install]"));
        assert!(text.contains("[Release]"));
        assert!(text.contains("[Homebrew]"));
        assert!(text.contains("[Session]"));
        assert!(text.contains("[Cache]"));
        assert!(text.contains("[Findings]"));
    }

    #[test]
    fn render_json_contains_status_summary() {
        let json = render_doctor_json(&sample_report()).expect("json should render");
        assert!(json.contains("\"status_summary\": \"warn\""));
        assert!(json.contains("\"latest_tag\": \"capmind-v0.5.0\""));
    }
}
