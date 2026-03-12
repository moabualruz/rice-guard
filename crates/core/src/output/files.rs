//! File I/O helpers for writing output files.
//!
//! Implements the individual write functions for each output file:
//! - `issues.json` — all issues, WSJF-sorted
//! - `issues-fixable.json` — auto_fixable = true subset
//! - `issues-remaining.json` — auto_fixable = false subset
//! - `summary.json` — aggregated scan statistics
//! - `summary.txt` — human-readable ASCII table

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use crate::issue::Issue;
use crate::output::summary::ScanSummary;

/// JSON wrapper that adds a `schema_version` field at the top level of all
/// three issue output files (issues.json, issues-fixable.json, issues-remaining.json).
///
/// This enables forward-compatible parsing by consumers (AI tools, CI pipelines).
#[derive(Serialize)]
struct IssueOutput<'a> {
    /// Schema version — always `"1.0"`.
    schema_version: &'static str,
    /// The issue list for this file.
    issues: &'a [Issue],
}

/// Write a serializable value to a file as pretty-printed JSON.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let f = std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    serde_json::to_writer_pretty(f, value)
        .with_context(|| format!("serialize JSON to {}", path.display()))?;
    Ok(())
}

/// Normalize a file path to always use forward slashes (EVID-12).
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Clone issues with all `file_path` values normalized to forward slashes.
pub(crate) fn normalize_issues(issues: &[Issue]) -> Vec<Issue> {
    issues
        .iter()
        .cloned()
        .map(|mut i| {
            i.file_path = normalize_path(&i.file_path);
            i
        })
        .collect()
}

/// Write the human-readable ASCII table to `summary.txt`.
///
/// Uses only ASCII `+`, `-`, `|` characters — no Unicode box-drawing.
pub(crate) fn write_summary_txt(path: &Path, issues: &[Issue]) -> anyhow::Result<()> {
    use std::fmt::Write as FmtWrite;
    use std::io::Write as IoWrite;

    // Column widths: ID(8), Severity(8), Score(5), File(30), Message(50), Fix(3)
    let sep = "+----------+----------+-------+--------------------------------+----------------------------------------------------+-----+";
    let header = "| ID       | Severity | Score | File                           | Message                                            | Fix |";

    let mut buf = String::new();
    writeln!(buf, "{sep}").unwrap();
    writeln!(buf, "{header}").unwrap();
    writeln!(buf, "{sep}").unwrap();

    for issue in issues {
        let id = if issue.id.len() >= 8 {
            &issue.id[..8]
        } else {
            &issue.id
        };
        let severity = truncate(&issue.severity, 8);
        let score = issue.priority_score;

        // Last 30 chars of file path
        let fp = normalize_path(&issue.file_path);
        let file = if fp.len() > 30 {
            &fp[fp.len() - 30..]
        } else {
            &fp
        };

        let message = truncate(&issue.message, 50);
        let fix = if issue.fix.auto_fixable { "Y" } else { "N" };

        writeln!(
            buf,
            "| {:<8} | {:<8} | {:<5} | {:<30} | {:<50} | {:<3} |",
            id, severity, score, file, message, fix
        )
        .unwrap();
    }

    writeln!(buf, "{sep}").unwrap();

    // Verify all ASCII before writing
    debug_assert!(buf.is_ascii(), "summary.txt must be pure ASCII");

    let mut f =
        std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    f.write_all(buf.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Truncate a string to at most `max` characters (ASCII-safe).
fn truncate(s: &str, max: usize) -> String {
    // Use char_indices to avoid splitting multi-byte chars, but
    // since summary.txt must be ASCII, any non-ASCII is replaced.
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect();
    if cleaned.len() <= max {
        cleaned
    } else {
        cleaned[..max].to_string()
    }
}

/// Write all five output files.
///
/// Called by [`OutputWriter::write_all`].
pub(crate) fn write_all_files(
    dir: &Path,
    issues: &[Issue],
    summary: &ScanSummary,
) -> anyhow::Result<()> {
    // Normalize paths in all issues before writing (EVID-12).
    let normalized = normalize_issues(issues);

    // Split into fixable and remaining subsets.
    let fixable: Vec<Issue> = normalized.iter().filter(|i| i.fix.auto_fixable).cloned().collect();
    let remaining: Vec<Issue> = normalized.iter().filter(|i| !i.fix.auto_fixable).cloned().collect();

    // Wrap each issue list with schema_version for forward compatibility (EVID-09).
    let all_output = IssueOutput {
        schema_version: "1.0",
        issues: &normalized,
    };
    let fixable_output = IssueOutput {
        schema_version: "1.0",
        issues: &fixable,
    };
    let remaining_output = IssueOutput {
        schema_version: "1.0",
        issues: &remaining,
    };

    // Write all five files.
    write_json(&dir.join("issues.json"), &all_output)?;
    write_json(&dir.join("issues-fixable.json"), &fixable_output)?;
    write_json(&dir.join("issues-remaining.json"), &remaining_output)?;
    write_json(&dir.join("summary.json"), summary)?;
    write_summary_txt(&dir.join("summary.txt"), &normalized)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_converts_backslashes() {
        assert_eq!(normalize_path(r"src\foo\bar.py"), "src/foo/bar.py");
        assert_eq!(normalize_path("src/foo/bar.py"), "src/foo/bar.py");
    }

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_cut() {
        let s = "a".repeat(60);
        assert_eq!(truncate(&s, 50).len(), 50);
    }

    #[test]
    fn truncate_non_ascii_replaced() {
        let s = "hello\u{00e9}world"; // é is non-ASCII
        let result = truncate(s, 20);
        assert!(result.is_ascii());
    }
}
