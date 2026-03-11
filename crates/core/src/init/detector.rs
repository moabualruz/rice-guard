use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::errors::InitError;
use crate::registry::loader::{load_fixer_descriptors, load_scanner_descriptors};
use crate::registry::probe::{probe_fixer_step, probe_scanner, ProbeResult};

/// A single language entry as reported by `scc --format json`.
///
/// scc uses PascalCase JSON keys.
#[derive(Debug, Clone, Deserialize)]
pub struct SccLanguage {
    /// Language name, e.g. `"Rust"` or `"Go"`.
    #[serde(rename = "Name")]
    pub name: String,

    /// Number of source files in this language.
    #[serde(rename = "Files")]
    pub files: u64,

    /// Total lines (code + comment + blank).
    #[serde(rename = "Lines")]
    pub lines: u64,

    /// Lines of actual code (excludes comments and blanks).
    #[serde(rename = "Code")]
    pub code: u64,
}

/// The result of running the full `init` detection pipeline.
///
/// Contains detected languages plus availability probes for every
/// configured scanner and fixer tool.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Languages with at least one line of code, detected by `scc`.
    pub languages: Vec<SccLanguage>,

    /// Scanner tool availability keyed by scanner name.
    ///
    /// Example: `{ "semgrep" => Available { version: "1.90.0" }, "trivy" => Missing, ... }`
    pub scanner_probes: HashMap<String, ProbeResult>,

    /// Fixer tool availability keyed by language then step name.
    ///
    /// Example: `{ "rust" => { "rustfmt" => Available { version: "" }, ... }, ... }`
    pub fixer_probes: HashMap<String, HashMap<String, ProbeResult>>,

    /// Absolute path of the project that was probed.
    pub project_path: PathBuf,
}

/// Run the full init detection pipeline:
///
/// 1. Check that `scc` is in PATH — returns [`InitError::SccNotFound`] if not.
/// 2. Run `scc --format json <project_path>` and parse the language list.
/// 3. Probe every built-in scanner descriptor for availability.
/// 4. For each detected language, probe every fixer step in its descriptor.
///
/// A progress spinner is shown on stderr only when stderr is a real TTY
/// (guards against CI/pipe noise).
pub async fn detect(project_path: &Path) -> Result<DetectionResult, InitError> {
    // ── 1. Require scc ───────────────────────────────────────────────────────
    if which::which("scc").is_err() {
        return Err(InitError::SccNotFound);
    }

    // ── 2. Progress bar (TTY-only) ───────────────────────────────────────────
    let pb = if atty::is(atty::Stream::Stderr) {
        let bar = indicatif::ProgressBar::new_spinner();
        bar.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
        );
        bar.set_message("Detecting languages...");
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(bar)
    } else {
        None
    };

    // ── 3. Run scc ───────────────────────────────────────────────────────────
    let output = tokio::process::Command::new("scc")
        .arg("--format")
        .arg("json")
        .arg(project_path)
        .output()
        .await
        .map_err(|e| InitError::SccFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if let Some(ref pb) = pb {
            pb.finish_and_clear();
        }
        return Err(InitError::SccFailed(stderr));
    }

    let all_languages: Vec<SccLanguage> =
        serde_json::from_slice(&output.stdout).map_err(|e| InitError::SccFailed(e.to_string()))?;

    // Filter out languages with zero code lines.
    let languages: Vec<SccLanguage> = all_languages.into_iter().filter(|l| l.code > 0).collect();

    // ── 4. Probe scanners ────────────────────────────────────────────────────
    if let Some(ref pb) = pb {
        pb.set_message("Probing scanner tools...");
    }

    let scanner_descriptors = load_scanner_descriptors(project_path)?;
    let mut scanner_probes: HashMap<String, ProbeResult> =
        HashMap::with_capacity(scanner_descriptors.len());

    for desc in &scanner_descriptors {
        let result = probe_scanner(desc).await;
        scanner_probes.insert(desc.name.clone(), result);
    }

    // ── 5. Probe fixers for detected languages ───────────────────────────────
    if let Some(ref pb) = pb {
        pb.set_message("Probing fixer tools...");
    }

    let fixer_descriptors = load_fixer_descriptors(project_path)?;
    let detected_names: std::collections::HashSet<String> =
        languages.iter().map(|l| l.name.to_lowercase()).collect();

    let mut fixer_probes: HashMap<String, HashMap<String, ProbeResult>> = HashMap::new();

    for desc in &fixer_descriptors {
        if !detected_names.contains(&desc.language.to_lowercase()) {
            continue;
        }
        let mut step_probes: HashMap<String, ProbeResult> = HashMap::new();
        let all_steps = desc
            .stages
            .format
            .iter()
            .chain(desc.stages.lint.iter())
            .chain(desc.stages.security.iter())
            .chain(desc.stages.ast.iter())
            .chain(desc.stages.deps.iter())
            .chain(desc.stages.import.iter());

        for step in all_steps {
            let result = probe_fixer_step(step).await;
            step_probes.insert(step.name.clone(), result);
        }
        fixer_probes.insert(desc.language.clone(), step_probes);
    }

    if let Some(ref pb) = pb {
        pb.finish_and_clear();
    }

    Ok(DetectionResult {
        languages,
        scanner_probes,
        fixer_probes,
        project_path: project_path.to_path_buf(),
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// When `scc` is not in PATH, `detect()` must return `InitError::SccNotFound`.
    ///
    /// We simulate a missing scc by invoking `detect()` with a modified PATH
    /// that contains no known scc binary.  On most CI environments scc is not
    /// installed, so this test covers the real path too.  We use the env-var
    /// approach: set PATH to a temporary empty dir so scc lookup fails.
    #[test]
    fn scc_not_found() {
        // Override PATH to an empty temp directory so `which("scc")` fails.
        let tmp = tempfile::TempDir::new().unwrap();
        let old_path = std::env::var("PATH").unwrap_or_default();
        // Scope the test: set PATH only for this thread during the sync check.
        // `detect` is async, but the `which::which` check happens synchronously
        // at the start.  We test the synchronous guard via a blocking runtime.
        std::env::set_var("PATH", tmp.path());
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(detect(std::path::Path::new(".")));
        std::env::set_var("PATH", &old_path); // restore immediately
        assert!(
            matches!(result, Err(InitError::SccNotFound)),
            "expected SccNotFound when scc is absent from PATH, got: {result:?}"
        );
    }

    /// Parse a hard-coded scc JSON fixture into `Vec<SccLanguage>`.
    ///
    /// Verifies field mapping (PascalCase rename), filtering (code == 0 excluded),
    /// and that the struct populates correctly.
    #[test]
    fn parse_scc_json() {
        // Minimal scc JSON output with two entries: one real language, one empty.
        let json = r#"[
            {
                "Name": "Rust",
                "Lines": 500,
                "Code": 400,
                "Comment": 50,
                "Blank": 50,
                "Complexity": 10,
                "Count": 12,
                "Files": 12,
                "WeightedComplexity": 10.0,
                "Bytes": 8192
            },
            {
                "Name": "Markdown",
                "Lines": 100,
                "Code": 0,
                "Comment": 0,
                "Blank": 100,
                "Complexity": 0,
                "Count": 3,
                "Files": 3,
                "WeightedComplexity": 0.0,
                "Bytes": 1024
            }
        ]"#;

        let all: Vec<SccLanguage> = serde_json::from_str(json).expect("scc JSON must parse");
        assert_eq!(all.len(), 2, "raw parse should include both entries");

        // Apply the same filter as `detect()`.
        let filtered: Vec<SccLanguage> = all.into_iter().filter(|l| l.code > 0).collect();
        assert_eq!(
            filtered.len(),
            1,
            "Markdown with code==0 must be filtered out"
        );
        assert_eq!(filtered[0].name, "Rust");
        assert_eq!(filtered[0].files, 12);
        assert_eq!(filtered[0].lines, 500);
        assert_eq!(filtered[0].code, 400);
    }
}
