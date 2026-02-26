mod auth;
mod cli;
mod env_loader;
mod error;
mod self_update;
mod session_store;
mod submission;
mod supabase;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, resolve_text, rewrite_shortcut_args};

use crate::auth::login_interactive;
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
        Commands::Add(args) => {
            let client = SupabaseClient::from_env()?;
            let text = resolve_text(args.text.clone())?;
            let submitted = submit_memo(&client, &text).await?;
            println!(
                "Inserted memo successfully\nmemo_id: {}\ncreated_at: {}\nexpires_at: {}",
                submitted.inserted.id, submitted.inserted.created_at, submitted.session.expires_at
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
