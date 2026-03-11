//! Wave 0 test stubs for EVID-08 through EVID-12.
//!
//! All tests that depend on unimplemented functionality are `#[ignore]`.
//! Compile-time structural checks run immediately.

#[cfg(test)]
mod issue_tests {
    // ── EVID-06: fix.auto_fixable and fix.auto_fix_tool present ──────────────

    /// EVID-06 — fix.auto_fixable is true when a deterministic fixer can apply.
    #[test]
    #[ignore = "Wave 0 stub: implement FixMetadata.auto_fixable population in Plan 03-03"]
    fn fix_meta_auto_fixable_present() {
        use rice_guard_core::issue::FixMetadata;
        // Will test that IssueBuilder populates auto_fixable correctly.
        let _ = FixMetadata {
            auto_fixable: true,
            auto_fix_tool: Some("ruff check --fix".to_string()),
            auto_fix_category: Some("linter".to_string()),
            suggested_replacement: None,
            complexity: rice_guard_core::issue::FixComplexity::Trivial,
        };
        todo!("verify auto_fixable populated from fixer descriptor coverage");
    }

    /// EVID-06 — fix.auto_fix_tool is Some when auto_fixable, None otherwise.
    #[test]
    #[ignore = "Wave 0 stub: implement FixMetadata.auto_fix_tool population in Plan 03-03"]
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
    #[ignore = "Wave 0 stub: implement VerificationInfo population in Plan 03-03"]
    fn verification_rerun_command_present() {
        use rice_guard_core::issue::VerificationInfo;
        let info = VerificationInfo {
            rerun_command: "rice-guard scan . --security".to_string(),
            success_condition: "exit 0 with no findings".to_string(),
        };
        assert!(
            !info.rerun_command.is_empty(),
            "rerun_command must not be empty"
        );
    }

    // ── EVID-08: WSJF priority scoring ───────────────────────────────────────

    /// EVID-08 — WSJF score computation: severity(40) + auto_fixable(20) +
    /// category(20) + file_freq(10) - cross_file(10).
    #[test]
    #[ignore = "Wave 0 stub: implement wsjf_score() function in Plan 03-03"]
    fn wsjf_score_correct() {
        use rice_guard_core::issue::wsjf_score;
        // error + auto_fixable + formatter + freq=5 + not cross_file
        // = 40 + 20 + 20 + 5 - 0 = 85
        let score = wsjf_score("error", true, Some("formatter"), 5, false);
        assert_eq!(score, 85, "WSJF score must match formula");
    }

    /// EVID-08 — issues.json must be sorted highest WSJF score first.
    #[test]
    #[ignore = "Wave 0 stub: implement WSJF sort in OutputWriter (Plan 03-04)"]
    fn wsjf_sort_order_highest_first() {
        todo!("verify issues.json first entry has highest priority_score");
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
