/// Integration tests for descriptor loading, validation, and command building.
///
/// These tests exercise the public API of `rice_guard_core::registry`
/// using temporary directories and fixture files.
use rice_guard_core::errors::DescriptorError;
use rice_guard_core::registry::{loader, probe};
use tempfile::TempDir;

// ── load_scanner_descriptors ─────────────────────────────────────────────────

#[test]
fn builtin_scanner_descriptors_load() {
    let tmp = TempDir::new().unwrap();
    let descriptors = loader::load_scanner_descriptors(tmp.path())
        .expect("built-in scanner descriptors must load");
    assert_eq!(
        descriptors.len(),
        5,
        "expected exactly 5 built-in scanner descriptors"
    );
    assert!(descriptors.iter().any(|d| d.name == "semgrep"));
    assert!(descriptors.iter().any(|d| d.name == "trivy"));
    assert!(descriptors.iter().any(|d| d.name == "gitleaks"));
    assert!(descriptors.iter().any(|d| d.name == "jscpd"));
    assert!(descriptors.iter().any(|d| d.name == "scc"));
}

#[test]
fn builtin_fixer_descriptors_load() {
    let tmp = TempDir::new().unwrap();
    let descriptors =
        loader::load_fixer_descriptors(tmp.path()).expect("built-in fixer descriptors must load");
    assert_eq!(
        descriptors.len(),
        12,
        "expected exactly 12 built-in fixer descriptors"
    );
}

// ── invalid YAML ─────────────────────────────────────────────────────────────

#[test]
fn invalid_yaml_returns_parse_error() {
    let tmp = TempDir::new().unwrap();
    let scanner_dir = tmp.path().join("descriptors").join("scanners");
    std::fs::create_dir_all(&scanner_dir).unwrap();
    // Write the bad-syntax fixture content directly.
    std::fs::write(
        scanner_dir.join("bad.yaml"),
        "name: broken\n  bad indent: here\n: invalid",
    )
    .unwrap();
    let result = loader::load_scanner_descriptors(tmp.path());
    assert!(
        matches!(result, Err(DescriptorError::ParseError { .. })),
        "malformed YAML must return ParseError, got: {result:?}"
    );
}

#[test]
fn empty_name_returns_validation_error() {
    let tmp = TempDir::new().unwrap();
    let scanner_dir = tmp.path().join("descriptors").join("scanners");
    std::fs::create_dir_all(&scanner_dir).unwrap();
    // Write the empty-name fixture content directly.
    let empty_name_yaml = r#"name: ""
version: ">=1.0"
languages: []
install:
  check: ""
commands:
  scan:
    cmd: ""
    timeout: 60
output_format: json
severity_map: {}
"#;
    std::fs::write(scanner_dir.join("empty-name.yaml"), empty_name_yaml).unwrap();
    let result = loader::load_scanner_descriptors(tmp.path());
    assert!(
        matches!(result, Err(DescriptorError::ValidationError { .. })),
        "empty name must return ValidationError, got: {result:?}"
    );
}

// ── build_command ────────────────────────────────────────────────────────────

#[test]
fn build_command_preserves_spaces_in_values() {
    let args = probe::build_command(
        "tool run {{output_dir}}",
        &[("output_dir", "my output dir")],
    )
    .unwrap();
    // "my output dir" must be a single opaque token, not split into 3.
    assert_eq!(
        args,
        vec!["tool", "run", "my output dir"],
        "value with spaces must remain a single token"
    );
}

#[test]
fn build_command_injection_is_prevented() {
    // Semicolons in a value must NOT be interpreted as shell command separators.
    let args = probe::build_command(
        "tool --out {{output_dir}}",
        &[("output_dir", "dir; rm -rf /")],
    )
    .unwrap();
    // The value with semicolons should be a single token, not two commands.
    assert_eq!(args.len(), 3, "must produce exactly 3 tokens");
    assert_eq!(
        args[2], "dir; rm -rf /",
        "injection payload must be a verbatim token"
    );
}

#[test]
fn custom_scanner_fixture_parses() {
    // Load the valid custom-scanner fixture from the test fixtures directory.
    let fixture_yaml =
        include_str!("../../../tests/fixtures/descriptors/valid/custom-scanner.yaml");
    use rice_guard_core::registry::scanner_descriptor::ScannerDescriptor;
    let desc: ScannerDescriptor =
        serde_yaml_ng::from_str(fixture_yaml).expect("valid custom-scanner fixture must parse");
    assert_eq!(desc.name, "custom-linter");
    desc.validate()
        .expect("custom-scanner fixture must be valid");
}
