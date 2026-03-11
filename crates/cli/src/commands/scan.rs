/// Scan subcommand handler — full implementation (Phase 2, Plan 04).
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use owo_colors::Stream;

use crate::args::ScanArgs;
use crate::output;

/// Run the scan subcommand.
///
/// Exit codes:
/// - `0` — scan completed, no findings.
/// - `1` — scan completed, findings found.
/// - `2` — tool error (config missing/invalid, infrastructure failure).
pub async fn run(args: ScanArgs) -> anyhow::Result<i32> {
    // ── Step 1: resolve target path ───────────────────────────────────────────
    let target = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    // ── Step 2: load config ───────────────────────────────────────────────────
    let config_path = target.join(".riceguard.yaml");
    let config = match rice_guard_core::config::load(&config_path) {
        Ok(c) => c,
        Err(rice_guard_core::errors::ConfigError::NotFound { .. }) => {
            output::print_error(&format!(
                "No .riceguard.yaml found in {}. Run `rice-guard init` first.",
                target.display()
            ));
            return Ok(2);
        }
        Err(e) => {
            output::print_error(&format!("Failed to load config: {e}"));
            return Ok(2);
        }
    };

    // ── Step 3: load scanner descriptors ─────────────────────────────────────
    let descriptors = match rice_guard_core::registry::loader::load_scanner_descriptors(&target) {
        Ok(d) => d,
        Err(e) => {
            output::print_error(&format!("Failed to load scanner descriptors: {e}"));
            return Ok(2);
        }
    };

    // ── Step 4: create output directory ──────────────────────────────────────
    let reports_base = target.join("reports").to_string_lossy().to_string();

    let output_dir =
        match rice_guard_core::scanner::OutputDir::new(&config.project.name, &reports_base) {
            Ok(d) => d,
            Err(e) => {
                output::print_error(&format!("Failed to create output directory: {e}"));
                return Ok(2);
            }
        };

    // ── Step 5: determine scan mode ───────────────────────────────────────────
    let mode = if args.quick {
        rice_guard_core::scanner::ScanMode::Quick
    } else if args.security {
        rice_guard_core::scanner::ScanMode::Security
    } else if args.diff_only {
        rice_guard_core::scanner::ScanMode::DiffOnly
    } else {
        rice_guard_core::scanner::ScanMode::Full
    };

    let enabled_count = config.tools.scanners.values().filter(|&&v| v).count();

    // ── Step 6: progress indicator ────────────────────────────────────────────
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message(format!("Scanning with {} scanner(s)...", enabled_count));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // ── Step 7: run scanner engine ────────────────────────────────────────────
    let engine = rice_guard_core::scanner::ScannerEngine::new(descriptors, config);
    let findings = match engine.run(&target, mode, &output_dir).await {
        Ok(f) => f,
        Err(e) => {
            pb.finish_and_clear();
            output::print_error(&format!("Scan failed: {e}"));
            return Ok(2);
        }
    };

    pb.finish_and_clear();

    // ── Step 8: print summary ─────────────────────────────────────────────────
    let count = findings.len();
    let count_str = if output::supports_color(Stream::Stdout) {
        if count > 0 {
            format!("{}", count.if_supports_color(Stream::Stdout, |t| t.red()))
        } else {
            format!("{}", count.if_supports_color(Stream::Stdout, |t| t.green()))
        }
    } else {
        count.to_string()
    };

    output::print_info(&format!(
        "Found {} issue(s). Output: {}",
        count_str,
        output_dir.path().display()
    ));

    // ── Step 9: return exit code ──────────────────────────────────────────────
    if findings.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}
