/// Fix pipeline engine.
///
/// `FixerEngine` orchestrates the 6-stage fix pipeline, running deterministic
/// fixers in order: format -> lint -> security -> ast -> deps -> import.
/// Each stage runs all applicable tools before advancing to the next.
use crate::config::RiceGuardConfig;
use crate::fixer::report::FixReport;
use crate::registry::DescriptorRegistry;

/// Orchestrates the full deterministic fix pipeline.
///
/// Fields are populated during construction and read during `run()`.
/// `config` and `registry` are `#[allow(dead_code)]` because `run()` is
/// implemented in Plan 03 — the skeleton must compile cleanly.
pub struct FixerEngine {
    #[allow(dead_code)]
    config: RiceGuardConfig,
    #[allow(dead_code)]
    registry: DescriptorRegistry,
}

impl FixerEngine {
    /// Construct a new engine from the project config and descriptor registry.
    pub fn new(config: RiceGuardConfig, registry: DescriptorRegistry) -> Self {
        Self { config, registry }
    }

    /// Run the full fix pipeline and return the completed report.
    pub async fn run(&self) -> anyhow::Result<FixReport> {
        todo!("FixerEngine::run() — implementation in Plan 03")
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 04"]
    fn issue_targeting_routes_correctly() {
        todo!("RED stub — verify --issue routes to correct fixer stage and tool");
    }

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 04"]
    fn issues_file_reads_and_routes() {
        todo!("RED stub — verify --issues file is read and issues routed to correct fixers");
    }

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 04"]
    fn unsafe_tools_skipped_without_flag() {
        todo!("RED stub — verify unsafe fixer tools are skipped when unsafe_fixes=false");
    }
}
