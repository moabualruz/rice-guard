/// Enroll subcommand handler (stub — Phase 6).
use crate::args::EnrollArgs;
use crate::output;

/// Run the enroll subcommand.
///
/// Phase 6 will implement SonarQube project enrollment. Currently prints
/// a "not yet implemented" warning and exits with code 0.
pub async fn run(_args: EnrollArgs) -> anyhow::Result<i32> {
    output::print_warning("enroll: not yet implemented (Phase 6)");
    Ok(0)
}
