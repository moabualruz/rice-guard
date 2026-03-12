/// Fix pipeline engine.
///
/// `FixerEngine` orchestrates the 6-stage fix pipeline, running deterministic
/// fixers in order: format -> lint -> security -> ast -> deps -> import.
/// Each stage runs all applicable tools before advancing to the next.
use crate::fixer::report::FixReport;

/// Orchestrates the full deterministic fix pipeline.
///
/// Fields are populated during construction and read during `run()`.
/// All fields are `#[allow(dead_code)]` because the implementation is
/// in a subsequent plan — the skeleton must compile cleanly.
#[allow(dead_code)]
pub struct FixerEngine {
    /// Project root directory.
    project_dir: std::path::PathBuf,
    /// Whether to run in dry-run mode (no files modified).
    dry_run: bool,
    /// Whether unsafe fixers are allowed.
    unsafe_fixes: bool,
    /// Global pipeline timeout in seconds.
    timeout_secs: u64,
}

impl FixerEngine {
    /// Run the full fix pipeline and return the completed report.
    pub async fn run(&self) -> anyhow::Result<FixReport> {
        todo!("FixerEngine::run() — implementation in Plan 04")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 02"]
    fn pipeline_stages_in_order() {
        use crate::fixer::stage_filter::PIPELINE_STAGES;
        assert_eq!(PIPELINE_STAGES[0], "format");
        assert_eq!(PIPELINE_STAGES[1], "lint");
        assert_eq!(PIPELINE_STAGES[2], "security");
        assert_eq!(PIPELINE_STAGES[3], "ast");
        assert_eq!(PIPELINE_STAGES[4], "deps");
        assert_eq!(PIPELINE_STAGES[5], "import");
        assert_eq!(PIPELINE_STAGES.len(), 6);
        todo!("RED stub — verify PIPELINE_STAGES constant ordering");
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
