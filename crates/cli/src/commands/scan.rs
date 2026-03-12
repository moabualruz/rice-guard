/// Scan subcommand handler — full implementation (Phase 2, Plan 04-06 / Phase 3, Plan 04).
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::args::ScanArgs;
use crate::output;
use crate::terminal::{
    print_scan_summary_table, ScanProgressReporter, ScannerResult, ScannerStatus,
};

/// Run the scan subcommand.
///
/// Exit codes:
/// - `0` — scan completed, no findings.
/// - `1` — scan completed, findings found.
/// - `2` — tool error (config missing/invalid, infrastructure failure) OR all scanners failed.
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

    // ── Step 5: determine scan mode (composable flags) ────────────────────────
    // --quick and --security select the scanner subset.
    // --diff-only is orthogonal: it filters findings to git-changed files only.
    // Flags stack: --quick --diff-only runs quick scanners AND applies diff filter.
    let scanner_subset = if args.quick {
        rice_guard_core::scanner::ScanMode::Quick
    } else if args.security {
        rice_guard_core::scanner::ScanMode::Security
    } else {
        rice_guard_core::scanner::ScanMode::Full
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
    let engine = rice_guard_core::scanner::ScannerEngine::new(descriptors, config);
    let mut findings = match engine.run(&target, scanner_subset, &output_dir).await {
        Ok(f) => f,
        Err(e) => {
            reporter.clear();
            output::print_error(&format!("Scan failed: {e}"));
            return Ok(2);
        }
    };

    // ── Step 9: apply diff-only filter (orthogonal to scanner subset) ─────────
    if apply_diff_filter {
        match rice_guard_core::scanner::diff_only_filter(&target).await {
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
    // If all attempted scanners produced zero results, it likely indicates
    // that no scanner could run (all unavailable or all failed).
    // This is exit code 2 (tool error), not exit code 1 (findings).
    let attempted_count = scanner_names.len();
    let producing_scanners: HashSet<&str> = findings.iter().map(|f| f.scanner.as_str()).collect();
    let all_failed = attempted_count > 0 && producing_scanners.is_empty();

    if all_failed {
        output::print_warning("All scanners failed or were unavailable.");
    }

    // ── Step 11: Phase 3 pipeline — enrich findings into Issues ──────────────
    use rice_guard_core::issue::{
        file_freq_map, sort_issues, EvidenceBlock, EvidenceExtractor, IssueBuilder,
    };
    use rice_guard_core::output::{OutputWriter, ScanSummary};

    let freq_map: HashMap<String, u32> = file_freq_map(&findings);
    let extractor = EvidenceExtractor::new();

    // Group findings by file_path (one parse per file, EVID-04).
    let mut by_file: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, finding) in findings.iter().enumerate() {
        by_file
            .entry(finding.file_path.clone())
            .or_default()
            .push(idx);
    }

    // Pre-read source files.
    let mut file_sources: HashMap<String, Option<String>> = HashMap::new();
    for file_path in by_file.keys() {
        let norm = file_path.replace('/', std::path::MAIN_SEPARATOR_STR);
        let abs_path = target.join(&norm);
        let source = std::fs::read_to_string(&abs_path).ok();
        file_sources.insert(file_path.clone(), source);
    }

    // Build Issues — one per finding, with evidence extracted per-file.
    let mut issues = Vec::with_capacity(findings.len());

    for (file_path, indices) in &by_file {
        let file_findings: Vec<&rice_guard_core::scanner::RawFinding> =
            indices.iter().map(|&i| &findings[i]).collect();

        // Extract all evidence blocks for this file in one parse pass.
        let evidence_map = if let Some(Some(source)) = file_sources.get(file_path) {
            extractor.extract_file(file_path, source, &file_findings)
        } else {
            HashMap::new()
        };

        for finding in &file_findings {
            // Look up pre-extracted evidence; fall back to empty block.
            let evidence = evidence_map
                .get(&(finding.rule_id.clone(), finding.line))
                .cloned()
                .unwrap_or_else(|| EvidenceBlock {
                    matched_code: finding.matched_code.clone().unwrap_or_default(),
                    context_before: vec![],
                    context_after: vec![],
                    enclosing_function: None,
                    enclosing_class: None,
                    imports: vec![],
                });

            let file_freq = freq_map.get(&finding.file_path).copied().unwrap_or(1);
            let issue = IssueBuilder::build_with_evidence(finding, evidence, file_freq);
            issues.push(issue);
        }
    }

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
            let status = if producing_scanners.contains(name.as_str()) || scanner_issues.is_empty()
            {
                ScannerStatus::Success
            } else {
                ScannerStatus::Failed
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
                error_msg: None,
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
