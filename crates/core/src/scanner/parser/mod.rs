//! Scanner output parser dispatch.
//!
//! Converts raw scanner output files into normalized [`RawFinding`] structs.
//! Parser selection is based on the `output_format` field of [`RawScanResult`].
//!
//! # Supported formats
//!
//! | `output_format` | Parser           | Used by             |
//! |-----------------|------------------|---------------------|
//! | `"sarif"`       | [`sarif`]        | Trivy, Gitleaks     |
//! | `"json"`        | [`semgrep_json`] | Semgrep             |
//! | `"jscpd-json"`  | [`jscpd_json`]   | jscpd               |
//! | `"scc-json"`    | (empty)          | scc (metrics only)  |

mod jscpd_json;
mod sarif;
mod semgrep_json;

pub use jscpd_json::parse_jscpd_json;
pub use sarif::parse_sarif;
pub use semgrep_json::parse_semgrep_json;

use crate::scanner::RawScanResult;
use thiserror::Error;

/// Normalized finding from a single scanner.
///
/// Phase 3 enriches this into a full `Issue` with evidence extraction.
#[derive(Debug, Clone)]
pub struct RawFinding {
    /// Canonical scanner name (e.g., `"semgrep"`, `"trivy"`).
    pub scanner: String,

    /// Rule or check identifier (e.g., `"python.security.sql-injection"`).
    pub rule_id: String,

    /// Normalized severity: `"error"`, `"warning"`, `"info"`, or `"high"`.
    pub severity: String,

    /// Relative file path, forward-slash normalized.
    ///
    /// `"<project>"` when the finding applies to the whole project (e.g.,
    /// Trivy CVE with no physicalLocation).
    pub file_path: String,

    /// Source line number (1-based). `0` when unknown.
    pub line: u32,

    /// Human-readable finding description.
    pub message: String,
}

/// Error variants produced by the parser dispatch.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The output file could not be read.
    #[error("I/O error reading scanner output: {0}")]
    IoError(#[from] std::io::Error),

    /// The file content could not be parsed as the expected format.
    #[error("JSON parse error: {0}")]
    JsonError(String),

    /// The `output_format` value is not recognized.
    #[error("unknown output format: {0}")]
    InvalidFormat(String),
}

/// Dispatch scanner output parsing based on `result.output_format`.
///
/// Returns `Ok(vec![])` for formats that produce no findings (e.g., `"scc-json"`).
/// Returns `Err(ParseError::InvalidFormat)` for unrecognized formats.
pub fn parse_scanner_output(result: &RawScanResult) -> Result<Vec<RawFinding>, ParseError> {
    match result.output_format.as_str() {
        "sarif" => parse_sarif(result),
        "json" => parse_semgrep_json(result),
        "jscpd-json" => parse_jscpd_json(result),
        // scc is metrics only — no findings produced
        "scc-json" | "scc" => Ok(vec![]),
        other => Err(ParseError::InvalidFormat(other.to_string())),
    }
}
