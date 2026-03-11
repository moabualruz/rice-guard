/// Serve subcommand handler (stub — Phase 5).
use crate::args::ServeArgs;
use crate::output;

/// Run the serve subcommand.
///
/// Phase 5 will implement the REST API + MCP server. Currently prints
/// a "not yet implemented" warning and exits with code 0.
pub async fn run(_args: ServeArgs) -> anyhow::Result<i32> {
    output::print_warning("serve: not yet implemented (Phase 5)");
    Ok(0)
}
