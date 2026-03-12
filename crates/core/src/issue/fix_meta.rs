//! Fix metadata for code findings.
//!
//! Captures what kind of automated fix is applicable (if any) and which tool
//! can apply it. This is surfaced in the AI-ready JSON output so that an
//! orchestration layer can route issues to the appropriate fixer without
//! re-running the scanner.
//!
//! ## Fields
//!
//! - `auto_fixable` — true when a deterministic fixer can apply the fix
//! - `auto_fix_tool` — CLI command/tool name (e.g., `"ruff check --fix"`)
//! - `auto_fix_category` — category string: `"formatter"` | `"linter"` |
//!   `"security"` | `"ast"` | `"deps"` | `"import"` | `None`
//! - `suggested_replacement` — ready-to-apply code snippet when available
//! - `complexity` — effort estimate: `trivial` | `moderate` | `complex`

use serde::{Deserialize, Serialize};

/// Lightweight descriptor of a fixer tool for a specific language.
///
/// Created from fixer YAML descriptors at runtime (or directly in tests).
/// Used by [`FixMetadata::from_finding_with_descriptors`] to determine
/// whether a deterministic fixer is available for the file's language.
#[derive(Debug, Clone)]
pub struct FixerDescriptorInfo {
    /// Language identifier matching the fixer descriptor (e.g. `"python"`, `"rust"`).
    pub language: String,

    /// Names of the stages present in this descriptor (e.g. `["formatters", "linters"]`).
    pub stages: Vec<String>,

    /// The first non-empty fix command from the descriptor, if any.
    /// Used as the `auto_fix_tool` value in the resulting `FixMetadata`.
    pub fix_tool: Option<String>,
}

/// Effort complexity of applying a fix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FixComplexity {
    /// Simple, mechanical fix (e.g., formatter, single-line replacement).
    Trivial,
    /// Moderate effort (e.g., restructuring a function body).
    Moderate,
    /// Complex change (e.g., architectural refactoring, cross-file change).
    Complex,
}

/// Fix information attached to a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixMetadata {
    /// Whether a deterministic fixer can apply this fix automatically.
    pub auto_fixable: bool,

    /// Name of the tool/command that applies the fix.
    /// Example: `"ruff check --fix"`, `"cargo clippy --fix"`.
    /// `None` when `auto_fixable` is false.
    pub auto_fix_tool: Option<String>,

    /// Category of the fix tool.
    /// One of: `"formatter"`, `"linter"`, `"security"`, `"ast"`, `"deps"`, `"import"`.
    /// `None` when `auto_fixable` is false.
    pub auto_fix_category: Option<String>,

    /// Ready-to-apply replacement snippet, when available (from scanner autofix).
    pub suggested_replacement: Option<String>,

    /// Estimated effort complexity for applying the fix.
    pub complexity: FixComplexity,
}

impl FixMetadata {
    /// Build fix metadata for a finding with no known automated fix.
    pub fn manual(complexity: FixComplexity) -> Self {
        Self {
            auto_fixable: false,
            auto_fix_tool: None,
            auto_fix_category: None,
            suggested_replacement: None,
            complexity,
        }
    }

    /// Build fix metadata for a finding with a scanner-provided replacement.
    pub fn scanner_autofix(suggested_replacement: String, tool: String, category: String) -> Self {
        Self {
            auto_fixable: true,
            auto_fix_tool: Some(tool),
            auto_fix_category: Some(category),
            suggested_replacement: Some(suggested_replacement),
            complexity: FixComplexity::Trivial,
        }
    }

    /// Build fix metadata using fixer descriptor lookup.
    ///
    /// Resolution order:
    /// 1. If `suggested_replacement` present → scanner autofix (same as `from_finding`).
    /// 2. Match `file_ext` (case-insensitive) against descriptor `language` fields.
    ///    - Common language → extension mapping:
    ///      `python` → `.py`, `rust` → `.rs`, `go` → `.go`, `typescript` → `.ts`,
    ///      `javascript` → `.js`, `java` → `.java`, `kotlin` → `.kt`,
    ///      `php` → `.php`, `csharp`/`c#` → `.cs`, `ruby` → `.rb`,
    ///      `dart` → `.dart`, `shell`/`bash` → `.sh`.
    /// 3. If a descriptor matches, `auto_fixable=true` with the descriptor's fix_tool.
    /// 4. Else fall back to `from_finding` heuristics.
    pub fn from_finding_with_descriptors(
        scanner: &str,
        rule_id: &str,
        suggested_replacement: Option<&str>,
        file_ext: &str,
        descriptors: &[FixerDescriptorInfo],
    ) -> Self {
        // Step 1: scanner-provided replacement takes priority.
        if let Some(snippet) = suggested_replacement {
            return Self::scanner_autofix(
                snippet.to_string(),
                format!("semgrep --autofix --config={rule_id} <file>"),
                "security".to_string(),
            );
        }

        // Step 2: try to find a descriptor for the file's language.
        let ext_lower = file_ext.to_lowercase();
        let lang = ext_to_language(&ext_lower);

        if let Some(lang_name) = lang {
            if let Some(descriptor) = descriptors
                .iter()
                .find(|d| d.language.to_lowercase() == lang_name)
            {
                // Determine category from first matching stage.
                let category = descriptor
                    .stages
                    .first()
                    .map(|s| stage_to_category(s))
                    .unwrap_or("linter");

                let complexity = match category {
                    "formatter" | "import" => FixComplexity::Trivial,
                    "linter" | "security" => FixComplexity::Trivial,
                    _ => FixComplexity::Moderate,
                };

                return Self {
                    auto_fixable: true,
                    auto_fix_tool: descriptor.fix_tool.clone(),
                    auto_fix_category: Some(category.to_string()),
                    suggested_replacement: None,
                    complexity,
                };
            }
        }

        // Step 3: fall back to heuristic-based lookup.
        Self::from_finding(scanner, rule_id, None)
    }

    /// Build fix metadata from scanner fields and rule identifiers.
    ///
    /// Resolution order:
    /// 1. If `suggested_replacement` present → scanner autofix via Semgrep.
    /// 2. If scanner is `"clippy"` or rule starts with `"clippy::"` → linter fix.
    /// 3. Else → manual.
    pub fn from_finding(scanner: &str, rule_id: &str, suggested_replacement: Option<&str>) -> Self {
        if let Some(snippet) = suggested_replacement {
            return Self::scanner_autofix(
                snippet.to_string(),
                format!("semgrep --autofix --config={rule_id} <file>"),
                "security".to_string(),
            );
        }

        if scanner == "clippy" || rule_id.starts_with("clippy::") {
            return Self {
                auto_fixable: true,
                auto_fix_tool: Some("cargo clippy --fix --allow-dirty".to_string()),
                auto_fix_category: Some("linter".to_string()),
                suggested_replacement: None,
                complexity: FixComplexity::Trivial,
            };
        }

        Self::manual(FixComplexity::Moderate)
    }
}

/// Map a file extension to a canonical language name matching fixer descriptors.
///
/// Returns `None` for unsupported extensions.
fn ext_to_language(ext: &str) -> Option<&'static str> {
    match ext {
        "py" => Some("python"),
        "rs" => Some("rust"),
        "go" => Some("go"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "java" => Some("java"),
        "kt" => Some("kotlin"),
        "php" => Some("php"),
        "cs" => Some("csharp"),
        "rb" => Some("ruby"),
        "dart" => Some("dart"),
        "sh" | "bash" => Some("shell"),
        _ => None,
    }
}

/// Map a fixer descriptor stage name to a fix category string.
fn stage_to_category(stage: &str) -> &'static str {
    match stage {
        "formatters" => "formatter",
        "linters" => "linter",
        "security" => "security",
        "ast" => "ast",
        "deps" => "deps",
        "imports" => "import",
        // singular forms (for compatibility)
        "formatter" => "formatter",
        "linter" => "linter",
        _ => "linter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_autofix_when_replacement_present() {
        let meta = FixMetadata::from_finding(
            "semgrep",
            "python.lang.security.eval-injection",
            Some("ast.literal_eval(user_input)"),
        );
        assert!(meta.auto_fixable);
        assert!(meta.auto_fix_tool.is_some());
        assert!(meta.auto_fix_category.is_some());
        assert_eq!(
            meta.suggested_replacement.as_deref(),
            Some("ast.literal_eval(user_input)")
        );
        assert_eq!(meta.complexity, FixComplexity::Trivial);
    }

    #[test]
    fn clippy_fix_detected() {
        let meta = FixMetadata::from_finding("clippy", "clippy::needless_return", None);
        assert!(meta.auto_fixable);
        assert_eq!(meta.auto_fix_category.as_deref(), Some("linter"));
        assert_eq!(meta.complexity, FixComplexity::Trivial);
    }

    #[test]
    fn manual_when_no_known_fix() {
        let meta = FixMetadata::from_finding("trivy", "CVE-2023-12345", None);
        assert!(!meta.auto_fixable);
        assert!(meta.auto_fix_tool.is_none());
        assert!(meta.auto_fix_category.is_none());
    }

    #[test]
    fn manual_constructor() {
        let meta = FixMetadata::manual(FixComplexity::Complex);
        assert!(!meta.auto_fixable);
        assert_eq!(meta.complexity, FixComplexity::Complex);
    }
}
