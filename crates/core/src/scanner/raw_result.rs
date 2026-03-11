use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Raw output from a single scanner invocation.
///
/// Produced by `ScannerEngine::run()` for each scanner that completes.
/// Downstream parsers consume this to produce normalised `Issue` structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawScanResult {
    /// Canonical scanner name (e.g., `"semgrep"`, `"trivy"`).
    pub scanner: String,

    /// Raw output format: `"json"` or `"sarif"`.
    pub output_format: String,

    /// Path to the scanner output file on disk (inside the `OutputDir`).
    pub output_file: PathBuf,

    /// Exit code returned by the scanner process.
    /// Non-zero does not always mean failure (e.g., Semgrep exits 1 when findings exist).
    pub exit_code: i32,
}
