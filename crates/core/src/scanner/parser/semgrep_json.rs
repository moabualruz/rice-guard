//! Semgrep `--json` output parser.
//!
//! Semgrep uses a custom JSON format (not SARIF). The top-level structure is:
//! ```json
//! { "results": [...], "errors": [...] }
//! ```
//!
//! When `results` is absent or null, returns an empty `Vec` (SCAN-08 edge case).

use serde::Deserialize;

use super::{ParseError, RawFinding};
use crate::scanner::RawScanResult;

// -- Semgrep JSON deserialization structs ------------------------------------

#[derive(Deserialize)]
struct SemgrepOutput {
    /// `None` when the key is absent; `Some(vec![])` when the key is present but empty.
    results: Option<Vec<SemgrepResult>>,
}

#[derive(Deserialize)]
struct SemgrepResult {
    check_id: String,
    path: String,
    start: SemgrepStart,
    extra: SemgrepExtra,
}

#[derive(Deserialize)]
struct SemgrepStart {
    line: u32,
}

#[derive(Deserialize)]
struct SemgrepExtra {
    severity: String,
    message: String,
    /// The matched source code lines (Semgrep `extra.lines` field).
    #[serde(default)]
    lines: Option<String>,
    /// Auto-fix snippet when a Semgrep rule has an `autofix` field.
    #[serde(rename = "fix", default)]
    fix: Option<String>,
}

// -- Parsing -----------------------------------------------------------------

/// Parse Semgrep `--json` output into a list of [`RawFinding`]s.
///
/// Severity normalization:
/// - `"ERROR"` / `"error"` -> `"error"`
/// - `"WARNING"` / `"warning"` -> `"warning"`
/// - anything else -> `"info"`
///
/// Returns `Ok(vec![])` when `results` key is absent (SCAN-08 edge case).
///
/// Evidence fields populated:
/// - `matched_code` ← `extra.lines` (trimmed)
/// - `suggested_replacement` ← `extra.fix` (autofix snippet)
pub fn parse_semgrep_json(result: &RawScanResult) -> Result<Vec<RawFinding>, ParseError> {
    let content = std::fs::read_to_string(&result.output_file)?;
    let output: SemgrepOutput =
        serde_json::from_str(&content).map_err(|e| ParseError::JsonError(e.to_string()))?;

    let results = match output.results {
        None => return Ok(vec![]),
        Some(r) => r,
    };

    let findings = results
        .into_iter()
        .map(|r| {
            let severity = normalize_semgrep_severity(&r.extra.severity);
            let file_path = r.path.replace('\\', "/");
            // Trim matched_code so leading/trailing whitespace from Semgrep indentation
            // does not pollute content-hash IDs.
            let matched_code = r
                .extra
                .lines
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let suggested_replacement = r.extra.fix;
            RawFinding {
                scanner: result.scanner.clone(),
                rule_id: r.check_id,
                severity,
                file_path,
                line: r.start.line,
                message: r.extra.message,
                matched_code,
                suggested_replacement,
            }
        })
        .collect();

    Ok(findings)
}

/// Map Semgrep severity string to normalized lowercase severity.
///
/// Semgrep emits uppercase severity strings (`"ERROR"`, `"WARNING"`).
fn normalize_semgrep_severity(severity: &str) -> String {
    match severity.to_ascii_uppercase().as_str() {
        "ERROR" => "error".to_string(),
        "WARNING" => "warning".to_string(),
        _ => "info".to_string(),
    }
}
