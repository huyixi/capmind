use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::env_loader;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InsertedMemo {
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct RecentMemo {
    pub text: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SupabaseClient {
    base_url: String,
    anon_key: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct AuthTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

#[derive(Debug, Serialize)]
struct InsertMemoRequest<'a> {
    user_id: &'a str,
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct InsertMemoResponse {
    id: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct ListMemoResponse {
    text: String,
    created_at: String,
}

impl SupabaseClient {
    pub fn from_env() -> Result<Self, AppError> {
        let base_url = env_loader::get_value(&["SUPABASE_URL", "NEXT_PUBLIC_SUPABASE_URL"])
            .ok_or_else(|| {
                AppError::MissingEnv(
                    "Missing env: SUPABASE_URL (or NEXT_PUBLIC_SUPABASE_URL)".to_string(),
                )
            })?;
        let anon_key =
            env_loader::get_value(&["SUPABASE_ANON_KEY", "NEXT_PUBLIC_SUPABASE_ANON_KEY"])
                .ok_or_else(|| {
                    AppError::MissingEnv(
                        "Missing env: SUPABASE_ANON_KEY (or NEXT_PUBLIC_SUPABASE_ANON_KEY)"
                            .to_string(),
                    )
                })?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|err| AppError::Network(format!("Failed to build HTTP client: {err}")))?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            anon_key,
            http,
        })
    }

    pub async fn login_with_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Session, AppError> {
        let endpoint = format!("{}/auth/v1/token?grant_type=password", self.base_url);
        let response = self
            .http
            .post(endpoint)
            .header("apikey", &self.anon_key)
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await
            .map_err(|err| AppError::Network(format!("Supabase auth request failed: {err}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            return Err(AppError::Auth(format!(
                "Supabase auth failed ({status}): {body}"
            )));
        }

        let data: AuthTokenResponse = response
            .json()
            .await
            .map_err(|err| AppError::Auth(format!("Invalid auth response JSON: {err}")))?;
        Ok(to_session(data))
    }

    pub async fn refresh_session(&self, refresh_token: &str) -> Result<Session, AppError> {
        let endpoint = format!("{}/auth/v1/token?grant_type=refresh_token", self.base_url);
        let response = self
            .http
            .post(endpoint)
            .header("apikey", &self.anon_key)
            .json(&serde_json::json!({ "refresh_token": refresh_token }))
            .send()
            .await
            .map_err(|err| AppError::Network(format!("Supabase refresh request failed: {err}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            return Err(AppError::Auth(format!(
                "Supabase token refresh failed ({status}): {body}"
            )));
        }

        let data: AuthTokenResponse = response
            .json()
            .await
            .map_err(|err| AppError::Auth(format!("Invalid refresh response JSON: {err}")))?;
        Ok(to_session(data))
    }

    pub async fn insert_memo(
        &self,
        access_token: &str,
        text: &str,
    ) -> Result<InsertedMemo, AppError> {
        let user_id = extract_user_id_from_jwt(access_token)?;
        let endpoint = format!("{}/rest/v1/memos", self.base_url);
        let payload = InsertMemoRequest {
            user_id: &user_id,
            text,
        };

        let response = self
            .http
            .post(endpoint)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Prefer", "return=representation")
            .json(&payload)
            .send()
            .await
            .map_err(|err| AppError::Network(format!("Supabase insert request failed: {err}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("Insert memo failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<InsertMemoResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid insert response JSON: {err}")))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Api("Insert response is empty".to_string()))?;

        Ok(InsertedMemo {
            id: row.id,
            created_at: row.created_at,
        })
    }

    pub async fn list_recent_memos(
        &self,
        access_token: &str,
        limit: usize,
    ) -> Result<Vec<RecentMemo>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let endpoint = format!(
            "{}/rest/v1/memos?select=text,created_at&order=created_at.desc&limit={limit}",
            self.base_url
        );
        let response = self
            .http
            .get(endpoint)
            .header("apikey", &self.anon_key)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|err| AppError::Network(format!("Supabase list request failed: {err}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("List memos failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<ListMemoResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid list response JSON: {err}")))?;
        Ok(rows
            .into_iter()
            .map(|row| RecentMemo {
                text: row.text,
                created_at: row.created_at,
            })
            .collect())
    }
}

fn to_session(value: AuthTokenResponse) -> Session {
    let now = Utc::now();
    let ttl = value.expires_in.max(0);
    let expires_at = now + Duration::seconds(ttl);
    Session {
        access_token: value.access_token,
        refresh_token: value.refresh_token,
        expires_at,
    }
}

async fn read_error_body(response: reqwest::Response) -> String {
    match response.text().await {
        Ok(body) if !body.trim().is_empty() => body,
        Ok(_) => "empty error body".to_string(),
        Err(err) => format!("unable to read error body: {err}"),
    }
}

fn extract_user_id_from_jwt(access_token: &str) -> Result<String, AppError> {
    let mut parts = access_token.split('.');
    let _header = parts
        .next()
        .ok_or_else(|| AppError::Auth("Invalid access token format".to_string()))?;
    let payload = parts
        .next()
        .ok_or_else(|| AppError::Auth("Invalid access token format".to_string()))?;

    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| {
            let mut padded = payload.to_string();
            while !padded.len().is_multiple_of(4) {
                padded.push('=');
            }
            URL_SAFE.decode(padded)
        })
        .map_err(|err| AppError::Auth(format!("Failed to decode access token payload: {err}")))?;

    let value: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|err| AppError::Auth(format!("Invalid access token JSON payload: {err}")))?;
    let sub = value
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Auth("Access token payload missing `sub`".to_string()))?;

    Ok(sub.to_string())
}

#[cfg(test)]
mod tests {
    use super::extract_user_id_from_jwt;

    #[test]
    fn extract_sub_from_jwt_payload() {
        let token = "a.eyJzdWIiOiIxMjM0In0.b";
        let user_id = extract_user_id_from_jwt(token).expect("should parse sub");
        assert_eq!(user_id, "1234");
    }
}
