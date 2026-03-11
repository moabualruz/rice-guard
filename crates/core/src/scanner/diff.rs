use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors from the diff-only file filter.
#[derive(Debug, Error)]
pub enum DiffError {
    /// `git` binary not found in PATH.
    #[error("git not found in PATH")]
    GitNotFound,

    /// The target directory is not inside a git repository.
    #[error("Not a git repository: {reason}")]
    NotAGitRepo { reason: String },

    /// A git command ran but failed for an unexpected reason.
    #[error("git command failed: {reason}")]
    GitCommandFailed { reason: String },
}

/// Return the list of files changed between `HEAD` and `origin/main`.
///
/// Uses `git merge-base HEAD origin/main` to find the divergence point, then
/// `git diff --name-only <base_sha> HEAD` to enumerate changed paths.
///
/// All returned paths are absolute (relative git paths are prepended with
/// `target`).
///
/// # Fallback behaviour
///
/// | Situation                          | Returns                            |
/// |------------------------------------|------------------------------------|
/// | `git` not in PATH                  | `Err(DiffError::GitNotFound)`       |
/// | Not a git repo                     | `Err(DiffError::NotAGitRepo)`       |
/// | `origin/main` doesn't exist        | `Ok(vec![])` — full scan fallback  |
/// | No changed files (empty diff)      | `Ok(vec![])` — zero findings       |
///
/// When `Ok(vec![])` is returned the caller (ScannerEngine) treats it as
/// "no filter" and reports all findings (full scan semantics).
pub async fn diff_only_filter(target: &Path) -> Result<Vec<PathBuf>, DiffError> {
    // ── Step 1: find git binary ───────────────────────────────────────────────
    // Attempt to run `git merge-base HEAD origin/main`.
    let merge_base_output = tokio::process::Command::new("git")
        .args(["merge-base", "HEAD", "origin/main"])
        .current_dir(target)
        .output()
        .await;

    let merge_base_output = match merge_base_output {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(DiffError::GitNotFound);
        }
        Err(e) => {
            return Err(DiffError::GitCommandFailed {
                reason: e.to_string(),
            });
        }
    };

    if !merge_base_output.status.success() {
        let stderr = String::from_utf8_lossy(&merge_base_output.stderr);

        // Check if this looks like "not a git repo".
        if stderr.contains("not a git repository")
            || stderr.contains("fatal: not a git")
            || merge_base_output.status.code() == Some(128)
        {
            return Err(DiffError::NotAGitRepo {
                reason: stderr.trim().to_string(),
            });
        }

        // origin/main probably doesn't exist — fall back to full scan.
        tracing_or_eprintln("diff-only: no origin/main found; falling back to full scan");
        return Ok(vec![]);
    }

    let base_sha = String::from_utf8_lossy(&merge_base_output.stdout)
        .trim()
        .to_string();

    if base_sha.is_empty() {
        // Merge-base returned nothing — fall back to full scan.
        tracing_or_eprintln("diff-only: merge-base returned empty SHA; falling back to full scan");
        return Ok(vec![]);
    }

    // ── Step 2: list changed files ────────────────────────────────────────────
    let diff_output = tokio::process::Command::new("git")
        .args(["diff", "--name-only", &base_sha, "HEAD"])
        .current_dir(target)
        .output()
        .await;

    let diff_output = match diff_output {
        Ok(o) => o,
        Err(e) => {
            return Err(DiffError::GitCommandFailed {
                reason: e.to_string(),
            });
        }
    };

    if !diff_output.status.success() {
        let reason = String::from_utf8_lossy(&diff_output.stderr)
            .trim()
            .to_string();
        return Err(DiffError::GitCommandFailed { reason });
    }

    // ── Step 3: parse output into absolute PathBufs ───────────────────────────
    let stdout = String::from_utf8_lossy(&diff_output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| target.join(line))
        .collect();

    Ok(files)
}

/// Minimal log helper — uses `eprintln!` since the `tracing` crate is not
/// yet wired up in this crate.  Replace with `tracing::warn!` in Phase 5.
fn tracing_or_eprintln(msg: &str) {
    eprintln!("WARN rice-guard: {msg}");
}
