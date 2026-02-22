use crate::cli::{prompt_email, prompt_password};
use crate::error::AppError;
use crate::session_store::{load_refresh_token, save_refresh_token};
use crate::supabase::{Session, SupabaseClient};

pub async fn authenticate_with_stored_token(client: &SupabaseClient) -> Result<Session, AppError> {
    match load_refresh_token() {
        Ok(Some(refresh_token)) => match client.refresh_session(&refresh_token).await {
            Ok(session) => {
                if let Err(err) = save_refresh_token(&session.refresh_token) {
                    eprintln!("Warning: failed to persist refreshed session token: {err}");
                }
                Ok(session)
            }
            Err(err) => Err(AppError::Auth(format!(
                "Saved token is invalid ({err}). Run `cap login` to re-authenticate."
            ))),
        },
        Ok(None) => Err(AppError::Auth(
            "No saved token found. Run `cap login` first.".to_string(),
        )),
        Err(err) => Err(AppError::Auth(format!(
            "Failed to load saved token ({err}). Run `cap login`."
        ))),
    }
}

pub async fn login_interactive(client: &SupabaseClient) -> Result<Session, AppError> {
    let email = prompt_email()?;
    let password = prompt_password()?;
    let session = client.login_with_password(&email, &password).await?;
    if let Err(err) = save_refresh_token(&session.refresh_token) {
        eprintln!("Warning: failed to persist session token to ~/.capmind/auth.json: {err}");
    }
    Ok(session)
}
