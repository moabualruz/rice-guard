//! WSJF-based priority scoring for findings.
//!
//! Weighted Shortest Job First (WSJF) is used to surface the highest-value,
//! lowest-effort issues first.  Although we do not have reliable job-size
//! estimates from static analysis, we approximate with a simplified formula:
//!
//! ```text
//! wsjf_score = (business_value + time_criticality + risk_reduction)
//!              / job_size_estimate
//! ```
//!
//! ## Input Signals
//!
//! | Signal              | Source                         |
//! |---------------------|--------------------------------|
//! | `severity`          | Normalized scanner severity    |
//! | `scanner`           | Which tool found the issue     |
//! | `has_autofix`       | Whether a fix snippet exists   |
//!
//! ## Score Ranges
//!
//! | `Priority` level | `wsjf_score` range |
//! |-------------------|--------------------|
//! | `Critical`        | ≥ 16               |
//! | `High`            | 8–15               |
//! | `Medium`          | 4–7                |
//! | `Low`             | < 4                |

use serde::{Deserialize, Serialize};

/// Priority level derived from WSJF scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PriorityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Computed priority attached to a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    /// Discrete priority level for display/filtering.
    pub level: PriorityLevel,
    /// The raw WSJF score (higher = more urgent).
    pub wsjf_score: f32,
    /// Breakdown of component scores used (for transparency).
    pub components: PriorityComponents,
}

/// Sub-scores that feed into the WSJF numerator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityComponents {
    pub business_value: f32,
    pub time_criticality: f32,
    pub risk_reduction: f32,
    pub job_size_estimate: f32,
}

/// Compute [`Priority`] for a finding.
///
/// # Arguments
/// * `severity` – normalized severity string (`"error"`, `"warning"`, `"high"`, `"info"`)
/// * `scanner`  – scanner name (e.g. `"gitleaks"`, `"trivy"`, `"semgrep"`)
/// * `has_autofix` – whether a suggested replacement is available
pub fn compute_priority(severity: &str, scanner: &str, has_autofix: bool) -> Priority {
    // Business value: based on severity
    let business_value: f32 = match severity {
        "error" | "high" | "critical" => 8.0,
        "warning" | "medium" => 4.0,
        _ => 1.0, // info / note
    };

    // Time criticality: secrets and CVEs are most time-critical.
    // Gitleaks findings always reach Critical level via explicit override below.
    let time_criticality: f32 = match scanner {
        "gitleaks" => 12.0, // exposed secrets must be rotated immediately
        "trivy" => 8.0,     // CVEs have SLA obligations
        "semgrep" => 2.0,
        _ => 2.0,
    };

    // Risk reduction: items without an autofix carry more residual risk
    let risk_reduction: f32 = if has_autofix { 2.0 } else { 5.0 };

    // Job size: autofixable items cost less effort
    let job_size_estimate: f32 = if has_autofix { 1.0 } else { 2.0 };

    let wsjf_score = (business_value + time_criticality + risk_reduction) / job_size_estimate;

    // Secret scanners always warrant Critical, regardless of numeric score.
    let level = if scanner == "gitleaks" {
        PriorityLevel::Critical
    } else {
        match wsjf_score as u32 {
            s if s >= 16 => PriorityLevel::Critical,
            s if s >= 8 => PriorityLevel::High,
            s if s >= 4 => PriorityLevel::Medium,
            _ => PriorityLevel::Low,
        }
    };

    Priority {
        level,
        wsjf_score,
        components: PriorityComponents {
            business_value,
            time_criticality,
            risk_reduction,
            job_size_estimate,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_finding_is_critical() {
        let p = compute_priority("high", "gitleaks", false);
        assert_eq!(p.level, PriorityLevel::Critical);
    }

    #[test]
    fn info_semgrep_with_autofix_is_medium_or_lower() {
        let p = compute_priority("info", "semgrep", true);
        assert!(p.level <= PriorityLevel::Medium);
    }

    #[test]
    fn trivy_error_is_high_or_above() {
        let p = compute_priority("error", "trivy", false);
        assert!(p.level >= PriorityLevel::High);
    }

    #[test]
    fn wsjf_score_is_positive() {
        let p = compute_priority("warning", "clippy", true);
        assert!(p.wsjf_score > 0.0);
    }
}
