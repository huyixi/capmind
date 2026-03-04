use chrono::{DateTime, Duration, Utc};

use crate::cli::{prompt_email, prompt_password};
use crate::error::AppError;
use crate::session_store::{
    clear_saved_session, load_access_token, load_access_token_expires_at, load_refresh_token,
    save_session,
};
use crate::supabase::{
    Session, SupabaseClient, extract_expires_at_from_access_token,
    extract_user_id_from_access_token,
};

const REFRESH_AHEAD_SECONDS: i64 = 300;
const AUTH_REQUIRED_MESSAGE: &str = "You are not logged in. Run `cap login`.";

pub async fn authenticate_with_stored_token(client: &SupabaseClient) -> Result<Session, AppError> {
    let refresh_token = match load_refresh_token() {
        Ok(Some(value)) => value,
        Ok(None) | Err(_) => return Err(auth_required()),
    };

    let cached_session = load_cached_access_session(&refresh_token);
    if let Some(session) = cached_session.clone()
        && !needs_refresh(session.expires_at)
    {
        return Ok(session);
    }

    match client.refresh_session(&refresh_token).await {
        Ok(session) => {
            persist_session(&session);
            Ok(session)
        }
        Err(_) => {
            if let Some(session) = cached_session
                && is_still_valid(session.expires_at)
            {
                eprintln!("Warning: refresh failed; using cached access token until expiration.");
                return Ok(session);
            }
            Err(auth_required())
        }
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

    if let Err(err) = save_session(
        &session.refresh_token,
        Some(&session.access_token),
        Some(&session.expires_at.to_rfc3339()),
        user_id.as_deref(),
    ) {
        eprintln!("Warning: failed to persist session token to ~/.capmind/auth.json: {err}");
    }
}

fn load_cached_access_session(refresh_token: &str) -> Option<Session> {
    let access_token = match load_access_token() {
        Ok(Some(token)) => token,
        Ok(None) => return None,
        Err(err) => {
            eprintln!("Warning: failed to read cached access token: {err}");
            return None;
        }
    };

    let expires_at = match load_access_token_expires_at() {
        Ok(Some(raw)) => match parse_expires_at(&raw) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Warning: invalid cached access token expiry: {err}");
                return None;
            }
        },
        Ok(None) => match extract_expires_at_from_access_token(&access_token) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Warning: failed to derive access token expiry from JWT: {err}");
                return None;
            }
        },
        Err(err) => {
            eprintln!("Warning: failed to read cached access token expiry: {err}");
            return None;
        }
    };

    Some(Session {
        access_token,
        refresh_token: refresh_token.to_string(),
        expires_at,
    })
}

fn parse_expires_at(raw: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|err| AppError::Auth(format!("Invalid cached expires_at: {err}")))
}

fn needs_refresh(expires_at: DateTime<Utc>) -> bool {
    let refresh_at = Utc::now() + Duration::seconds(REFRESH_AHEAD_SECONDS);
    expires_at <= refresh_at
}

fn is_still_valid(expires_at: DateTime<Utc>) -> bool {
    expires_at > Utc::now()
}

fn auth_required() -> AppError {
    AppError::Auth(AUTH_REQUIRED_MESSAGE.to_string())
}
