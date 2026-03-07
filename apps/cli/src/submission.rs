use crate::auth::authenticate_with_stored_token;
use crate::composer_image::PastedImage;
use crate::error::AppError;
use crate::supabase::{InsertedMemo, Session, SupabaseClient};

#[derive(Debug, Clone)]
pub struct SubmitOutcome {
    pub inserted: InsertedMemo,
    pub session: Session,
}

pub async fn submit_memo(client: &SupabaseClient, text: &str) -> Result<SubmitOutcome, AppError> {
    submit_memo_with_images(client, text, &[]).await
}

pub async fn submit_memo_with_images(
    client: &SupabaseClient,
    text: &str,
    images: &[PastedImage],
) -> Result<SubmitOutcome, AppError> {
    let session = authenticate_with_stored_token(client).await?;
    let inserted = client
        .insert_memo_with_images(&session.access_token, text, images)
        .await?;
    Ok(SubmitOutcome { inserted, session })
}
