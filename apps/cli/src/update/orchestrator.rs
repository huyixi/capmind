use crate::cli::UpdateArgs;
use crate::error::AppError;
use crate::self_update::SelfUpdateOutcome;

use super::backends::{homebrew, standalone};
use super::detector::{
    detect_current_reported_version, detect_current_reported_version_opt, detect_install_source,
};
use super::domain::{InstallSource, UpdateAction, UpdateOutcome, UpdateStatus};

const FORMULA_NAME: &str = "capmind";

pub async fn run_update(args: &UpdateArgs) -> Result<UpdateOutcome, AppError> {
    let install_source = detect_install_source(FORMULA_NAME);
    let current_version = detect_current_reported_version();

    let mode = determine_mode(args, install_source)?;

    if mode == UpdateMode::Check {
        let latest = standalone::check_latest_release().await?;
        return Ok(UpdateOutcome {
            status: UpdateStatus::Checked,
            install_source,
            action: UpdateAction::CheckOnly,
            from_version: Some(current_version),
            to_version: None,
            latest_version: Some(latest.version),
            release_tag: Some(latest.tag),
            recommended_command: Some(recommended_update_command(install_source).to_string()),
        });
    }

    if mode == UpdateMode::Homebrew {
        let before = current_version;
        homebrew::run_update_flow(FORMULA_NAME)?;
        let after = detect_current_reported_version();
        let status = if after == before {
            UpdateStatus::UpToDate
        } else {
            UpdateStatus::Updated
        };

        return Ok(UpdateOutcome {
            status,
            install_source,
            action: UpdateAction::DelegatedToBrew,
            from_version: Some(before),
            to_version: Some(after),
            latest_version: None,
            release_tag: None,
            recommended_command: Some("brew upgrade capmind".to_string()),
        });
    }

    match standalone::apply_update(args.version.as_deref()).await? {
        SelfUpdateOutcome::UpToDate { version } => Ok(UpdateOutcome {
            status: UpdateStatus::UpToDate,
            install_source,
            action: UpdateAction::SelfUpdate,
            from_version: Some(version.clone()),
            to_version: Some(version),
            latest_version: None,
            release_tag: None,
            recommended_command: Some(recommended_update_command(install_source).to_string()),
        }),
        SelfUpdateOutcome::Updated {
            from_version,
            to_version,
            tag,
        } => {
            if let Some(actual) = detect_current_reported_version_opt()
                && actual != to_version
            {
                return Err(AppError::Api(format!(
                    "Update verification failed. Expected version `{to_version}`, but `cap -V` reports `{actual}`."
                )));
            }

            Ok(UpdateOutcome {
                status: UpdateStatus::Updated,
                install_source,
                action: UpdateAction::SelfUpdate,
                from_version: Some(from_version),
                to_version: Some(to_version),
                latest_version: None,
                release_tag: Some(tag),
                recommended_command: Some(recommended_update_command(install_source).to_string()),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateMode {
    Check,
    Homebrew,
    Standalone,
}

fn determine_mode(
    args: &UpdateArgs,
    install_source: InstallSource,
) -> Result<UpdateMode, AppError> {
    if args.check {
        return Ok(UpdateMode::Check);
    }
    if install_source == InstallSource::Homebrew && !args.force_standalone {
        if args.version.is_some() {
            return Err(AppError::InvalidInput(
                "Homebrew-managed install detected. `cap update --version` is not supported; run `brew upgrade capmind`."
                    .to_string(),
            ));
        }
        return Ok(UpdateMode::Homebrew);
    }
    Ok(UpdateMode::Standalone)
}

fn recommended_update_command(source: InstallSource) -> &'static str {
    match source {
        InstallSource::Homebrew => "brew upgrade capmind",
        InstallSource::Standalone | InstallSource::Unknown => "cap update",
    }
}

#[cfg(test)]
mod tests {
    use super::super::domain::InstallSource;
    use super::{UpdateMode, determine_mode};
    use crate::cli::UpdateArgs;

    #[test]
    fn determine_mode_prefers_check() {
        let args = UpdateArgs {
            version: Some("0.2.10".to_string()),
            check: true,
            json: false,
            force_standalone: false,
        };
        let mode = determine_mode(&args, InstallSource::Homebrew).expect("mode");
        assert_eq!(mode, UpdateMode::Check);
    }

    #[test]
    fn determine_mode_uses_homebrew_when_managed() {
        let args = UpdateArgs {
            version: None,
            check: false,
            json: false,
            force_standalone: false,
        };
        let mode = determine_mode(&args, InstallSource::Homebrew).expect("mode");
        assert_eq!(mode, UpdateMode::Homebrew);
    }

    #[test]
    fn determine_mode_rejects_version_on_homebrew() {
        let args = UpdateArgs {
            version: Some("0.2.11".to_string()),
            check: false,
            json: false,
            force_standalone: false,
        };
        let err = determine_mode(&args, InstallSource::Homebrew).expect_err("should reject");
        assert!(
            err.to_string()
                .contains("Homebrew-managed install detected")
        );
    }

    #[test]
    fn determine_mode_uses_standalone_when_forced() {
        let args = UpdateArgs {
            version: None,
            check: false,
            json: false,
            force_standalone: true,
        };
        let mode = determine_mode(&args, InstallSource::Homebrew).expect("mode");
        assert_eq!(mode, UpdateMode::Standalone);
    }
}
