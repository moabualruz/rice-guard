use std::collections::HashSet;
use std::path::Path;

use thiserror::Error;
use tokio::task::JoinSet;

use crate::config::RiceGuardConfig;
use crate::registry::ScannerDescriptor;

use super::{
    diff_only_filter, parser::parse_scanner_output, run_one_scanner, OutputDir, RawFinding,
    ScannerRunError,
};

/// Result of a scan run, including per-scanner outcome information.
#[derive(Debug)]
pub struct ScanReport {
    /// All parsed findings from all scanners.
    pub findings: Vec<RawFinding>,
    /// Names of scanners that ran successfully (may have 0 findings).
    pub succeeded: HashSet<String>,
    /// `(scanner_name, error_message)` for scanners that failed or timed out.
    pub failed: Vec<(String, String)>,
    /// Names of scanners that were unavailable (disabled in config).
    pub unavailable: Vec<String>,
}

/// Scanner names included in Quick mode: jscpd + scc + Semgrep + Trivy.
pub const QUICK_SCANNERS: &[&str] = &["semgrep", "trivy", "jscpd", "scc"];

/// Scanner names included in Security mode: Semgrep + Trivy + Gitleaks.
pub const SECURITY_SCANNERS: &[&str] = &["semgrep", "trivy", "gitleaks"];

/// Errors that can occur during a scan run.
#[derive(Debug, Error)]
pub enum ScannerEngineError {
    /// A scanner process did not finish within its configured timeout.
    #[error("Scanner '{0}' timed out")]
    Timeout(String),

    /// Failed to spawn a scanner subprocess.
    #[error("Failed to spawn scanner '{scanner}': {reason}")]
    SpawnFailed { scanner: String, reason: String },

    /// An I/O error occurred while setting up or reading scanner output.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Scan mode — controls which scanners are active and which command variant
/// each scanner uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanMode {
    /// Run all enabled scanners with their full scan command.
    Full,
    /// Run a fast subset (jscpd + scc + Semgrep + Trivy).
    Quick,
    /// Run only security-focused scanners (Semgrep + Trivy + Gitleaks).
    Security,
    /// Only report issues on changed files (hold-the-line mode).
    DiffOnly,
}

/// Orchestrates parallel scanner execution.
///
/// Constructed with the loaded descriptors and project config, then
/// driven by `run()` to produce parsed `RawFinding`s for downstream use.
pub struct ScannerEngine {
    descriptors: Vec<ScannerDescriptor>,
    config: RiceGuardConfig,
}

impl ScannerEngine {
    /// Create a new `ScannerEngine`.
    ///
    /// `descriptors` — loaded scanner plugin descriptors (from YAML).
    /// `config` — project config (controls which scanners are enabled).
    pub fn new(descriptors: Vec<ScannerDescriptor>, config: RiceGuardConfig) -> Self {
        Self {
            descriptors,
            config,
        }
    }

    /// Run all enabled scanners against `target`, writing output to `output_dir`.
    ///
    /// All scanners in the selected subset run concurrently via [`tokio::task::JoinSet`].
    /// Results are collected as each scanner completes. Unavailable or failed scanners
    /// produce a `tracing::warn!` and are skipped — they are never fatal.
    ///
    /// In `DiffOnly` mode a post-filter is applied: only findings whose `file_path`
    /// matches one of the git-changed files is kept. If the diff filter fails (not a git
    /// repo, no origin/main) all findings are returned (graceful fallback).
    ///
    /// # Exit semantics
    ///
    /// Returns `Ok(vec![])` when no scanners are configured or all are unavailable.
    /// Returns `Err` only for infrastructure failures (e.g., cannot create a temp file).
    pub async fn run(
        &self,
        target: &Path,
        mode: ScanMode,
        output_dir: &OutputDir,
    ) -> Result<ScanReport, ScannerEngineError> {
        // ── Step 1: select scanner subset based on mode ───────────────────────
        let selected: Vec<&ScannerDescriptor> = match mode {
            ScanMode::Quick => self
                .descriptors
                .iter()
                .filter(|d| {
                    QUICK_SCANNERS
                        .iter()
                        .any(|n| *n == d.name.to_lowercase().as_str())
                })
                .collect(),
            ScanMode::Security => self
                .descriptors
                .iter()
                .filter(|d| {
                    SECURITY_SCANNERS
                        .iter()
                        .any(|n| *n == d.name.to_lowercase().as_str())
                })
                .collect(),
            ScanMode::Full | ScanMode::DiffOnly => self.descriptors.iter().collect(),
        };

        // ── Step 2: fan-out — spawn all selected scanners concurrently ────────
        let mut set: JoinSet<Result<(String, Vec<RawFinding>), ScannerRunError>> = JoinSet::new();

        for descriptor in selected {
            let desc = descriptor.clone();
            let cfg = self.config.clone();
            let out = output_dir.path().to_path_buf();
            let tgt = target.to_path_buf();
            let m = mode;

            set.spawn(async move {
                let raw = run_one_scanner(&desc, &cfg, &out, &tgt, &m).await?;
                let findings =
                    parse_scanner_output(&raw).map_err(|e| ScannerRunError::SpawnFailed {
                        scanner: raw.scanner.clone(),
                        reason: e.to_string(),
                    })?;
                Ok((raw.scanner, findings))
            });
        }

        // ── Step 3: collect results, track per-scanner outcomes ──────────────
        let mut all_findings: Vec<RawFinding> = Vec::new();
        let mut succeeded: HashSet<String> = HashSet::new();
        let mut failed: Vec<(String, String)> = Vec::new();
        let mut unavailable: Vec<String> = Vec::new();

        while let Some(join_result) = set.join_next().await {
            match join_result {
                Ok(Ok((scanner_name, findings))) => {
                    succeeded.insert(scanner_name);
                    all_findings.extend(findings);
                }
                Ok(Err(ScannerRunError::Unavailable(name))) => {
                    tracing::warn!("scanner {} unavailable — skipping", name);
                    unavailable.push(name);
                }
                Ok(Err(ScannerRunError::Timeout(name))) => {
                    let msg = format!("timed out");
                    tracing::warn!("scanner {} {} — skipping", name, msg);
                    failed.push((name, msg));
                }
                Ok(Err(ScannerRunError::SpawnFailed { scanner, reason })) => {
                    tracing::warn!("scanner {} failed: {} — skipping", scanner, reason);
                    failed.push((scanner, reason));
                }
                Ok(Err(ScannerRunError::CommandBuildError(e))) => {
                    let msg = e.to_string();
                    tracing::warn!("scanner command build error: {} — skipping", msg);
                    failed.push(("unknown".to_string(), msg));
                }
                Err(join_err) => {
                    tracing::warn!("scanner task panicked: {} — skipping", join_err);
                    failed.push(("unknown".to_string(), join_err.to_string()));
                }
            }
        }

        // ── Step 4: DiffOnly post-filter ──────────────────────────────────────
        if mode == ScanMode::DiffOnly {
            match diff_only_filter(target).await {
                Ok(changed_files) if !changed_files.is_empty() => {
                    // Filter findings to only those on changed files.
                    all_findings.retain(|f| {
                        changed_files.iter().any(|changed| {
                            // Normalize: compare the file_path suffix against
                            // the changed file's path component.
                            let changed_str = changed.to_string_lossy();
                            let changed_str = changed_str.replace('\\', "/");
                            let finding_path = f.file_path.replace('\\', "/");
                            changed_str.ends_with(&finding_path)
                                || finding_path.ends_with(
                                    changed
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default()
                                        .as_str(),
                                )
                        })
                    });
                }
                Ok(_empty) => {
                    // Empty changed_files => no filter; return all findings (full-scan semantics).
                    tracing::warn!("diff-only: no changed files detected; returning all findings");
                }
                Err(e) => {
                    // Not a git repo or git error — fall back to full results.
                    tracing::warn!(
                        "diff-only filter failed ({}); falling back to full scan results",
                        e
                    );
                }
            }
        }

        Ok(ScanReport {
            findings: all_findings,
            succeeded,
            failed,
            unavailable,
        })
    }
}
