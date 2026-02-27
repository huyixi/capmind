use base64::Engine;
use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const MEMO_LIST_PAGE_SIZE: usize = 200;
const DEFAULT_SUPABASE_URL: &str = "https://fpeudcmnzirzjjjqtjep.supabase.co";
const DEFAULT_SUPABASE_ANON_KEY: &str = "sb_publishable_m_5H3rQuAMJg2HhL-bWOJQ_MCGlZ4Vv";
const NETWORK_RETRY_DELAYS_MS: [u64; 2] = [250, 1000];

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
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct RecentMemo {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub version: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone)]
pub enum UpdateMemoOutcome {
    Updated(RecentMemo),
    Conflict {
        server_memo: RecentMemo,
        forked: InsertedMemo,
    },
}

#[derive(Debug, Clone)]
pub enum DeleteMemoOutcome {
    Deleted,
    Conflict { server_memo: RecentMemo },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RestoreMemoOutcome {
    Restored(RecentMemo),
    Conflict { server_memo: RecentMemo },
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
    version: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ListMemoResponse {
    id: String,
    text: String,
    created_at: String,
    version: serde_json::Value,
    deleted_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateMemoRpcRequest<'a> {
    arg_memo_id: &'a str,
    arg_text: &'a str,
    arg_expected_version: &'a str,
}

#[derive(Debug, Serialize)]
struct DeleteMemoRpcRequest<'a> {
    arg_memo_id: &'a str,
    arg_expected_version: &'a str,
    arg_deleted_at: &'a str,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct RestoreMemoRpcRequest<'a> {
    arg_memo_id: &'a str,
    arg_expected_version: &'a str,
    arg_restored_at: &'a str,
}

#[derive(Debug, Deserialize)]
struct MemoRowResponse {
    id: String,
    text: String,
    created_at: String,
    version: serde_json::Value,
    deleted_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MemoResolveConflictRpcResponse {
    status: String,
    memo_id: Option<String>,
    memo: Option<serde_json::Value>,
    server_memo: Option<serde_json::Value>,
    forked_memo: Option<serde_json::Value>,
}

impl SupabaseClient {
    pub fn from_env() -> Result<Self, AppError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|err| AppError::Network(format!("Failed to build HTTP client: {err}")))?;

        Ok(Self {
            base_url: DEFAULT_SUPABASE_URL.trim_end_matches('/').to_string(),
            anon_key: DEFAULT_SUPABASE_ANON_KEY.to_string(),
            http,
        })
    }

    pub async fn login_with_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Session, AppError> {
        let endpoint = format!("{}/auth/v1/token?grant_type=password", self.base_url);
        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .json(&serde_json::json!({ "email": email, "password": password })),
            "Supabase auth request failed",
        )
        .await?;

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
        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .json(&serde_json::json!({ "refresh_token": refresh_token })),
            "Supabase refresh request failed",
        )
        .await?;

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

        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}"))
                .header("Prefer", "return=representation")
                .json(&payload),
            "Supabase insert request failed",
        )
        .await?;

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
            version: normalize_version_value(&row.version)?,
        })
    }

    pub async fn list_recent_memos(&self, access_token: &str) -> Result<Vec<RecentMemo>, AppError> {
        let mut memos = Vec::new();
        let mut offset = 0usize;

        loop {
            let page = self
                .list_recent_memos_page(access_token, MEMO_LIST_PAGE_SIZE, offset)
                .await?;
            let page_len = page.len();
            memos.extend(page);
            if !should_fetch_next_page(page_len, MEMO_LIST_PAGE_SIZE) {
                break;
            }
            offset = offset.saturating_add(page_len);
        }

        Ok(memos)
    }

    async fn list_recent_memos_page(
        &self,
        access_token: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecentMemo>, AppError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let endpoint = format!(
            "{}/rest/v1/memos?select=id,text,created_at,version,deleted_at&deleted_at=is.null&order=updated_at.desc&limit={limit}&offset={offset}",
            self.base_url
        );
        let response = send_with_network_retry(
            self.http
                .get(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}")),
            "Supabase list request failed",
        )
        .await?;

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
        rows.into_iter()
            .map(|row| {
                row_to_recent_memo(MemoRowResponse {
                    id: row.id,
                    text: row.text,
                    created_at: row.created_at,
                    version: row.version,
                    deleted_at: row.deleted_at,
                })
            })
            .collect()
    }

    #[allow(dead_code)]
    pub async fn get_memo_by_id(
        &self,
        access_token: &str,
        memo_id: &str,
    ) -> Result<RecentMemo, AppError> {
        let endpoint = format!(
            "{}/rest/v1/memos?select=id,text,created_at,version,deleted_at&id=eq.{memo_id}&limit=1",
            self.base_url
        );
        let response = send_with_network_retry(
            self.http
                .get(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}")),
            "Supabase get request failed",
        )
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("Get memo failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<MemoRowResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid get response JSON: {err}")))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Api("Memo not found".to_string()))?;
        row_to_recent_memo(row)
    }

    pub async fn update_memo(
        &self,
        access_token: &str,
        memo_id: &str,
        text: &str,
        expected_version: &str,
    ) -> Result<UpdateMemoOutcome, AppError> {
        let endpoint = format!("{}/rest/v1/rpc/memo_update_resolve_conflict", self.base_url);
        let payload = UpdateMemoRpcRequest {
            arg_memo_id: memo_id,
            arg_text: text,
            arg_expected_version: expected_version,
        };
        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}"))
                .json(&payload),
            "Supabase update RPC request failed",
        )
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("Update memo RPC failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<MemoResolveConflictRpcResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid update RPC response JSON: {err}")))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Api("Update memo RPC response is empty".to_string()))?;
        let status = row.status.as_str();
        let _memo_id = row.memo_id.as_deref().unwrap_or(memo_id);

        match status {
            "updated" => {
                let memo_json = row.memo.ok_or_else(|| {
                    AppError::Api("Update memo RPC missing `memo` payload".to_string())
                })?;
                let memo_row: MemoRowResponse =
                    serde_json::from_value(memo_json).map_err(|err| {
                        AppError::Api(format!("Invalid update memo payload from RPC: {err}"))
                    })?;
                Ok(UpdateMemoOutcome::Updated(row_to_recent_memo(memo_row)?))
            }
            "conflict" => {
                let server_json = row.server_memo.ok_or_else(|| {
                    AppError::Api("Update memo conflict missing `server_memo` payload".to_string())
                })?;
                let forked_json = row.forked_memo.ok_or_else(|| {
                    AppError::Api("Update memo conflict missing `forked_memo` payload".to_string())
                })?;
                let server_row: MemoRowResponse =
                    serde_json::from_value(server_json).map_err(|err| {
                        AppError::Api(format!(
                            "Invalid update conflict `server_memo` payload: {err}"
                        ))
                    })?;
                let forked_row: InsertMemoResponse =
                    serde_json::from_value(forked_json).map_err(|err| {
                        AppError::Api(format!(
                            "Invalid update conflict `forked_memo` payload: {err}"
                        ))
                    })?;
                Ok(UpdateMemoOutcome::Conflict {
                    server_memo: row_to_recent_memo(server_row)?,
                    forked: InsertedMemo {
                        id: forked_row.id,
                        created_at: forked_row.created_at,
                        version: normalize_version_value(&forked_row.version)?,
                    },
                })
            }
            "not_found" => Err(AppError::Api("Memo not found".to_string())),
            other => Err(AppError::Api(format!(
                "Unexpected update memo RPC status `{other}`"
            ))),
        }
    }

    pub async fn delete_memo(
        &self,
        access_token: &str,
        memo_id: &str,
        expected_version: &str,
    ) -> Result<DeleteMemoOutcome, AppError> {
        let deleted_at = Utc::now().to_rfc3339();
        let payload = DeleteMemoRpcRequest {
            arg_memo_id: memo_id,
            arg_expected_version: expected_version,
            arg_deleted_at: &deleted_at,
        };
        let endpoint = format!("{}/rest/v1/rpc/memo_delete_resolve_conflict", self.base_url);
        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}"))
                .json(&payload),
            "Supabase delete RPC request failed",
        )
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("Delete memo RPC failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<MemoResolveConflictRpcResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid delete RPC response JSON: {err}")))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Api("Delete memo RPC response is empty".to_string()))?;

        match row.status.as_str() {
            "deleted" => Ok(DeleteMemoOutcome::Deleted),
            "conflict" => {
                let server_json = row.server_memo.ok_or_else(|| {
                    AppError::Api("Delete memo conflict missing `server_memo` payload".to_string())
                })?;
                let server_row: MemoRowResponse =
                    serde_json::from_value(server_json).map_err(|err| {
                        AppError::Api(format!(
                            "Invalid delete conflict `server_memo` payload: {err}"
                        ))
                    })?;
                Ok(DeleteMemoOutcome::Conflict {
                    server_memo: row_to_recent_memo(server_row)?,
                })
            }
            "not_found" => Err(AppError::Api("Memo not found".to_string())),
            other => Err(AppError::Api(format!(
                "Unexpected delete memo RPC status `{other}`"
            ))),
        }
    }

    #[allow(dead_code)]
    pub async fn restore_memo(
        &self,
        access_token: &str,
        memo_id: &str,
        expected_version: &str,
    ) -> Result<RestoreMemoOutcome, AppError> {
        let restored_at = Utc::now().to_rfc3339();
        let payload = RestoreMemoRpcRequest {
            arg_memo_id: memo_id,
            arg_expected_version: expected_version,
            arg_restored_at: &restored_at,
        };
        let endpoint = format!(
            "{}/rest/v1/rpc/memo_restore_resolve_conflict",
            self.base_url
        );
        let response = send_with_network_retry(
            self.http
                .post(endpoint)
                .header("apikey", &self.anon_key)
                .header("Authorization", format!("Bearer {access_token}"))
                .json(&payload),
            "Supabase restore RPC request failed",
        )
        .await?;

        let status = response.status();
        if !status.is_success() {
            let body = read_error_body(response).await;
            let message = format!("Restore memo RPC failed ({status}): {body}");
            if status == StatusCode::UNAUTHORIZED {
                return Err(AppError::Auth(message));
            }
            return Err(AppError::Api(message));
        }

        let rows: Vec<MemoResolveConflictRpcResponse> = response
            .json()
            .await
            .map_err(|err| AppError::Api(format!("Invalid restore RPC response JSON: {err}")))?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Api("Restore memo RPC response is empty".to_string()))?;

        match row.status.as_str() {
            "restored" => {
                let memo_json = row.memo.ok_or_else(|| {
                    AppError::Api("Restore memo RPC missing `memo` payload".to_string())
                })?;
                let memo_row: MemoRowResponse =
                    serde_json::from_value(memo_json).map_err(|err| {
                        AppError::Api(format!("Invalid restore memo payload from RPC: {err}"))
                    })?;
                Ok(RestoreMemoOutcome::Restored(row_to_recent_memo(memo_row)?))
            }
            "conflict" => {
                let server_json = row.server_memo.ok_or_else(|| {
                    AppError::Api("Restore memo conflict missing `server_memo` payload".to_string())
                })?;
                let server_row: MemoRowResponse =
                    serde_json::from_value(server_json).map_err(|err| {
                        AppError::Api(format!(
                            "Invalid restore conflict `server_memo` payload: {err}"
                        ))
                    })?;
                Ok(RestoreMemoOutcome::Conflict {
                    server_memo: row_to_recent_memo(server_row)?,
                })
            }
            "not_found" => Err(AppError::Api("Memo not found".to_string())),
            other => Err(AppError::Api(format!(
                "Unexpected restore memo RPC status `{other}`"
            ))),
        }
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

async fn send_with_network_retry(
    request: reqwest::RequestBuilder,
    context: &str,
) -> Result<reqwest::Response, AppError> {
    let attempts = NETWORK_RETRY_DELAYS_MS.len() + 1;
    let mut last_error: Option<reqwest::Error> = None;

    for attempt in 0..attempts {
        let cloned = request.try_clone().ok_or_else(|| {
            AppError::Network(format!(
                "{context}: failed to clone request for retry"
            ))
        })?;
        match cloned.send().await {
            Ok(response) => return Ok(response),
            Err(err) => {
                last_error = Some(err);
                if let Some(delay_ms) = NETWORK_RETRY_DELAYS_MS.get(attempt) {
                    tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
                }
            }
        }
    }

    let err = last_error.expect("request attempts should produce an error");
    Err(AppError::Network(format!(
        "{context} after {attempts} attempts: {err}"
    )))
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

fn row_to_recent_memo(row: MemoRowResponse) -> Result<RecentMemo, AppError> {
    Ok(RecentMemo {
        id: row.id,
        text: row.text,
        created_at: row.created_at,
        version: normalize_version_value(&row.version)?,
        deleted_at: row.deleted_at,
    })
}

fn should_fetch_next_page(page_len: usize, page_size: usize) -> bool {
    page_size > 0 && page_len >= page_size
}

fn normalize_version_value(value: &serde_json::Value) -> Result<String, AppError> {
    match value {
        serde_json::Value::String(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        serde_json::Value::Number(v) => Ok(v.to_string()),
        _ => Err(AppError::Api(
            "Memo version is missing or invalid".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_SUPABASE_ANON_KEY, DEFAULT_SUPABASE_URL, SupabaseClient, extract_user_id_from_jwt,
        should_fetch_next_page,
    };

    #[test]
    fn extract_sub_from_jwt_payload() {
        let token = "a.eyJzdWIiOiIxMjM0In0.b";
        let user_id = extract_user_id_from_jwt(token).expect("should parse sub");
        assert_eq!(user_id, "1234");
    }

    #[test]
    fn should_fetch_next_page_when_page_is_full() {
        assert!(should_fetch_next_page(200, 200));
    }

    #[test]
    fn should_not_fetch_next_page_when_page_is_partial() {
        assert!(!should_fetch_next_page(199, 200));
    }

    #[test]
    fn should_not_fetch_next_page_with_zero_page_size() {
        assert!(!should_fetch_next_page(10, 0));
    }

    #[test]
    fn from_env_uses_built_in_defaults() {
        let client = SupabaseClient::from_env().expect("client should build");
        assert_eq!(client.base_url, DEFAULT_SUPABASE_URL);
        assert_eq!(client.anon_key, DEFAULT_SUPABASE_ANON_KEY);
    }
}
