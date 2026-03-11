//! Wave 0 test stubs for SCAN-01 through SCAN-08.
//!
//! Tests that rely on types not yet implemented (parsers, full engine) are
//! marked `#[ignore]` with an explanation.  Tests for structural types
//! (RawScanResult, ScanMode, OutputDir) compile and pass immediately.

use rice_guard_core::scanner::{OutputDir, RawScanResult, ScanMode, ScannerEngine};
use std::path::PathBuf;

// ── Structural / compile-time tests ─────────────────────────────────────────

/// SCAN-06 / SCAN-07 — OutputDir creates the directory on construction.
///
/// Verifies that `OutputDir::new()` returns Ok and that the resulting path
/// exists on disk.
#[test]
fn output_dir_creates_directory_on_disk() {
    let base = tempfile::tempdir().expect("tempdir");
    let dir = OutputDir::new("test-project", base.path().to_str().unwrap())
        .expect("OutputDir::new should succeed");

    let meta = std::fs::metadata(dir.path());
    assert!(
        meta.is_ok(),
        "OutputDir path must exist on disk after construction"
    );
    assert!(meta.unwrap().is_dir(), "OutputDir path must be a directory");
}

/// OutputDir path contains the project name and a timestamp component.
#[test]
fn output_dir_path_contains_project_name() {
    let base = tempfile::tempdir().expect("tempdir");
    let dir = OutputDir::new("my-cool-project", base.path().to_str().unwrap())
        .expect("OutputDir::new should succeed");

    let path_str = dir.path().to_string_lossy();
    assert!(
        path_str.contains("my-cool-project"),
        "path should contain the project name, got: {path_str}"
    );
}

/// SCAN-03 — ScanMode variants: all 4 exist, are Copy (no clone required).
#[test]
fn scan_mode_variants_all_exist_and_are_copy() {
    let full = ScanMode::Full;
    let quick = ScanMode::Quick;
    let security = ScanMode::Security;
    let diff = ScanMode::DiffOnly;

    // Copy: use after "move" is fine
    let _a = full;
    let _b = full;
    let _c = quick;
    let _d = quick;
    let _e = security;
    let _f = security;
    let _g = diff;
    let _h = diff;

    assert_ne!(ScanMode::Full, ScanMode::Quick);
    assert_ne!(ScanMode::Security, ScanMode::DiffOnly);
}

/// ScanMode derives Debug — must be formattable.
#[test]
fn scan_mode_debug_format() {
    assert_eq!(format!("{:?}", ScanMode::Full), "Full");
    assert_eq!(format!("{:?}", ScanMode::Quick), "Quick");
    assert_eq!(format!("{:?}", ScanMode::Security), "Security");
    assert_eq!(format!("{:?}", ScanMode::DiffOnly), "DiffOnly");
}

/// RawScanResult can be constructed and all fields are readable.
#[test]
fn raw_scan_result_construction() {
    let result = RawScanResult {
        scanner: "semgrep".to_string(),
        output_format: "json".to_string(),
        output_file: PathBuf::from("/tmp/semgrep-output.json"),
        exit_code: 0,
    };

    assert_eq!(result.scanner, "semgrep");
    assert_eq!(result.output_format, "json");
    assert_eq!(
        result.output_file,
        PathBuf::from("/tmp/semgrep-output.json")
    );
    assert_eq!(result.exit_code, 0);
}

/// RawScanResult with non-zero exit code (Semgrep exits 1 when findings exist).
#[test]
fn raw_scan_result_nonzero_exit_code() {
    let result = RawScanResult {
        scanner: "semgrep".to_string(),
        output_format: "json".to_string(),
        output_file: PathBuf::from("/tmp/semgrep-output.json"),
        exit_code: 1,
    };
    assert_eq!(result.exit_code, 1);
}

/// RawScanResult can be cloned (derives Clone).
#[test]
fn raw_scan_result_is_clone() {
    let original = RawScanResult {
        scanner: "trivy".to_string(),
        output_format: "sarif".to_string(),
        output_file: PathBuf::from("/tmp/trivy.sarif"),
        exit_code: 0,
    };
    let cloned = original.clone();
    assert_eq!(cloned.scanner, original.scanner);
    assert_eq!(cloned.output_format, original.output_format);
}

/// ScannerEngine can be constructed (skeleton).
#[test]
fn scanner_engine_construction() {
    use rice_guard_core::config::RiceGuardConfig;
    let engine = ScannerEngine::new(vec![], RiceGuardConfig::default());
    // Construction must not panic.
    drop(engine);
}

// ── Fixture file existence tests ─────────────────────────────────────────────

/// SCAN-02 / SCAN-08 — fixture: semgrep_sample.json exists and is valid JSON
/// with the expected schema (results array, errors array).
#[test]
fn fixture_semgrep_sample_json_exists_and_is_valid() {
    let path = std::path::Path::new("tests/fixtures/semgrep_sample.json");
    assert!(path.exists(), "semgrep_sample.json must exist");

    let content = std::fs::read_to_string(path).expect("read semgrep_sample.json");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("semgrep_sample.json must be valid JSON");

    assert!(value["results"].is_array(), "must have 'results' array");
    assert!(value["errors"].is_array(), "must have 'errors' array");
    assert!(
        !value["results"].as_array().unwrap().is_empty(),
        "results must not be empty"
    );
}

/// SCAN-02 — fixture: trivy_sample.sarif exists and contains the CVE finding
/// with an empty locations array (Trivy edge case — no physicalLocation).
#[test]
fn fixture_trivy_sample_sarif_has_empty_locations_edge_case() {
    let path = std::path::Path::new("tests/fixtures/trivy_sample.sarif");
    assert!(path.exists(), "trivy_sample.sarif must exist");

    let content = std::fs::read_to_string(path).expect("read trivy_sample.sarif");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("trivy_sample.sarif must be valid JSON");

    let results = value["runs"][0]["results"]
        .as_array()
        .expect("runs[0].results must be array");
    assert!(!results.is_empty(), "must have at least one result");

    // At least one result must have an empty locations array (edge case).
    let has_empty_locations = results.iter().any(|r| {
        r["locations"]
            .as_array()
            .map(|l| l.is_empty())
            .unwrap_or(false)
    });
    assert!(
        has_empty_locations,
        "trivy fixture must include a CVE result with empty locations (no physicalLocation)"
    );
}

/// SCAN-02 — fixture: gitleaks_sample.sarif exists and the result has NO
/// level field (Gitleaks edge case).
#[test]
fn fixture_gitleaks_sample_sarif_has_no_level_field() {
    let path = std::path::Path::new("tests/fixtures/gitleaks_sample.sarif");
    assert!(path.exists(), "gitleaks_sample.sarif must exist");

    let content = std::fs::read_to_string(path).expect("read gitleaks_sample.sarif");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("gitleaks_sample.sarif must be valid JSON");

    let results = value["runs"][0]["results"]
        .as_array()
        .expect("runs[0].results must be array");
    assert!(!results.is_empty(), "must have at least one result");

    // The level field must be absent (not null, not empty string).
    let result = &results[0];
    assert!(
        result.get("level").is_none(),
        "gitleaks fixture result[0] must NOT have a 'level' field (edge case)"
    );
}

/// SCAN-02 — fixture: jscpd_sample.json exists and has duplicates array.
#[test]
fn fixture_jscpd_sample_json_has_duplicates() {
    let path = std::path::Path::new("tests/fixtures/jscpd_sample.json");
    assert!(path.exists(), "jscpd_sample.json must exist");

    let content = std::fs::read_to_string(path).expect("read jscpd_sample.json");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("jscpd_sample.json must be valid JSON");

    let duplicates = value["duplicates"]
        .as_array()
        .expect("must have 'duplicates' array");
    assert!(!duplicates.is_empty(), "duplicates must not be empty");

    let dup = &duplicates[0];
    assert!(dup["firstFile"].is_object(), "must have 'firstFile'");
    assert!(dup["secondFile"].is_object(), "must have 'secondFile'");
    assert!(dup["fragment"].is_string(), "must have 'fragment'");
}

// ── Ignored stubs: SCAN-01, SCAN-03, SCAN-04, SCAN-05, SCAN-06 (engine) ──────
// These tests are `#[ignore]` because they require the full ScannerEngine
// implementation (Phase 2, Plan 04).

/// SCAN-01 — All enabled scanners run against a target path in Full mode.
///
/// Requires: ScannerEngine::run() implementation (Plan 04).
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_01_all_enabled_scanners_run_in_full_mode() {
    todo!("implement after Plan 04: verify each enabled scanner produces a RawScanResult")
}

/// SCAN-03 — Quick mode only runs jscpd + scc + Semgrep + Trivy.
///
/// Requires: ScannerEngine::run() implementation (Plan 04).
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_03_quick_mode_runs_subset_of_scanners() {
    todo!("implement after Plan 04: verify Quick mode only activates 4 scanners")
}

/// SCAN-04 — Security mode only runs Semgrep + Trivy + Gitleaks.
///
/// Requires: ScannerEngine::run() implementation (Plan 04).
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_04_security_mode_runs_security_scanners() {
    todo!("implement after Plan 04: verify Security mode activates Semgrep, Trivy, Gitleaks")
}

/// SCAN-05 — DiffOnly mode passes diff context to scanners.
///
/// Requires: ScannerEngine::run() implementation (Plan 04).
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_05_diff_only_mode_passes_diff_context() {
    todo!("implement after Plan 04: verify DiffOnly mode only reports issues on changed files")
}

/// SCAN-06 — Scanner output is written to the OutputDir with correct filenames.
///
/// Requires: ScannerEngine::run() implementation (Plan 04).
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_06_scanner_output_written_to_output_dir() {
    todo!("implement after Plan 04: verify output files appear in the OutputDir path")
}

// ── Ignored stubs: SCAN-02, SCAN-08 (parsers) ────────────────────────────────
// These tests are `#[ignore]` because they require the scanner output
// parsers (Plan 02).

/// SCAN-02 — Semgrep JSON output is parsed into normalised Issues.
///
/// Requires: Semgrep parser (Phase 2, Plan 02).
#[test]
#[ignore = "Requires Semgrep parser implementation (Phase 2, Plan 02)"]
fn scan_02_semgrep_json_parsed_into_issues() {
    todo!("implement after Plan 02: parse semgrep_sample.json into Issue structs")
}

/// SCAN-08 — SARIF output (Trivy / Gitleaks) is parsed into normalised Issues,
/// handling edge cases (empty locations, missing level field).
///
/// Requires: SARIF parser (Phase 2, Plan 02).
#[test]
#[ignore = "Requires SARIF parser implementation (Phase 2, Plan 02)"]
fn scan_08_sarif_parsed_with_edge_cases() {
    todo!("implement after Plan 02: parse trivy + gitleaks SARIF with edge cases")
}
