pub mod error;
pub mod handlers;
pub mod request;
pub mod state;

use std::path::PathBuf;

pub fn build_router(_state: state::AppState) -> axum::Router {
    todo!()
}

pub async fn start_server(_port: u16, _working_dir: PathBuf) -> anyhow::Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore]
    async fn health_returns_200() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn scan_handler_returns_summary() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn issues_handler_returns_vec() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn issue_by_id_returns_single_or_404() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_handler_returns_fix_report() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn status_handler_returns_summary() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_dry_run_does_not_modify_files() {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn fix_category_filter_runs_only_that_stage() {
        todo!()
    }
}
