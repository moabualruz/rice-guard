//! Issue domain model — the normalized, AI-ready representation of a finding.
//!
//! A `RawFinding` from any scanner is enriched into an `Issue` by attaching:
//! - Pre-embedded code evidence (matched code, context lines, enclosing scope)
//! - Fix metadata (auto-fixable flag, tool, category, complexity)
//! - Verification info (how to confirm the fix worked)
//! - A WSJF priority score
//!
//! ## Module Structure
//!
//! - `evidence` — [`EvidenceBlock`] and extraction logic
//! - `fix_meta` — [`FixMetadata`] and [`FixComplexity`]
//! - `verification` — [`VerificationInfo`]
//! - `priority` — [`wsjf_score`] and [`priority_level`] functions
//! - `builder` — [`IssueBuilder`] and [`fingerprint`] (content-hash ID)

pub mod builder;
pub mod evidence;
pub mod fix_meta;
pub mod priority;
pub mod verification;

pub use builder::{fingerprint, IssueBuilder};
pub use evidence::{
    extract_evidence_block, EvidenceBlock, EvidenceExtractor, LanguageNodeKinds, CONTEXT_LINES,
};
pub use fix_meta::{FixComplexity, FixMetadata, FixerDescriptorInfo};
pub use priority::{
    file_freq_map, file_freq_with_churn, priority_level, sort_issues, wsjf_score, PriorityLevel,
};
pub use verification::VerificationInfo;

use serde::{Deserialize, Serialize};

/// A fully-enriched finding ready for AI-assisted analysis and reporting.
///
/// Constructed via [`IssueBuilder::build`]. Serializes to the AI-ready JSON
/// output format where all evidence is pre-embedded so any AI tool can reason
/// about the issue without reading source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Stable, deterministic content-hash ID (SHA-256 of rule_id + path + code).
    ///
    /// Does NOT include line number — stable when formatters shift code.
    pub id: String,

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

    /// Scanner that produced this finding (e.g. `"semgrep"`, `"trivy"`, `"clippy"`).
    pub scanner: String,

    /// Pre-embedded code evidence (matched code, context, enclosing scope, imports).
    pub evidence: EvidenceBlock,

    /// Fix metadata: auto-fixable flag, tool, category, complexity.
    pub fix: FixMetadata,

    /// Verification instructions: command and expected success condition.
    pub verification: VerificationInfo,

    /// WSJF priority score (higher = more urgent).
    ///
    /// Formula: severity(40) + auto_fixable(20) + category(20) + file_freq(10) - cross_file(10)
    pub priority_score: i32,

    /// Human-readable priority tier derived from `priority_score`.
    ///
    /// One of `"critical"` (≥70), `"high"` (≥40), `"medium"` (≥20), or `"low"` (<20).
    pub priority_tier: String,

    /// Whether this issue spans multiple files (architecture violation, etc.).
    /// Applies a -10 score penalty.
    pub cross_file: bool,
}
