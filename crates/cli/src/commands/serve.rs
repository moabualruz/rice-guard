/// Serve subcommand handler — REST API server (Phase 5).
use crate::args::ServeArgs;

/// Run the serve subcommand.
///
/// Starts the REST API server on `127.0.0.1:{port}` with the current
/// working directory as the default project root for all requests.
pub async fn run(args: ServeArgs) -> anyhow::Result<i32> {
    let working_dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Cannot determine working directory: {e}"))?;
    rguard_server::rest::start_server(args.port, working_dir).await?;
    Ok(0)
}
