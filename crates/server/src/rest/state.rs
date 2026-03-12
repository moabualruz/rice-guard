use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppState {
    pub working_dir: PathBuf,
}
