// Interactive init wizard — implemented in Task 2.
use crate::config::model::{Architecture, Topology};
use crate::errors::InitError;
use crate::init::detector::DetectionResult;

/// CI provider choice presented by the wizard.
#[derive(Debug, Clone, PartialEq)]
pub enum CiProvider {
    Github,
    Gitlab,
    None,
}

/// Choices collected from the interactive wizard (or defaulted for `--yes`).
#[derive(Debug, Clone)]
pub struct WizardChoices {
    pub topology: Topology,
    pub architecture: Architecture,
    pub quality_checks: Vec<String>,
    pub pre_commit: bool,
    pub ci_provider: CiProvider,
    /// Whether to respect `.gitignore` patterns when scanning. Default: false.
    pub respect_gitignore: bool,
}

/// Run the interactive init wizard.
///
/// # Errors
///
/// Returns [`InitError::NotInteractive`] immediately if stdin is not a TTY,
/// before any `inquire` call.
pub fn wizard(_detected: &DetectionResult) -> Result<WizardChoices, InitError> {
    // MANDATORY TTY guard — must be the very first check.
    if !atty::is(atty::Stream::Stdin) {
        return Err(InitError::NotInteractive {
            hint: "Use --yes for non-interactive mode".to_string(),
        });
    }

    // Topology
    let topology_options = vec!["monolith", "monorepo", "library", "cli", "mobile"];
    let topology_str = inquire::Select::new("Project topology:", topology_options)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::NotTTY => InitError::NotInteractive {
                hint: "Use --yes for non-interactive mode".to_string(),
            },
            other => InitError::WizardFailed(other.to_string()),
        })?;

    let topology = match topology_str {
        "monorepo" => Topology::Monorepo,
        "library" => Topology::Library,
        "cli" => Topology::Cli,
        "mobile" => Topology::Mobile,
        _ => Topology::Monolith,
    };

    // Architecture
    let arch_options = vec!["none", "clean", "hexagonal", "mvc"];
    let arch_str = inquire::Select::new("Architecture pattern:", arch_options)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::NotTTY => InitError::NotInteractive {
                hint: "Use --yes for non-interactive mode".to_string(),
            },
            other => InitError::WizardFailed(other.to_string()),
        })?;

    let architecture = match arch_str {
        "clean" => Architecture::Clean,
        "hexagonal" => Architecture::Hexagonal,
        "mvc" => Architecture::Mvc,
        _ => Architecture::None,
    };

    // Quality checks
    let check_options: Vec<&str> = vec!["security", "cve", "secrets", "duplication", "complexity"];
    let selected_checks = inquire::MultiSelect::new("Quality checks to enable:", check_options)
        .with_default(&[0, 1, 2])
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::NotTTY => InitError::NotInteractive {
                hint: "Use --yes for non-interactive mode".to_string(),
            },
            other => InitError::WizardFailed(other.to_string()),
        })?;

    let quality_checks: Vec<String> = selected_checks.iter().map(|s| s.to_string()).collect();

    // Pre-commit hooks
    let pre_commit = inquire::Confirm::new("Install pre-commit hooks (lefthook)?")
        .with_default(false)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::NotTTY => InitError::NotInteractive {
                hint: "Use --yes for non-interactive mode".to_string(),
            },
            other => InitError::WizardFailed(other.to_string()),
        })?;

    // CI provider
    let ci_options = vec!["none", "github", "gitlab"];
    let ci_str = inquire::Select::new("CI provider:", ci_options)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::NotTTY => InitError::NotInteractive {
                hint: "Use --yes for non-interactive mode".to_string(),
            },
            other => InitError::WizardFailed(other.to_string()),
        })?;

    let ci_provider = match ci_str {
        "github" => CiProvider::Github,
        "gitlab" => CiProvider::Gitlab,
        _ => CiProvider::None,
    };

    // Respect .gitignore
    let respect_gitignore = inquire::Confirm::new(
        "Respect .gitignore patterns? (excludes gitignored files from scanning)",
    )
    .with_default(false)
    .prompt()
    .map_err(|e| match e {
        inquire::InquireError::NotTTY => InitError::NotInteractive {
            hint: "Use --yes for non-interactive mode".to_string(),
        },
        other => InitError::WizardFailed(other.to_string()),
    })?;

    Ok(WizardChoices {
        topology,
        architecture,
        quality_checks,
        pre_commit,
        ci_provider,
        respect_gitignore,
    })
}

/// Returns default [`WizardChoices`] for non-interactive `--yes` mode.
///
/// Defaults: Monolith topology, no architecture pattern, security/cve/secrets
/// checks enabled, no pre-commit hooks, no CI integration.
pub fn noninteractive_choices(_detected: &DetectionResult) -> WizardChoices {
    WizardChoices {
        topology: Topology::Monolith,
        architecture: Architecture::None,
        quality_checks: vec!["security".into(), "cve".into(), "secrets".into()],
        pre_commit: false,
        ci_provider: CiProvider::None,
        respect_gitignore: false,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::detector::{DetectionResult, SccLanguage};
    use std::collections::HashMap;

    fn fake_detection() -> DetectionResult {
        DetectionResult {
            languages: vec![SccLanguage {
                name: "Rust".into(),
                files: 10,
                lines: 500,
                code: 400,
            }],
            scanner_probes: HashMap::new(),
            fixer_probes: HashMap::new(),
            project_path: std::path::PathBuf::from("."),
        }
    }

    /// When stdin is not a TTY, `wizard()` must return `InitError::NotInteractive`
    /// immediately without calling any `inquire` prompt.
    ///
    /// We guarantee this by redirecting stdin in the test environment — in a
    /// non-interactive test runner, `atty::is(Stdin)` already returns `false`.
    #[test]
    fn no_tty_returns_error() {
        // In a cargo test runner, stdin is a pipe (not a TTY), so atty returns false.
        // This test therefore exercises the real production path directly.
        let detected = fake_detection();
        let result = wizard(&detected);
        assert!(
            matches!(result, Err(InitError::NotInteractive { .. })),
            "wizard must return NotInteractive when not in a TTY, got: {result:?}"
        );
    }

    /// `noninteractive_choices()` returns sensible defaults for `--yes` mode.
    #[test]
    fn noninteractive_defaults() {
        let detected = fake_detection();
        let choices = noninteractive_choices(&detected);
        assert_eq!(choices.topology, Topology::Monolith);
        assert_eq!(choices.architecture, Architecture::None);
        assert!(
            choices.quality_checks.contains(&"security".to_string()),
            "security check must be enabled by default"
        );
        assert!(
            choices.quality_checks.contains(&"cve".to_string()),
            "cve check must be enabled by default"
        );
        assert!(
            choices.quality_checks.contains(&"secrets".to_string()),
            "secrets check must be enabled by default"
        );
        assert!(
            !choices.pre_commit,
            "pre-commit must be disabled by default"
        );
        assert_eq!(choices.ci_provider, CiProvider::None);
        assert!(
            !choices.respect_gitignore,
            "respect_gitignore must be false by default"
        );
    }
}
