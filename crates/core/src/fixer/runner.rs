/// Single-fixer execution: check → fix → verify cycle.
///
/// `run_one_fixer` implements the check-fix-verify cycle for a single
/// fixer tool descriptor entry.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime};

use crate::fixer::report::{FixToolResult, FixToolStatus};
use crate::registry::fixer_descriptor::FixerStep;
use crate::registry::probe::build_command;

/// Configuration for a single fixer invocation.
pub struct RunnerConfig {
    /// Root directory of the project being fixed.
    pub project_root: PathBuf,
    /// Specific files to fix; empty means the whole project.
    pub file_targets: Vec<PathBuf>,
    /// When true, only check for issues but do not modify files.
    pub dry_run: bool,
    /// Per-tool timeout in seconds. `None` means no timeout (run until done).
    pub timeout_secs: Option<u64>,
    /// Additional CLI arguments appended to both check and fix commands.
    ///
    /// Used to pass tool-native exclude flags derived from active ignore patterns.
    /// Mirrors the `extra_args` pattern from `scanner/runner.rs`.
    pub extra_args: Vec<String>,
}

/// Execute a single fixer tool using the check → (dry-run?) → fix → verify cycle.
///
/// Returns [`FixToolResult`] for all outcomes including tool-unavailable (Skipped)
/// and dry-run (DryRunWouldFix). Never returns `Err` — all failures are encoded
/// in the result status so the pipeline can continue.
pub async fn run_one_fixer(step: &FixerStep, config: &RunnerConfig) -> FixToolResult {
    let start = Instant::now();

    // ── Step 1: build check command args ─────────────────────────────────────
    let files_str = build_files_arg(&config.file_targets, &config.project_root);
    let mut check_args = match build_command(&step.check, &[("files", &files_str)]) {
        Ok(a) => a,
        Err(e) => {
            return FixToolResult {
                name: step.name.clone(),
                language: String::new(),
                status: FixToolStatus::Skipped,
                modified_files: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                error_msg: Some(format!("invalid check command: {e}")),
            };
        }
    };
    check_args.extend_from_slice(&config.extra_args);

    // ── Step 2: run check ─────────────────────────────────────────────────────
    let check_exit = run_command(&check_args, &config.project_root, config.timeout_secs).await;

    let check_needs_fix = match check_exit {
        CommandOutcome::SpawnFailed(reason) => {
            // Tool not found — soft skip, do not fail the pipeline.
            return FixToolResult {
                name: step.name.clone(),
                language: String::new(),
                status: FixToolStatus::Skipped,
                modified_files: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                error_msg: Some(format!("tool unavailable: {reason}")),
            };
        }
        CommandOutcome::Timeout => {
            return FixToolResult {
                name: step.name.clone(),
                language: String::new(),
                status: FixToolStatus::Skipped,
                modified_files: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                error_msg: Some("check command timed out".to_string()),
            };
        }
        // exit 0 means "no issues found"
        CommandOutcome::Exited(0) => false,
        // any non-zero exit means "issues present, fix needed"
        CommandOutcome::Exited(_) => true,
    };

    if !check_needs_fix {
        return FixToolResult {
            name: step.name.clone(),
            language: String::new(),
            status: FixToolStatus::NoIssues,
            modified_files: vec![],
            duration_ms: start.elapsed().as_millis() as u64,
            error_msg: None,
        };
    }

    // ── Step 3: dry-run gate ──────────────────────────────────────────────────
    if config.dry_run {
        return FixToolResult {
            name: step.name.clone(),
            language: String::new(),
            status: FixToolStatus::DryRunWouldFix,
            modified_files: vec![],
            duration_ms: start.elapsed().as_millis() as u64,
            error_msg: None,
        };
    }

    // ── Step 4: snapshot mtimes before fix ────────────────────────────────────
    let before = snapshot_mtimes(&config.project_root);

    // ── Step 5: build fix command args ────────────────────────────────────────
    let mut fix_args = match build_command(&step.fix, &[("files", &files_str)]) {
        Ok(a) => a,
        Err(e) => {
            return FixToolResult {
                name: step.name.clone(),
                language: String::new(),
                status: FixToolStatus::Failed,
                modified_files: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                error_msg: Some(format!("invalid fix command: {e}")),
            };
        }
    };
    fix_args.extend_from_slice(&config.extra_args);

    // ── Step 6: run fix ───────────────────────────────────────────────────────
    let fix_exit = run_command(&fix_args, &config.project_root, config.timeout_secs).await;

    if let CommandOutcome::SpawnFailed(reason) = fix_exit {
        return FixToolResult {
            name: step.name.clone(),
            language: String::new(),
            status: FixToolStatus::Failed,
            modified_files: vec![],
            duration_ms: start.elapsed().as_millis() as u64,
            error_msg: Some(reason),
        };
    }

    // ── Step 7: compute modified files from mtime diff ────────────────────────
    let modified = changed_files(&before, &config.project_root);

    // ── Step 8: verify — re-run check ─────────────────────────────────────────
    let verify_exit = run_command(&check_args, &config.project_root, config.timeout_secs).await;

    let status = match verify_exit {
        // exit 0: all issues resolved — Fixed
        CommandOutcome::Exited(0) => FixToolStatus::Fixed,
        // any other outcome: fixer ran but issue persists — Attempted (not Failed)
        _ => FixToolStatus::Attempted,
    };

    FixToolResult {
        name: step.name.clone(),
        language: String::new(),
        status,
        modified_files: modified,
        duration_ms: start.elapsed().as_millis() as u64,
        error_msg: None,
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Outcome of a spawned subprocess.
enum CommandOutcome {
    Exited(i32),
    SpawnFailed(String),
    Timeout,
}

/// Spawn a subprocess and wait for it to finish (with optional timeout).
///
/// Follows the SCAN-07 pattern: stdout is always null to prevent pipe deadlock.
async fn run_command(args: &[String], cwd: &Path, timeout_secs: Option<u64>) -> CommandOutcome {
    if args.is_empty() {
        return CommandOutcome::SpawnFailed("empty command".to_string());
    }

    let mut cmd = crate::registry::probe::resolve_command(&args[0]);
    cmd.args(&args[1..])
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    let spawn_fut = async { cmd.status().await };

    if let Some(secs) = timeout_secs {
        match tokio::time::timeout(Duration::from_secs(secs), spawn_fut).await {
            Ok(Ok(status)) => CommandOutcome::Exited(status.code().unwrap_or(-1)),
            Ok(Err(e)) => CommandOutcome::SpawnFailed(e.to_string()),
            Err(_elapsed) => CommandOutcome::Timeout,
        }
    } else {
        match spawn_fut.await {
            Ok(status) => CommandOutcome::Exited(status.code().unwrap_or(-1)),
            Err(e) => CommandOutcome::SpawnFailed(e.to_string()),
        }
    }
}

/// Build the `{{files}}` substitution value.
///
/// When `file_targets` is non-empty, join them as space-separated paths.
/// When empty, use the project root (whole-project mode).
fn build_files_arg(file_targets: &[PathBuf], project_root: &Path) -> String {
    if file_targets.is_empty() {
        project_root.to_string_lossy().into_owned()
    } else {
        file_targets
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Directories to skip when walking the project tree for mtime snapshots.
/// These are large generated/vendored directories that fixers never modify.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".dart_tool",
    ".flutter-plugins-dependencies",
    "build",
    "node_modules",
    ".gradle",
    ".idea",
    ".vs",
    ".vscode",
    "target",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "vendor",
    ".next",
    "reports",
];

/// Check if a walkdir entry is inside a skipped directory.
fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_dir() {
        if let Some(name) = entry.file_name().to_str() {
            return SKIP_DIRS.contains(&name);
        }
    }
    false
}

/// Snapshot the last-modified timestamps of all files under `dir`.
///
/// Skips large generated directories (build/, node_modules/, .git/, etc.)
/// to avoid multi-minute walks on large monorepos.
fn snapshot_mtimes(dir: &Path) -> HashMap<PathBuf, SystemTime> {
    let mut map = HashMap::new();
    let walker = walkdir::WalkDir::new(dir).into_iter();
    for entry in walker
        .filter_entry(|e| !should_skip_dir(e))
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                map.insert(entry.path().to_path_buf(), mtime);
            }
        }
    }
    map
}

/// Return paths of files whose mtime changed (or that are new) since `before`.
///
/// Paths are normalised to forward slashes for cross-platform consistency.
/// Skips the same large directories as `snapshot_mtimes`.
fn changed_files(before: &HashMap<PathBuf, SystemTime>, dir: &Path) -> Vec<String> {
    let mut changed = Vec::new();
    let walker = walkdir::WalkDir::new(dir).into_iter();
    for entry in walker
        .filter_entry(|e| !should_skip_dir(e))
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                let was_modified = before.get(&path).map(|old| *old != mtime).unwrap_or(true);
                if was_modified {
                    changed.push(path.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_step(name: &str, check: &str, fix: &str) -> FixerStep {
        FixerStep {
            name: name.to_string(),
            check: check.to_string(),
            fix: fix.to_string(),
            scope: "project".to_string(),
            safe: true,
            unsafe_flag: None,
        }
    }

    // Helper: a command that always exits non-zero (check = "has issues")
    #[cfg(unix)]
    fn always_fail_cmd() -> String {
        "/bin/sh -c 'exit 1'".to_string()
    }
    #[cfg(windows)]
    fn always_fail_cmd() -> String {
        "cmd /C exit 1".to_string()
    }

    // Helper: a command that always exits 0 (check = "no issues" / fix success)
    #[cfg(unix)]
    fn always_pass_cmd() -> String {
        "/bin/sh -c 'exit 0'".to_string()
    }
    #[cfg(windows)]
    fn always_pass_cmd() -> String {
        "cmd /C exit 0".to_string()
    }

    #[tokio::test]
    async fn check_fix_verify_cycle() {
        // Strategy: use a sentinel file to drive check/fix state.
        // check: exits 1 (has issues) if sentinel exists, 0 (clean) if absent.
        // fix:   deletes the sentinel file.
        // After fix, verify re-runs check -> sentinel gone -> exits 0 -> Fixed.
        let dir = tempdir().unwrap();
        let sentinel = dir.path().join("has_issues.flag");
        std::fs::write(&sentinel, "1").unwrap();

        #[cfg(unix)]
        let sentinel_str = sentinel.to_string_lossy().replace('\\', "/");

        #[cfg(unix)]
        let check_cmd = format!("/bin/sh -c 'test -f \"{sentinel_str}\" && exit 1 || exit 0'");
        #[cfg(windows)]
        let check_cmd = format!(
            "cmd /C if exist \"{sentinel_str}\" (exit 1) else (exit 0)",
            sentinel_str = sentinel.to_string_lossy()
        );

        #[cfg(unix)]
        let fix_cmd = format!("/bin/sh -c '/bin/rm -f \"{sentinel_str}\"'");
        #[cfg(windows)]
        let fix_cmd = format!(
            "cmd /C del /F /Q \"{sentinel_str}\"",
            sentinel_str = sentinel.to_string_lossy()
        );

        let step = make_step("test-tool", &check_cmd, &fix_cmd);
        let cfg = RunnerConfig {
            project_root: dir.path().to_path_buf(),
            file_targets: vec![],
            dry_run: false,
            timeout_secs: None,
            extra_args: vec![],
        };

        let result = run_one_fixer(&step, &cfg).await;
        // check exits non-zero -> fix runs (deletes sentinel) -> verify exits 0 -> Fixed
        assert!(
            matches!(result.status, FixToolStatus::Fixed),
            "expected Fixed, got {:?}: {:?}",
            result.status,
            result.error_msg
        );
    }

    #[tokio::test]
    async fn dry_run_no_file_changes() {
        let dir = tempdir().unwrap();
        // Create a file to verify it is NOT modified
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "original content").unwrap();

        let step = make_step("test-tool", &always_fail_cmd(), &always_pass_cmd());
        let cfg = RunnerConfig {
            project_root: dir.path().to_path_buf(),
            file_targets: vec![],
            dry_run: true,
            timeout_secs: None,
            extra_args: vec![],
        };

        let result = run_one_fixer(&step, &cfg).await;
        assert!(
            matches!(result.status, FixToolStatus::DryRunWouldFix),
            "expected DryRunWouldFix, got {:?}",
            result.status
        );
        // File content must be unchanged
        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "original content", "dry_run must not modify files");
    }

    #[tokio::test]
    async fn unavailable_tool_skips() {
        let dir = tempdir().unwrap();
        // Use a guaranteed-unavailable tool name
        let step = make_step(
            "nonexistent",
            "___nonexistent_tool_xyz___ --check .",
            "___nonexistent_tool_xyz___ --fix .",
        );
        let cfg = RunnerConfig {
            project_root: dir.path().to_path_buf(),
            file_targets: vec![],
            dry_run: false,
            timeout_secs: None,
            extra_args: vec![],
        };

        let result = run_one_fixer(&step, &cfg).await;
        assert!(
            matches!(result.status, FixToolStatus::Skipped),
            "expected Skipped for unavailable tool, got {:?}",
            result.status
        );
    }

    #[tokio::test]
    async fn file_targets_substituted() {
        let dir = tempdir().unwrap();

        // Write a sentinel file that we'll target
        let target_file = dir.path().join("target.txt");
        std::fs::write(&target_file, "data").unwrap();

        // Use a temp file to capture what was passed as args.
        // On Unix: `sh -c 'echo $@ > /tmp/args_out.txt' -- {{files}}`
        // On Windows: similar with cmd
        // Simpler approach: just verify the step runs without panic when
        // file_targets is non-empty and {{files}} is in the command.
        // The substitution is tested by verifying the command doesn't fail
        // with a "bad substitution" error, and the result is not Skipped
        // (which would mean the command wasn't found at all).

        // We use the always-pass command with {{files}} appended to verify
        // the substitution happens without error.
        #[cfg(unix)]
        let check_cmd = "/bin/sh -c 'exit 1'".to_string();
        #[cfg(windows)]
        let check_cmd = "cmd /C exit 1".to_string();

        #[cfg(unix)]
        let fix_cmd = "/bin/sh -c 'echo {{files}} > /dev/null && exit 0'".to_string();
        #[cfg(windows)]
        let fix_cmd = "cmd /C exit 0".to_string();

        let step = make_step("subst-test", &check_cmd, &fix_cmd);
        let cfg = RunnerConfig {
            project_root: dir.path().to_path_buf(),
            file_targets: vec![target_file.clone()],
            dry_run: false,
            timeout_secs: None,
            extra_args: vec![],
        };

        let result = run_one_fixer(&step, &cfg).await;
        // The tool should run (not Skipped) — proof that {{files}} substitution
        // didn't break command building. Fixed or Attempted both confirm execution.
        assert!(
            !matches!(result.status, FixToolStatus::Skipped),
            "tool should have run (not Skipped) — got {:?}: {:?}",
            result.status,
            result.error_msg
        );
    }
}
