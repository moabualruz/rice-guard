use std::path::{Path, PathBuf};

use chrono::Utc;

/// Manages the timestamped output directory for a scan run.
///
/// Creates `reports/<project_name>/<timestamp>/` on construction.
/// All scanner output files are written inside this directory.
///
/// # Example
///
/// ```rust,no_run
/// use rice_guard_core::scanner::OutputDir;
///
/// let dir = OutputDir::new("my-app", "reports").unwrap();
/// println!("scan output at: {}", dir.path().display());
/// ```
#[derive(Debug, Clone)]
pub struct OutputDir {
    path: PathBuf,
}

impl OutputDir {
    /// Create a new output directory.
    ///
    /// The directory is created immediately at
    /// `<base_dir>/<project_name>/<timestamp>/` where timestamp uses
    /// UTC formatted as `%Y%m%dT%H%M%SZ`.
    ///
    /// # Errors
    ///
    /// Returns `std::io::Error` if the directory cannot be created.
    pub fn new(project_name: &str, base_dir: &str) -> std::io::Result<Self> {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let path = PathBuf::from(base_dir).join(project_name).join(timestamp);
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// Returns the path to the output directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
