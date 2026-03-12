/// Fix pipeline module.
///
/// Provides the full deterministic fix pipeline: stage filtering, tool execution,
/// check-fix-verify cycles, and structured reporting.
pub mod engine;
pub mod report;
pub mod runner;
pub mod stage_filter;

pub use engine::FixerEngine;
pub use report::{FixReport, FixToolResult, FixToolStatus};
pub use stage_filter::{StageFilter, PIPELINE_STAGES};
