use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::model::{
    Architecture, CiConfig, FiltersConfig, FixersConfig, HooksConfig, OutputConfig,
    PreCommitConfig, ProjectConfig, RiceGuardConfig, ScannersConfig, ToolsConfig,
};
use crate::errors::InitError;
use crate::init::detector::DetectionResult;
use crate::init::wizard::{CiProvider, WizardChoices};
use crate::registry::probe::ProbeResult;

// ── Embedded templates ────────────────────────────────────────────────────────

/// Main config template, embedded at compile time.
///
/// Not used directly in generation (config is produced via serde_yaml_ng);
/// kept for `include_str!()` compile-time validation of the template file.
#[allow(dead_code)]
const TMPL_RICEGUARD: &str = include_str!("../../../../templates/riceguard.yaml.tmpl");

/// lefthook pre-commit template.
const TMPL_LEFTHOOK: &str = include_str!("../../../../templates/lefthook.yml.tmpl");

/// GitHub Actions workflow template.
const TMPL_GITHUB_ACTIONS: &str = include_str!("../../../../templates/github-actions.yml.tmpl");

/// GitLab CI template.
const TMPL_GITLAB_CI: &str = include_str!("../../../../templates/gitlab-ci.yml.tmpl");

/// Semgrep architecture rules template.
const TMPL_SEMGREP_ARCH: &str =
    include_str!("../../../../templates/semgrep/architecture.yaml.tmpl");

/// ast-grep patterns template.
const TMPL_AST_GREP: &str = include_str!("../../../../templates/ast-grep/patterns.yaml.tmpl");

// ── Public API ────────────────────────────────────────────────────────────────

/// All inputs needed to generate the config and supporting template files.
#[derive(Debug)]
pub struct GeneratorInput {
    /// Short project name (e.g. `"my-app"`).
    pub project_name: String,
    /// Absolute path to the project root.
    pub project_path: PathBuf,
    /// Result of the detection pipeline.
    pub detection: DetectionResult,
    /// Choices from the interactive wizard or `noninteractive_choices()`.
    pub choices: WizardChoices,
}

/// Generate the `.riceguard.yaml` config file and any optional support files.
///
/// Returns the list of files created on disk.
///
/// # Generated files
///
/// - `.riceguard.yaml` — always written
/// - `lefthook.yml` — when `choices.pre_commit == true`
/// - `.github/workflows/quality.yml` — when `ci_provider == Github`
/// - `.gitlab-ci.yml` — when `ci_provider == Gitlab`
/// - `.riceguard/semgrep/architecture.yaml` — when `architecture != None`
/// - `.riceguard/ast-grep/patterns.yaml` — always written
pub async fn generate(input: &GeneratorInput) -> Result<Vec<PathBuf>, InitError> {
    let mut created: Vec<PathBuf> = Vec::new();

    // ── Build RiceGuardConfig from detection + choices ───────────────────────
    let config = build_config(input);

    // ── Serialize to YAML ────────────────────────────────────────────────────
    let config_yaml =
        serde_yaml_ng::to_string(&config).map_err(|e| InitError::GenerationFailed {
            path: ".riceguard.yaml".into(),
            reason: e.to_string(),
        })?;

    // ── Write .riceguard.yaml ────────────────────────────────────────────────
    let config_path = input.project_path.join(".riceguard.yaml");
    write_file(&config_path, &config_yaml)?;
    created.push(normalize_path(&config_path));

    // ── lefthook.yml ─────────────────────────────────────────────────────────
    if input.choices.pre_commit {
        let content = TMPL_LEFTHOOK.replace("{{project_name}}", &input.project_name);
        let path = input.project_path.join("lefthook.yml");
        write_file(&path, &content)?;
        created.push(normalize_path(&path));
    }

    // ── CI workflows ─────────────────────────────────────────────────────────
    match &input.choices.ci_provider {
        CiProvider::Github => {
            let dir = input.project_path.join(".github").join("workflows");
            std::fs::create_dir_all(&dir).map_err(|e| InitError::GenerationFailed {
                path: dir.to_string_lossy().replace('\\', "/"),
                reason: e.to_string(),
            })?;
            let path = dir.join("quality.yml");
            write_file(&path, TMPL_GITHUB_ACTIONS)?;
            created.push(normalize_path(&path));
        }
        CiProvider::Gitlab => {
            let path = input.project_path.join(".gitlab-ci.yml");
            write_file(&path, TMPL_GITLAB_CI)?;
            created.push(normalize_path(&path));
        }
        CiProvider::None => {}
    }

    // ── Semgrep architecture rules ───────────────────────────────────────────
    if input.choices.architecture != Architecture::None {
        let dir = input.project_path.join(".riceguard").join("semgrep");
        std::fs::create_dir_all(&dir).map_err(|e| InitError::GenerationFailed {
            path: dir.to_string_lossy().replace('\\', "/"),
            reason: e.to_string(),
        })?;
        let primary_lang = input
            .detection
            .languages
            .first()
            .map(|l| l.name.to_lowercase())
            .unwrap_or_else(|| "generic".into());
        let arch_name = format!("{:?}", input.choices.architecture).to_lowercase();
        let content = TMPL_SEMGREP_ARCH
            .replace("{{project_name}}", &input.project_name)
            .replace("{{architecture}}", &arch_name)
            .replace("{{primary_language}}", &primary_lang);
        let path = dir.join("architecture.yaml");
        write_file(&path, &content)?;
        created.push(normalize_path(&path));
    }

    // ── ast-grep pattern stubs ───────────────────────────────────────────────
    {
        let dir = input.project_path.join(".riceguard").join("ast-grep");
        std::fs::create_dir_all(&dir).map_err(|e| InitError::GenerationFailed {
            path: dir.to_string_lossy().replace('\\', "/"),
            reason: e.to_string(),
        })?;
        let primary_lang = input
            .detection
            .languages
            .first()
            .map(|l| l.name.to_lowercase())
            .unwrap_or_else(|| "generic".into());
        let content = TMPL_AST_GREP
            .replace("{{project_name}}", &input.project_name)
            .replace("{{primary_language}}", &primary_lang);
        let path = dir.join("patterns.yaml");
        write_file(&path, &content)?;
        created.push(normalize_path(&path));
    }

    Ok(created)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a [`RiceGuardConfig`] from detection results and wizard choices.
fn build_config(input: &GeneratorInput) -> RiceGuardConfig {
    let languages: Vec<String> = input
        .detection
        .languages
        .iter()
        .map(|l| l.name.to_lowercase())
        .collect();

    // Scanner enable flags — only enable if probe returned Available.
    let semgrep_enabled = is_available(&input.detection.scanner_probes, "semgrep");
    let trivy_enabled = is_available(&input.detection.scanner_probes, "trivy");
    let gitleaks_enabled = is_available(&input.detection.scanner_probes, "gitleaks");
    let jscpd_enabled = is_available(&input.detection.scanner_probes, "jscpd");

    // Build ToolsConfig from probe results.
    let mut tools_scanners: HashMap<String, bool> = HashMap::new();
    for (name, probe) in &input.detection.scanner_probes {
        tools_scanners.insert(name.clone(), matches!(probe, ProbeResult::Available { .. }));
    }

    let mut tools_fixers: HashMap<String, HashMap<String, bool>> = HashMap::new();
    for (lang, steps) in &input.detection.fixer_probes {
        let mut step_map: HashMap<String, bool> = HashMap::new();
        for (step_name, probe) in steps {
            step_map.insert(
                step_name.clone(),
                matches!(probe, ProbeResult::Available { .. }),
            );
        }
        tools_fixers.insert(lang.clone(), step_map);
    }

    let ci_provider_str = match input.choices.ci_provider {
        CiProvider::Github => "github",
        CiProvider::Gitlab => "gitlab",
        CiProvider::None => "none",
    };

    RiceGuardConfig {
        version: "1".into(),
        project: ProjectConfig {
            name: input.project_name.clone(),
            languages,
            topology: input.choices.topology.clone(),
            architecture: input.choices.architecture.clone(),
        },
        scanners: ScannersConfig {
            semgrep: crate::config::model::SemgrepConfig {
                enabled: semgrep_enabled,
                rulesets: vec!["p/default".into()],
            },
            trivy: crate::config::model::TrivyConfig {
                enabled: trivy_enabled,
                scanners: vec!["vuln".into(), "secret".into()],
            },
            gitleaks: crate::config::model::GitleaksConfig {
                enabled: gitleaks_enabled,
            },
            jscpd: crate::config::model::JscpdConfig {
                enabled: jscpd_enabled,
                threshold: 10,
            },
            scc: crate::config::model::SccConfig { enabled: true },
            ..Default::default()
        },
        fixers: FixersConfig {
            formatters: true,
            linters: true,
            security: semgrep_enabled,
            ast: false,
            deps: true,
            r#unsafe: false,
        },
        filters: FiltersConfig {
            include: vec![],
            exclude: FiltersConfig::default_excludes(),
        },
        tools: ToolsConfig {
            scanners: tools_scanners,
            fixers: tools_fixers,
        },
        output: OutputConfig::default(),
        hooks: HooksConfig {
            pre_commit: PreCommitConfig {
                enabled: input.choices.pre_commit,
                manager: if input.choices.pre_commit {
                    "lefthook".into()
                } else {
                    String::new()
                },
                checks: if input.choices.pre_commit {
                    vec!["security".into(), "cve".into()]
                } else {
                    vec![]
                },
            },
        },
        ci: CiConfig {
            provider: ci_provider_str.into(),
            diff_only: true,
        },
    }
}

/// Returns `true` if the named tool in `probes` has an `Available` result.
fn is_available(probes: &HashMap<String, ProbeResult>, name: &str) -> bool {
    matches!(probes.get(name), Some(ProbeResult::Available { .. }))
}

/// Write content to `path`, creating parent directories as needed.
fn write_file(path: &Path, content: &str) -> Result<(), InitError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| InitError::GenerationFailed {
            path: parent.to_string_lossy().replace('\\', "/"),
            reason: e.to_string(),
        })?;
    }
    std::fs::write(path, content).map_err(|e| InitError::GenerationFailed {
        path: path.to_string_lossy().replace('\\', "/"),
        reason: e.to_string(),
    })
}

/// Normalize a path to use forward slashes (consistent across platforms).
fn normalize_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('\\', "/"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Architecture, RiceGuardConfig, Topology};
    use crate::init::detector::{DetectionResult, SccLanguage};
    use crate::init::wizard::{CiProvider, WizardChoices};
    use crate::registry::probe::ProbeResult;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_input(tmp: &TempDir) -> GeneratorInput {
        let mut scanner_probes = HashMap::new();
        scanner_probes.insert(
            "semgrep".into(),
            ProbeResult::Available {
                version: "1.90.0".into(),
            },
        );
        scanner_probes.insert("trivy".into(), ProbeResult::Missing);
        scanner_probes.insert("gitleaks".into(), ProbeResult::Missing);
        scanner_probes.insert("jscpd".into(), ProbeResult::Missing);

        let mut rust_steps = HashMap::new();
        rust_steps.insert(
            "rustfmt".into(),
            ProbeResult::Available { version: "".into() },
        );
        rust_steps.insert("clippy".into(), ProbeResult::Missing);
        let mut fixer_probes = HashMap::new();
        fixer_probes.insert("rust".into(), rust_steps);

        let detection = DetectionResult {
            languages: vec![SccLanguage {
                name: "Rust".into(),
                files: 12,
                lines: 500,
                code: 400,
            }],
            scanner_probes,
            fixer_probes,
            project_path: tmp.path().to_path_buf(),
        };

        let choices = WizardChoices {
            topology: Topology::Cli,
            architecture: Architecture::None,
            quality_checks: vec!["security".into(), "cve".into(), "secrets".into()],
            pre_commit: false,
            ci_provider: CiProvider::None,
        };

        GeneratorInput {
            project_name: "test-project".into(),
            project_path: tmp.path().to_path_buf(),
            detection,
            choices,
        }
    }

    /// `generate()` writes a `.riceguard.yaml` that parses back as
    /// `RiceGuardConfig` without error (round-trip test).
    #[tokio::test]
    async fn roundtrip() {
        let tmp = TempDir::new().unwrap();
        let input = make_input(&tmp);

        let created = generate(&input).await.expect("generate must succeed");
        assert!(
            !created.is_empty(),
            "generate must return at least one created file"
        );

        let config_path = tmp.path().join(".riceguard.yaml");
        assert!(
            config_path.exists(),
            ".riceguard.yaml must be written to disk"
        );

        let yaml = std::fs::read_to_string(&config_path).expect(".riceguard.yaml must be readable");

        let config: RiceGuardConfig =
            serde_yaml_ng::from_str(&yaml).expect(".riceguard.yaml must be valid YAML config");

        assert_eq!(config.version, "1");
        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.project.topology, Topology::Cli);
        assert_eq!(config.project.architecture, Architecture::None);
        assert!(
            config.project.languages.contains(&"rust".to_string()),
            "rust must be in languages list"
        );
    }

    /// Only tools marked `Available` in probe results must be `enabled: true`.
    #[tokio::test]
    async fn only_available_tools_enabled() {
        let tmp = TempDir::new().unwrap();
        let input = make_input(&tmp);

        generate(&input).await.expect("generate must succeed");
        let yaml = std::fs::read_to_string(tmp.path().join(".riceguard.yaml")).unwrap();
        let config: RiceGuardConfig = serde_yaml_ng::from_str(&yaml).unwrap();

        // semgrep was Available → must be enabled.
        assert!(
            config.scanners.semgrep.enabled,
            "semgrep must be enabled (was Available)"
        );
        // trivy was Missing → must be disabled.
        assert!(
            !config.scanners.trivy.enabled,
            "trivy must be disabled (was Missing)"
        );
        // ToolsConfig reflects probe availability.
        assert_eq!(
            config.tools.scanners.get("semgrep"),
            Some(&true),
            "tools.scanners.semgrep must be true"
        );
        assert_eq!(
            config.tools.scanners.get("trivy"),
            Some(&false),
            "tools.scanners.trivy must be false"
        );
    }

    /// When `choices.pre_commit == true`, lefthook.yml must be written.
    #[tokio::test]
    async fn pre_commit_writes_lefthook() {
        let tmp = TempDir::new().unwrap();
        let mut input = make_input(&tmp);
        input.choices.pre_commit = true;

        let created = generate(&input).await.expect("generate must succeed");
        let lefthook_path = tmp.path().join("lefthook.yml");
        assert!(lefthook_path.exists(), "lefthook.yml must be written");
        assert!(
            created
                .iter()
                .any(|p| p.to_string_lossy().ends_with("lefthook.yml")),
            "lefthook.yml must appear in created files list"
        );
    }

    /// When `ci_provider == Github`, `.github/workflows/quality.yml` is written.
    #[tokio::test]
    async fn github_ci_writes_workflow() {
        let tmp = TempDir::new().unwrap();
        let mut input = make_input(&tmp);
        input.choices.ci_provider = CiProvider::Github;

        generate(&input).await.expect("generate must succeed");
        let workflow_path = tmp
            .path()
            .join(".github")
            .join("workflows")
            .join("quality.yml");
        assert!(
            workflow_path.exists(),
            ".github/workflows/quality.yml must be written"
        );
    }

    /// All created file paths must use forward slashes (platform-neutral).
    #[tokio::test]
    async fn paths_use_forward_slashes() {
        let tmp = TempDir::new().unwrap();
        let input = make_input(&tmp);

        let created = generate(&input).await.expect("generate must succeed");
        for path in &created {
            let s = path.to_string_lossy();
            assert!(!s.contains('\\'), "path must not contain backslashes: {s}");
        }
    }
}
