//! Wave 0 + Wave 1 tests for SCAN-01 through SCAN-08.
//!
//! Tests that rely on types not yet implemented (full engine runner) are
//! marked `#[ignore]` with an explanation. Tests for structural types
//! (RawScanResult, ScanMode, OutputDir) and parsers (Plan 02) pass immediately.

use rice_guard_core::scanner::{
    parser::{parse_scanner_output, ParseError},
    OutputDir, RawScanResult, ScanMode, ScannerEngine,
};
use std::path::PathBuf;

// -- Structural / compile-time tests -----------------------------------------

/// SCAN-06 / SCAN-07 -- OutputDir creates the directory on construction.
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

/// SCAN-03 -- ScanMode variants: all 4 exist, are Copy (no clone required).
#[test]
fn scan_mode_variants_all_exist_and_are_copy() {
    let full = ScanMode::Full;
    let quick = ScanMode::Quick;
    let security = ScanMode::Security;
    let diff = ScanMode::DiffOnly;

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

/// ScanMode derives Debug.
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

/// RawScanResult with non-zero exit code.
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

/// RawScanResult can be cloned.
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
    drop(engine);
}

// -- Fixture file existence tests --------------------------------------------

/// SCAN-02 / SCAN-08 -- fixture: semgrep_sample.json exists and is valid JSON.
#[test]
fn fixture_semgrep_sample_json_exists_and_is_valid() {
    let path = std::path::Path::new("tests/fixtures/semgrep_sample.json");
    assert!(path.exists(), "semgrep_sample.json must exist");

    let content = std::fs::read_to_string(path).expect("read semgrep_sample.json");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("semgrep_sample.json must be valid JSON");

    assert!(value["results"].is_array(), "must have results array");
    assert!(value["errors"].is_array(), "must have errors array");
    assert!(
        !value["results"].as_array().unwrap().is_empty(),
        "results must not be empty"
    );
}

/// SCAN-02 -- fixture: trivy_sample.sarif has empty locations edge case.
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

/// SCAN-02 -- fixture: gitleaks_sample.sarif has no level field.
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

    let result = &results[0];
    assert!(
        result.get("level").is_none(),
        "gitleaks fixture result[0] must NOT have a level field (edge case)"
    );
}

/// SCAN-02 -- fixture: jscpd_sample.json has duplicates array.
#[test]
fn fixture_jscpd_sample_json_has_duplicates() {
    let path = std::path::Path::new("tests/fixtures/jscpd_sample.json");
    assert!(path.exists(), "jscpd_sample.json must exist");

    let content = std::fs::read_to_string(path).expect("read jscpd_sample.json");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("jscpd_sample.json must be valid JSON");

    let duplicates = value["duplicates"]
        .as_array()
        .expect("must have duplicates array");
    assert!(!duplicates.is_empty(), "duplicates must not be empty");

    let dup = &duplicates[0];
    assert!(dup["firstFile"].is_object(), "must have firstFile");
    assert!(dup["secondFile"].is_object(), "must have secondFile");
    assert!(dup["fragment"].is_string(), "must have fragment");
}

// -- Ignored stubs: SCAN-01, SCAN-03, SCAN-04, SCAN-05, SCAN-06 (engine) -----

/// SCAN-01 -- All enabled scanners run against a target path in Full mode.
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_01_all_enabled_scanners_run_in_full_mode() {
    todo!("implement after Plan 04: verify each enabled scanner produces a RawScanResult")
}

/// SCAN-03 -- Quick mode only runs jscpd + scc + Semgrep + Trivy.
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_03_quick_mode_runs_subset_of_scanners() {
    todo!("implement after Plan 04: verify Quick mode only activates 4 scanners")
}

/// SCAN-04 -- Security mode only runs Semgrep + Trivy + Gitleaks.
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_04_security_mode_runs_security_scanners() {
    todo!("implement after Plan 04: verify Security mode activates Semgrep, Trivy, Gitleaks")
}

/// SCAN-05 -- DiffOnly mode passes diff context to scanners.
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_05_diff_only_mode_passes_diff_context() {
    todo!("implement after Plan 04: verify DiffOnly mode only reports issues on changed files")
}

/// SCAN-06 -- Scanner output is written to the OutputDir with correct filenames.
#[test]
#[ignore = "Requires ScannerEngine::run() implementation (Phase 2, Plan 04)"]
fn scan_06_scanner_output_written_to_output_dir() {
    todo!("implement after Plan 04: verify output files appear in the OutputDir path")
}

// -- SCAN-02 / SCAN-08: Parser tests (Plan 02) -------------------------------

/// SCAN-08 -- Trivy SARIF: CVE with empty locations gets file_path="<project>", line=0.
#[test]
fn sarif_trivy_cve_empty_locations_yields_project_sentinel() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("trivy.sarif");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/trivy_sample.sarif"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "trivy".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert!(
        !findings.is_empty(),
        "trivy SARIF must produce at least one finding"
    );

    let cve_finding = findings
        .iter()
        .find(|f| f.rule_id == "CVE-2023-45803")
        .expect("CVE-2023-45803 finding must be present");

    assert_eq!(
        cve_finding.file_path, "<project>",
        "Trivy CVE with empty locations must use <project> sentinel"
    );
    assert_eq!(
        cve_finding.line, 0,
        "Trivy CVE with empty locations must have line=0"
    );
}

/// SCAN-08 -- Trivy SARIF: finding with physicalLocation gets correct file_path.
#[test]
fn sarif_trivy_finding_with_location_gets_file_path() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("trivy.sarif");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/trivy_sample.sarif"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "trivy".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");

    let located = findings
        .iter()
        .find(|f| f.rule_id == "CVE-2024-22195")
        .expect("CVE-2024-22195 must be present");

    assert_eq!(
        located.file_path, "go.sum",
        "finding with physicalLocation must have correct file_path"
    );
}

/// SCAN-08 -- Gitleaks SARIF: absent level field -> severity = "high".
#[test]
fn sarif_gitleaks_no_level_defaults_to_high() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("gitleaks.sarif");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/gitleaks_sample.sarif"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "gitleaks".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert!(
        !findings.is_empty(),
        "gitleaks SARIF must produce at least one finding"
    );

    let finding = &findings[0];
    assert_eq!(
        finding.severity, "high",
        "Gitleaks finding with absent level must default to high"
    );
    assert_eq!(
        finding.file_path, "config/settings.py",
        "Gitleaks finding file_path must be correct"
    );
}

/// SCAN-08 -- SARIF: file:/// URI prefix is stripped from file_path.
#[test]
fn sarif_file_uri_prefix_stripped() {
    let base = tempfile::tempdir().expect("tempdir");
    let sarif_json = serde_json::json!({
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "test", "rules": [] } },
            "results": [{
                "ruleId": "test-rule",
                "level": "error",
                "message": { "text": "test" },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": "file:///workspace/src/main.py" },
                        "region": { "startLine": 10 }
                    }
                }]
            }]
        }]
    });
    let fixture_path = base.path().join("test.sarif");
    std::fs::write(&fixture_path, sarif_json.to_string()).expect("write fixture");

    let result = RawScanResult {
        scanner: "test".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].file_path, "workspace/src/main.py",
        "file:/// prefix must be stripped from SARIF uri"
    );
}

/// SCAN-08 -- SARIF: severity normalization (error/warning/note/None).
#[test]
fn sarif_severity_normalization() {
    let base = tempfile::tempdir().expect("tempdir");
    let sarif_json = serde_json::json!({
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "test", "rules": [] } },
            "results": [
                { "ruleId": "rule-error", "level": "error",   "message": { "text": "e" }, "locations": [] },
                { "ruleId": "rule-note",  "level": "note",    "message": { "text": "n" }, "locations": [] },
                { "ruleId": "rule-nolvl",                     "message": { "text": "x" }, "locations": [] },
            ]
        }]
    });
    let fixture_path = base.path().join("test.sarif");
    std::fs::write(&fixture_path, sarif_json.to_string()).expect("write fixture");

    let result = RawScanResult {
        scanner: "test".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert_eq!(findings.len(), 3);

    let error_f = findings.iter().find(|f| f.rule_id == "rule-error").unwrap();
    assert_eq!(error_f.severity, "error");

    let note_f = findings.iter().find(|f| f.rule_id == "rule-note").unwrap();
    assert_eq!(note_f.severity, "info");

    let no_level_f = findings.iter().find(|f| f.rule_id == "rule-nolvl").unwrap();
    assert_eq!(no_level_f.severity, "warning");
}

/// SCAN-08 -- unknown output format returns ParseError::InvalidFormat.
#[test]
fn parse_scanner_output_unknown_format_returns_error() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("output.xyz");
    std::fs::write(&fixture_path, "{}").expect("write fixture");

    let result = RawScanResult {
        scanner: "unknown".to_string(),
        output_format: "xyz".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let err = parse_scanner_output(&result).expect_err("unknown format must error");
    assert!(
        matches!(err, ParseError::InvalidFormat(_)),
        "expected InvalidFormat, got: {err:?}"
    );
}

/// SCAN-02 -- Semgrep JSON parsed into two findings.
#[test]
fn scan_02_semgrep_json_parsed_into_issues() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("semgrep.json");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/semgrep_sample.json"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "semgrep".to_string(),
        output_format: "json".to_string(),
        output_file: fixture_path,
        exit_code: 1,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert_eq!(
        findings.len(),
        2,
        "semgrep_sample.json has 2 findings; got: {findings:?}"
    );

    let sql = findings
        .iter()
        .find(|f| f.rule_id == "python.security.sql-injection")
        .expect("SQL injection finding must be present");
    assert_eq!(sql.severity, "error");
    assert_eq!(sql.file_path, "src/db.py");
    assert_eq!(sql.line, 42);

    let fstr = findings
        .iter()
        .find(|f| f.rule_id == "python.best-practice.use-fstring")
        .expect("use-fstring finding must be present");
    assert_eq!(fstr.severity, "warning");
    assert_eq!(fstr.file_path, "src/utils.py");
    assert_eq!(fstr.line, 15);
}

/// SCAN-02 -- Semgrep JSON with missing results key returns empty Vec (not error).
#[test]
fn semgrep_missing_results_key_returns_empty() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("semgrep_empty.json");
    let empty_json = serde_json::json!({ "errors": [] });
    std::fs::write(&fixture_path, empty_json.to_string()).expect("write fixture");

    let result = RawScanResult {
        scanner: "semgrep".to_string(),
        output_format: "json".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("missing results must not error");
    assert!(
        findings.is_empty(),
        "missing results key must yield empty vec"
    );
}

/// SCAN-02 -- jscpd JSON parsed into one duplication finding with severity=warning.
#[test]
fn jscpd_json_parsed_into_duplication_finding() {
    let base = tempfile::tempdir().expect("tempdir");
    let subdir = base.path().join("jscpd_output");
    std::fs::create_dir_all(&subdir).expect("create subdir");
    let fixture_path = subdir.join("jscpd-report.json");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/jscpd_sample.json"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "jscpd".to_string(),
        output_format: "jscpd-json".to_string(),
        // intentionally wrong filename -- parser must glob the parent dir
        output_file: subdir.join("nonexistent-output.json"),
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    assert_eq!(
        findings.len(),
        1,
        "jscpd_sample.json has 1 duplicate; got: {findings:?}"
    );

    let dup = &findings[0];
    assert_eq!(dup.rule_id, "jscpd.duplication");
    assert_eq!(dup.severity, "warning");
    assert_eq!(dup.file_path, "src/a.ts");
    assert!(
        dup.message.contains("src/b.ts"),
        "message must mention second file; got: {}",
        dup.message
    );
}

/// SCAN-02 -- scc format returns empty findings (metrics only, not issues).
#[test]
fn scc_format_returns_empty_findings() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("scc.json");
    std::fs::write(&fixture_path, "[]").expect("write fixture");

    let result = RawScanResult {
        scanner: "scc".to_string(),
        output_format: "scc-json".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("scc must not error");
    assert!(
        findings.is_empty(),
        "scc produces no findings (metrics only)"
    );
}

/// SCAN-08 -- SARIF: scanner name is propagated to each RawFinding.
#[test]
fn sarif_scanner_name_propagated_to_findings() {
    let base = tempfile::tempdir().expect("tempdir");
    let fixture_path = base.path().join("trivy.sarif");
    std::fs::copy(
        std::path::Path::new("tests/fixtures/trivy_sample.sarif"),
        &fixture_path,
    )
    .expect("copy fixture");

    let result = RawScanResult {
        scanner: "trivy".to_string(),
        output_format: "sarif".to_string(),
        output_file: fixture_path,
        exit_code: 0,
    };

    let findings = parse_scanner_output(&result).expect("parse should succeed");
    for f in &findings {
        assert_eq!(
            f.scanner, "trivy",
            "scanner name must be propagated to each finding"
        );
    }
}
