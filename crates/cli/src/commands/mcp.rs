//! MCP stdio server subcommand handler.
//!
//! Starts the rice-guard MCP server on stdio (JSON-RPC) for Claude Code / Cursor integration.
//! All tracing output goes to stderr — stdout is reserved for the MCP protocol.

use crate::args::McpArgs;

/// Run the `rice-guard mcp` subcommand.
///
/// Determines the working directory from the current process working directory,
/// then delegates to [`rice_guard_server::mcp::run_mcp_server`].
pub async fn run(_args: McpArgs) -> anyhow::Result<i32> {
    let working_dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Cannot determine working directory: {e}"))?;
    rice_guard_server::mcp::run_mcp_server(working_dir).await?;
    Ok(0)
}
