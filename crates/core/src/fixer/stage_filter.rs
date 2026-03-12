//! Stage filter for the fix pipeline.
//!
//! Maps CLI flags to the ordered list of pipeline stages that should run.
//! Stages are always executed in PIPELINE_STAGES order, never reordered.

/// All pipeline stages in execution order.
///
/// Internal names match fixer descriptor stage keys (singular form).
pub const PIPELINE_STAGES: &[&str] = &["format", "lint", "security", "ast", "deps", "import"];

/// Controls which pipeline stages are active for a fix run.
///
/// When no category flags are set, all stages run. When one or more flags
/// are set, only the selected stages run (combined with logical OR).
#[derive(Debug, Clone)]
pub struct StageFilter {
    selected: Option<Vec<&'static str>>,
}

impl StageFilter {
    /// Build a [`StageFilter`] from individual CLI flag booleans.
    ///
    /// If all flags are `false`, `selected` is `None` meaning all stages run.
    /// Otherwise only stages with `true` flags are included.
    pub fn from_args(
        formatters: bool,
        linters: bool,
        security: bool,
        ast: bool,
        deps: bool,
        imports: bool,
    ) -> Self {
        let any = formatters || linters || security || ast || deps || imports;
        if !any {
            return Self { selected: None };
        }
        let mut stages: Vec<&'static str> = Vec::new();
        if formatters {
            stages.push("format");
        }
        if linters {
            stages.push("lint");
        }
        if security {
            stages.push("security");
        }
        if ast {
            stages.push("ast");
        }
        if deps {
            stages.push("deps");
        }
        if imports {
            stages.push("import");
        }
        Self {
            selected: Some(stages),
        }
    }

    /// Returns `true` if the given stage name should run.
    pub fn includes(&self, stage: &str) -> bool {
        match &self.selected {
            None => true,
            Some(stages) => stages.contains(&stage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_stages_in_order() {
        assert_eq!(PIPELINE_STAGES[0], "format");
        assert_eq!(PIPELINE_STAGES[1], "lint");
        assert_eq!(PIPELINE_STAGES[2], "security");
        assert_eq!(PIPELINE_STAGES[3], "ast");
        assert_eq!(PIPELINE_STAGES[4], "deps");
        assert_eq!(PIPELINE_STAGES[5], "import");
        assert_eq!(PIPELINE_STAGES.len(), 6);
    }

    #[test]
    fn no_flags_runs_all() {
        let filter = StageFilter::from_args(false, false, false, false, false, false);
        assert!(filter.includes("format"), "format should run with no flags");
        assert!(
            filter.includes("security"),
            "security should run with no flags"
        );
        assert!(filter.includes("import"), "import should run with no flags");
    }

    #[test]
    fn formatters_flag_selects_format() {
        let filter = StageFilter::from_args(true, false, false, false, false, false);
        assert!(filter.includes("format"), "format stage should be included");
        assert!(
            !filter.includes("lint"),
            "lint stage should NOT be included"
        );
    }

    #[test]
    fn combined_flags_run_both() {
        let filter = StageFilter::from_args(true, true, false, false, false, false);
        assert!(filter.includes("format"), "format should be included");
        assert!(filter.includes("lint"), "lint should be included");
        assert!(
            !filter.includes("security"),
            "security should NOT be included"
        );
    }
}
