/// Scan subcommand handler — full implementation (Phase 2, Plan 04-06 / Phase 3, Plan 04).
use std::time::Instant;

use crate::args::ScanArgs;
use crate::output;
use crate::terminal::{
    print_scan_summary_table, ScanProgressReporter, ScannerResult, ScannerStatus,
};

/// Resolve the effective `respect_gitignore` value from CLI flags and config.
///
/// CLI flags override the config value:
/// - `--respect-gitignore` forces `true`
/// - `--no-gitignore` forces `false`
/// - Neither flag → use `config.filters.respect_gitignore`
fn resolve_respect_gitignore(
    cli_respect: bool,
    cli_no_gitignore: bool,
    config_value: bool,
) -> bool {
    if cli_respect {
        true
    } else if cli_no_gitignore {
        false
    } else {
        config_value
    }
}

/// Run the scan subcommand.
///
/// Exit codes:
/// - `0` — scan completed, no findings.
/// - `1` — scan completed, findings found.
/// - `2` — tool error (config missing/invalid, infrastructure failure) OR all scanners failed.
pub async fn run(args: ScanArgs) -> anyhow::Result<i32> {
    // ── Step 1: resolve target path ───────────────────────────────────────────
    let target = crate::paths::safe_canonicalize(&args.path);

    // ── Step 2: load config ───────────────────────────────────────────────────
    let config_path = target.join(".rguard.yaml");
    let config = match rguard_core::config::load(&config_path) {
        Ok(c) => c,
        Err(rguard_core::errors::ConfigError::NotFound { .. }) => {
            output::print_error(&format!(
                "No .rguard.yaml found in {}. Run `rguard init` first.",
                target.display()
            ));
            return Ok(2);
        }
        Err(e) => {
            output::print_error(&format!("Failed to load config: {e}"));
            return Ok(2);
        }
    };

    // ── Step 3: resolve respect_gitignore and run --debug-ignores if requested ─
    let respect_gitignore = resolve_respect_gitignore(
        args.respect_gitignore,
        args.no_gitignore,
        config.filters.respect_gitignore,
    );

    if args.debug_ignores {
        use rguard_core::ignore::IgnoreEngine;
        use walkdir::WalkDir;

        match IgnoreEngine::build(&target, &config, respect_gitignore) {
            Ok(engine) => {
                let walker = WalkDir::new(&target).into_iter();
                for entry in walker.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let is_dir = entry.file_type().is_dir();
                    let rel = path
                        .strip_prefix(&target)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    if rel.is_empty() {
                        continue;
                    }
                    if engine.is_ignored(path, is_dir) {
                        eprintln!("[IGNORED] {rel}");
                    } else {
                        eprintln!("[INCLUDED] {rel}");
                    }
                }
            }
            Err(e) => {
                output::print_warning(&format!(
                    "--debug-ignores: failed to build ignore engine: {e}"
                ));
            }
        }
    }

    // ── Step 4: load scanner descriptors ─────────────────────────────────────
    let descriptors = match rguard_core::registry::loader::load_scanner_descriptors(&target) {
        Ok(d) => d,
        Err(e) => {
            output::print_error(&format!("Failed to load scanner descriptors: {e}"));
            return Ok(2);
        }
    };

    // ── Step 4: create output directory ──────────────────────────────────────
    let reports_base = target.join("reports").to_string_lossy().to_string();

    let output_dir = match rguard_core::scanner::OutputDir::new(&config.project.name, &reports_base)
    {
        Ok(d) => d,
        Err(e) => {
            output::print_error(&format!("Failed to create output directory: {e}"));
            return Ok(2);
        }
    };

    // ── Step 5: determine scan mode (composable flags) ────────────────────────
    // --quick and --security select the scanner subset.
    // --diff-only is orthogonal: it filters findings to git-changed files only.
    // Flags stack: --quick --diff-only runs quick scanners AND applies diff filter.
    let scanner_subset = if args.quick {
        rguard_core::scanner::ScanMode::Quick
    } else if args.security {
        rguard_core::scanner::ScanMode::Security
    } else {
        rguard_core::scanner::ScanMode::Full
    };

    let apply_diff_filter = args.diff_only;

    let mode_label = if args.quick {
        "Quick scan"
    } else if args.security {
        "Security scan"
    } else {
        "Full scan"
    };

    // ── Step 6: collect scanner names for progress reporter ───────────────────
    // Names of all enabled scanners for the given mode subset.
    let scanner_names: Vec<String> = descriptors
        .iter()
        .filter(|d| config.tools.scanners.get(&d.name).copied().unwrap_or(false))
        .map(|d| d.name.clone())
        .collect();

    // ── Step 7: progress indicator ────────────────────────────────────────────
    let reporter = ScanProgressReporter::new(&scanner_names);
    for name in &scanner_names {
        reporter.set_running(name);
    }

    let scan_start = Instant::now();

    // ── Step 8: run scanner engine ────────────────────────────────────────────
    let engine = rguard_core::scanner::ScannerEngine::new(descriptors, config);
    let scan_report = match engine.run(&target, scanner_subset, &output_dir).await {
        Ok(r) => r,
        Err(e) => {
            reporter.clear();
            output::print_error(&format!("Scan failed: {e}"));
            return Ok(2);
        }
    };

    let mut findings = scan_report.findings;
    let succeeded_scanners = scan_report.succeeded;
    let failed_scanners = scan_report.failed;

    // ── Step 9: apply diff-only filter (orthogonal to scanner subset) ─────────
    if apply_diff_filter {
        match rguard_core::scanner::diff_only_filter(&target).await {
            Ok(changed_files) if !changed_files.is_empty() => {
                findings.retain(|f| {
                    changed_files.iter().any(|cf| {
                        let changed_str = cf.to_string_lossy().replace('\\', "/");
                        let finding_path = f.file_path.replace('\\', "/");
                        changed_str.ends_with(&finding_path)
                            || finding_path.ends_with(
                                cf.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default()
                                    .as_str(),
                            )
                    })
                });
            }
            Ok(_) => {
                tracing::warn!("diff-only: no changed files found; returning all findings");
            }
            Err(e) => {
                tracing::warn!("diff-only filter failed ({}); returning all findings", e);
            }
        }
    }

    // ── Step 10: detect ALL-scanner-failure ───────────────────────────────────
    let attempted_count = scanner_names.len();
    let all_failed = attempted_count > 0 && succeeded_scanners.is_empty();

    if all_failed {
        output::print_warning("All scanners failed or were unavailable.");
    }

    // ── Step 11: Phase 3 pipeline — enrich findings into Issues ──────────────
    use rguard_core::issue::{sort_issues, FixerDescriptorInfo, IssueBuilder};
    use rguard_core::output::{OutputWriter, ScanSummary};

    // Load fixer descriptors and convert to FixerDescriptorInfo for build_batch().
    // On failure, fall back to an empty list — scan still works, just without
    // descriptor-driven auto_fixable detection.
    let fixer_descriptor_infos: Vec<FixerDescriptorInfo> =
        match rguard_core::registry::loader::load_fixer_descriptors(&target) {
            Ok(fds) => fds
                .into_iter()
                .map(|fd| {
                    // Collect present stage names in pipeline order.
                    let mut stages: Vec<String> = Vec::new();
                    if !fd.stages.format.is_empty() {
                        stages.push("formatters".to_string());
                    }
                    if !fd.stages.lint.is_empty() {
                        stages.push("linters".to_string());
                    }
                    if !fd.stages.security.is_empty() {
                        stages.push("security".to_string());
                    }
                    if !fd.stages.ast.is_empty() {
                        stages.push("ast".to_string());
                    }
                    if !fd.stages.deps.is_empty() {
                        stages.push("deps".to_string());
                    }
                    if !fd.stages.import.is_empty() {
                        stages.push("import".to_string());
                    }

                    // First fix command from any stage, used as the reported fix_tool.
                    let fix_tool = fd
                        .stages
                        .format
                        .first()
                        .or_else(|| fd.stages.lint.first())
                        .or_else(|| fd.stages.security.first())
                        .or_else(|| fd.stages.ast.first())
                        .or_else(|| fd.stages.deps.first())
                        .or_else(|| fd.stages.import.first())
                        .map(|step| step.fix.clone());

                    FixerDescriptorInfo {
                        language: fd.language.clone(),
                        stages,
                        fix_tool,
                    }
                })
                .collect(),
            Err(e) => {
                tracing::warn!("Failed to load fixer descriptors: {}; using empty list", e);
                vec![]
            }
        };

    // Build all issues in one descriptor-driven batch call.
    // build_batch() internally:
    //   - Uses file_freq_with_churn() for scoring (git churn + issue count)
    //   - Detects cross_file when rule_id appears in 3+ distinct files
    //   - Uses descriptor-driven FixMetadata via fixer_descriptor_infos
    let mut issues = IssueBuilder::build_batch(&findings, &target, &fixer_descriptor_infos);

    sort_issues(&mut issues);

    let scan_duration_ms = scan_start.elapsed().as_millis() as u64;

    // ── Step 12: write output files ───────────────────────────────────────────
    let scanners_run: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        findings
            .iter()
            .filter(|f| seen.insert(f.scanner.clone()))
            .map(|f| f.scanner.clone())
            .collect()
    };

    let summary = ScanSummary::from_issues(
        &issues,
        scanners_run,
        &target.to_string_lossy(),
        scan_duration_ms,
    );

    let writer = OutputWriter::new(output_dir.path());
    if let Err(e) = writer.write_all(&issues, &summary) {
        reporter.clear();
        output::print_error(&format!("Failed to write output files: {e}"));
        return Ok(2);
    }

    // ── Step 13: create reports/latest symlink ────────────────────────────────
    // Best-effort — failure is warned but never aborts the scan.
    if let Err(e) = output_dir.create_latest_symlink(&reports_base) {
        tracing::warn!("Failed to create reports/latest: {}", e);
    }

    // ── Step 14: build per-scanner results for summary table ──────────────────
    let scanner_results: Vec<ScannerResult> = scanner_names
        .iter()
        .map(|name| {
            let scanner_issues: Vec<_> = findings.iter().filter(|f| &f.scanner == name).collect();
            let high = scanner_issues
                .iter()
                .filter(|f| f.severity == "error")
                .count();
            let medium = scanner_issues
                .iter()
                .filter(|f| f.severity == "warning")
                .count();
            let low = scanner_issues
                .iter()
                .filter(|f| f.severity == "info")
                .count();
            // info bucket: anything not classified above
            let info = scanner_issues.len().saturating_sub(high + medium + low);

            // Determine status from engine outcomes, not from finding counts.
            let (status, error_msg) = if succeeded_scanners.contains(name) {
                (ScannerStatus::Success, None)
            } else if let Some((_, reason)) = failed_scanners.iter().find(|(n, _)| n == name) {
                (ScannerStatus::Failed, Some(reason.clone()))
            } else {
                // Unavailable (not enabled, not installed)
                (ScannerStatus::Failed, Some("unavailable".to_string()))
            };

            ScannerResult {
                name: name.clone(),
                status,
                findings: scanner_issues.len(),
                high,
                medium,
                low,
                info,
                duration_ms: 0, // engine does not yet return per-scanner timing
                error_msg,
            }
        })
        .collect();

    // ── Step 15: clear bars and print summary table ───────────────────────────
    reporter.clear();
    print_scan_summary_table(
        mode_label,
        &scanner_results,
        &output_dir.path().to_string_lossy(),
    );

    // ── Step 16: return exit code ─────────────────────────────────────────────
    // 0 = clean (no findings), 1 = findings, 2 = all scanners failed / infra error
    if all_failed {
        Ok(2)
    } else if issues.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}
