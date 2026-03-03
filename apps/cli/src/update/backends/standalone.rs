use crate::error::AppError;
use crate::self_update::{
    SelfUpdateOutcome, latest_release_tag, release_version_from_tag, run_self_update,
};

pub struct LatestRelease {
    pub tag: String,
    pub version: String,
}

pub async fn check_latest_release() -> Result<LatestRelease, AppError> {
    let tag = latest_release_tag().await?;
    let version = release_version_from_tag(&tag).to_string();
    Ok(LatestRelease { tag, version })
}

pub async fn apply_update(requested_version: Option<&str>) -> Result<SelfUpdateOutcome, AppError> {
    run_self_update(requested_version).await
}
