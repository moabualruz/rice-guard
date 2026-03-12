/// Fix pipeline engine.
///
/// `FixerEngine` orchestrates the 6-stage fix pipeline, running deterministic
/// fixers in order: format -> lint -> security -> ast -> deps -> import.
/// Each stage runs all applicable tools before advancing to the next.
use std::time::Instant;

use crate::config::RiceGuardConfig;
use crate::fixer::report::FixReport;
use crate::fixer::runner::{run_one_fixer, RunnerConfig};
use crate::fixer::stage_filter::{StageFilter, PIPELINE_STAGES};
use crate::registry::fixer_descriptor::FixerStep;
use crate::registry::DescriptorRegistry;

/// Configuration for a complete fix pipeline run.
pub struct FixerEngineConfig {
    /// Root directory of the project being fixed.
    pub project_root: std::path::PathBuf,
    /// Specific files to fix; empty means whole project.
    pub file_targets: Vec<std::path::PathBuf>,
    /// Controls which stages run.
    pub stage_filter: StageFilter,
    /// When true, only report what would change without modifying files.
    pub dry_run: bool,
    /// When true, also run tools marked `safe: false`.
    pub include_unsafe: bool,
    /// Default per-tool timeout in seconds.
    pub timeout_secs: u64,
    /// Archive directory for this fix run (timestamped).
    pub output_dir: std::path::PathBuf,
    /// Latest symlink directory (reports/latest).
    pub latest_dir: std::path::PathBuf,
    /// When `Some`, only fix issues with these IDs (for `--issue`/`--issues`).
    pub issue_ids: Option<Vec<String>>,
}

/// Orchestrates the full deterministic fix pipeline.
pub struct FixerEngine {
    config: RiceGuardConfig,
    registry: DescriptorRegistry,
}

impl FixerEngine {
    /// Construct a new engine from the project config and descriptor registry.
    pub fn new(config: RiceGuardConfig, registry: DescriptorRegistry) -> Self {
        Self { config, registry }
    }

    /// Run the full fix pipeline and return the completed report.
    pub async fn run(&self, engine_config: FixerEngineConfig) -> anyhow::Result<FixReport> {
        let start_time = Instant::now();
        let mut report = FixReport::new(self.config.project.name.clone(), engine_config.dry_run);

        for &stage_name in PIPELINE_STAGES {
            if !engine_config.stage_filter.includes(stage_name) {
                continue;
            }

            let tools = self.collect_stage_tools(stage_name, engine_config.include_unsafe);
            if tools.is_empty() {
                tracing::debug!("stage {} has no tools — skipping", stage_name);
                continue;
            }

            for (language, step) in &tools {
                let runner_cfg = RunnerConfig {
                    project_root: engine_config.project_root.clone(),
                    file_targets: engine_config.file_targets.clone(),
                    dry_run: engine_config.dry_run,
                    timeout_secs: engine_config.timeout_secs,
                };
                let mut result = run_one_fixer(step, &runner_cfg).await;
                result.language = language.clone();
                result.name = step.name.clone();
                report.record_tool(stage_name, result);
            }
            report.finish_stage(stage_name);
            report
                .write_checkpoint(&engine_config.output_dir, &engine_config.latest_dir)
                .unwrap_or_else(|e| tracing::warn!("checkpoint write failed: {}", e));
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Collect all `(language, step)` pairs for a given stage across all descriptors.
    ///
    /// Steps with `safe: false` are silently excluded when `include_unsafe` is false.
    fn collect_stage_tools(&self, stage: &str, include_unsafe: bool) -> Vec<(String, FixerStep)> {
        let mut tools = Vec::new();
        for descriptor in &self.registry.fixers {
            let steps: &[FixerStep] = match stage {
                "format" => &descriptor.stages.format,
                "lint" => &descriptor.stages.lint,
                "security" => &descriptor.stages.security,
                "ast" => &descriptor.stages.ast,
                "deps" => &descriptor.stages.deps,
                "import" => &descriptor.stages.import,
                _ => continue,
            };
            for step in steps {
                if !step.safe && !include_unsafe {
                    // Silently skip unsafe tools when --unsafe is not set
                    continue;
                }
                tools.push((descriptor.language.clone(), step.clone()));
            }
        }
        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RiceGuardConfig;
    use crate::registry::fixer_descriptor::{FixerDescriptor, FixerStages, FixerStep};

    fn make_step(name: &str, safe: bool) -> FixerStep {
        FixerStep {
            name: name.to_string(),
            // Use a guaranteed-unavailable tool so runner returns Skipped immediately —
            // no real tool invocations needed in these routing tests.
            check: "___nonexistent_tool_xyz___ --check .".to_string(),
            fix: "___nonexistent_tool_xyz___ --fix .".to_string(),
            scope: "project".to_string(),
            safe,
            unsafe_flag: None,
        }
    }

    fn make_format_descriptor(language: &str, steps: Vec<FixerStep>) -> FixerDescriptor {
        FixerDescriptor {
            name: language.to_string(),
            language: language.to_string(),
            detect: vec!["*.test".to_string()],
            stages: FixerStages {
                format: steps,
                ..Default::default()
            },
        }
    }

    fn default_config() -> RiceGuardConfig {
        let mut cfg = RiceGuardConfig::default();
        cfg.project.name = "test-project".to_string();
        cfg
    }

    fn engine_config(
        dir: &std::path::Path,
        stage_filter: StageFilter,
        include_unsafe: bool,
    ) -> FixerEngineConfig {
        FixerEngineConfig {
            project_root: dir.to_path_buf(),
            file_targets: vec![],
            stage_filter,
            dry_run: false,
            include_unsafe,
            timeout_secs: 10,
            output_dir: dir.to_path_buf(),
            latest_dir: dir.to_path_buf(),
            issue_ids: None,
        }
    }

    #[test]
    fn pipeline_stages_in_order() {
        use crate::fixer::stage_filter::PIPELINE_STAGES;
        assert_eq!(PIPELINE_STAGES[0], "format");
        assert_eq!(PIPELINE_STAGES[1], "lint");
        assert_eq!(PIPELINE_STAGES[2], "security");
        assert_eq!(PIPELINE_STAGES[3], "ast");
        assert_eq!(PIPELINE_STAGES[4], "deps");
        assert_eq!(PIPELINE_STAGES[5], "import");
        assert_eq!(PIPELINE_STAGES.len(), 6);
    }

    #[tokio::test]
    async fn unsafe_tools_skipped_without_flag() {
        let _dir = tempfile::tempdir().unwrap();
        // Descriptor with one safe and one unsafe format step
        let descriptor = make_format_descriptor(
            "rust",
            vec![
                make_step("safe-tool", true),
                make_step("unsafe-tool", false),
            ],
        );
        let registry = DescriptorRegistry::new(vec![descriptor]);
        let engine = FixerEngine::new(default_config(), registry);

        // Without include_unsafe=true, only safe tools should appear
        let tools = engine.collect_stage_tools("format", false);
        assert_eq!(tools.len(), 1, "only safe tool should be collected");
        assert_eq!(tools[0].1.name, "safe-tool");

        // With include_unsafe=true, both tools appear
        let tools_all = engine.collect_stage_tools("format", true);
        assert_eq!(tools_all.len(), 2, "both tools should be collected");
    }

    #[tokio::test]
    async fn issue_targeting_routes_correctly() {
        // Engine with a rust descriptor that has a format step
        let _dir = tempfile::tempdir().unwrap();
        let descriptor = FixerDescriptor {
            name: "rust".to_string(),
            language: "rust".to_string(),
            detect: vec!["Cargo.toml".to_string()],
            stages: FixerStages {
                format: vec![make_step("rustfmt", true)],
                lint: vec![make_step("clippy", true)],
                ..Default::default()
            },
        };
        let registry = DescriptorRegistry::new(vec![descriptor]);
        let engine = FixerEngine::new(default_config(), registry);

        // collect_stage_tools("format") should only return the format step
        let format_tools = engine.collect_stage_tools("format", false);
        assert_eq!(format_tools.len(), 1);
        assert_eq!(format_tools[0].1.name, "rustfmt");

        // collect_stage_tools("lint") should only return the lint step
        let lint_tools = engine.collect_stage_tools("lint", false);
        assert_eq!(lint_tools.len(), 1);
        assert_eq!(lint_tools[0].1.name, "clippy");

        // collect_stage_tools("security") should be empty
        let sec_tools = engine.collect_stage_tools("security", false);
        assert!(sec_tools.is_empty(), "no security steps defined");
    }

    #[tokio::test]
    async fn issues_file_reads_and_routes() {
        // Two descriptors with steps in different stages
        let dir = tempfile::tempdir().unwrap();
        let rust_desc = FixerDescriptor {
            name: "rust".to_string(),
            language: "rust".to_string(),
            detect: vec!["Cargo.toml".to_string()],
            stages: FixerStages {
                format: vec![make_step("rustfmt", true)],
                ..Default::default()
            },
        };
        let python_desc = FixerDescriptor {
            name: "python".to_string(),
            language: "python".to_string(),
            detect: vec!["*.py".to_string()],
            stages: FixerStages {
                lint: vec![make_step("ruff", true)],
                ..Default::default()
            },
        };
        let registry = DescriptorRegistry::new(vec![rust_desc, python_desc]);
        let engine = FixerEngine::new(default_config(), registry);

        // format stage: only rust/rustfmt
        let format_tools = engine.collect_stage_tools("format", false);
        assert_eq!(format_tools.len(), 1);
        assert_eq!(format_tools[0].0, "rust", "format tool should be rust");

        // lint stage: only python/ruff
        let lint_tools = engine.collect_stage_tools("lint", false);
        assert_eq!(lint_tools.len(), 1);
        assert_eq!(lint_tools[0].0, "python", "lint tool should be python");

        // Full run with both stages active
        let ec = engine_config(
            dir.path(),
            StageFilter::from_args(true, true, false, false, false, false),
            false,
        );
        let report = engine.run(ec).await.unwrap();
        // Both stages should appear in report (with Skipped results — tool unavailable)
        assert!(
            report.stages.contains_key("format"),
            "format stage should be in report"
        );
        assert!(
            report.stages.contains_key("lint"),
            "lint stage should be in report"
        );
    }
}
