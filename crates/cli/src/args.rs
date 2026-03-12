/// Command-line argument definitions using clap derive macros.
///
/// Defines the full CLI surface for `rice-guard`: all 8 subcommands
/// with their flags and arguments.
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

/// rice-guard — cross-platform code quality, security scanning, and
/// deterministic auto-fixing.
#[derive(Debug, Parser)]
#[command(
    name = "rice-guard",
    version,
    about = "Cross-platform code quality, security scanning, and deterministic auto-fixing",
    long_about = "rice-guard scans your project for code quality, security, and \
                  duplication issues, then applies deterministic fixes — zero AI, \
                  zero tokens, zero API keys required.\n\n\
                  Exit codes: 0 = clean/success, 1 = issues found, 2 = tool error",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging (sets RUST_LOG=debug).
    #[arg(long, global = true)]
    pub verbose: bool,
}

/// All available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(InitArgs),
    Scan(ScanArgs),
    Fix(FixArgs),
    Status(StatusArgs),
    Serve(ServeArgs),
    Enroll(EnrollArgs),
    Report(ReportArgs),
    Version(VersionArgs),
}

/// Detect project languages, probe available tools, and generate `.riceguard.yaml`.
#[derive(Debug, clap::Args)]
#[command(
    about = "Detect project languages, probe available tools, and generate .riceguard.yaml",
    long_about = "Runs the 3-step init pipeline:\n  \
                  1. scc language detection\n  \
                  2. Scanner tool probing\n  \
                  3. Fixer tool probing\n\n\
                  Then runs an interactive wizard (or uses defaults with --yes) to\n\
                  generate .riceguard.yaml and optional CI/pre-commit configs."
)]
pub struct InitArgs {
    /// Path to the project to initialize (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Non-interactive mode: auto-select all defaults without prompts.
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Preview files that would be generated without writing them.
    #[arg(long)]
    pub dry_run: bool,
}

/// Run all enabled scanners and produce AI-ready reports.
#[derive(Debug, clap::Args)]
#[command(about = "Run all enabled scanners and produce AI-ready reports")]
pub struct ScanArgs {
    /// Path to the project to scan (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Quick scan: jscpd + scc + Semgrep + Trivy only.
    #[arg(long)]
    pub quick: bool,

    /// Security-focused scan: Semgrep + Trivy + Gitleaks only.
    #[arg(long)]
    pub security: bool,

    /// Hold-the-line mode: report new issues only (compared to baseline).
    #[arg(long)]
    pub diff_only: bool,
}

/// Run deterministic fixers: formatters, linters, security patches, dep updates.
#[derive(Debug, clap::Args)]
#[command(
    about = "Run deterministic fixers: formatters, linters, security patches, dep updates",
    long_about = "Applies only deterministic fixes — zero AI, zero tokens.\n\
                  Fix categories can be combined; omitting all flags runs everything."
)]
pub struct FixArgs {
    /// Path to the project to fix (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Run formatters only (gofumpt, rustfmt, ruff format, biome, etc.).
    #[arg(long)]
    pub formatters: bool,

    /// Run linter auto-fix only (golangci-lint --fix, cargo clippy --fix, etc.).
    #[arg(long)]
    pub linters: bool,

    /// Run Semgrep --autofix rules only.
    #[arg(long)]
    pub security: bool,

    /// Run ast-grep fix rules only.
    #[arg(long)]
    pub ast: bool,

    /// Run dependency updates only (npm audit fix, cargo update, etc.).
    #[arg(long)]
    pub deps: bool,

    /// Preview fixes without applying them.
    #[arg(long)]
    pub dry_run: bool,

    /// Include unsafe linter fixes (e.g., cargo clippy --fix --allow-dirty).
    #[arg(long, name = "unsafe")]
    pub unsafe_fixes: bool,

    /// Fix a specific issue by its ID.
    #[arg(long, value_name = "ID")]
    pub issue: Option<String>,

    /// Fix issues from a JSON file (output of `scan`).
    #[arg(long, value_name = "FILE")]
    pub issues: Option<PathBuf>,

    /// Run import cleanup fixers (ruff F401/I, goimports, dart fix, etc.).
    #[arg(long)]
    pub imports: bool,

    /// Include file diffs in fix-report.json (off by default; expensive for large projects).
    #[arg(long)]
    pub diff: bool,

    /// Re-run check commands after all stages and write issues-remaining.json.
    #[arg(long)]
    pub rescan: bool,

    /// Global pipeline timeout in seconds (default: 600).
    #[arg(long, default_value = "600")]
    pub timeout: u64,

    /// Skip the --unsafe confirmation prompt (for CI pipelines; use with --unsafe).
    #[arg(long)]
    pub yes: bool,

    /// Additional file targets (when given, only these files are fixed instead of entire project).
    #[arg(trailing_var_arg = true)]
    pub files: Vec<std::path::PathBuf>,
}

/// Show current issue counts and trends for the project.
#[derive(Debug, clap::Args)]
#[command(about = "Show current issue counts and trends for the project")]
pub struct StatusArgs {
    /// Path to the project (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Start the rice-guard REST API server.
#[derive(Debug, clap::Args)]
#[command(about = "Start the rice-guard REST API server")]
pub struct ServeArgs {
    /// Port to listen on.
    #[arg(long, default_value = "8080")]
    pub port: u16,
}

/// Add a project to SonarQube (optional integration).
#[derive(Debug, clap::Args)]
#[command(about = "Add a project to SonarQube (optional integration)")]
pub struct EnrollArgs {
    /// Path to the project to enroll (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Pull SonarQube report for the project.
#[derive(Debug, clap::Args)]
#[command(about = "Pull SonarQube report for the project")]
pub struct ReportArgs {
    /// Path to the project (defaults to current directory).
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Print version information and optionally generate shell completions.
#[derive(Debug, clap::Args)]
#[command(about = "Print version information and optionally generate shell completions")]
pub struct VersionArgs {
    /// Generate shell completion script for the given shell.
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
}
