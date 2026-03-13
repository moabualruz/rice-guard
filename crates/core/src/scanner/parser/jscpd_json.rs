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
    let sanitized = sanitize_json_surrogates(content);
    let output: JscpdOutput =
        serde_json::from_str(&sanitized).map_err(|e| ParseError::JsonError(e.to_string()))?;

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

// -- UTF-16 surrogate sanitization -------------------------------------------

/// Sanitize orphaned UTF-16 surrogates in raw JSON text.
///
/// jscpd truncates code fragments mid-emoji, producing orphaned surrogates
/// like `\ud83d` that `serde_json` rejects. This replaces orphaned
/// `\uD800`–`\uDFFF` escapes with `\uFFFD` (replacement character).
fn sanitize_json_surrogates(input: &str) -> String {
    // Fast path: no surrogate escapes present.
    if !input.contains("\\ud") && !input.contains("\\uD") {
        return input.to_string();
    }

    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);
    let mut copy_from = 0;
    let mut i = 0;

    while i + 5 < len {
        if bytes[i] == b'\\' && bytes[i + 1] == b'u' {
            if let Some(cp) = hex4(bytes, i + 2) {
                if (0xD800..=0xDBFF).contains(&cp) {
                    // High surrogate — valid only if followed by \uDC00–\uDFFF.
                    let has_low = i + 11 < len
                        && bytes[i + 6] == b'\\'
                        && bytes[i + 7] == b'u'
                        && hex4(bytes, i + 8).is_some_and(|low| (0xDC00..=0xDFFF).contains(&low));
                    if has_low {
                        i += 12;
                        continue;
                    }
                    // Orphaned high surrogate.
                    result.push_str(&input[copy_from..i]);
                    result.push_str("\\uFFFD");
                    i += 6;
                    copy_from = i;
                    continue;
                } else if (0xDC00..=0xDFFF).contains(&cp) {
                    // Orphaned low surrogate.
                    result.push_str(&input[copy_from..i]);
                    result.push_str("\\uFFFD");
                    i += 6;
                    copy_from = i;
                    continue;
                }
            }
        }
        i += 1;
    }

    result.push_str(&input[copy_from..]);
    result
}

/// Parse 4 hex digits from `bytes[start..start+4]` into a u16.
fn hex4(bytes: &[u8], start: usize) -> Option<u16> {
    if start + 4 > bytes.len() {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..start + 4]).ok()?;
    u16::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_no_surrogates() {
        let input = r#"{"key": "hello \u0041 world"}"#;
        assert_eq!(sanitize_json_surrogates(input), input);
    }

    #[test]
    fn sanitize_orphaned_high_surrogate() {
        let input = r#"{"fragment": "emoji \ud83d end"}"#;
        let expected = r#"{"fragment": "emoji \uFFFD end"}"#;
        assert_eq!(sanitize_json_surrogates(input), expected);
    }

    #[test]
    fn sanitize_orphaned_low_surrogate() {
        let input = r#"{"fragment": "bad \ude00 end"}"#;
        let expected = r#"{"fragment": "bad \uFFFD end"}"#;
        assert_eq!(sanitize_json_surrogates(input), expected);
    }

    #[test]
    fn sanitize_valid_surrogate_pair_preserved() {
        // Valid pair: \uD83D\uDE00 = 😀
        let input = r#"{"fragment": "emoji \uD83D\uDE00 end"}"#;
        assert_eq!(sanitize_json_surrogates(input), input);
    }

    #[test]
    fn sanitize_multiple_orphans() {
        let input = r#"{"a": "\ud83d", "b": "\ude00"}"#;
        let expected = r#"{"a": "\uFFFD", "b": "\uFFFD"}"#;
        assert_eq!(sanitize_json_surrogates(input), expected);
    }

    #[test]
    fn sanitize_high_surrogate_at_end_of_string() {
        let input = r#"{"fragment": "trunc\ud83d"}"#;
        let expected = r#"{"fragment": "trunc\uFFFD"}"#;
        assert_eq!(sanitize_json_surrogates(input), expected);
    }

    #[test]
    fn parse_jscpd_content_with_orphaned_surrogates() {
        let json = r#"{"duplicates": [{"firstFile": {"name": "src/a.ts"}, "secondFile": {"name": "src/b.ts"}, "fragment": "code \ud83d here"}]}"#;
        let findings = parse_jscpd_content(json, "jscpd").unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file_path, "src/a.ts");
        assert_eq!(findings[0].rule_id, "jscpd.duplication");
    }

    #[test]
    fn parse_jscpd_content_empty_duplicates() {
        let json = r#"{"duplicates": []}"#;
        let findings = parse_jscpd_content(json, "jscpd").unwrap();
        assert!(findings.is_empty());
    }
}
