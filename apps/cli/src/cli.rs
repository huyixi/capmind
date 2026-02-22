use clap::{Args, Parser, Subcommand};
use std::io::{self, IsTerminal, Read, Write};

use crate::error::AppError;

#[derive(Debug, Parser)]
#[command(name = "cap-cli", version, about = "Insert memos into Supabase")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Add(AddArgs),
    Login,
    Compose,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[arg(long)]
    pub text: Option<String>,
}

pub fn rewrite_shortcut_args<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let argv: Vec<String> = args.into_iter().collect();
    if argv.len() == 1 {
        return vec![argv[0].clone(), "compose".to_string()];
    }

    if argv.len() != 2 {
        return argv;
    }

    let first = argv[1].as_str();
    if first.starts_with('-') {
        return argv;
    }
    if matches!(first, "add" | "login" | "help" | "compose") {
        return argv;
    }

    vec![
        argv[0].clone(),
        "add".to_string(),
        "--text".to_string(),
        argv[1].clone(),
    ]
}

pub fn resolve_text(input: Option<String>) -> Result<String, AppError> {
    let raw = if let Some(value) = input {
        value
    } else {
        let mut stdin = io::stdin();
        if stdin.is_terminal() {
            return Err(AppError::InvalidInput(
                "Missing memo text. Pass --text or pipe stdin.".to_string(),
            ));
        }
        let mut buffer = String::new();
        stdin.read_to_string(&mut buffer).map_err(|err| {
            AppError::InvalidInput(format!("Failed reading stdin for memo text: {err}"))
        })?;
        buffer
    };

    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "Memo text is empty after trimming.".to_string(),
        ));
    }
    if trimmed.len() > 20_000 {
        return Err(AppError::InvalidInput(
            "Memo text is too long (max 20000 characters).".to_string(),
        ));
    }

    Ok(trimmed)
}

pub fn prompt_email() -> Result<String, AppError> {
    ensure_interactive()?;
    prompt("Supabase email: ")
}

pub fn prompt_password() -> Result<String, AppError> {
    ensure_interactive()?;
    rpassword::prompt_password("Supabase password: ")
        .map_err(|err| AppError::InvalidInput(format!("Failed reading password input: {err}")))
}

fn ensure_interactive() -> Result<(), AppError> {
    if io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(AppError::InvalidInput(
            "Interactive login is required. Run `cap login` in a terminal.".to_string(),
        ))
    }
}

fn prompt(label: &str) -> Result<String, AppError> {
    let mut stdout = io::stdout();
    stdout
        .write_all(label.as_bytes())
        .map_err(|err| AppError::InvalidInput(format!("Failed writing prompt: {err}")))?;
    stdout
        .flush()
        .map_err(|err| AppError::InvalidInput(format!("Failed flushing prompt: {err}")))?;

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| AppError::InvalidInput(format!("Failed reading input: {err}")))?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("Email cannot be empty.".to_string()));
    }

    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::rewrite_shortcut_args;

    #[test]
    fn rewrites_single_text_shortcut() {
        let input = vec!["cap".to_string(), "hello".to_string()];
        let output = rewrite_shortcut_args(input);
        assert_eq!(output, vec!["cap", "add", "--text", "hello"]);
    }

    #[test]
    fn rewrites_no_arg_to_compose() {
        let input = vec!["cap".to_string()];
        let output = rewrite_shortcut_args(input);
        assert_eq!(output, vec!["cap", "compose"]);
    }

    #[test]
    fn does_not_rewrite_known_subcommands() {
        let input = vec!["cap".to_string(), "login".to_string()];
        let output = rewrite_shortcut_args(input);
        assert_eq!(output, vec!["cap", "login"]);
    }

    #[test]
    fn does_not_rewrite_flags() {
        let input = vec!["cap".to_string(), "--help".to_string()];
        let output = rewrite_shortcut_args(input);
        assert_eq!(output, vec!["cap", "--help"]);
    }

    #[test]
    fn does_not_rewrite_multiple_positionals() {
        let input = vec!["cap".to_string(), "hello".to_string(), "world".to_string()];
        let output = rewrite_shortcut_args(input);
        assert_eq!(output, vec!["cap", "hello", "world"]);
    }
}
