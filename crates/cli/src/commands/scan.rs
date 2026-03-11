/// Scan subcommand handler (stub — Phase 2).
use crate::args::ScanArgs;
use crate::output;

/// Run the scan subcommand.
///
/// Phase 2 will implement full scanner orchestration. Currently prints a
/// "not yet implemented" warning and exits with code 0.
pub async fn run(_args: ScanArgs) -> anyhow::Result<i32> {
    output::print_warning("scan: not yet implemented (Phase 2)");
    Ok(0)
}
