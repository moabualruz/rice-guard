use std::path::PathBuf;

/// Shared server state passed to all handlers via axum State extractor.
/// Working dir is set at server startup; config is reloaded per-request.
#[derive(Debug, Clone)]
pub struct AppState {
    pub working_dir: PathBuf,
}
