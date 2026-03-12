use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RiceGuardMcpServer {
    pub working_dir: PathBuf,
}

impl RiceGuardMcpServer {
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }
}
