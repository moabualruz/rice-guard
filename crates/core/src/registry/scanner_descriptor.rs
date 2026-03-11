use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Describes a scanner tool (Semgrep, Trivy, Gitleaks, jscpd, scc).
/// Loaded from YAML descriptor files at compile time (built-in) or runtime
/// (user-provided). Drives `init` probing and `scan` command construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerDescriptor {
    /// Canonical tool name, e.g. `"semgrep"`.
    pub name: String,
    /// Semver requirement, e.g. `">=1.60"`.
    pub version: String,
    /// Languages this scanner covers (lowercase strings).
    pub languages: Vec<String>,
    /// How to install / check the tool.
    pub install: ScannerInstall,
    /// Scan commands for different modes.
    pub commands: ScannerCommands,
    /// Raw output format: `"sarif"` or `"json"`.
    pub output_format: String,
    /// Maps scanner-native severity labels to rice-guard severities.
    #[serde(default)]
    pub severity_map: HashMap<String, String>,
}

/// Install / availability metadata for a scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerInstall {
    /// Command used to verify the tool is installed, e.g. `"semgrep --version"`.
    pub check: String,
    /// Package-manager install commands keyed by manager name (pip, brew, npm, …).
    #[serde(flatten)]
    pub methods: HashMap<String, String>,
}

/// Available scan command variants for a scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerCommands {
    /// Full scan command (always required).
    pub scan: ScannerCommand,
    /// Security-focused subset (optional).
    #[serde(default)]
    pub security: Option<ScannerCommand>,
    /// Quick / incremental subset (optional).
    #[serde(default)]
    pub quick: Option<ScannerCommand>,
}

/// A single scanner invocation template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerCommand {
    /// Command template string. Placeholders: `{{output_dir}}`, `{{target}}`.
    pub cmd: String,
    /// Timeout in seconds before the scanner is killed.
    pub timeout: u32,
}

impl ScannerDescriptor {
    /// Validate that required fields are non-empty and structurally sound.
    pub fn validate(&self) -> Result<(), crate::errors::DescriptorError> {
        if self.name.is_empty() {
            return Err(crate::errors::DescriptorError::ValidationError {
                name: "(unnamed)".to_string(),
                reason: "name is empty".to_string(),
            });
        }
        if self.install.check.is_empty() {
            return Err(crate::errors::DescriptorError::ValidationError {
                name: self.name.clone(),
                reason: "install.check is empty".to_string(),
            });
        }
        if self.commands.scan.cmd.is_empty() {
            return Err(crate::errors::DescriptorError::ValidationError {
                name: self.name.clone(),
                reason: "commands.scan.cmd is empty".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEMGREP_YAML: &str = include_str!("../../../../descriptors/scanners/semgrep.yaml");
    const TRIVY_YAML: &str = include_str!("../../../../descriptors/scanners/trivy.yaml");
    const GITLEAKS_YAML: &str = include_str!("../../../../descriptors/scanners/gitleaks.yaml");
    const JSCPD_YAML: &str = include_str!("../../../../descriptors/scanners/jscpd.yaml");
    const SCC_YAML: &str = include_str!("../../../../descriptors/scanners/scc.yaml");

    #[test]
    fn semgrep_descriptor_parses() {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(SEMGREP_YAML).expect("semgrep.yaml must parse");
        assert_eq!(desc.name, "semgrep");
        assert!(!desc.install.check.is_empty());
        assert!(!desc.commands.scan.cmd.is_empty());
        assert_eq!(desc.output_format, "json");
        desc.validate().expect("semgrep descriptor must be valid");
    }

    #[test]
    fn trivy_descriptor_parses() {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(TRIVY_YAML).expect("trivy.yaml must parse");
        assert_eq!(desc.name, "trivy");
        desc.validate().expect("trivy descriptor must be valid");
    }

    #[test]
    fn gitleaks_descriptor_parses() {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(GITLEAKS_YAML).expect("gitleaks.yaml must parse");
        assert_eq!(desc.name, "gitleaks");
        desc.validate().expect("gitleaks descriptor must be valid");
    }

    #[test]
    fn jscpd_descriptor_parses() {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(JSCPD_YAML).expect("jscpd.yaml must parse");
        assert_eq!(desc.name, "jscpd");
        desc.validate().expect("jscpd descriptor must be valid");
    }

    #[test]
    fn scc_descriptor_parses() {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(SCC_YAML).expect("scc.yaml must parse");
        assert_eq!(desc.name, "scc");
        desc.validate().expect("scc descriptor must be valid");
    }

    #[test]
    fn malformed_yaml_returns_error() {
        let bad = "name: [invalid: yaml: structure: {{{{";
        let result = serde_yaml_ng::from_str::<ScannerDescriptor>(bad);
        assert!(result.is_err(), "malformed YAML must not parse");
    }

    #[test]
    fn all_scanner_descriptors_have_languages() {
        for (name, yaml) in [
            ("semgrep", SEMGREP_YAML),
            ("trivy", TRIVY_YAML),
            ("gitleaks", GITLEAKS_YAML),
            ("jscpd", JSCPD_YAML),
            ("scc", SCC_YAML),
        ] {
            let desc: ScannerDescriptor =
                serde_yaml_ng::from_str(yaml).unwrap_or_else(|e| panic!("{name}.yaml failed: {e}"));
            assert!(
                !desc.languages.is_empty(),
                "{name} must declare at least one language"
            );
        }
    }
}
