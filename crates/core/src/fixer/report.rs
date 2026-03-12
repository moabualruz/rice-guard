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

    /// Record a tool result for a given stage, updating summary counters.
    pub fn record_tool(&mut self, stage: &str, tool_result: FixToolResult) {
        let stage_entry = self
            .stages
            .entry(stage.to_string())
            .or_insert_with(|| FixStageResult {
                status: "running".to_string(),
                tools: vec![],
            });
        match &tool_result.status {
            FixToolStatus::Fixed => self.summary.total_fixed += 1,
            FixToolStatus::Attempted => self.summary.total_attempted += 1,
            FixToolStatus::Skipped => self.summary.total_skipped += 1,
            FixToolStatus::Failed => self.summary.total_failed += 1,
            FixToolStatus::NoIssues | FixToolStatus::DryRunWouldFix => {
                self.summary.total_no_issues += 1
            }
        }
        stage_entry.tools.push(tool_result);
    }

    /// Record a skipped stage (stage was filtered out or no tools available).
    pub fn record_skipped(&mut self, stage: &str) {
        self.stages.insert(
            stage.to_string(),
            FixStageResult {
                status: "skipped".to_string(),
                tools: vec![],
            },
        );
    }

    /// Record a failed stage (stage could not run due to an error).
    pub fn record_failed(&mut self, stage: &str) {
        self.stages.insert(
            stage.to_string(),
            FixStageResult {
                status: "failed".to_string(),
                tools: vec![],
            },
        );
    }

    /// Mark a stage as completed.
    pub fn finish_stage(&mut self, stage: &str) {
        if let Some(entry) = self.stages.get_mut(stage) {
            entry.status = "completed".to_string();
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
        use anyhow::Context as _;
        let json = serde_json::to_string_pretty(self)?;
        let archive_path = output_dir.join("fix-report.json");
        let latest_path = latest_dir.join("fix-report.json");
        std::fs::write(&archive_path, &json)
            .with_context(|| format!("failed to write fix-report.json to {archive_path:?}"))?;
        std::fs::write(&latest_path, &json)
            .with_context(|| format!("failed to write fix-report.json to {latest_path:?}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_writes_after_each_stage() -> anyhow::Result<()> {
        use tempfile::tempdir;
        let out = tempdir()?;
        let latest = tempdir()?;
        let report = FixReport::new("test-project".to_string(), false);
        report.write_checkpoint(out.path(), latest.path())?;
        assert!(out.path().join("fix-report.json").exists());
        assert!(latest.path().join("fix-report.json").exists());
        let content = std::fs::read_to_string(out.path().join("fix-report.json"))?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;
        assert_eq!(parsed["schema_version"], "1.0");
        Ok(())
    }

    #[test]
    fn record_tool_updates_summary() {
        let mut report = FixReport::new("test-project".to_string(), false);

        let fixed = FixToolResult {
            name: "rustfmt".to_string(),
            language: "rust".to_string(),
            status: FixToolStatus::Fixed,
            modified_files: vec!["src/main.rs".to_string()],
            duration_ms: 10,
            error_msg: None,
        };
        let attempted = FixToolResult {
            name: "clippy".to_string(),
            language: "rust".to_string(),
            status: FixToolStatus::Attempted,
            modified_files: vec![],
            duration_ms: 20,
            error_msg: None,
        };

        report.record_tool("format", fixed);
        report.record_tool("lint", attempted);

        assert_eq!(report.summary.total_fixed, 1);
        assert_eq!(report.summary.total_attempted, 1);
        assert_eq!(report.summary.total_skipped, 0);
    }
}
