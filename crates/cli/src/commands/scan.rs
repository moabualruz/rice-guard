/// Scan subcommand handler — full implementation (Phase 2, Plan 04 / Phase 3, Plan 04).
use std::collections::HashMap;
use std::time::Instant;

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

    let scan_start = Instant::now();

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

    pb.set_message("Building AI-ready output...");

    // ── Step 8: Phase 3 pipeline — enrich findings into Issues ───────────────
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

    // ── Step 9: write output files ────────────────────────────────────────────
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
        output::print_error(&format!("Failed to write output files: {e}"));
        return Ok(2);
    }

    pb.finish_and_clear();

    // ── Step 10: print summary ────────────────────────────────────────────────
    let count = issues.len();
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
        "Wrote {} issue(s) to {}",
        count_str,
        output_dir.path().display()
    ));

    // ── Step 11: return exit code ─────────────────────────────────────────────
    if issues.is_empty() {
        Ok(0)
    } else {
        Ok(1)
    }
}
