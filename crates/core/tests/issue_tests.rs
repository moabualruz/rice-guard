//! Wave 0 test stubs for EVID-08 through EVID-12.
//!
//! All tests that depend on unimplemented functionality are `#[ignore]`.
//! Compile-time structural checks run immediately.

#[cfg(test)]
mod issue_tests {
    // ── EVID-06: fix.auto_fixable and fix.auto_fix_tool present ──────────────

    /// EVID-06 — fix.auto_fixable is true when a deterministic fixer can apply.
    #[test]
    fn fix_meta_auto_fixable_present() {
        use rice_guard_core::issue::FixMetadata;
        // IssueBuilder populates auto_fixable via FixMetadata::from_finding().
        // For a clippy finding, auto_fixable must be true.
        let meta = FixMetadata::from_finding("clippy", "clippy::needless_return", None);
        assert!(
            meta.auto_fixable,
            "auto_fixable must be true for clippy findings"
        );
        assert!(
            meta.auto_fix_tool.is_some(),
            "auto_fix_tool must be Some when auto_fixable"
        );
        // For a trivy CVE with no known fix, auto_fixable must be false.
        let manual = FixMetadata::from_finding("trivy", "CVE-2023-12345", None);
        assert!(
            !manual.auto_fixable,
            "auto_fixable must be false when no deterministic fix exists"
        );
    }

    /// EVID-06 — fix.auto_fix_tool is Some when auto_fixable, None otherwise.
    #[test]
    fn fix_meta_tool_present() {
        use rice_guard_core::issue::FixMetadata;
        let meta = FixMetadata {
            auto_fixable: true,
            auto_fix_tool: Some("cargo clippy --fix".to_string()),
            auto_fix_category: Some("linter".to_string()),
            suggested_replacement: None,
            complexity: rice_guard_core::issue::FixComplexity::Trivial,
        };
        assert!(
            meta.auto_fix_tool.is_some(),
            "auto_fix_tool must be Some when auto_fixable"
        );
    }

    // ── EVID-07: verification.rerun_command present ───────────────────────────

    /// EVID-07 — verification.rerun_command is a non-empty shell command.
    #[test]
    fn verification_rerun_command_present() {
        use rice_guard_core::issue::VerificationInfo;
        // from_scanner() produces a rerun_command for every scanner.
        let semgrep_info = VerificationInfo::from_scanner("semgrep", "python.security.eval");
        assert!(
            !semgrep_info.rerun_command.is_empty(),
            "rerun_command must not be empty for semgrep"
        );
        let trivy_info = VerificationInfo::from_scanner("trivy", "CVE-2023-12345");
        assert!(
            !trivy_info.rerun_command.is_empty(),
            "rerun_command must not be empty for trivy"
        );
        let unknown_info = VerificationInfo::from_scanner("custom-tool", "custom-rule");
        assert!(
            !unknown_info.rerun_command.is_empty(),
            "rerun_command must not be empty for unknown scanner"
        );
    }

    // ── EVID-08: WSJF priority scoring ───────────────────────────────────────

    /// EVID-08 — WSJF score computation: severity(40) + auto_fixable(20) +
    /// category(20) + file_freq(10) - cross_file(10).
    #[test]
    fn wsjf_score_correct() {
        use rice_guard_core::issue::wsjf_score;
        // error + auto_fixable + formatter + freq=5 + not cross_file
        // = 40 + 20 + 20 + 5 - 0 = 85
        let score = wsjf_score("error", true, Some("formatter"), 5, false);
        assert_eq!(score, 85, "WSJF score must match formula");

        // warning + no_fix + no_category + freq=0 = 20
        let score2 = wsjf_score("warning", false, None, 0, false);
        assert_eq!(score2, 20, "warning with no fix must score 20");
    }

    /// EVID-08 — sort_issues produces highest WSJF score first.
    #[test]
    fn wsjf_sort_order_highest_first() {
        use rice_guard_core::issue::{
            sort_issues, wsjf_score, EvidenceBlock, FixComplexity, FixMetadata, Issue,
            VerificationInfo,
        };

        let make_issue = |id: &str, score: i32| Issue {
            id: id.to_string(),
            rule_id: "test-rule".to_string(),
            severity: "warning".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: 1,
            message: "test".to_string(),
            scanner: "semgrep".to_string(),
            evidence: EvidenceBlock {
                matched_code: String::new(),
                context_before: vec![],
                context_after: vec![],
                enclosing_function: None,
                enclosing_class: None,
                imports: vec![],
            },
            fix: FixMetadata {
                auto_fixable: false,
                auto_fix_tool: None,
                auto_fix_category: None,
                suggested_replacement: None,
                complexity: FixComplexity::Moderate,
            },
            verification: VerificationInfo {
                rerun_command: "rice-guard scan .".to_string(),
                success_condition: "no findings".to_string(),
            },
            priority_score: score,
            cross_file: false,
        };

        // Confirm wsjf_score values match expected formula.
        let high = wsjf_score("error", true, Some("formatter"), 10, false); // 90
        let mid = wsjf_score("warning", false, None, 0, false); // 20
        let low = wsjf_score("info", false, None, 0, false); // 5

        let mut issues = vec![
            make_issue("low", low),
            make_issue("high", high),
            make_issue("mid", mid),
        ];

        sort_issues(&mut issues);

        assert_eq!(issues[0].id, "high", "highest score must be first");
        assert_eq!(issues[1].id, "mid", "mid score must be second");
        assert_eq!(issues[2].id, "low", "lowest score must be last");
        assert!(
            issues[0].priority_score >= issues[1].priority_score,
            "scores must be non-increasing"
        );
        assert!(
            issues[1].priority_score >= issues[2].priority_score,
            "scores must be non-increasing"
        );
    }

    // ── EVID-09: Three output files written ───────────────────────────────────

    /// EVID-09 — OutputWriter produces issues.json, issues-fixable.json,
    /// and issues-remaining.json in the output directory.
    #[test]
    #[ignore = "Wave 0 stub: implement OutputWriter.write_all in Plan 03-04"]
    fn output_three_files_written() {
        todo!("verify three JSON files exist after write_all");
    }

    /// EVID-09 — issues-fixable.json contains only auto_fixable=true issues;
    /// issues-remaining.json contains only auto_fixable=false issues.
    #[test]
    #[ignore = "Wave 0 stub: implement fixable/remaining split in Plan 03-04"]
    fn fixable_remaining_split_correct() {
        todo!("verify fixable and remaining JSON files contain correct subsets");
    }

    // ── EVID-10: summary.json counts match ───────────────────────────────────

    /// EVID-10 — summary.json total_issues matches the actual issue count.
    #[test]
    #[ignore = "Wave 0 stub: implement ScanSummary serialization in Plan 03-04"]
    fn summary_json_counts_match() {
        todo!("verify summary.json total_issues == len(issues)");
    }

    // ── EVID-11: summary.txt is ASCII table ───────────────────────────────────

    /// EVID-11 — summary.txt uses only ASCII characters (no Unicode box-drawing).
    #[test]
    #[ignore = "Wave 0 stub: implement summary.txt writer in Plan 03-04"]
    fn summary_txt_ascii_table() {
        todo!("verify summary.txt contains only ASCII characters");
    }

    // ── EVID-12: forward-slash paths ─────────────────────────────────────────

    /// EVID-12 — all file_path values in output use forward slashes,
    /// even on Windows where the OS separator is backslash.
    #[test]
    #[ignore = "Wave 0 stub: verify path normalization propagation in Plan 03-04"]
    fn path_forward_slashes_in_output() {
        todo!("verify no backslashes in any file_path in issues.json");
    }

    // ── Compile-time: ScanSummary struct has required fields ─────────────────

    /// Verify ScanSummary has all required Phase 3 fields (compile-time check).
    #[test]
    fn scan_summary_has_required_fields() {
        use rice_guard_core::output::ScanSummary;
        use std::collections::HashMap;

        let summary = ScanSummary {
            scanned_at: "2026-03-11T00:00:00Z".to_string(),
            project_path: "/path/to/project".to_string(),
            total_issues: 10,
            fixable_count: 3,
            remaining_count: 7,
            by_severity: HashMap::from([
                ("error".to_string(), 2usize),
                ("warning".to_string(), 8usize),
            ]),
            by_complexity: HashMap::from([
                ("trivial".to_string(), 3usize),
                ("moderate".to_string(), 5usize),
                ("complex".to_string(), 2usize),
            ]),
            scanners_run: vec!["semgrep".to_string(), "trivy".to_string()],
            scan_duration_ms: 1234,
        };

        assert_eq!(summary.total_issues, 10);
        assert_eq!(summary.fixable_count, 3);
        assert_eq!(summary.remaining_count, 7);
        assert_eq!(summary.scan_duration_ms, 1234);
    }
}
