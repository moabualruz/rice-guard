/// Fix report structures.
///
/// `FixReport` is the top-level document written to `fix-report.json`.
/// It is built incrementally — written after each stage completes as a
/// checkpoint — so that an interrupted pipeline still produces a valid
/// partial report.
use std::collections::BTreeMap;

/// Per-tool execution status in a fix stage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixToolStatus {
    /// Fixer ran and at least one file was modified.
    Fixed,
    /// Fixer ran and modified files, but verify step shows the issue persists.
    Attempted,
    /// Tool was skipped (unavailable, unsafe without --unsafe flag, or not in selected stages).
    Skipped,
    /// Fixer process returned a non-zero exit code.
    Failed,
    /// Fixer ran but reported nothing to fix.
    NoIssues,
    /// Dry-run mode: fixer would have applied changes.
    DryRunWouldFix,
}

/// Result for a single fixer tool within a pipeline stage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixToolResult {
    /// Tool name (e.g., "rustfmt", "ruff format").
    pub name: String,
    /// Target language (e.g., "rust", "python").
    pub language: String,
    /// Execution outcome.
    pub status: FixToolStatus,
    /// Files that were modified (populated when status is Fixed or Attempted).
    pub modified_files: Vec<String>,
    /// Wall-clock duration of the tool invocation in milliseconds.
    pub duration_ms: u64,
    /// Error message if the tool failed (status == Failed).
    pub error_msg: Option<String>,
}

/// Aggregated result for a single pipeline stage (format, lint, etc.).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixStageResult {
    /// Stage lifecycle status: "completed" or "skipped".
    pub status: String,
    /// Per-tool results for all tools that were attempted in this stage.
    pub tools: Vec<FixToolResult>,
}

/// Top-level summary counts across all stages.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FixReportSummary {
    /// Number of tools that reported Fixed status.
    pub total_fixed: u32,
    /// Number of tools that reported Attempted status (ran but issue persists).
    pub total_attempted: u32,
    /// Number of tools that were Skipped.
    pub total_skipped: u32,
    /// Number of tools that Failed.
    pub total_failed: u32,
    /// Number of tools that reported NoIssues.
    pub total_no_issues: u32,
}

/// Top-level fix report document.
///
/// Written to `fix-report.json` after each stage completes, allowing
/// interrupted pipelines to produce accurate partial reports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FixReport {
    /// Report schema version. Always "1.0".
    pub schema_version: String,
    /// Project root path as a string.
    pub project: String,
    /// ISO-8601 timestamp when the fix run started.
    pub fixed_at: String,
    /// Whether this was a dry-run (no files modified).
    pub dry_run: bool,
    /// Total wall-clock duration of the entire pipeline in milliseconds.
    pub duration_ms: u64,
    /// Summary counts across all stages.
    pub summary: FixReportSummary,
    /// Per-stage results in pipeline order (BTreeMap preserves alphabetical key order for stable JSON).
    pub stages: BTreeMap<String, FixStageResult>,
}

impl FixReport {
    /// Create a new empty fix report.
    pub fn new(project: String, dry_run: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            schema_version: "1.0".to_string(),
            project,
            fixed_at: now,
            dry_run,
            duration_ms: 0,
            summary: FixReportSummary::default(),
            stages: BTreeMap::new(),
        }
    }

    /// Serialize the report and write it to both the archive and latest directories.
    ///
    /// This is called after each stage completes so the report accurately
    /// reflects progress even if the pipeline is interrupted.
    pub fn write_checkpoint(
        &self,
        output_dir: &std::path::Path,
        latest_dir: &std::path::Path,
    ) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let archive_path = output_dir.join("fix-report.json");
        let latest_path = latest_dir.join("fix-report.json");
        std::fs::write(&archive_path, &json)?;
        std::fs::write(&latest_path, &json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 03"]
    fn checkpoint_writes_after_each_stage() {
        let _report = FixReport::new("test-project".to_string(), false);
        todo!("RED stub — verify write_checkpoint produces valid JSON in both paths");
    }
}
