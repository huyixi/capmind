mod auth;
mod cli;
mod env_loader;
mod error;
mod export_selector;
mod memo_export;
mod self_update;
mod session_store;
mod submission;
mod supabase;
mod tui;

use chrono::{Local, Utc};
use clap::Parser;
use cli::{Cli, Commands, resolve_text, rewrite_shortcut_args};
use std::io::{self, IsTerminal};

use crate::auth::{authenticate_with_stored_token, login_interactive, logout};
use crate::export_selector::prompt_export_range;
use crate::memo_export::{
    ExportRangePreset, build_export_payload, date_range_for_preset, next_export_file_path,
    write_export_file,
};
use crate::self_update::{SelfUpdateOutcome, run_self_update};
use crate::submission::submit_memo;
use crate::supabase::SupabaseClient;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(err.exit_code());
    }
}

async fn run() -> Result<(), error::AppError> {
    env_loader::init();

    let cli = Cli::parse_from(rewrite_shortcut_args(std::env::args()));

    match cli.command {
        Commands::Login => {
            let client = SupabaseClient::from_env()?;
            let session = login_interactive(&client).await?;
            println!(
                "Login successful\nexpires_at: {}\nstorage: ~/.capmind/auth.json",
                session.expires_at
            );
        }
        Commands::Logout => {
            let removed = logout()?;
            if removed {
                println!("Logout successful\nstorage: ~/.capmind/auth.json");
            } else {
                println!("Already logged out");
            }
        }
        Commands::Add(args) => {
            let client = SupabaseClient::from_env()?;
            let text = resolve_text(args.text.clone())?;
            let submitted = submit_memo(&client, &text).await?;
            println!(
                "Inserted memo successfully\nmemo_id: {}\ncreated_at: {}\nexpires_at: {}",
                submitted.inserted.id, submitted.inserted.created_at, submitted.session.expires_at
            );
        }
        Commands::Export => {
            let is_interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
            let selected_range = if is_interactive {
                prompt_export_range()?
            } else {
                ExportRangePreset::Last3Days
            };
            let client = SupabaseClient::from_env()?;
            let session = authenticate_with_stored_token(&client).await?;
            let memos = client.list_recent_memos(&session.access_token).await?;
            let date_range = date_range_for_preset(selected_range, Utc::now());
            let payload = build_export_payload(&memos, &date_range);
            let cwd = std::env::current_dir()
                .map_err(|err| error::AppError::Api(format!("Failed to resolve cwd: {err}")))?;
            let output_path = next_export_file_path(&cwd, Local::now())?;
            write_export_file(&payload.text, &output_path)?;
            println!(
                "Exported {} memo(s)\nrange: {}\nfile: {}",
                payload.memo_count,
                selected_range.label(),
                output_path.display()
            );
        }
        Commands::Compose => {
            let client = SupabaseClient::from_env()?;
            ensure_logged_in_or_prompt(&client).await?;
            tui::run(&client).await?;
        }
        Commands::List => {
            let client = SupabaseClient::from_env()?;
            ensure_logged_in_or_prompt(&client).await?;
            tui::run_list(&client).await?;
        }
        Commands::Update(args) => match run_self_update(args.version.as_deref()).await? {
            SelfUpdateOutcome::UpToDate { version } => {
                println!("Already up to date (version {version})");
            }
            SelfUpdateOutcome::Updated {
                from_version,
                to_version,
                tag,
            } => {
                println!(
                    "Update successful\nfrom: {from_version}\nto: {to_version}\nrelease_tag: {tag}"
                );
            }
        },
    }

    Ok(())
}

async fn ensure_logged_in_or_prompt(client: &SupabaseClient) -> Result<(), error::AppError> {
    if authenticate_with_stored_token(client).await.is_ok() {
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        return Err(error::AppError::Auth(
            "You are not logged in. Run `cap login`.".to_string(),
        ));
    }

    println!("You are not logged in.");
    println!("Press Enter to login now (Ctrl+C to cancel).");

    let mut line = String::new();
    io::stdin().read_line(&mut line).map_err(|err| {
        error::AppError::InvalidInput(format!("Failed reading login confirmation: {err}"))
    })?;

    let _ = login_interactive(client).await?;
    println!("Login successful.");
    Ok(())
}
