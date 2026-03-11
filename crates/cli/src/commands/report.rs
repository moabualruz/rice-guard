/// Report subcommand handler (stub — Phase 6).
use crate::args::ReportArgs;
use crate::output;

/// Run the report subcommand.
///
/// Phase 6 will implement SonarQube report retrieval. Currently prints
/// a "not yet implemented" warning and exits with code 0.
pub async fn run(_args: ReportArgs) -> anyhow::Result<i32> {
    output::print_warning("report: not yet implemented (Phase 6)");
    Ok(0)
}
