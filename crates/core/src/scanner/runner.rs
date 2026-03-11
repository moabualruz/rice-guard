use std::path::{Path, PathBuf};
use std::process::Stdio;

use thiserror::Error;
use tokio::time::{timeout, Duration};

use crate::config::RiceGuardConfig;
use crate::errors::DescriptorError;
use crate::registry::{probe::build_command, ScannerDescriptor};

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

    // ── Step 4 + 5: spawn with timeout ────────────────────────────────────────
    let scanner_name = descriptor.name.clone();
    let timeout_secs = cmd.timeout as u64;

    let spawn_fut = async {
        tokio::process::Command::new(&args[0])
            .args(&args[1..])
            .stdout(Stdio::null()) // SCAN-07: MUST be null — never pipe stdout
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .status()
            .await
    };

    let status = match timeout(Duration::from_secs(timeout_secs), spawn_fut).await {
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
    };

    // ── Step 6: determine expected output file path ───────────────────────────
    // Convention: `<output_dir>/<scanner_name>.output`
    // The actual file may differ for some scanners (e.g. jscpd writes its own
    // path), but parsers handle that discrepancy — we record the expected path.
    let output_file: PathBuf = output_dir.join(format!("{}.output", descriptor.name));

    Ok(RawScanResult {
        scanner: descriptor.name.clone(),
        output_format: descriptor.output_format.clone(),
        output_file,
        exit_code: status.code().unwrap_or(-1),
    })
}
