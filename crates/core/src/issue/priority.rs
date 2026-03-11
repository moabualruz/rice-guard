//! WSJF-based priority scoring for findings.
//!
//! ## Formula
//!
//! ```text
//! priority_score = severity(40) + auto_fixable(20) + category(20)
//!                + file_freq(10) - cross_file(10)
//! ```
//!
//! ## Score Components
//!
//! | Signal           | Value                                                            |
//! |------------------|------------------------------------------------------------------|
//! | `severity`       | `error` → 40, `warning` → 20, `info` → 5, other → 0            |
//! | `auto_fixable`   | true → 20, false → 0                                            |
//! | `category`       | `formatter`/`import` → 20, `linter`/`security` → 15, else → 10 |
//! | `file_freq`      | min(findings in file, 10) × 1 (capped at 10)                    |
//! | `cross_file`     | true → -10 penalty                                               |

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::scanner::parser::RawFinding;

/// Priority level derived from WSJF scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriorityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Compute WSJF priority score for a finding.
///
/// # Arguments
/// * `severity` — normalized severity string: `"error"`, `"warning"`, `"info"`
/// * `auto_fixable` — whether an automated fix is available
/// * `auto_fix_category` — category of the fix tool (may be `None`)
/// * `file_freq` — number of findings in the same file (capped at 10)
/// * `cross_file` — whether the issue spans multiple files
pub fn wsjf_score(
    severity: &str,
    auto_fixable: bool,
    auto_fix_category: Option<&str>,
    file_freq: u32,
    cross_file: bool,
) -> i32 {
    let severity_score: i32 = match severity {
        "error" | "high" | "critical" => 40,
        "warning" | "medium" => 20,
        "info" | "note" => 5,
        _ => 0,
    };

    let auto_fix_score: i32 = if auto_fixable { 20 } else { 0 };

    let category_score: i32 = match auto_fix_category {
        Some("formatter") | Some("import") => 20,
        Some("linter") | Some("security") => 15,
        Some("ast") | Some("deps") => 10,
        _ => 0,
    };

    let freq_score: i32 = file_freq.min(10) as i32;

    let cross_file_penalty: i32 = if cross_file { 10 } else { 0 };

    severity_score + auto_fix_score + category_score + freq_score - cross_file_penalty
}

/// Map a raw WSJF score to a discrete priority level.
pub fn priority_level(score: i32) -> PriorityLevel {
    match score {
        s if s >= 70 => PriorityLevel::Critical,
        s if s >= 40 => PriorityLevel::High,
        s if s >= 20 => PriorityLevel::Medium,
        _ => PriorityLevel::Low,
    }
}

/// Count how many findings exist per file path.
///
/// Returns a map of `file_path -> count` across all provided findings.
/// Used to populate the `file_freq` argument to [`wsjf_score`].
pub fn file_freq_map(findings: &[RawFinding]) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    for finding in findings {
        *map.entry(finding.file_path.clone()).or_insert(0) += 1;
    }
    map
}

/// Sort a slice of [`super::Issue`] by `priority_score` descending (highest first).
///
/// Issues with equal scores retain their original relative order (stable sort).
pub fn sort_issues(issues: &mut Vec<super::Issue>) {
    issues.sort_by(|a, b| b.priority_score.cmp(&a.priority_score));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_auto_fixable_formatter_max_score() {
        // error(40) + auto_fixable(20) + formatter(20) + freq=10(10) - no_cross(0) = 90
        let score = wsjf_score("error", true, Some("formatter"), 10, false);
        assert_eq!(score, 90);
    }

    #[test]
    fn info_no_fix_no_freq() {
        // info(5) + no_fix(0) + no_cat(0) + freq=0(0) - no_cross(0) = 5
        let score = wsjf_score("info", false, None, 0, false);
        assert_eq!(score, 5);
    }

    #[test]
    fn cross_file_penalty_applied() {
        // warning(20) + no_fix(0) + no_cat(0) + freq=5(5) - cross_file(10) = 15
        let score = wsjf_score("warning", false, None, 5, true);
        assert_eq!(score, 15);
    }

    #[test]
    fn file_freq_capped_at_10() {
        // error(40) with huge file_freq — capped at 10
        let score_100 = wsjf_score("error", false, None, 100, false);
        let score_10 = wsjf_score("error", false, None, 10, false);
        assert_eq!(score_100, score_10, "file_freq must be capped at 10");
        assert_eq!(score_100, 50);
    }

    #[test]
    fn linter_category_scores_15() {
        let score = wsjf_score("error", true, Some("linter"), 0, false);
        // error(40) + auto_fix(20) + linter(15) = 75
        assert_eq!(score, 75);
    }

    #[test]
    fn priority_level_thresholds() {
        assert_eq!(priority_level(90), PriorityLevel::Critical);
        assert_eq!(priority_level(70), PriorityLevel::Critical);
        assert_eq!(priority_level(69), PriorityLevel::High);
        assert_eq!(priority_level(40), PriorityLevel::High);
        assert_eq!(priority_level(39), PriorityLevel::Medium);
        assert_eq!(priority_level(20), PriorityLevel::Medium);
        assert_eq!(priority_level(19), PriorityLevel::Low);
        assert_eq!(priority_level(0), PriorityLevel::Low);
    }
}
