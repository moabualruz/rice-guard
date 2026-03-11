/// Status subcommand handler (stub — Phase 2).
use crate::args::StatusArgs;
use crate::output;

/// Run the status subcommand.
///
/// Phase 2 will implement issue count + trend reporting. Currently prints
/// a "not yet implemented" warning and exits with code 0.
pub async fn run(_args: StatusArgs) -> anyhow::Result<i32> {
    output::print_warning("status: not yet implemented (Phase 2)");
    Ok(0)
}
