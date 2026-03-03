use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    Homebrew,
    Standalone,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    Checked,
    Updated,
    UpToDate,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateAction {
    CheckOnly,
    DelegatedToBrew,
    SelfUpdate,
}

#[derive(Debug, Serialize)]
pub struct UpdateOutcome {
    pub status: UpdateStatus,
    pub install_source: InstallSource,
    pub action: UpdateAction,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub latest_version: Option<String>,
    pub release_tag: Option<String>,
    pub recommended_command: Option<String>,
}
