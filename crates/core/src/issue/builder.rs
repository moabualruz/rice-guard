//! `IssueBuilder` — joins `RawFinding` + evidence + fix metadata into an `Issue`.
//!
//! This is the single integration point for all sub-modules; callers should
//! only need to interact with `IssueBuilder::build(...)`.
//!
//! ## ID Generation
//!
//! Issue IDs use content-hash fingerprinting: SHA-256 of
//! `"{rule_id}\x00{normalized_path}\x00{matched_code_or_message}"`.
//! This makes IDs stable when a formatter shifts line numbers.
//!
//! See [`fingerprint`] for the standalone hashing function.

use sha2::{Digest, Sha256};
use std::path::Path;

use super::{
    evidence::extract_evidence_block, fix_meta::FixMetadata, priority::wsjf_score,
    verification::VerificationInfo, Issue,
};
use crate::scanner::parser::RawFinding;

/// Builds a fully-enriched [`Issue`] from a [`RawFinding`].
pub struct IssueBuilder;

impl IssueBuilder {
    /// Construct an [`Issue`] from `finding`, reading source files from `project_root`.
    ///
    /// `file_freq` is the number of findings in the same file (for WSJF scoring).
    pub fn build(finding: &RawFinding, project_root: &Path, file_freq: u32) -> Issue {
        let evidence = extract_evidence_block(
            finding.matched_code.as_deref(),
            &finding.file_path,
            finding.line,
            project_root,
        );

        let fix = FixMetadata::from_finding(
            &finding.scanner,
            &finding.rule_id,
            finding.suggested_replacement.as_deref(),
        );

        let verification = VerificationInfo::from_scanner(&finding.scanner, &finding.rule_id);

        let score = wsjf_score(
            &finding.severity,
            fix.auto_fixable,
            fix.auto_fix_category.as_deref(),
            file_freq,
            false, // cross_file: determined by caller for multi-file issues
        );

        let id = fingerprint(
            &finding.rule_id,
            &finding.file_path,
            if evidence.matched_code.is_empty() {
                &finding.message
            } else {
                &evidence.matched_code
            },
        );

        Issue {
            id,
            rule_id: finding.rule_id.clone(),
            severity: finding.severity.clone(),
            file_path: finding.file_path.clone(),
            line: finding.line,
            message: finding.message.clone(),
            scanner: finding.scanner.clone(),
            evidence,
            fix,
            verification,
            priority_score: score,
            cross_file: false,
        }
    }
}

/// Compute a deterministic, content-hash-based ID for deduplication.
///
/// Format: hex-encoded SHA-256 of `"{rule_id}\x00{normalized_path}\x00{code_or_msg}"`
///
/// This ID is:
/// - **Stable** across formatter runs (no line number)
/// - **Unique** when `matched_code` or `message` differs
/// - **Collision-resistant** for multiple CVEs in the same file (uses `message`)
pub fn fingerprint(rule_id: &str, file_path: &str, code_or_message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\x00");
    hasher.update(file_path.as_bytes());
    hasher.update(b"\x00");
    hasher.update(code_or_message.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_finding(scanner: &str, severity: &str, line: u32) -> RawFinding {
        RawFinding {
            scanner: scanner.to_string(),
            rule_id: "test-rule".to_string(),
            severity: severity.to_string(),
            file_path: "src/lib.rs".to_string(),
            line,
            message: "test message".to_string(),
            matched_code: None,
            suggested_replacement: None,
        }
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let id1 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        let id2 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        assert_eq!(id1, id2);
    }

    #[test]
    fn fingerprint_differs_by_rule() {
        let id1 = fingerprint("rule-a", "src/app.py", "code");
        let id2 = fingerprint("rule-b", "src/app.py", "code");
        assert_ne!(id1, id2);
    }

    #[test]
    fn fingerprint_differs_by_code() {
        let id1 = fingerprint("rule", "src/app.py", "eval(x)");
        let id2 = fingerprint("rule", "src/app.py", "exec(x)");
        assert_ne!(id1, id2);
    }

    #[test]
    fn fingerprint_is_hex_sha256() {
        let id = fingerprint("rule", "file.py", "code");
        // SHA-256 produces 64 hex characters
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn build_produces_non_empty_id() {
        let f = make_finding("semgrep", "warning", 42);
        let issue = IssueBuilder::build(&f, Path::new("/tmp/nonexistent_project_xyz"), 1);
        assert!(!issue.id.is_empty());
        assert_eq!(issue.id.len(), 64); // SHA-256 hex
    }

    #[test]
    fn build_priority_score_positive() {
        let f = make_finding("clippy", "error", 1);
        let issue = IssueBuilder::build(&f, Path::new("."), 1);
        assert!(
            issue.priority_score > 0,
            "priority_score must be positive for error"
        );
    }

    #[test]
    fn build_cross_file_defaults_false() {
        let f = make_finding("semgrep", "warning", 10);
        let issue = IssueBuilder::build(&f, Path::new("."), 1);
        assert!(!issue.cross_file);
    }
}
