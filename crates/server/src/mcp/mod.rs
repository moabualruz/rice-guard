pub mod server;
pub mod tools;

use std::path::PathBuf;

pub async fn run_mcp_server(_working_dir: PathBuf) -> anyhow::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore]
    async fn mcp_server_init_no_panic() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn mcp_scan_tool_returns_json() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn mcp_get_issues_returns_array() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn mcp_no_stdout_on_startup() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn mcp_stderr_only_for_tracing() {
        todo!()
    }
}
