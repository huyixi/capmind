use crate::auth::authenticate_with_stored_token;
use crate::error::AppError;
use crate::supabase::{InsertedMemo, Session, SupabaseClient};

#[derive(Debug, Clone)]
pub struct SubmitOutcome {
    pub inserted: InsertedMemo,
    pub session: Session,
}

pub async fn submit_memo(client: &SupabaseClient, text: &str) -> Result<SubmitOutcome, AppError> {
    let session = authenticate_with_stored_token(client).await?;
    let inserted = client.insert_memo(&session.access_token, text).await?;
    Ok(SubmitOutcome { inserted, session })
}
