use std::process::{Command, Output};

use crate::error::AppError;

pub fn run_update_flow(formula_name: &str) -> Result<(), AppError> {
    run_brew_command(&["update"])?;
    run_brew_command(&["upgrade", formula_name])?;
    Ok(())
}

fn run_brew_command(args: &[&str]) -> Result<(), AppError> {
    let output = Command::new("brew").args(args).output().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::Api(
                "Homebrew-managed install detected, but `brew` is unavailable in PATH. Run `brew upgrade capmind`."
                    .to_string(),
            )
        } else {
            AppError::Api(format!("Failed to run `brew {}`: {err}", args.join(" ")))
        }
    })?;

    if output.status.success() {
        return Ok(());
    }

    let body = compact_command_output(&output_text(&output));
    Err(AppError::Api(format!(
        "Homebrew command failed: `brew {}`\n{}",
        args.join(" "),
        body
    )))
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
    let start = lines.len().saturating_sub(8);
    lines[start..].join("\n")
}
