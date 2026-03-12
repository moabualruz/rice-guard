/// Fix subcommand handler — full deterministic fix pipeline (Phase 4).
///
/// Wires the `FixerEngine` into the CLI: config loading, OutputDir setup,
/// unsafe-fixes UX (warning banner + inquire confirmation), SIGINT handler,
/// `--issue`/`--issues` auto-scan-if-missing flow, exit codes, and terminal
/// summary table.
///
/// Exit codes:
/// - `0` — dry-run completed, or no fixes applied and all tools clean.
/// - `1` — fix completed; some issues remain (partially fixed or skipped).
/// - `2` — tool error: config missing/invalid, infra failure, or `--unsafe`
///   without `--yes` in a non-TTY environment.
use std::time::Instant;

use rice_guard_core::fixer::engine::FixerEngineConfig;
use rice_guard_core::fixer::{FixReport, FixerEngine, StageFilter};
use rice_guard_core::registry::DescriptorRegistry;

use crate::args::FixArgs;
use crate::output;
use crate::terminal::{is_tty, print_fix_summary_table, print_unsafe_warning_banner};

/// Run the fix subcommand.
pub async fn run(args: FixArgs) -> anyhow::Result<i32> {
    let fix_start = Instant::now();

    // ── Step 1: resolve project root ─────────────────────────────────────────
    let project_root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    // ── Step 2: load config ───────────────────────────────────────────────────
    let config_path = project_root.join(".riceguard.yaml");
    let config = match rice_guard_core::config::load(&config_path) {
        Ok(c) => c,
        Err(rice_guard_core::errors::ConfigError::NotFound { .. }) => {
            output::print_error(&format!(
                "No .riceguard.yaml found in {}. Run `rice-guard init` first.",
                project_root.display()
            ));
            return Ok(2);
        }
        Err(e) => {
            output::print_error(&format!("Failed to load config: {e}"));
            return Ok(2);
        }
    };

    // ── Step 3: load fixer descriptors ───────────────────────────────────────
    let fixer_descriptors =
        match rice_guard_core::registry::loader::load_fixer_descriptors(&project_root) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    "Failed to load fixer descriptors: {}; continuing with empty list",
                    e
                );
                vec![]
            }
        };
    let registry = DescriptorRegistry::new(fixer_descriptors);

    // ── Step 4: handle --issue / --issues auto-scan-if-missing ───────────────
    let issue_ids: Option<Vec<String>> = if args.issue.is_some() || args.issues.is_some() {
        let latest_issues = project_root
            .join("reports")
            .join("latest")
            .join("issues.json");
        if !latest_issues.exists() {
            tracing::info!("issues.json not found — running scan first");
            output::print_info("No issues.json found — running scan first...");
            let scan_args = crate::args::ScanArgs {
                path: project_root.clone(),
                quick: false,
                security: false,
                diff_only: false,
            };
            let scan_exit = crate::commands::scan::run(scan_args).await?;
            if scan_exit == 2 {
                output::print_error("Auto-scan failed; cannot target specific issues.");
                return Ok(2);
            }
        }

        // Collect issue IDs to target.
        let mut ids: Vec<String> = Vec::new();

        if let Some(single_id) = &args.issue {
            ids.push(single_id.clone());
        }

        if let Some(issues_file) = &args.issues {
            match std::fs::read_to_string(issues_file) {
                Ok(contents) => {
                    match serde_json::from_str::<serde_json::Value>(&contents) {
                        Ok(serde_json::Value::Array(arr)) => {
                            for item in arr {
                                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                                    ids.push(id.to_string());
                                }
                            }
                        }
                        Ok(other) => {
                            // Handle IssueOutput wrapper with schema_version + issues array
                            if let Some(arr) = other.get("issues").and_then(|v| v.as_array()) {
                                for item in arr {
                                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                                        ids.push(id.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            output::print_error(&format!(
                                "Failed to parse issues file {}: {e}",
                                issues_file.display()
                            ));
                            return Ok(2);
                        }
                    }
                }
                Err(e) => {
                    output::print_error(&format!(
                        "Failed to read issues file {}: {e}",
                        issues_file.display()
                    ));
                    return Ok(2);
                }
            }
        }

        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    } else {
        None
    };

    // ── Step 5: handle --unsafe confirmation ─────────────────────────────────
    if args.unsafe_fixes {
        // Collect names of unsafe tools from the registry.
        let unsafe_tool_names: Vec<String> = registry
            .fixers
            .iter()
            .flat_map(|d| {
                d.stages
                    .format
                    .iter()
                    .chain(d.stages.lint.iter())
                    .chain(d.stages.security.iter())
                    .chain(d.stages.ast.iter())
                    .chain(d.stages.deps.iter())
                    .chain(d.stages.import.iter())
                    .filter(|step| !step.safe)
                    .map(|step| step.name.clone())
            })
            .collect();

        print_unsafe_warning_banner(&unsafe_tool_names);

        if !args.yes {
            if !is_tty() {
                output::print_error(
                    "error: --unsafe requires --yes to skip confirmation in non-TTY (CI) mode\n\
                     Use: rice-guard fix --unsafe --yes",
                );
                return Ok(2);
            }
            // Interactive TTY: prompt with inquire.
            let confirmed = inquire::Confirm::new("Apply unsafe fixes? (cannot be undone)")
                .with_default(false)
                .prompt()
                .unwrap_or(false);
            if !confirmed {
                output::print_info("Aborted.");
                return Ok(0);
            }
        }
    }

    // ── Step 6: set up output directories ────────────────────────────────────
    let reports_base = project_root.join("reports").to_string_lossy().to_string();

    let fix_prefix = format!("fix-{}", config.project.name);
    let fix_output_dir = match rice_guard_core::scanner::OutputDir::new(&fix_prefix, &reports_base)
    {
        Ok(d) => d,
        Err(e) => {
            output::print_error(&format!("Failed to create fix output directory: {e}"));
            return Ok(2);
        }
    };

    let latest_dir = project_root.join("reports").join("latest");
    if let Err(e) = std::fs::create_dir_all(&latest_dir) {
        tracing::warn!("Failed to create reports/latest directory: {}", e);
    }

    // ── Step 7: build StageFilter from args ──────────────────────────────────
    let stage_filter = StageFilter::from_args(
        args.formatters,
        args.linters,
        args.security,
        args.ast,
        args.deps,
        args.imports,
    );

    // ── Step 8: build FixerEngineConfig ──────────────────────────────────────
    let engine_config = FixerEngineConfig {
        project_root: project_root.clone(),
        file_targets: args.files.clone(),
        stage_filter,
        dry_run: args.dry_run,
        include_unsafe: args.unsafe_fixes,
        timeout_secs: args.timeout,
        output_dir: fix_output_dir.path().to_path_buf(),
        latest_dir: latest_dir.clone(),
        issue_ids,
    };

    // ── Step 9: run fix engine with SIGINT handler ────────────────────────────
    let engine = FixerEngine::new(config, registry);

    let report: FixReport = tokio::select! {
        r = engine.run(engine_config) => {
            match r {
                Ok(rep) => rep,
                Err(e) => {
                    output::print_error(&format!("Fix pipeline failed: {e}"));
                    return Ok(2);
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::warn!("fix interrupted by user (Ctrl+C) — partial fix-report written");
            output::print_warning("Fix interrupted by user — partial fix-report.json written.");
            return Ok(1);
        }
    };

    // ── Step 10: write final checkpoint to both archive and latest ────────────
    if let Err(e) = report.write_checkpoint(fix_output_dir.path(), &latest_dir) {
        tracing::warn!("Failed to write final fix-report.json: {}", e);
    }

    // ── Step 11: create reports/latest symlink (best-effort) ─────────────────
    if let Err(e) = fix_output_dir.create_latest_symlink(&reports_base) {
        tracing::warn!("Failed to create reports/latest: {}", e);
    }

    // ── Step 12: print summary table ─────────────────────────────────────────
    print_fix_summary_table(&report);

    // ── Step 13: compute and return exit code ────────────────────────────────
    // dry-run: always 0
    if args.dry_run {
        return Ok(0);
    }

    // All tools attempted failed: exit 2
    let total_attempted = report.summary.total_fixed
        + report.summary.total_attempted
        + report.summary.total_failed
        + report.summary.total_no_issues;
    if report.summary.total_failed > 0 && report.summary.total_failed == total_attempted {
        return Ok(2);
    }

    // Some issues remain or partially fixed: exit 1
    if report.summary.total_attempted > 0 {
        return Ok(1);
    }

    let _ = fix_start; // suppress unused warning
    Ok(0)
}
