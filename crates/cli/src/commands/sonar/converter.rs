/// SARIF to SonarQube Generic Issue Data converter.
///
/// Converts rice-guard SARIF output files into the SonarQube Generic Issue
/// Data JSON format for import via `sonar.externalIssuesReportPaths`.
use std::path::Path;

use anyhow::Context;
use serde_json::Value;

use super::models::{PrimaryLocation, SonarGenericIssue, SonarGenericIssuesFile, TextRange};

// ── Severity mapping ──────────────────────────────────────────────────────────

/// Map a SARIF result level string to a SonarQube severity string.
///
/// | SARIF level | SonarQube severity |
/// |-------------|-------------------|
/// | `error`     | `CRITICAL`        |
/// | `warning`   | `MAJOR`           |
/// | `note`/`none`/absent | `MINOR` |
fn map_severity(level: Option<&str>) -> &'static str {
    match level {
        Some("error") => "CRITICAL",
        Some("warning") => "MAJOR",
        _ => "MINOR",
    }
}

// ── Type mapping ──────────────────────────────────────────────────────────────

/// Map a scanner engine id (tool.driver.name, lowercased) to a SonarQube type.
///
/// | Engine           | SonarQube type  |
/// |------------------|-----------------|
/// | semgrep, trivy, gitleaks | `VULNERABILITY` |
/// | jscpd, scc       | `CODE_SMELL`    |
/// | everything else  | `BUG`           |
fn map_type(engine_id: &str) -> &'static str {
    match engine_id {
        "semgrep" | "trivy" | "gitleaks" => "VULNERABILITY",
        "jscpd" | "scc" => "CODE_SMELL",
        _ => "BUG",
    }
}

// ── Path normalisation ────────────────────────────────────────────────────────

/// Strip `project_root` prefix from a file URI or absolute path and return a
/// relative forward-slash path.
///
/// Falls back to the original path (forward-slash normalised) if stripping
/// fails (e.g. the URI already is relative or uses a different root).
fn relative_path(uri: &str, project_root: &Path) -> String {
    // SARIF URIs may be "file:///abs/path" or just "path/to/file.rs"
    let raw = uri
        .trim_start_matches("file:///")
        .trim_start_matches("file://");

    // On Windows SARIF may encode as file:///C:/... — normalise the drive letter
    // by trying to parse as a Path first.
    let as_path = Path::new(raw);

    let relative = as_path.strip_prefix(project_root).unwrap_or(as_path);

    // Normalise to forward slashes for SonarQube
    relative.to_string_lossy().replace('\\', "/")
}

// ── Main converter ────────────────────────────────────────────────────────────

/// Convert a SARIF file produced by rice-guard into SonarQube Generic Issue Data.
///
/// Missing or malformed fields get safe defaults (empty strings, line 1).
/// Returns an empty issues list if the SARIF `runs` array is absent.
pub fn convert_sarif_to_generic_issues(
    sarif_path: &Path,
    project_root: &Path,
) -> anyhow::Result<SonarGenericIssuesFile> {
    let content = std::fs::read_to_string(sarif_path)
        .with_context(|| format!("read SARIF file {}", sarif_path.display()))?;

    let sarif: Value = serde_json::from_str(&content)
        .with_context(|| format!("parse SARIF JSON from {}", sarif_path.display()))?;

    let mut issues = Vec::new();

    let runs = match sarif.get("runs").and_then(|r| r.as_array()) {
        Some(r) => r,
        None => return Ok(SonarGenericIssuesFile { issues }),
    };

    for run in runs {
        let engine_id = run
            .pointer("/tool/driver/name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let results = match run.get("results").and_then(|r| r.as_array()) {
            Some(r) => r,
            None => continue,
        };

        for result in results {
            let level = result.get("level").and_then(|v| v.as_str());
            let severity = map_severity(level).to_string();
            let issue_type = map_type(&engine_id).to_string();

            let rule_id = result
                .get("ruleId")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown-rule")
                .to_string();

            let message = result
                .pointer("/message/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract primary location from first physicalLocation
            let location = result
                .pointer("/locations/0/physicalLocation")
                .cloned()
                .unwrap_or(Value::Null);

            let uri = location
                .pointer("/artifactLocation/uri")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let file_path = if uri.is_empty() {
                "unknown".to_string()
            } else {
                relative_path(uri, project_root)
            };

            let start_line = location
                .pointer("/region/startLine")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32;
            let end_line = location
                .pointer("/region/endLine")
                .and_then(|v| v.as_u64())
                .unwrap_or(start_line as u64) as u32;
            let start_column = location
                .pointer("/region/startColumn")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let end_column = location
                .pointer("/region/endColumn")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;

            issues.push(SonarGenericIssue {
                engine_id: engine_id.clone(),
                rule_id,
                severity,
                issue_type,
                primary_location: PrimaryLocation {
                    message,
                    file_path,
                    text_range: TextRange {
                        start_line,
                        end_line,
                        start_column,
                        end_column,
                    },
                },
            });
        }
    }

    Ok(SonarGenericIssuesFile { issues })
}

// ── derive_project_key ────────────────────────────────────────────────────────

/// Derive a SonarQube project key from a directory name.
///
/// Rules:
/// - Characters outside `[a-zA-Z0-9\-_.]` are replaced with `-`
/// - Result is lowercased
/// - If the first character is a digit, prepend `rg-`
pub fn derive_project_key(dir_name: &str) -> String {
    let sanitized: String = dir_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase();

    if sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("rg-{sanitized}")
    } else {
        sanitized
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_maps_error_to_critical() {
        assert_eq!(map_severity(Some("error")), "CRITICAL");
    }

    #[test]
    fn severity_maps_warning_to_major() {
        assert_eq!(map_severity(Some("warning")), "MAJOR");
    }

    #[test]
    fn severity_maps_none_to_minor() {
        assert_eq!(map_severity(Some("none")), "MINOR");
        assert_eq!(map_severity(None), "MINOR");
        assert_eq!(map_severity(Some("note")), "MINOR");
    }

    #[test]
    fn type_maps_gitleaks_to_vulnerability() {
        assert_eq!(map_type("gitleaks"), "VULNERABILITY");
        assert_eq!(map_type("semgrep"), "VULNERABILITY");
        assert_eq!(map_type("trivy"), "VULNERABILITY");
    }

    #[test]
    fn type_maps_jscpd_to_code_smell() {
        assert_eq!(map_type("jscpd"), "CODE_SMELL");
        assert_eq!(map_type("scc"), "CODE_SMELL");
    }

    #[test]
    fn type_maps_unknown_to_bug() {
        assert_eq!(map_type("ast-grep"), "BUG");
        assert_eq!(map_type("unknown"), "BUG");
    }

    #[test]
    fn relative_path_strips_prefix() {
        let root = Path::new("/home/user/myproject");
        let uri = "/home/user/myproject/src/main.rs";
        let result = relative_path(uri, root);
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn relative_path_normalizes_backslashes() {
        let root = Path::new("C:/project");
        let uri = "C:/project/src/lib.rs";
        let result = relative_path(uri, root);
        assert_eq!(result, "src/lib.rs");
    }

    #[test]
    fn derive_project_key_sanitizes_spaces() {
        assert_eq!(derive_project_key("my project"), "my-project");
    }

    #[test]
    fn derive_project_key_prefixes_digit() {
        assert_eq!(derive_project_key("123abc"), "rg-123abc");
    }

    #[test]
    fn derive_project_key_valid_stays() {
        assert_eq!(derive_project_key("rice-guard"), "rice-guard");
    }
}
