//! jscpd JSON duplication report parser.
//!
//! jscpd outputs a JSON file with the following structure:
//! ```json
//! { "duplicates": [ { "firstFile": { "name": "..." }, "secondFile": { "name": "..." }, "fragment": "..." }, ... ] }
//! ```
//!
//! Each duplicate is mapped to one [`RawFinding`] with:
//! - `rule_id = "jscpd.duplication"`
//! - `severity = "warning"`
//! - `file_path = firstFile.name` (forward-slash normalized)
//! - `message = "Duplication with {secondFile.name}"`
//!
//! # jscpd filename instability (RESEARCH.md Pitfall 5)
//!
//! jscpd output filename is not stable across versions. Instead of reading
//! `output_file` directly, this parser globs the parent directory for any
//! `*.json` file whose content contains a `"duplicates"` key.

use serde::Deserialize;

use super::{ParseError, RawFinding};
use crate::scanner::RawScanResult;

// -- jscpd JSON deserialization structs -------------------------------------

#[derive(Deserialize)]
struct JscpdOutput {
    duplicates: Vec<JscpdDuplicate>,
}

#[derive(Deserialize)]
struct JscpdDuplicate {
    #[serde(rename = "firstFile")]
    first_file: JscpdFile,
    #[serde(rename = "secondFile")]
    second_file: JscpdFile,
    #[allow(dead_code)]
    fragment: String,
}

#[derive(Deserialize)]
struct JscpdFile {
    name: String,
}

// -- Parsing -----------------------------------------------------------------

/// Parse jscpd JSON output into a list of [`RawFinding`]s.
///
/// Uses a glob fallback because jscpd output filenames change across versions.
/// Globs the parent directory of `result.output_file` for any `.json` file
/// containing a `"duplicates"` key.
///
/// Returns `Ok(vec![])` when no matching file is found.
pub fn parse_jscpd_json(result: &RawScanResult) -> Result<Vec<RawFinding>, ParseError> {
    // The output_file path may not be the exact filename. Glob the parent dir.
    let search_dir = result
        .output_file
        .parent()
        .unwrap_or(std::path::Path::new("."));

    let content = find_jscpd_json(search_dir)?;
    match content {
        None => Ok(vec![]),
        Some(text) => parse_jscpd_content(&text, &result.scanner),
    }
}

/// Walk `dir` and return the content of the first `.json` file that contains
/// a `"duplicates"` key at the top level.
fn find_jscpd_json(dir: &std::path::Path) -> Result<Option<String>, ParseError> {
    if !dir.exists() {
        return Ok(None);
    }

    for entry in walkdir::WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Quick check: does this file contain "duplicates"?
        if text.contains("\"duplicates\"") {
            return Ok(Some(text));
        }
    }

    Ok(None)
}

/// Parse jscpd JSON content string into findings.
fn parse_jscpd_content(content: &str, scanner: &str) -> Result<Vec<RawFinding>, ParseError> {
    let output: JscpdOutput =
        serde_json::from_str(content).map_err(|e| ParseError::JsonError(e.to_string()))?;

    let findings = output
        .duplicates
        .into_iter()
        .map(|dup| {
            let file_path = dup.first_file.name.replace('\\', "/");
            let message = format!("Duplication with {}", dup.second_file.name);
            RawFinding {
                scanner: scanner.to_string(),
                rule_id: "jscpd.duplication".to_string(),
                severity: "warning".to_string(),
                file_path,
                line: 0,
                message,
                matched_code: None,
                suggested_replacement: None,
            }
        })
        .collect();

    Ok(findings)
}
