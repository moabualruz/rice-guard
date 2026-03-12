//! Evidence extraction for findings.
//!
//! Evidence provides the surrounding code context for a finding, making the
//! output self-contained for AI consumption and human review.
//!
//! ## Fields
//!
//! - `matched_code` — the flagged code snippet (from scanner or line-window)
//! - `context_before` — lines before the finding
//! - `context_after` — lines after the finding
//! - `enclosing_function` — function/method name extracted via tree-sitter
//! - `enclosing_class` — class/struct name extracted via tree-sitter
//! - `imports` — file-level import statements extracted via tree-sitter

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

use crate::scanner::parser::RawFinding;

/// Pre-embedded code evidence for a finding.
///
/// Designed to be serialized into the AI-ready JSON output so that an LLM
/// can reason about the finding without access to the source repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBlock {
    /// The flagged code snippet as reported by the scanner.
    /// May be an empty string when the scanner provided no snippet.
    pub matched_code: String,

    /// Lines immediately before the finding (up to ±5 lines by default).
    pub context_before: Vec<String>,

    /// Lines immediately after the finding.
    pub context_after: Vec<String>,

    /// Name of the function/method that contains the finding.
    /// `None` when at module level or when tree-sitter parsing failed.
    pub enclosing_function: Option<String>,

    /// Name of the class/struct/interface that contains the finding.
    /// `None` when at module level or when tree-sitter parsing failed.
    pub enclosing_class: Option<String>,

    /// File-level import/use statements extracted from the source file.
    /// Empty when tree-sitter parsing failed or file has no imports.
    pub imports: Vec<String>,
}

impl EvidenceBlock {
    /// Build an `EvidenceBlock` from a scanner-provided code snippet and
    /// a surrounding line window extracted from the source file.
    ///
    /// This is the line-window-only constructor used until tree-sitter
    /// extraction is implemented in Plan 03-02.
    pub fn from_line_window(
        matched_code: String,
        context_before: Vec<String>,
        context_after: Vec<String>,
    ) -> Self {
        Self {
            matched_code,
            context_before,
            context_after,
            enclosing_function: None,
            enclosing_class: None,
            imports: vec![],
        }
    }
}

/// Number of context lines to include above and below the finding line.
pub const CONTEXT_LINES: usize = 5;

/// When the enclosing function body has fewer than this many lines, the full
/// function body is used as context instead of the ±`CONTEXT_LINES` window.
pub const SMALL_FUNCTION_LINES: usize = 15;

// ── Line-number conversion ────────────────────────────────────────────────────

/// Convert a 1-based SARIF line number to a 0-based Vec index.
///
/// Called exactly once at the evidence extraction boundary to avoid
/// off-by-one errors (see RESEARCH.md Pitfall 2).
#[inline]
fn line_to_index(line: u32) -> usize {
    line.saturating_sub(1) as usize
}

/// Extract context lines and matched code from `source` given a 1-based line.
///
/// Returns `(matched_line, context_before, context_after)`.
/// - `matched_line` is the source line at `line` (empty string if out-of-range)
/// - `context_before` contains up to `window` lines before the finding
/// - `context_after` contains up to `window` lines after the finding
fn extract_context(source: &str, line: u32, window: usize) -> (String, Vec<String>, Vec<String>) {
    let lines: Vec<&str> = source.lines().collect();
    let idx = line_to_index(line);

    let matched = lines.get(idx).map(|s| s.to_string()).unwrap_or_default();

    let before_start = idx.saturating_sub(window);
    let before: Vec<String> = lines[before_start..idx.min(lines.len())]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let after_end = (idx + 1 + window).min(lines.len());
    let after: Vec<String> = if idx < lines.len() {
        lines[(idx + 1).min(lines.len())..after_end]
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    };

    (matched, before, after)
}

// ── LanguageNodeKinds ─────────────────────────────────────────────────────────

/// Per-language AST node kind names for evidence extraction.
///
/// Each language uses different node kind strings in tree-sitter; this struct
/// holds the relevant kinds for function/class scope detection and import
/// extraction.
pub struct LanguageNodeKinds {
    /// Node kinds that represent function or method definitions.
    pub function_kinds: &'static [&'static str],
    /// Node kinds that represent class, struct, trait, or interface definitions.
    pub class_kinds: &'static [&'static str],
    /// Node kinds that represent import/use/require statements at file scope.
    pub import_kinds: &'static [&'static str],
}

/// Return the [`LanguageNodeKinds`] for a given file extension.
///
/// Returns `None` for unsupported extensions.
fn language_node_kinds(ext: &str) -> Option<LanguageNodeKinds> {
    match ext {
        "py" => Some(LanguageNodeKinds {
            function_kinds: &["function_definition"],
            class_kinds: &["class_definition"],
            import_kinds: &["import_statement", "import_from_statement"],
        }),
        "rs" => Some(LanguageNodeKinds {
            function_kinds: &["function_item"],
            class_kinds: &["struct_item", "impl_item", "trait_item"],
            import_kinds: &["use_declaration"],
        }),
        "go" => Some(LanguageNodeKinds {
            function_kinds: &["function_declaration", "method_declaration"],
            class_kinds: &["type_declaration"],
            import_kinds: &["import_declaration"],
        }),
        "js" | "jsx" | "mjs" | "cjs" => Some(LanguageNodeKinds {
            function_kinds: &[
                "function_declaration",
                "arrow_function",
                "method_definition",
            ],
            class_kinds: &["class_declaration"],
            import_kinds: &["import_statement"],
        }),
        "ts" | "tsx" => Some(LanguageNodeKinds {
            function_kinds: &[
                "function_declaration",
                "arrow_function",
                "method_definition",
            ],
            class_kinds: &["class_declaration"],
            import_kinds: &["import_statement"],
        }),
        "java" => Some(LanguageNodeKinds {
            function_kinds: &["method_declaration"],
            class_kinds: &["class_declaration"],
            import_kinds: &["import_declaration"],
        }),
        "kt" => Some(LanguageNodeKinds {
            function_kinds: &["function_declaration"],
            class_kinds: &["class_declaration"],
            import_kinds: &["import_header"],
        }),
        "php" => Some(LanguageNodeKinds {
            function_kinds: &["function_definition"],
            class_kinds: &["class_declaration"],
            import_kinds: &["namespace_use_declaration"],
        }),
        "cs" => Some(LanguageNodeKinds {
            function_kinds: &["method_declaration"],
            class_kinds: &["class_declaration"],
            import_kinds: &["using_directive"],
        }),
        "rb" => Some(LanguageNodeKinds {
            function_kinds: &["method", "singleton_method"],
            class_kinds: &["class", "module"],
            import_kinds: &["call"],
        }),
        "sh" | "bash" => Some(LanguageNodeKinds {
            function_kinds: &["function_definition"],
            class_kinds: &[],
            import_kinds: &["command"],
        }),
        "dart" => Some(LanguageNodeKinds {
            function_kinds: &["function_signature", "method_signature"],
            class_kinds: &["class_definition"],
            import_kinds: &["import_or_export"],
        }),
        _ => None,
    }
}

// ── Tree-sitter AST walking helpers ──────────────────────────────────────────

/// Walk ancestors of the node covering `line` (0-based) looking for any node
/// whose kind is in `kinds`. Returns the first matching ancestor `Node`, or
/// `None` if no ancestor of that kind is found.
///
/// Used by both [`find_enclosing_by_kinds`] (name extraction) and the
/// small-function body logic (range extraction) to avoid duplicating the
/// ancestor-walking loop.
fn find_enclosing_node_by_kinds<'a>(
    root: Node<'a>,
    line: usize,
    kinds: &[&str],
) -> Option<Node<'a>> {
    if kinds.is_empty() {
        return None;
    }
    let point = tree_sitter::Point {
        row: line,
        column: 0,
    };
    let leaf = root.descendant_for_point_range(point, point)?;
    let mut cursor = leaf;
    loop {
        if kinds.contains(&cursor.kind()) {
            return Some(cursor);
        }
        match cursor.parent() {
            Some(p) => cursor = p,
            None => break,
        }
    }
    None
}

/// Walk ancestors of the node covering `line` (0-based) looking for any node
/// whose kind is in `kinds`. Returns the text of the first matching ancestor's
/// identifier child, or `None`.
fn find_enclosing_by_kinds(
    root: Node<'_>,
    line: usize,
    kinds: &[&str],
    source_bytes: &[u8],
) -> Option<String> {
    let node = find_enclosing_node_by_kinds(root, line, kinds)?;

    // Extract the name from an identifier or name child.
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        if matches!(child.kind(), "identifier" | "name" | "simple_identifier") {
            if let Ok(text) = child.utf8_text(source_bytes) {
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }
    // If no named identifier child, return the node text itself (trimmed).
    if let Ok(text) = node.utf8_text(source_bytes) {
        let snippet: String = text.chars().take(80).collect();
        let first_line = snippet.lines().next().unwrap_or("").trim().to_string();
        if !first_line.is_empty() {
            return Some(first_line);
        }
    }
    None
}

/// Collect text of all direct children of `root` whose kind is in `kinds`.
fn extract_imports_from_tree(root: Node<'_>, source_bytes: &[u8], kinds: &[&str]) -> Vec<String> {
    if kinds.is_empty() {
        return vec![];
    }
    let mut imports = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if kinds.contains(&child.kind()) {
            if let Ok(text) = child.utf8_text(source_bytes) {
                let s = text.trim().to_string();
                if !s.is_empty() {
                    imports.push(s);
                }
            }
        }
    }
    imports
}

// ── EvidenceExtractor ─────────────────────────────────────────────────────────

/// Extracts evidence blocks for scanner findings using line-number math
/// (matched_code + context_before/after) and tree-sitter AST walking
/// (enclosing_function, enclosing_class, imports).
///
/// ## File-grouping (EVID-04)
///
/// All findings for a file are processed from a single parse tree
/// (`extract_file`). Callers should group findings by `file_path` and call
/// `extract_file` once per file.
pub struct EvidenceExtractor {
    /// Map from file extension → tree-sitter Language
    language_registry: HashMap<String, Language>,
}

impl EvidenceExtractor {
    /// Build a new `EvidenceExtractor` with all available language grammars.
    pub fn new() -> Self {
        let mut registry: HashMap<String, Language> = HashMap::new();

        macro_rules! register {
            ($ext:expr, $lang_fn:expr) => {
                registry.insert($ext.to_string(), Language::new($lang_fn));
            };
        }

        register!("py", tree_sitter_python::LANGUAGE);
        register!("rs", tree_sitter_rust::LANGUAGE);
        register!("go", tree_sitter_go::LANGUAGE);
        register!("js", tree_sitter_javascript::LANGUAGE);
        register!("jsx", tree_sitter_javascript::LANGUAGE);
        register!("mjs", tree_sitter_javascript::LANGUAGE);
        register!("cjs", tree_sitter_javascript::LANGUAGE);
        register!("ts", tree_sitter_typescript::LANGUAGE_TYPESCRIPT);
        register!("tsx", tree_sitter_typescript::LANGUAGE_TSX);
        register!("java", tree_sitter_java::LANGUAGE);
        register!("php", tree_sitter_php::LANGUAGE_PHP);
        register!("cs", tree_sitter_c_sharp::LANGUAGE);
        register!("rb", tree_sitter_ruby::LANGUAGE);
        register!("sh", tree_sitter_bash::LANGUAGE);
        register!("bash", tree_sitter_bash::LANGUAGE);

        #[cfg(feature = "full-grammars")]
        {
            register!("dart", tree_sitter_dart::LANGUAGE);
            register!("kt", tree_sitter_kotlin::LANGUAGE);
        }

        Self {
            language_registry: registry,
        }
    }

    /// Extract the context lines (matched_code, context_before, context_after)
    /// for a single finding using only line-number arithmetic.
    ///
    /// Tree-sitter fields (enclosing_function, enclosing_class, imports) are
    /// not populated here — use `extract_file` for the full evidence block.
    pub fn extract_line_context(
        &self,
        raw: &RawFinding,
        source: &str,
    ) -> (String, Vec<String>, Vec<String>) {
        let (source_line, before, after) = extract_context(source, raw.line, CONTEXT_LINES);
        let matched_code = raw
            .matched_code
            .clone()
            .unwrap_or_else(|| source_line.clone());
        (matched_code, before, after)
    }

    /// Parse `source` for the given file extension. Returns `None` if the
    /// extension is unsupported or parsing fails (graceful degradation).
    fn parse_source(&self, ext: &str, source: &str) -> Option<tree_sitter::Tree> {
        let lang = self.language_registry.get(ext)?;
        let mut parser = Parser::new();
        parser.set_language(lang).ok()?;
        parser.parse(source.as_bytes(), None)
    }

    /// Extract evidence for all `findings` in a single file.
    ///
    /// The source file is parsed exactly **once** (EVID-04). All queries
    /// (enclosing function/class, imports) are answered from that single tree.
    ///
    /// Returns a map keyed by `(rule_id, line)` → [`EvidenceBlock`].
    pub fn extract_file(
        &self,
        file_path: &str,
        source: &str,
        findings: &[&RawFinding],
    ) -> HashMap<(String, u32), EvidenceBlock> {
        // Detect language from extension.
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Parse source once (may be None for unsupported/broken files).
        let tree = self.parse_source(&ext, source);
        let node_kinds = language_node_kinds(&ext);
        let source_bytes = source.as_bytes();

        let mut result = HashMap::new();

        for f in findings {
            let (matched_code, mut context_before, mut context_after) =
                self.extract_line_context(f, source);

            let (enclosing_function, enclosing_class, imports) = if let (Some(tree), Some(nk)) =
                (tree.as_ref(), node_kinds.as_ref())
            {
                let root = tree.root_node();
                let ts_line = line_to_index(f.line);

                // Attempt to find the enclosing function node for small-function
                // body inclusion. If the function is < SMALL_FUNCTION_LINES,
                // replace context_before/after with the full function body.
                if let Some(func_node) =
                    find_enclosing_node_by_kinds(root, ts_line, nk.function_kinds)
                {
                    let start_row = func_node.start_position().row; // 0-based
                    let end_row = func_node.end_position().row; // 0-based, inclusive
                    let body_lines = end_row.saturating_sub(start_row) + 1;

                    if body_lines < SMALL_FUNCTION_LINES {
                        let all_lines: Vec<&str> = source.lines().collect();
                        let func_end = end_row.min(all_lines.len().saturating_sub(1));
                        let finding_row = ts_line; // 0-based index of the finding line

                        // context_before: function body lines before the finding.
                        let before_end = finding_row.min(func_end + 1);
                        context_before = all_lines[start_row..before_end]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();

                        // context_after: function body lines after the finding.
                        let after_start = (finding_row + 1).min(func_end + 1);
                        context_after = all_lines[after_start..=func_end]
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                    }
                }

                let func = find_enclosing_by_kinds(root, ts_line, nk.function_kinds, source_bytes);
                let class = find_enclosing_by_kinds(root, ts_line, nk.class_kinds, source_bytes);
                let imps = extract_imports_from_tree(root, source_bytes, nk.import_kinds);
                (func, class, imps)
            } else {
                (None, None, vec![])
            };

            let block = EvidenceBlock {
                matched_code,
                context_before,
                context_after,
                enclosing_function,
                enclosing_class,
                imports,
            };

            result.insert((f.rule_id.clone(), f.line), block);
        }

        result
    }
}

impl Default for EvidenceExtractor {
    fn default() -> Self {
        Self::new()
    }
}

// ── Legacy line-window extraction (used by IssueBuilder until fully wired) ───

/// Extract an [`EvidenceBlock`] for a finding using line-window strategy.
///
/// Tree-sitter extraction (enclosing_function, enclosing_class, imports) is
/// fully implemented via [`EvidenceExtractor`]. This function provides the
/// line-window portion only (used by `IssueBuilder` for backwards compat).
///
/// # Arguments
/// * `scanner_code` – value of `RawFinding::matched_code`; used as `matched_code`.
/// * `file_path` – relative source file path; joined with `project_root`.
/// * `line` – 1-based line number (0 means unknown).
/// * `project_root` – working directory of the scan.
pub fn extract_evidence_block(
    scanner_code: Option<&str>,
    file_path: &str,
    line: u32,
    project_root: &std::path::Path,
) -> EvidenceBlock {
    let matched_code = scanner_code.unwrap_or("").to_string();

    // Line-window: read the source file and extract surrounding context.
    if line > 0 && file_path != "<project>" {
        let full_path = project_root.join(file_path);
        if let Ok(source) = std::fs::read_to_string(&full_path) {
            let (source_line, context_before, context_after) =
                extract_context(&source, line, CONTEXT_LINES);

            // Matched code from source if scanner didn't provide it.
            let code = if matched_code.trim().is_empty() {
                source_line
            } else {
                matched_code.clone()
            };

            return EvidenceBlock::from_line_window(code, context_before, context_after);
        }
    }

    // Fallback: just the matched code from the scanner, no context.
    EvidenceBlock::from_line_window(matched_code, vec![], vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn scanner_code_used_as_matched_code() {
        let tmp = write_temp("fn foo() {}\n");
        let root = tmp.path().parent().unwrap();
        let block = extract_evidence_block(
            Some("let x = eval(input)"),
            tmp.path().file_name().unwrap().to_str().unwrap(),
            1,
            root,
        );
        assert_eq!(block.matched_code, "let x = eval(input)");
    }

    #[test]
    fn context_lines_extracted() {
        let src = "line1\nline2\nline3\nline4\nline5\n";
        let tmp = write_temp(src);
        let root = tmp.path().parent().unwrap();
        let filename = tmp
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let block = extract_evidence_block(None, &filename, 3, root);
        // context_before should have lines 1 and 2
        assert!(
            !block.context_before.is_empty(),
            "context_before should be populated"
        );
        // context_after should have lines 4 and 5
        assert!(
            !block.context_after.is_empty(),
            "context_after should be populated"
        );
    }

    #[test]
    fn empty_block_for_unknown_file() {
        let block = extract_evidence_block(None, "does_not_exist.py", 0, std::path::Path::new("."));
        assert_eq!(block.matched_code, "");
        assert!(block.context_before.is_empty());
        assert!(block.context_after.is_empty());
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
        assert!(block.imports.is_empty());
    }

    #[test]
    fn from_line_window_constructor() {
        let block = EvidenceBlock::from_line_window(
            "eval(x)".to_string(),
            vec!["# before".to_string()],
            vec!["# after".to_string()],
        );
        assert_eq!(block.matched_code, "eval(x)");
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
    }

    #[test]
    fn line_to_index_converts_correctly() {
        assert_eq!(line_to_index(1), 0);
        assert_eq!(line_to_index(10), 9);
        assert_eq!(line_to_index(0), 0); // saturating_sub(1) from 0
    }

    #[test]
    fn extract_context_edge_line1() {
        let source = "aaa\nbbb\nccc\n";
        let (matched, before, after) = extract_context(source, 1, 5);
        assert_eq!(matched, "aaa");
        assert_eq!(before.len(), 0, "no lines before line 1");
        assert_eq!(after.len(), 2); // bbb, ccc
    }

    #[test]
    fn extract_context_middle() {
        let source = (1..=20)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (matched, before, after) = extract_context(&source, 10, 5);
        assert_eq!(matched, "line10");
        assert_eq!(before.len(), 5);
        assert_eq!(after.len(), 5);
    }
}
