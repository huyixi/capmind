use crate::error::AppError;

use super::domain::{InstallSource, UpdateAction, UpdateOutcome, UpdateStatus};

pub fn render_update_result(result: &UpdateOutcome) -> String {
    match result.status {
        UpdateStatus::Checked => format!(
            "Update check complete\ninstall_source: {}\ncurrent: {}\nlatest: {}\nrelease_tag: {}\nrecommended: {}",
            install_source_label(result.install_source),
            result.from_version.as_deref().unwrap_or("unknown"),
            result.latest_version.as_deref().unwrap_or("unknown"),
            result.release_tag.as_deref().unwrap_or("unknown"),
            result
                .recommended_command
                .as_deref()
                .unwrap_or("cap update"),
        ),
        UpdateStatus::Updated => format!(
            "Update successful\ninstall_source: {}\naction: {}\nfrom: {}\nto: {}\nrelease_tag: {}",
            install_source_label(result.install_source),
            action_label(result.action),
            result.from_version.as_deref().unwrap_or("unknown"),
            result.to_version.as_deref().unwrap_or("unknown"),
            result.release_tag.as_deref().unwrap_or("-"),
        ),
        UpdateStatus::UpToDate => format!(
            "Already up to date\ninstall_source: {}\naction: {}\nversion: {}",
            install_source_label(result.install_source),
            action_label(result.action),
            result.to_version.as_deref().unwrap_or("unknown"),
        ),
    }
}

pub fn render_update_result_json(result: &UpdateOutcome) -> Result<String, AppError> {
    serde_json::to_string_pretty(result)
        .map_err(|err| AppError::Api(format!("Failed to serialize update result as JSON: {err}")))
}

fn install_source_label(source: InstallSource) -> &'static str {
    match source {
        InstallSource::Homebrew => "homebrew",
        InstallSource::Standalone => "standalone",
        InstallSource::Unknown => "unknown",
    }
}

fn action_label(action: UpdateAction) -> &'static str {
    match action {
        UpdateAction::CheckOnly => "check_only",
        UpdateAction::DelegatedToBrew => "delegated_to_brew",
        UpdateAction::SelfUpdate => "self_update",
    }
}

#[cfg(test)]
mod tests {
    use super::super::domain::{InstallSource, UpdateAction, UpdateOutcome, UpdateStatus};
    use super::{render_update_result, render_update_result_json};

    #[test]
    fn renders_human_checked_output() {
        let outcome = UpdateOutcome {
            status: UpdateStatus::Checked,
            install_source: InstallSource::Homebrew,
            action: UpdateAction::CheckOnly,
            from_version: Some("0.2.10".to_string()),
            to_version: None,
            latest_version: Some("0.2.11".to_string()),
            release_tag: Some("capmind-v0.2.11".to_string()),
            recommended_command: Some("brew upgrade capmind".to_string()),
        };
        let rendered = render_update_result(&outcome);
        assert!(rendered.contains("Update check complete"));
        assert!(rendered.contains("install_source: homebrew"));
        assert!(rendered.contains("recommended: brew upgrade capmind"));
    }

    #[test]
    fn renders_json_with_enum_values() {
        let outcome = UpdateOutcome {
            status: UpdateStatus::Updated,
            install_source: InstallSource::Standalone,
            action: UpdateAction::SelfUpdate,
            from_version: Some("0.2.10".to_string()),
            to_version: Some("0.2.11".to_string()),
            latest_version: None,
            release_tag: Some("capmind-v0.2.11".to_string()),
            recommended_command: Some("cap update".to_string()),
        };
        let json = render_update_result_json(&outcome).expect("json");
        assert!(json.contains("\"status\": \"updated\""));
        assert!(json.contains("\"install_source\": \"standalone\""));
        assert!(json.contains("\"action\": \"self_update\""));
    }
}
