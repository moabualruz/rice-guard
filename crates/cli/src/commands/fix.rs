/// Fix subcommand handler (stub — Phase 4).
use crate::args::FixArgs;
use crate::output;

/// Run the fix subcommand.
///
/// Phase 4 will implement the full deterministic fix pipeline. Currently
/// prints a "not yet implemented" warning and exits with code 0.
pub async fn run(_args: FixArgs) -> anyhow::Result<i32> {
    output::print_warning("fix: not yet implemented (Phase 4)");
    Ok(0)
}
