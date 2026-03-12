/// CLI integration tests using assert_cmd.
///
/// These tests verify the binary's exit codes, help output, and stub behavior.
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

// ── Test helpers ─────────────────────────────────────────────────────────────

/// Write a minimal `.riceguard.yaml` into `dir` so that `fix` and `scan` can
/// load a valid config without running `init` first.
fn write_minimal_config(dir: &std::path::Path) {
    let yaml = r#"version: "1"
project:
  name: test-project
  languages:
    - rust
  topology: monolith
  architecture: none
scanners:
  semgrep:
    enabled: false
  trivy:
    enabled: false
  gitleaks:
    enabled: false
  jscpd:
    enabled: false
  scc:
    enabled: false
"#;
    std::fs::write(dir.join(".riceguard.yaml"), yaml).expect("failed to write test config");
}

// ── Additional integration tests (Plan 01-05) ────────────────────────────────

/// `rice-guard scan --help` exits 0 (covers CLI-02 help discoverability).
#[test]
fn scan_help_exits_zero() {
    rice_guard().args(["scan", "--help"]).assert().success();
}

/// `rice-guard version` (subcommand) exits 0 and stdout contains the package version.
#[test]
fn version_subcommand_contains_pkg_version() {
    rice_guard()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

/// `rice-guard init . --yes` with piped (non-TTY) stdin exits 0.
///
/// --yes mode must not require a TTY. Pipe empty stdin to ensure no TTY is
/// allocated. Requires `scc` in PATH; marked `#[ignore]` when running in
/// environments where scc is unavailable.
#[test]
#[ignore = "Requires scc in PATH — run manually or in CI with scc installed"]
fn init_yes_no_tty_exits_zero() {
    // Use assert_cmd::Command (not std::process::Command) for write_stdin support.
    assert_cmd::Command::cargo_bin("rice-guard")
        .unwrap()
        .args(["init", ".", "--yes"])
        .write_stdin("")
        .assert()
        .success();
}

fn rice_guard() -> Command {
    Command::cargo_bin("rice-guard").unwrap()
}

// ── Help / version tests ────────────────────────────────────────────────────

#[test]
fn help_lists_all_subcommands() {
    rice_guard()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("fix"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("enroll"))
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("version"));
}

#[test]
fn init_help_shows_yes_and_dry_run() {
    rice_guard()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn version_flag_prints_version() {
    rice_guard()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"rice-guard \d+\.\d+\.\d+").unwrap());
}

// ── Stub subcommand tests ───────────────────────────────────────────────────

#[test]
fn scan_missing_config_exits_two() {
    // With no .riceguard.yaml in the target dir, scan exits 2 (tool error)
    // and prints a helpful "run init" message to stderr.
    let tmp = tempfile::tempdir().unwrap();
    rice_guard()
        .args(["scan", tmp.path().to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("rice-guard init"));
}

#[test]
fn fix_no_config_exits_two() {
    // Running `fix` without a .riceguard.yaml exits 2 with a helpful message.
    // Uses the crate root (no config) as the target directory.
    rice_guard()
        .args(["fix", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No .riceguard.yaml"));
}

#[test]
fn status_not_implemented_exits_zero() {
    rice_guard()
        .args(["status", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn serve_not_implemented_exits_zero() {
    rice_guard()
        .args(["serve"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn enroll_not_implemented_exits_zero() {
    rice_guard()
        .args(["enroll"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn report_not_implemented_exits_zero() {
    rice_guard()
        .args(["report"])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

// ── Version subcommand tests ────────────────────────────────────────────────

#[test]
fn version_subcommand_prints_version() {
    rice_guard()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rice-guard"));
}

#[test]
fn version_completions_bash_outputs_script() {
    rice_guard()
        .args(["version", "--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rice-guard"));
}

// ── Exit code tests ─────────────────────────────────────────────────────────

#[test]
fn scan_help_shows_flags() {
    rice_guard()
        .args(["scan", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--quick"))
        .stdout(predicate::str::contains("--security"));
}

#[test]
fn fix_help_shows_flags() {
    rice_guard()
        .args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--formatters"));
}

#[test]
fn fix_help_shows_new_flags() {
    rice_guard()
        .args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--imports"))
        .stdout(predicate::str::contains("--diff"))
        .stdout(predicate::str::contains("--rescan"))
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn serve_help_shows_port_flag() {
    rice_guard()
        .args(["serve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--port"));
}

// ── Fix subcommand integration tests (Phase 4 gate — FIX-01..FIX-11) ────────

/// Verify that `fix --help` exposes all 11 user-visible flags.
/// Covers FIX-03, FIX-06, FIX-07, FIX-10, FIX-11.
#[test]
fn fix_help_shows_all_flags() {
    rice_guard()
        .args(["fix", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--formatters"))
        .stdout(predicate::str::contains("--linters"))
        .stdout(predicate::str::contains("--security"))
        .stdout(predicate::str::contains("--ast"))
        .stdout(predicate::str::contains("--deps"))
        .stdout(predicate::str::contains("--imports"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--unsafe"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--diff"))
        .stdout(predicate::str::contains("--rescan"))
        .stdout(predicate::str::contains("--timeout"));
}

/// `fix <dir> --dry-run` on an initialised directory exits 0.
/// Covers FIX-06 exit-code contract: dry-run always exits 0.
#[test]
fn fix_dry_run_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_config(dir.path());
    rice_guard()
        .args(["fix", dir.path().to_str().unwrap(), "--dry-run"])
        .assert()
        .code(0);
}

/// `fix <dir> --dry-run` writes a valid `fix-report.json` with `schema_version` and
/// `dry_run: true`.  Covers FIX-08 (fix-report.json produced on every run).
#[test]
fn fix_dry_run_writes_report() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_config(dir.path());
    rice_guard()
        .args(["fix", dir.path().to_str().unwrap(), "--dry-run"])
        .assert()
        .code(0);

    let latest = dir
        .path()
        .join("reports")
        .join("latest")
        .join("fix-report.json");
    assert!(
        latest.exists(),
        "fix-report.json must exist at reports/latest/"
    );

    let content = std::fs::read_to_string(&latest).expect("failed to read fix-report.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("fix-report.json must be valid JSON");
    assert_eq!(
        parsed["schema_version"], "1.0",
        "schema_version must be '1.0'"
    );
    assert_eq!(parsed["dry_run"], true, "dry_run field must be true");
}

/// `fix <dir> --unsafe` without `--yes` in a non-TTY environment exits 2.
/// Covers FIX-07: CI safety gate for unsafe fixes.
#[test]
fn fix_unsafe_without_yes_non_tty_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_config(dir.path());
    // Pipe stdin to simulate non-TTY
    let output = Command::cargo_bin("rice-guard")
        .unwrap()
        .args(["fix", dir.path().to_str().unwrap(), "--unsafe"])
        .stdin(std::process::Stdio::piped())
        .output()
        .expect("failed to spawn rice-guard");
    assert_eq!(
        output.status.code(),
        Some(2),
        "--unsafe without --yes in non-TTY must exit 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--yes") || stderr.contains("unsafe"),
        "stderr must mention --yes or unsafe; got: {stderr}"
    );
}

/// `fix <dir> --formatters --dry-run` exits 0 — verifies stage-filter flag is accepted.
/// Covers FIX-03 (stage filter flags).
#[test]
fn fix_formatters_stage_filter() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_config(dir.path());
    rice_guard()
        .args([
            "fix",
            dir.path().to_str().unwrap(),
            "--formatters",
            "--dry-run",
        ])
        .assert()
        .code(0);
}
