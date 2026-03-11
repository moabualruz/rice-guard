/// Version subcommand handler.
///
/// Prints version information and optionally generates shell completions.
use crate::args::{Cli, VersionArgs};
use clap::CommandFactory;
use clap_complete::generate;

/// Run the version subcommand.
///
/// If `--completions <SHELL>` is provided, writes a shell completion script
/// to stdout and exits 0. Otherwise, prints the version string.
pub fn run(args: VersionArgs) -> anyhow::Result<i32> {
    if let Some(shell) = args.completions {
        let mut app = Cli::command();
        let name = app.get_name().to_string();
        generate(shell, &mut app, name, &mut std::io::stdout());
        return Ok(0);
    }

    println!("rice-guard {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Build: {}",
        option_env!("RICE_GUARD_BUILD_INFO").unwrap_or("dev"),
    );
    Ok(0)
}
