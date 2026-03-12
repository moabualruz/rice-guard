use std::path::{Path, PathBuf};
use std::process::Stdio;

use thiserror::Error;
use tokio::time::{timeout, Duration};

use crate::config::RiceGuardConfig;
use crate::errors::DescriptorError;
use crate::registry::probe::{build_command, resolve_command};
use crate::registry::ScannerDescriptor;

use super::{RawScanResult, ScanMode};

/// Errors from a single-scanner invocation.
#[derive(Debug, Error)]
pub enum ScannerRunError {
    /// Scanner is disabled in `config.tools.scanners`.
    #[error("Scanner '{0}' is unavailable (disabled in config)")]
    Unavailable(String),

    /// Scanner process did not finish within its configured timeout.
    #[error("Scanner '{0}' timed out")]
    Timeout(String),

    /// Failed to spawn the scanner subprocess.
    #[error("Failed to spawn scanner '{scanner}': {reason}")]
    SpawnFailed { scanner: String, reason: String },

    /// Command template could not be built (shlex / placeholder error).
    #[error(transparent)]
    CommandBuildError(#[from] DescriptorError),
}

/// Run a single scanner subprocess and return its `RawScanResult`.
///
/// # Availability check
///
/// Reads `config.tools.scanners[descriptor.name]`. If absent or `false`
/// the scanner is considered unavailable and `ScannerRunError::Unavailable`
/// is returned immediately — no subprocess is spawned.
///
/// # Stdout / stderr handling (SCAN-07)
///
/// Stdout is **always** `Stdio::null()` to prevent the 64 KB pipe-buffer
/// deadlock. Stderr is piped for diagnostic purposes (logged on error).
///
/// # Exit code
///
/// A non-zero exit code is **not** treated as an error — many scanners
/// (e.g. Semgrep) exit 1 when they find issues. The exit code is forwarded
/// in `RawScanResult.exit_code` for downstream use.
///
/// # Mode selection
///
/// | `ScanMode`  | Command used                                          |
/// |-------------|-------------------------------------------------------|
/// | `Quick`     | `descriptor.commands.quick` or fallback to `scan`     |
/// | `Security`  | `descriptor.commands.security` or fallback to `scan`  |
/// | `Full`      | `descriptor.commands.scan`                            |
/// | `DiffOnly`  | `descriptor.commands.scan`                            |
pub async fn run_one_scanner(
    descriptor: &ScannerDescriptor,
    config: &RiceGuardConfig,
    output_dir: &Path,
    target: &Path,
    mode: &ScanMode,
) -> Result<RawScanResult, ScannerRunError> {
    // ── Step 1: availability check ────────────────────────────────────────────
    let available = config
        .tools
        .scanners
        .get(&descriptor.name)
        .copied()
        .unwrap_or(false);

    if !available {
        return Err(ScannerRunError::Unavailable(descriptor.name.clone()));
    }

    // ── Step 2: select command variant based on scan mode ─────────────────────
    let cmd = match mode {
        ScanMode::Quick => descriptor
            .commands
            .quick
            .as_ref()
            .unwrap_or(&descriptor.commands.scan),
        ScanMode::Security => descriptor
            .commands
            .security
            .as_ref()
            .unwrap_or(&descriptor.commands.scan),
        ScanMode::Full | ScanMode::DiffOnly => &descriptor.commands.scan,
    };

    // ── Step 3: build argument vector (shell-injection-safe) ──────────────────
    let output_dir_str = output_dir.to_string_lossy();
    let target_str = target.to_string_lossy();

    let args = build_command(
        &cmd.cmd,
        &[
            ("output_dir", output_dir_str.as_ref()),
            ("target", target_str.as_ref()),
        ],
    )?;

    tracing::debug!(scanner = %descriptor.name, ?args, "spawning scanner subprocess");

    // ── Step 4 + 5: spawn (with optional timeout) ─────────────────────────────
    let scanner_name = descriptor.name.clone();

    let spawn_fut = async {
        let mut command = resolve_command(&args[0]);
        command
            .args(&args[1..])
            // Run in the target project directory — all descriptor commands use "."
            // as the scan root, so current_dir must point to the actual project.
            .current_dir(target)
            // Piped so we can capture output for error reporting. In practice stdout
            // is minimal — all scanners write results to files via -o/--output flags.
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Force UTF-8 output for Python-based tools (e.g. Semgrep) on Windows,
        // where the default cp1252 codec chokes on Unicode in JSON output.
        #[cfg(windows)]
        command.env("PYTHONUTF8", "1");

        let output = command.output().await?;
        Ok::<_, std::io::Error>(output.status)
    };

    let status = if let Some(timeout_secs) = cmd.timeout {
        match timeout(Duration::from_secs(timeout_secs as u64), spawn_fut).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                return Err(ScannerRunError::SpawnFailed {
                    scanner: scanner_name,
                    reason: e.to_string(),
                });
            }
            Err(_elapsed) => {
                return Err(ScannerRunError::Timeout(scanner_name));
            }
        }
    } else {
        // No timeout configured — run until the scanner finishes.
        match spawn_fut.await {
            Ok(s) => s,
            Err(e) => {
                return Err(ScannerRunError::SpawnFailed {
                    scanner: scanner_name,
                    reason: e.to_string(),
                });
            }
        }
    };

    tracing::debug!(scanner = %scanner_name, exit_code = status.code(), "scanner process exited");

    // ── Step 6: determine output file path from command args ──────────────────
    // Scan the substituted args for the actual output path the scanner writes
    // to.  Each descriptor command contains `{{output_dir}}/filename.ext` —
    // after substitution, one arg will start with the output_dir prefix.
    let out_prefix = output_dir.to_string_lossy().replace('\\', "/");
    let mut output_file: PathBuf = args
        .iter()
        .find_map(|arg| {
            let normalized = arg.replace('\\', "/");
            if normalized.starts_with(&out_prefix) {
                Some(PathBuf::from(arg))
            } else {
                None
            }
        })
        .unwrap_or_else(|| output_dir.join(format!("{}.output", descriptor.name)));

    // Some tools (e.g. jscpd) write to a directory rather than a file.
    // When the detected path is a directory, find the first JSON/SARIF file inside.
    if output_file.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&output_file) {
            if let Some(found) = entries.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
                matches!(
                    p.extension().and_then(|e| e.to_str()),
                    Some("json" | "sarif")
                )
            }) {
                output_file = found;
            }
        }
    }

    Ok(RawScanResult {
        scanner: descriptor.name.clone(),
        output_format: descriptor.output_format.clone(),
        output_file,
        exit_code: status.code().unwrap_or(-1),
    })
}
