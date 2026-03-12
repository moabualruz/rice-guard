/// rice-guard CLI binary entry point.
///
/// Parses command-line arguments, initializes tracing, routes to subcommand
/// handlers, and translates `anyhow::Result<i32>` into process exit codes.
///
/// Exit codes:
///   0 — success / clean
///   1 — issues found (used by scan in Phase 2+)
///   2 — tool error (config parse, missing required tool, descriptor invalid)
mod args;
mod commands;
mod output;
mod terminal;

use args::{Cli, Commands};
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

    let result = match cli.command {
        Commands::Init(args) => commands::init::run(args).await,
        Commands::Scan(args) => commands::scan::run(args).await,
        Commands::Fix(args) => commands::fix::run(args).await,
        Commands::Status(args) => commands::status::run(args).await,
        Commands::Serve(args) => commands::serve::run(args).await,
        Commands::Enroll(args) => commands::enroll::run(args).await,
        Commands::Report(args) => commands::report::run(args).await,
        Commands::Version(args) => {
            // version::run is sync — wrap in Ok to match async signature
            std::future::ready(commands::version::run(args)).await
        }
    };

    match result {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            output::print_error(&format!("Error: {e}"));
            std::process::exit(2);
        }
    }
}

/// Initialize the tracing subscriber.
///
/// If `verbose` is true, sets `RUST_LOG=debug`. Otherwise uses the existing
/// `RUST_LOG` value, falling back to `info`.
fn init_tracing(verbose: bool) {
    if verbose {
        unsafe {
            std::env::set_var("RUST_LOG", "debug");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}
