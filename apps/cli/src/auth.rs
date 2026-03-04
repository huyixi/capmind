use crate::cli::{prompt_email, prompt_password};
use crate::error::AppError;
use crate::session_store::{clear_saved_session, load_refresh_token, save_session};
use crate::supabase::{Session, SupabaseClient, extract_user_id_from_access_token};

pub async fn authenticate_with_stored_token(client: &SupabaseClient) -> Result<Session, AppError> {
    match load_refresh_token() {
        Ok(Some(refresh_token)) => match client.refresh_session(&refresh_token).await {
            Ok(session) => {
                persist_session(&session);
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
    persist_session(&session);
    Ok(session)
}

pub fn logout() -> Result<bool, AppError> {
    clear_saved_session()
        .map_err(|err| AppError::Auth(format!("Failed to clear saved session: {err}")))
}

fn persist_session(session: &Session) {
    let user_id = match extract_user_id_from_access_token(&session.access_token) {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("Warning: failed to parse user id from session token: {err}");
            None
        }
    };

    if let Err(err) = save_session(&session.refresh_token, user_id.as_deref()) {
        eprintln!("Warning: failed to persist session token to ~/.capmind/auth.json: {err}");
    }
}
