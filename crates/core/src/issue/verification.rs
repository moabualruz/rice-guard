//! Verification metadata for findings.
//!
//! After applying a fix (deterministic, manual, or AI-assisted), the user or
//! automation can re-run the scanner to confirm the issue is resolved.
//! `VerificationInfo` provides the exact command and expected outcome.

use serde::{Deserialize, Serialize};

/// How to verify that a fix was successfully applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationInfo {
    /// Shell command to re-run after applying a fix.
    /// Example: `"rguard scan . --security"`, `"cargo clippy"`.
    pub rerun_command: String,

    /// Human-readable description of the expected outcome after a successful fix.
    /// Example: `"exit 0 with no findings for this rule"`.
    pub success_condition: String,
}

impl VerificationInfo {
    /// Build verification info from the scanner name and rule ID.
    ///
    /// Constructs a sensible default `rerun_command` based on the scanner.
    pub fn from_scanner(scanner: &str, rule_id: &str) -> Self {
        let rerun_command = match scanner {
            "semgrep" => format!("semgrep scan --config={rule_id} ."),
            "trivy" => "trivy fs .".to_string(),
            "gitleaks" => "gitleaks detect .".to_string(),
            "clippy" => "cargo clippy".to_string(),
            _ => "rguard scan .".to_string(),
        };

        Self {
            rerun_command,
            success_condition: format!("no {rule_id} findings reported"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semgrep_rerun_command_includes_rule() {
        let info = VerificationInfo::from_scanner("semgrep", "python.security.eval");
        assert!(info.rerun_command.contains("python.security.eval"));
    }

    #[test]
    fn clippy_rerun_command() {
        let info = VerificationInfo::from_scanner("clippy", "clippy::needless_return");
        assert_eq!(info.rerun_command, "cargo clippy");
    }

    #[test]
    fn unknown_scanner_falls_back_to_rguard() {
        let info = VerificationInfo::from_scanner("custom", "some-rule");
        assert!(info.rerun_command.contains("rguard"));
    }

    #[test]
    fn success_condition_mentions_rule() {
        let info = VerificationInfo::from_scanner("trivy", "CVE-2023-12345");
        assert!(info.success_condition.contains("CVE-2023-12345"));
    }
}
