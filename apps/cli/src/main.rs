mod auth;
mod cli;
mod doctor;
mod env_loader;
mod error;
mod export_selector;
mod memo_export;
mod pending_submit_cache_store;
mod self_update;
mod session_store;
mod submission;
mod supabase;
mod tui;
mod update;

use chrono::{Local, Utc};
use clap::Parser;
use cli::{Cli, Commands, resolve_text, rewrite_shortcut_args};
use std::io::{self, IsTerminal};

use crate::auth::{authenticate_with_stored_token, login_interactive, logout};
use crate::doctor::{render_doctor_json, render_doctor_text, run_doctor};
use crate::export_selector::prompt_export_range;
use crate::memo_export::{
    ExportRangePreset, build_export_payload, date_range_for_preset, next_export_file_path,
    write_export_file,
};
use crate::submission::submit_memo;
use crate::supabase::SupabaseClient;
use crate::update::{render_update_result, render_update_result_json, run_update};

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
            tui::run(&client).await?;
        }
        Commands::List => {
            let client = SupabaseClient::from_env()?;
            tui::run_list(&client).await?;
        }
        Commands::Doctor(args) => {
            let report = run_doctor(&args).await?;
            if args.json {
                println!("{}", render_doctor_json(&report)?);
            } else {
                println!("{}", render_doctor_text(&report));
            }
        }
        Commands::Update(args) => {
            let result = run_update(&args).await?;
            if args.json {
                println!("{}", render_update_result_json(&result)?);
            } else {
                println!("{}", render_update_result(&result));
            }
        }
    }

    Ok(())
}
