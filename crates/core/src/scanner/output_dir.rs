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
/// use rguard_core::scanner::OutputDir;
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

    /// Creates (or replaces) a `latest` symlink/junction in `base_dir` pointing
    /// to this scan's directory.
    ///
    /// Provides a stable `reports/latest` path for CI scripts that need to
    /// reference the most recent scan output without knowing the timestamp.
    ///
    /// - Linux/macOS: creates a relative symlink via `std::os::unix::fs::symlink`
    /// - Windows: attempts a directory symlink; if that fails (no SeCreateSymbolicLink
    ///   privilege), falls back to a directory junction via `cmd /c mklink /J`
    ///
    /// On any failure, logs a `tracing::warn` and returns `Ok(())` — this
    /// operation is best-effort and must never abort a scan.
    pub fn create_latest_symlink(&self, base_dir: &str) -> std::io::Result<()> {
        let latest = Path::new(base_dir).join("latest");

        // Remove existing symlink or junction.
        if latest.exists() || latest.is_symlink() {
            // On Windows, a junction is a directory — remove_dir.
            // On Unix, a symlink appears as a file-like — remove_file.
            #[cfg(windows)]
            {
                let _ = std::fs::remove_dir(&latest);
            }
            #[cfg(not(windows))]
            {
                let _ = std::fs::remove_file(&latest);
            }
        }

        // The relative target name (timestamp directory name only).
        // Used on non-Windows for the relative symlink target.
        #[allow(unused_variables)]
        let target_name = match self.path.file_name() {
            Some(n) => n,
            None => {
                tracing::warn!("reports/latest: could not determine scan dir name");
                return Ok(());
            }
        };

        #[cfg(not(windows))]
        {
            match std::os::unix::fs::symlink(
                self.path
                    .strip_prefix(base_dir)
                    .unwrap_or(target_name.as_ref()),
                &latest,
            ) {
                Ok(()) => {}
                Err(e) => {
                    tracing::warn!("reports/latest symlink failed: {}", e);
                }
            }
        }

        #[cfg(windows)]
        {
            // First try a real directory symlink (requires privilege on Windows).
            let result = std::os::windows::fs::symlink_dir(&self.path, &latest);
            if result.is_err() {
                // Fall back to a directory junction (no elevated permissions needed).
                let target_str = self.path.to_string_lossy();
                let latest_str = latest.to_string_lossy();
                let status = std::process::Command::new("cmd")
                    .args([
                        "/c",
                        "mklink",
                        "/J",
                        latest_str.as_ref(),
                        target_str.as_ref(),
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                match status {
                    Ok(s) if s.success() => {}
                    Ok(s) => {
                        tracing::warn!(
                            "reports/latest junction failed (exit {}): {}",
                            s.code().unwrap_or(-1),
                            latest_str
                        );
                    }
                    Err(e) => {
                        tracing::warn!("reports/latest junction command failed: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}
