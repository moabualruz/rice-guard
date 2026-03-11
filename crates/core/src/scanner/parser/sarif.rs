//! SARIF 2.1.0 parser for Trivy and Gitleaks output.
//!
//! Handles tool-specific edge cases per SCAN-08:
//! - Trivy CVE findings with empty `locations` array -> `file_path = "<project>"`
//! - Gitleaks findings with absent `level` field -> `severity = "high"`
//! - Windows path backslashes normalized to forward slashes
//! - `file:///` URI prefix stripped

use serde::Deserialize;

use super::{ParseError, RawFinding};
use crate::scanner::RawScanResult;

// -- SARIF 2.1.0 deserialization structs -------------------------------------

#[derive(Deserialize)]
struct SarifRoot {
    runs: Vec<SarifRun>,
}

#[derive(Deserialize)]
struct SarifRun {
    #[allow(dead_code)]
    tool: SarifTool,
    /// `#[serde(default)]` because Semgrep emits `"results": null` for zero findings.
    #[serde(default)]
    results: Vec<SarifResult>,
}

#[derive(Deserialize)]
struct SarifTool {
    #[allow(dead_code)]
    driver: SarifDriver,
}

#[derive(Deserialize)]
struct SarifDriver {
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    /// `None` when absent (Gitleaks omits this field).
    level: Option<String>,
    message: SarifMessage,
    /// `#[serde(default)]` handles both absent and `null`.
    locations: Vec<SarifLocation>,
}

#[derive(Deserialize, Default)]
struct SarifMessage {
    text: String,
}

#[derive(Deserialize, Default)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: Option<PhysicalLocation>,
}

#[derive(Deserialize)]
struct PhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: Option<ArtifactLocation>,
    region: Option<Region>,
}

#[derive(Deserialize)]
struct ArtifactLocation {
    uri: Option<String>,
}

#[derive(Deserialize)]
struct Region {
    #[serde(rename = "startLine")]
    start_line: Option<u32>,
}

// -- Parsing -----------------------------------------------------------------

/// Parse a SARIF file produced by Trivy or Gitleaks into a flat list of [`RawFinding`]s.
///
/// Edge cases handled:
/// - Empty `locations` or absent `physicalLocation` -> `file_path = "<project>"`, `line = 0`
/// - Gitleaks: absent `level` -> `severity = "high"` (secrets are always high severity)
/// - Other scanners: absent `level` -> `severity = "warning"` (conservative default)
/// - `file:///` URI prefix stripped
/// - Backslashes replaced with forward slashes
pub fn parse_sarif(result: &RawScanResult) -> Result<Vec<RawFinding>, ParseError> {
    let content = std::fs::read_to_string(&result.output_file)?;
    let root: SarifRoot =
        serde_json::from_str(&content).map_err(|e| ParseError::JsonError(e.to_string()))?;

    let mut findings = Vec::new();
    let is_gitleaks = result.scanner == "gitleaks";

    for run in root.runs {
        for sarif_result in run.results {
            let (file_path, line) = extract_location(&sarif_result.locations);
            let severity = normalize_severity(sarif_result.level.as_deref(), is_gitleaks);

            findings.push(RawFinding {
                scanner: result.scanner.clone(),
                rule_id: sarif_result.rule_id,
                severity,
                file_path,
                line,
                message: sarif_result.message.text,
                matched_code: None,
                suggested_replacement: None,
            });
        }
    }

    Ok(findings)
}

/// Extract (file_path, line) from a SARIF locations array.
///
/// Returns `("<project>", 0)` when:
/// - The locations array is empty
/// - All entries lack a `physicalLocation`
/// - The `physicalLocation` has no `artifactLocation`
fn extract_location(locations: &[SarifLocation]) -> (String, u32) {
    for loc in locations {
        if let Some(phys) = &loc.physical_location {
            if let Some(artifact) = &phys.artifact_location {
                if let Some(uri) = &artifact.uri {
                    let file_path = normalize_uri(uri);
                    let line = phys.region.as_ref().and_then(|r| r.start_line).unwrap_or(0);
                    return (file_path, line);
                }
            }
        }
    }
    // Trivy CVE edge case: no physicalLocation -> project-level finding
    ("<project>".to_string(), 0)
}

/// Normalize a SARIF URI to a relative forward-slash path.
///
/// - Strips `file:///` prefix
/// - Replaces backslashes with forward slashes
fn normalize_uri(uri: &str) -> String {
    let stripped = uri.strip_prefix("file:///").unwrap_or(uri);
    stripped.replace('\\', "/")
}

/// Map SARIF `level` string to normalized severity.
///
/// - `"error"` -> `"error"`
/// - `"warning"` -> `"warning"`
/// - `"note"` / `"info"` -> `"info"`
/// - `None` (absent) + Gitleaks scanner -> `"high"` (secrets are always high)
/// - `None` (absent) + other scanner -> `"warning"` (conservative default)
/// - unknown non-None -> `"warning"` (conservative default)
fn normalize_severity(level: Option<&str>, is_gitleaks: bool) -> String {
    match level {
        Some("error") => "error".to_string(),
        Some("warning") => "warning".to_string(),
        Some("note") | Some("info") => "info".to_string(),
        // Gitleaks omits the level field entirely; secrets are always high
        None if is_gitleaks => "high".to_string(),
        // Other scanners with absent level: conservative default
        None | Some(_) => "warning".to_string(),
    }
}
