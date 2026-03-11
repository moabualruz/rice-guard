use serde::{Deserialize, Serialize};

/// Describes all deterministic fixers for a single language.
/// Loaded from YAML descriptor files at compile time (built-in) or runtime
/// (user-provided). Drives `fix` stage ordering and `init` tool probing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixerDescriptor {
    /// Canonical language name, e.g. `"rust"` or `"go"`.
    pub name: String,
    /// Language identifier (matches scc / tree-sitter language names).
    pub language: String,
    /// Build-file globs used to detect this language, e.g. `["Cargo.toml"]`.
    pub detect: Vec<String>,
    /// Ordered fix stages: format → lint → security → ast → deps → import.
    pub stages: FixerStages,
}

/// Six deterministic fix stages executed in a fixed order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FixerStages {
    /// Code formatters (always safe, always run first).
    #[serde(default)]
    pub format: Vec<FixerStep>,
    /// Linter auto-fix (may have unsafe variants).
    #[serde(default)]
    pub lint: Vec<FixerStep>,
    /// Security pattern auto-fix (Semgrep --autofix, etc.).
    #[serde(default)]
    pub security: Vec<FixerStep>,
    /// AST-level rewrites (ast-grep rules).
    #[serde(default)]
    pub ast: Vec<FixerStep>,
    /// Dependency updates (semver-safe).
    #[serde(default)]
    pub deps: Vec<FixerStep>,
    /// Import cleanup (unused imports, sort order).
    #[serde(default)]
    pub import: Vec<FixerStep>,
}

/// A single fixer invocation (check + fix pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixerStep {
    /// Display name shown in CLI output.
    pub name: String,
    /// Command that exits non-zero when fixes are needed.
    /// May contain `{{files}}` placeholder for per-file mode.
    pub check: String,
    /// Command that applies the fix in-place.
    pub fix: String,
    /// `"file"` (per-file, parallel-safe) or `"project"` (whole-project, single-threaded).
    pub scope: String,
    /// Whether this step is always safe to run without `--unsafe` flag.
    pub safe: bool,
    /// Extra flag added when `--unsafe` mode is active (e.g. `"--allow-dirty"`).
    #[serde(default)]
    pub unsafe_flag: Option<String>,
}

impl FixerDescriptor {
    /// Returns `true` if at least one stage contains at least one step.
    pub fn has_any_stage(&self) -> bool {
        !self.stages.format.is_empty()
            || !self.stages.lint.is_empty()
            || !self.stages.security.is_empty()
            || !self.stages.ast.is_empty()
            || !self.stages.deps.is_empty()
            || !self.stages.import.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GO_YAML: &str = include_str!("../../../../descriptors/fixers/go.yaml");
    const RUST_YAML: &str = include_str!("../../../../descriptors/fixers/rust.yaml");
    const PYTHON_YAML: &str = include_str!("../../../../descriptors/fixers/python.yaml");
    const DART_YAML: &str = include_str!("../../../../descriptors/fixers/dart.yaml");
    const TYPESCRIPT_YAML: &str = include_str!("../../../../descriptors/fixers/typescript.yaml");
    const JAVASCRIPT_YAML: &str = include_str!("../../../../descriptors/fixers/javascript.yaml");
    const JAVA_YAML: &str = include_str!("../../../../descriptors/fixers/java.yaml");
    const KOTLIN_YAML: &str = include_str!("../../../../descriptors/fixers/kotlin.yaml");
    const PHP_YAML: &str = include_str!("../../../../descriptors/fixers/php.yaml");
    const CSHARP_YAML: &str = include_str!("../../../../descriptors/fixers/csharp.yaml");
    const RUBY_YAML: &str = include_str!("../../../../descriptors/fixers/ruby.yaml");
    const SHELL_YAML: &str = include_str!("../../../../descriptors/fixers/shell.yaml");

    #[test]
    fn rust_fixer_descriptor_parses() {
        let desc: FixerDescriptor =
            serde_yaml_ng::from_str(RUST_YAML).expect("rust.yaml must parse");
        assert_eq!(desc.name, "rust");
        assert_eq!(desc.language, "rust");
        assert!(!desc.detect.is_empty(), "rust must have detect patterns");
        assert!(
            desc.has_any_stage(),
            "rust must have at least one fixer stage"
        );
    }

    #[test]
    fn rust_fixer_has_format_and_lint_stages() {
        let desc: FixerDescriptor =
            serde_yaml_ng::from_str(RUST_YAML).expect("rust.yaml must parse");
        assert!(
            !desc.stages.format.is_empty(),
            "rust must have format steps"
        );
        assert!(!desc.stages.lint.is_empty(), "rust must have lint steps");
    }

    #[test]
    fn all_fixer_descriptors_parse() {
        let fixtures = [
            ("go", GO_YAML),
            ("rust", RUST_YAML),
            ("python", PYTHON_YAML),
            ("dart", DART_YAML),
            ("typescript", TYPESCRIPT_YAML),
            ("javascript", JAVASCRIPT_YAML),
            ("java", JAVA_YAML),
            ("kotlin", KOTLIN_YAML),
            ("php", PHP_YAML),
            ("csharp", CSHARP_YAML),
            ("ruby", RUBY_YAML),
            ("shell", SHELL_YAML),
        ];
        for (name, yaml) in fixtures {
            let desc: FixerDescriptor = serde_yaml_ng::from_str(yaml)
                .unwrap_or_else(|e| panic!("{name}.yaml failed to parse: {e}"));
            assert_eq!(desc.name, name, "{name}: name field mismatch");
            assert!(
                !desc.detect.is_empty(),
                "{name}: detect list must be non-empty"
            );
            assert!(
                desc.has_any_stage(),
                "{name}: must have at least one fixer stage"
            );
        }
    }

    #[test]
    fn fixer_step_safe_field_present() {
        let desc: FixerDescriptor =
            serde_yaml_ng::from_str(RUST_YAML).expect("rust.yaml must parse");
        for step in &desc.stages.format {
            // All format steps must be safe
            assert!(step.safe, "format step '{}' must be safe", step.name);
        }
    }

    #[test]
    fn malformed_fixer_yaml_returns_error() {
        let bad = "name: {broken: [yaml";
        let result = serde_yaml_ng::from_str::<FixerDescriptor>(bad);
        assert!(result.is_err(), "malformed fixer YAML must not parse");
    }
}
