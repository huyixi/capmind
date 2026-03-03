use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::domain::InstallSource;

const DEFAULT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn detect_install_source(formula_name: &str) -> InstallSource {
    let cap_path = which_in_path("cap");
    let cap_symlink_target = cap_path
        .as_ref()
        .and_then(|path| read_link_absolute(path).ok());
    let current_exe = std::env::current_exe().ok();
    let brew_formula_installed = brew_formula_installed(formula_name);
    let homebrew_cellar_exists = Path::new("/opt/homebrew/Cellar")
        .join(formula_name)
        .exists()
        || Path::new("/usr/local/Cellar").join(formula_name).exists();

    detect_install_source_from_signals(
        cap_path.as_deref(),
        cap_symlink_target.as_deref(),
        current_exe.as_deref(),
        formula_name,
        brew_formula_installed,
        homebrew_cellar_exists,
    )
}

pub fn detect_current_reported_version() -> String {
    detect_current_reported_version_opt().unwrap_or_else(|| DEFAULT_VERSION.to_string())
}

pub fn detect_current_reported_version_opt() -> Option<String> {
    if let Ok(output) = Command::new("cap").arg("-V").output()
        && output.status.success()
        && let Some(version) = parse_version_from_cap_output(&output_text(&output))
    {
        return Some(version);
    }

    let current_exe = std::env::current_exe().ok()?;
    let output = Command::new(current_exe).arg("-V").output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_version_from_cap_output(&output_text(&output))
}

fn detect_install_source_from_signals(
    cap_path: Option<&Path>,
    cap_symlink_target: Option<&Path>,
    current_exe: Option<&Path>,
    formula_name: &str,
    brew_formula_installed: bool,
    homebrew_cellar_exists: bool,
) -> InstallSource {
    let has_cellar_signal = cap_path
        .is_some_and(|path| path_has_homebrew_cellar_formula(path, formula_name))
        || cap_symlink_target
            .is_some_and(|path| path_has_homebrew_cellar_formula(path, formula_name))
        || current_exe.is_some_and(|path| path_has_homebrew_cellar_formula(path, formula_name));
    if has_cellar_signal {
        return InstallSource::Homebrew;
    }

    let cap_homebrew_bin = cap_path.is_some_and(path_is_homebrew_bin_cap);
    if cap_homebrew_bin && (brew_formula_installed || homebrew_cellar_exists) {
        return InstallSource::Homebrew;
    }

    if cap_path.is_some() || current_exe.is_some() {
        return InstallSource::Standalone;
    }

    InstallSource::Unknown
}

fn parse_version_from_cap_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        let maybe = if let Some(rest) = trimmed.strip_prefix("cap ") {
            rest.split_whitespace().next().map(|v| v.to_string())
        } else {
            trimmed.split_whitespace().nth(1).map(|v| v.to_string())
        };
        if let Some(version) = maybe
            && looks_like_version_token(&version)
        {
            return Some(version);
        }
    }
    None
}

fn looks_like_version_token(token: &str) -> bool {
    let mut saw_digit = false;
    let mut saw_dot = false;
    for c in token.chars() {
        if c.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if c == '.' {
            saw_dot = true;
            continue;
        }
        return false;
    }
    saw_digit && saw_dot
}

fn path_is_homebrew_bin_cap(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("/opt/homebrew/bin/cap") | Some("/usr/local/bin/cap")
    )
}

fn path_has_homebrew_cellar_formula(path: &Path, formula_name: &str) -> bool {
    path.to_string_lossy()
        .contains(&format!("/Cellar/{formula_name}/"))
}

fn read_link_absolute(path: &Path) -> Result<PathBuf, std::io::Error> {
    let target = fs::read_link(path)?;
    if target.is_absolute() {
        return Ok(target);
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    Ok(parent.join(target))
}

fn brew_formula_installed(formula: &str) -> bool {
    let output = Command::new("brew").args(["list", "--formula"]).output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    output_text(&output)
        .lines()
        .any(|line| line.trim() == formula)
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

#[cfg(test)]
mod tests {
    use super::{
        InstallSource, detect_install_source_from_signals, parse_version_from_cap_output,
        path_has_homebrew_cellar_formula, path_is_homebrew_bin_cap,
    };
    use std::path::Path;

    #[test]
    fn detects_homebrew_by_cellar_symlink_target() {
        let source = detect_install_source_from_signals(
            Some(Path::new("/opt/homebrew/bin/cap")),
            Some(Path::new("/opt/homebrew/Cellar/capmind/0.2.10/bin/cap")),
            Some(Path::new("/opt/homebrew/Cellar/capmind/0.2.10/bin/cap")),
            "capmind",
            true,
            true,
        );
        assert_eq!(source, InstallSource::Homebrew);
    }

    #[test]
    fn detects_homebrew_when_bin_path_is_brew_prefix_and_formula_exists() {
        let source = detect_install_source_from_signals(
            Some(Path::new("/opt/homebrew/bin/cap")),
            None,
            Some(Path::new("/opt/homebrew/bin/cap")),
            "capmind",
            true,
            true,
        );
        assert_eq!(source, InstallSource::Homebrew);
    }

    #[test]
    fn defaults_to_standalone_when_no_homebrew_signals() {
        let source = detect_install_source_from_signals(
            Some(Path::new("/Users/me/.local/bin/cap")),
            None,
            Some(Path::new("/Users/me/.local/bin/cap")),
            "capmind",
            false,
            false,
        );
        assert_eq!(source, InstallSource::Standalone);
    }

    #[test]
    fn detects_unknown_when_no_path_signals() {
        let source = detect_install_source_from_signals(None, None, None, "capmind", false, false);
        assert_eq!(source, InstallSource::Unknown);
    }

    #[test]
    fn parses_version_from_standard_output() {
        let parsed = parse_version_from_cap_output("cap 0.2.11");
        assert_eq!(parsed.as_deref(), Some("0.2.11"));
    }

    #[test]
    fn parses_version_from_multiline_output() {
        let parsed = parse_version_from_cap_output("\ncap 1.4.0\n");
        assert_eq!(parsed.as_deref(), Some("1.4.0"));
    }

    #[test]
    fn homebrew_bin_detection_is_precise() {
        assert!(path_is_homebrew_bin_cap(Path::new("/opt/homebrew/bin/cap")));
        assert!(path_is_homebrew_bin_cap(Path::new("/usr/local/bin/cap")));
        assert!(!path_is_homebrew_bin_cap(Path::new("/tmp/cap")));
    }

    #[test]
    fn cellar_path_detection_requires_formula_name() {
        assert!(path_has_homebrew_cellar_formula(
            Path::new("/opt/homebrew/Cellar/capmind/0.2.10/bin/cap"),
            "capmind"
        ));
        assert!(!path_has_homebrew_cellar_formula(
            Path::new("/opt/homebrew/Cellar/other/1.0.0/bin/other"),
            "capmind"
        ));
    }
}
