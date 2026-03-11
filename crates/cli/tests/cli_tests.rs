/// CLI integration tests using assert_cmd.
///
/// These tests verify the binary's exit codes, help output, and stub behavior.
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

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
fn scan_not_implemented_exits_zero() {
    rice_guard()
        .args(["scan", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn fix_not_implemented_exits_zero() {
    rice_guard()
        .args(["fix", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("not yet implemented"));
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
fn serve_help_shows_port_flag() {
    rice_guard()
        .args(["serve", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--port"));
}
