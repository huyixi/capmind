use crate::cli::{prompt_email, prompt_password};
use crate::error::AppError;
use crate::session_store::{clear_saved_session, load_refresh_token, save_refresh_token};
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
            Err(_) => Err(AppError::Auth(
                "You are not logged in. Run `cap login`.".to_string(),
            )),
        },
        Ok(None) => Err(AppError::Auth(
            "You are not logged in. Run `cap login`.".to_string(),
        )),
        Err(_) => Err(AppError::Auth(
            "You are not logged in. Run `cap login`.".to_string(),
        )),
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

pub fn logout() -> Result<bool, AppError> {
    clear_saved_session()
        .map_err(|err| AppError::Auth(format!("Failed to clear saved session: {err}")))
}
