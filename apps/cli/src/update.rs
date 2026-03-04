mod backends;
mod detector;
mod domain;
mod orchestrator;
mod presentation;

pub use detector::{
    detect_current_reported_version, detect_current_reported_version_opt, detect_install_source,
};
pub use domain::InstallSource;
pub use orchestrator::run_update;
pub use presentation::render_update_result;
pub use presentation::render_update_result_json;
