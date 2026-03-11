use std::path::Path;

use thiserror::Error;

use crate::config::RiceGuardConfig;
use crate::registry::ScannerDescriptor;

use super::{OutputDir, RawScanResult};

/// Errors that can occur during a scan run.
#[derive(Debug, Error)]
pub enum ScannerEngineError {
    /// A scanner process did not finish within its configured timeout.
    #[error("Scanner '{0}' timed out")]
    Timeout(String),

    /// Failed to spawn a scanner subprocess.
    #[error("Failed to spawn scanner '{scanner}': {reason}")]
    SpawnFailed { scanner: String, reason: String },

    /// An I/O error occurred while setting up or reading scanner output.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Scan mode — controls which scanners are active and which command variant
/// each scanner uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanMode {
    /// Run all enabled scanners with their full scan command.
    Full,
    /// Run a fast subset (jscpd + scc + Semgrep + Trivy).
    Quick,
    /// Run only security-focused scanners (Semgrep + Trivy + Gitleaks).
    Security,
    /// Only report issues on changed files (hold-the-line mode).
    DiffOnly,
}

/// Orchestrates parallel scanner execution.
///
/// Constructed with the loaded descriptors and project config, then
/// driven by `run()` to produce `RawScanResult`s for downstream parsing.
///
/// The full implementation lives in Phase 2, Plan 04.
pub struct ScannerEngine {
    #[allow(dead_code)]
    descriptors: Vec<ScannerDescriptor>,
    #[allow(dead_code)]
    config: RiceGuardConfig,
}

impl ScannerEngine {
    /// Create a new `ScannerEngine`.
    ///
    /// `descriptors` — loaded scanner plugin descriptors (from YAML).
    /// `config` — project config (controls which scanners are enabled).
    pub fn new(descriptors: Vec<ScannerDescriptor>, config: RiceGuardConfig) -> Self {
        Self {
            descriptors,
            config,
        }
    }

    /// Run all enabled scanners against `target`, writing output to `output_dir`.
    ///
    /// Returns one `RawScanResult` per scanner that completed (successfully or not).
    ///
    /// # Note
    ///
    /// Implementation deferred to Phase 2, Plan 04.
    pub async fn run(
        &self,
        _target: &Path,
        _mode: ScanMode,
        _output_dir: &OutputDir,
    ) -> Result<Vec<RawScanResult>, ScannerEngineError> {
        unimplemented!("Phase 2 Plan 04 implements this")
    }
}
