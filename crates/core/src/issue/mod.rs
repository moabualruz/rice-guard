//! Issue domain model — the normalized representation of a single finding.
//!
//! A finding from any scanner (`RawFinding`) is enriched into an `Issue` by
//! attaching evidence, fix metadata, and a WSJF priority score.

pub mod builder;
pub mod evidence;
pub mod fix;
pub mod priority;

use serde::{Deserialize, Serialize};

use evidence::Evidence;
use fix::FixMetadata;
use priority::Priority;

/// A fully-enriched finding ready for reporting and AI analysis.
///
/// Constructed via [`builder::IssueBuilder::build`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Stable, deterministic fingerprint ID formed from scanner + rule + file + line.
    ///
    /// Format: `<scanner>/<rule_id>@<file>:<line>`
    pub id: String,

    /// Scanner that produced this finding (e.g. `"semgrep"`, `"trivy"`, `"clippy"`).
    pub scanner: String,

    /// Rule identifier from the scanner (e.g. `"semgrep.python.security.sql-injection"`).
    pub rule_id: String,

    /// Normalized severity: `"error"`, `"warning"`, or `"info"`.
    pub severity: String,

    /// Relative file path, normalized to forward slashes.
    pub file_path: String,

    /// 1-based line number of the finding (0 = unknown).
    pub line: u32,

    /// Human-readable description of the issue.
    pub message: String,

    /// Code evidence for the finding (scanner-provided or line-window fallback).
    pub evidence: Evidence,

    /// Fix metadata: category, tool, snippet, and shell command.
    pub fix: FixMetadata,

    /// WSJF-derived priority level and score.
    pub priority: Priority,
}
