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
            priority_tier: "medium".to_string(),
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

    fn make_test_issue(
        id: &str,
        auto_fixable: bool,
        file_path: &str,
    ) -> rice_guard_core::issue::Issue {
        use rice_guard_core::issue::{EvidenceBlock, FixComplexity, FixMetadata, VerificationInfo};
        rice_guard_core::issue::Issue {
            id: id.to_string(),
            rule_id: "test-rule".to_string(),
            severity: "warning".to_string(),
            file_path: file_path.to_string(),
            line: 1,
            message: format!("Test message for {id}"),
            scanner: "semgrep".to_string(),
            evidence: EvidenceBlock {
                matched_code: "let x = 1;".to_string(),
                context_before: vec!["// before".to_string()],
                context_after: vec!["// after".to_string()],
                enclosing_function: Some("main".to_string()),
                enclosing_class: None,
                imports: vec![],
            },
            fix: FixMetadata {
                auto_fixable,
                auto_fix_tool: if auto_fixable {
                    Some("cargo clippy --fix".to_string())
                } else {
                    None
                },
                auto_fix_category: if auto_fixable {
                    Some("linter".to_string())
                } else {
                    None
                },
                suggested_replacement: None,
                complexity: FixComplexity::Trivial,
            },
            verification: VerificationInfo {
                rerun_command: "rice-guard scan .".to_string(),
                success_condition: "no findings".to_string(),
            },
            priority_score: if auto_fixable { 60 } else { 20 },
            priority_tier: if auto_fixable {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            cross_file: false,
        }
    }

    fn make_output_writer(dir: &std::path::Path) -> rice_guard_core::output::OutputWriter {
        rice_guard_core::output::OutputWriter::new(dir)
    }

    fn make_summary(
        issues: &[rice_guard_core::issue::Issue],
    ) -> rice_guard_core::output::ScanSummary {
        rice_guard_core::output::ScanSummary::from_issues(
            issues,
            vec!["semgrep".to_string()],
            "/test/project",
            500,
        )
    }

    /// EVID-09 — OutputWriter produces issues.json, issues-fixable.json,
    /// issues-remaining.json, summary.json, summary.txt in the output directory.
    #[test]
    fn output_three_files_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let issues = vec![
            make_test_issue("issue-1", true, "src/main.rs"),
            make_test_issue("issue-2", true, "src/lib.rs"),
            make_test_issue("issue-3", false, "src/utils.rs"),
        ];
        let summary = make_summary(&issues);
        let writer = make_output_writer(dir.path());
        writer.write_all(&issues, &summary).expect("write_all");

        let base = dir.path();
        assert!(base.join("issues.json").exists(), "issues.json must exist");
        assert!(
            base.join("issues-fixable.json").exists(),
            "issues-fixable.json must exist"
        );
        assert!(
            base.join("issues-remaining.json").exists(),
            "issues-remaining.json must exist"
        );
        assert!(
            base.join("summary.json").exists(),
            "summary.json must exist"
        );
        assert!(base.join("summary.txt").exists(), "summary.txt must exist");
    }

    /// EVID-09 — issues-fixable.json contains only auto_fixable=true issues;
    /// issues-remaining.json contains only auto_fixable=false issues.
    #[test]
    fn fixable_remaining_split_correct() {
        let dir = tempfile::tempdir().expect("tempdir");
        let issues = vec![
            make_test_issue("fix-1", true, "src/a.rs"),
            make_test_issue("fix-2", true, "src/b.rs"),
            make_test_issue("rem-1", false, "src/c.rs"),
        ];
        let summary = make_summary(&issues);
        let writer = make_output_writer(dir.path());
        writer.write_all(&issues, &summary).expect("write_all");

        let fixable_bytes =
            std::fs::read(dir.path().join("issues-fixable.json")).expect("read fixable");
        let fixable: Vec<rice_guard_core::issue::Issue> =
            serde_json::from_slice(&fixable_bytes).expect("parse fixable");
        assert_eq!(fixable.len(), 2, "fixable.json must contain 2 issues");
        assert!(
            fixable.iter().all(|i| i.fix.auto_fixable),
            "all issues in fixable must have auto_fixable=true"
        );

        let remaining_bytes =
            std::fs::read(dir.path().join("issues-remaining.json")).expect("read remaining");
        let remaining: Vec<rice_guard_core::issue::Issue> =
            serde_json::from_slice(&remaining_bytes).expect("parse remaining");
        assert_eq!(remaining.len(), 1, "remaining.json must contain 1 issue");
        assert!(
            remaining.iter().all(|i| !i.fix.auto_fixable),
            "all issues in remaining must have auto_fixable=false"
        );
    }

    // ── EVID-10: summary.json counts match ───────────────────────────────────

    /// EVID-10 — summary.json total_issues matches the actual issue count.
    #[test]
    fn summary_json_counts_match() {
        let dir = tempfile::tempdir().expect("tempdir");
        let issues = vec![
            make_test_issue("issue-1", true, "src/a.rs"),
            make_test_issue("issue-2", true, "src/b.rs"),
            make_test_issue("issue-3", false, "src/c.rs"),
        ];
        let summary = make_summary(&issues);
        let writer = make_output_writer(dir.path());
        writer.write_all(&issues, &summary).expect("write_all");

        let summary_bytes = std::fs::read(dir.path().join("summary.json")).expect("read summary");
        let parsed: rice_guard_core::output::ScanSummary =
            serde_json::from_slice(&summary_bytes).expect("parse summary");
        assert_eq!(parsed.total_issues, 3, "total_issues must be 3");
        assert_eq!(parsed.fixable_count, 2, "fixable_count must be 2");
        assert_eq!(parsed.remaining_count, 1, "remaining_count must be 1");
    }

    // ── EVID-11: summary.txt is ASCII table ───────────────────────────────────

    /// EVID-11 — summary.txt uses only ASCII characters (no Unicode box-drawing).
    #[test]
    fn summary_txt_ascii_table() {
        let dir = tempfile::tempdir().expect("tempdir");
        let issues = vec![
            make_test_issue("issue-1", true, "src/main.rs"),
            make_test_issue("issue-2", false, "src/lib.rs"),
        ];
        let summary = make_summary(&issues);
        let writer = make_output_writer(dir.path());
        writer.write_all(&issues, &summary).expect("write_all");

        let txt = std::fs::read(dir.path().join("summary.txt")).expect("read summary.txt");
        assert!(txt.is_ascii(), "summary.txt must contain only ASCII bytes");
        let content = String::from_utf8(txt).expect("valid utf8");
        assert!(
            content.contains('+'),
            "summary.txt must contain '+' borders"
        );
        assert!(
            content.contains('|'),
            "summary.txt must contain '|' separators"
        );
        assert!(
            content.contains('-'),
            "summary.txt must contain '-' borders"
        );
    }

    // ── EVID-12: forward-slash paths ─────────────────────────────────────────

    /// EVID-12 — all file_path values in output use forward slashes,
    /// even on Windows where the OS separator is backslash.
    #[test]
    fn path_forward_slashes_in_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let issues = vec![make_test_issue(
            "issue-1",
            true,
            r"src\foo\bar.py", // Windows-style backslash path
        )];
        let summary = make_summary(&issues);
        let writer = make_output_writer(dir.path());
        writer.write_all(&issues, &summary).expect("write_all");

        let bytes = std::fs::read(dir.path().join("issues.json")).expect("read issues.json");
        let content = String::from_utf8(bytes).expect("valid utf8");
        assert!(
            !content.contains('\\'),
            "issues.json must not contain backslashes in file_path: {}",
            content
        );
        assert!(
            content.contains("src/foo/bar.py"),
            "issues.json must contain forward-slash path"
        );
    }

    // ── EVID-08: priority_tier field ─────────────────────────────────────────

    /// EVID-08 — IssueBuilder::build_with_evidence() populates priority_tier
    /// with a non-empty string matching one of the four tier labels.
    #[test]
    fn priority_tier_present() {
        use rice_guard_core::issue::IssueBuilder;
        use rice_guard_core::scanner::parser::RawFinding;
        use std::path::Path;

        let finding = RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: "test-rule".to_string(),
            severity: "error".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: 1,
            message: "test".to_string(),
            matched_code: None,
            suggested_replacement: None,
        };
        let issue = IssueBuilder::build(&finding, Path::new("."), 5);
        let valid_tiers = ["critical", "high", "medium", "low"];
        assert!(
            valid_tiers.contains(&issue.priority_tier.as_str()),
            "priority_tier must be one of {:?}, got {:?}",
            valid_tiers,
            issue.priority_tier,
        );
        assert!(
            !issue.priority_tier.is_empty(),
            "priority_tier must not be empty"
        );
    }

    // ── EVID-10: fix_queue_by_category in summary ─────────────────────────────

    /// EVID-10 — ScanSummary::from_issues() populates fix_queue_by_category
    /// with correct stage keys when issues have auto_fix_category set.
    #[test]
    fn summary_json_has_fix_queue_by_category() {
        use rice_guard_core::output::ScanSummary;
        use std::collections::HashMap;

        // Build issues with known categories.
        let issues = vec![
            make_test_issue("fix-a", true, "src/a.rs"),
            make_test_issue("fix-b", true, "src/b.rs"),
            make_test_issue("rem-1", false, "src/c.rs"),
        ];
        // make_test_issue sets auto_fix_category = "linter" for auto_fixable issues.
        let summary = ScanSummary::from_issues(&issues, vec![], "/project", 100);

        // fix_queue_by_category must be a HashMap (compile-time + runtime check).
        let _: &HashMap<String, usize> = &summary.fix_queue_by_category;

        // Two auto_fixable issues with category "linter" -> stage "linters".
        assert_eq!(
            summary
                .fix_queue_by_category
                .get("linters")
                .copied()
                .unwrap_or(0),
            2,
            "fix_queue_by_category['linters'] must be 2"
        );

        // Non-fixable issue must not appear.
        let total: usize = summary.fix_queue_by_category.values().sum();
        assert_eq!(
            total, 2,
            "only auto_fixable issues counted in fix_queue_by_category"
        );
    }

    // ── EVID-05: Descriptor-driven auto_fixable ───────────────────────────────

    /// EVID-05 — auto_fixable is true when a fixer descriptor exists for the
    /// file's language (descriptor-driven lookup).
    #[test]
    fn descriptor_driven_auto_fixable_python() {
        use rice_guard_core::issue::{FixMetadata, FixerDescriptorInfo};
        let descriptors = vec![FixerDescriptorInfo {
            language: "python".to_string(),
            stages: vec!["linters".to_string()],
            fix_tool: Some("ruff check --fix .".to_string()),
        }];
        let meta = FixMetadata::from_finding_with_descriptors(
            "semgrep",
            "python.security.eval",
            None,
            "py",
            &descriptors,
        );
        assert!(
            meta.auto_fixable,
            "auto_fixable must be true when python fixer descriptor exists"
        );
        assert_eq!(
            meta.auto_fix_tool.as_deref(),
            Some("ruff check --fix ."),
            "auto_fix_tool must match descriptor fix_tool"
        );
    }

    /// EVID-05 — auto_fixable is false when no descriptor matches the extension.
    #[test]
    fn descriptor_driven_auto_fixable_no_descriptor() {
        use rice_guard_core::issue::{FixMetadata, FixerDescriptorInfo};
        let descriptors = vec![FixerDescriptorInfo {
            language: "python".to_string(),
            stages: vec!["linters".to_string()],
            fix_tool: Some("ruff check --fix .".to_string()),
        }];
        let meta = FixMetadata::from_finding_with_descriptors(
            "semgrep",
            "some-rule",
            None,
            "xyz",
            &descriptors,
        );
        assert!(
            !meta.auto_fixable,
            "auto_fixable must be false when no descriptor matches .xyz extension"
        );
    }

    /// EVID-05 — IssueBuilder::build_batch() sets cross_file=true when same
    /// rule_id appears in 3+ distinct files.
    #[test]
    fn cross_file_penalty_triggers_at_3_files() {
        use rice_guard_core::issue::{FixerDescriptorInfo, IssueBuilder};
        use rice_guard_core::scanner::parser::RawFinding;

        let make = |file: &str| RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: "python.security.eval".to_string(),
            severity: "warning".to_string(),
            file_path: file.to_string(),
            line: 1,
            message: "eval usage".to_string(),
            matched_code: None,
            suggested_replacement: None,
        };

        let findings = vec![make("src/a.py"), make("src/b.py"), make("src/c.py")];
        let descriptors: Vec<FixerDescriptorInfo> = vec![];
        let issues = IssueBuilder::build_batch(&findings, std::path::Path::new("."), &descriptors);
        assert_eq!(issues.len(), 3);
        assert!(
            issues.iter().all(|i| i.cross_file),
            "all issues must have cross_file=true when rule in 3+ files"
        );
    }

    /// EVID-05 — cross_file remains false when rule appears in fewer than 3 files.
    #[test]
    fn cross_file_no_penalty_below_threshold() {
        use rice_guard_core::issue::{FixerDescriptorInfo, IssueBuilder};
        use rice_guard_core::scanner::parser::RawFinding;

        let make = |file: &str| RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: "python.security.eval".to_string(),
            severity: "warning".to_string(),
            file_path: file.to_string(),
            line: 1,
            message: "eval usage".to_string(),
            matched_code: None,
            suggested_replacement: None,
        };

        let findings = vec![make("src/a.py"), make("src/b.py")];
        let descriptors: Vec<FixerDescriptorInfo> = vec![];
        let issues = IssueBuilder::build_batch(&findings, std::path::Path::new("."), &descriptors);
        assert_eq!(issues.len(), 2);
        assert!(
            issues.iter().all(|i| !i.cross_file),
            "cross_file must be false when rule in < 3 files"
        );
    }

    /// EVID-05 — build_batch() returns one issue per finding.
    #[test]
    fn build_batch_returns_all_issues() {
        use rice_guard_core::issue::{FixerDescriptorInfo, IssueBuilder};
        use rice_guard_core::scanner::parser::RawFinding;

        let findings: Vec<RawFinding> = (0..5)
            .map(|i| RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: format!("rule-{i}"),
                severity: "warning".to_string(),
                file_path: format!("src/file{i}.py"),
                line: 1,
                message: "test".to_string(),
                matched_code: None,
                suggested_replacement: None,
            })
            .collect();

        let descriptors: Vec<FixerDescriptorInfo> = vec![];
        let issues = IssueBuilder::build_batch(&findings, std::path::Path::new("."), &descriptors);
        assert_eq!(
            issues.len(),
            5,
            "build_batch must return one issue per finding"
        );
    }

    /// EVID-05 — all issues from build_batch() have a non-empty priority_tier.
    #[test]
    fn priority_tier_set_by_build_batch() {
        use rice_guard_core::issue::{FixerDescriptorInfo, IssueBuilder};
        use rice_guard_core::scanner::parser::RawFinding;

        let findings: Vec<RawFinding> = (0..3)
            .map(|i| RawFinding {
                scanner: "clippy".to_string(),
                rule_id: format!("clippy::rule-{i}"),
                severity: "error".to_string(),
                file_path: format!("src/lib{i}.rs"),
                line: 1,
                message: "test".to_string(),
                matched_code: None,
                suggested_replacement: None,
            })
            .collect();

        let descriptors: Vec<FixerDescriptorInfo> = vec![];
        let issues = IssueBuilder::build_batch(&findings, std::path::Path::new("."), &descriptors);
        let valid_tiers = ["critical", "high", "medium", "low"];
        for issue in &issues {
            assert!(
                valid_tiers.contains(&issue.priority_tier.as_str()),
                "priority_tier must be a valid tier, got {:?}",
                issue.priority_tier
            );
            assert!(
                !issue.priority_tier.is_empty(),
                "priority_tier must not be empty"
            );
        }
    }

    // ── EVID-05: Git churn combined file_freq ────────────────────────────────

    /// EVID-05 — file_freq_with_churn() does not panic in a non-git dir.
    #[test]
    fn git_churn_graceful_fallback() {
        use rice_guard_core::issue::file_freq_with_churn;
        use rice_guard_core::scanner::parser::RawFinding;

        let findings = vec![RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: "rule".to_string(),
            severity: "warning".to_string(),
            file_path: "src/app.py".to_string(),
            line: 1,
            message: "test".to_string(),
            matched_code: None,
            suggested_replacement: None,
        }];
        // Use a temp dir that is NOT a git repo.
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = file_freq_with_churn(&findings, tmp.path());
        assert!(
            result.contains_key("src/app.py"),
            "map must contain the file"
        );
        // Score must be ≥ 1 (issue count contribution) and ≤ 10.
        let score = result["src/app.py"];
        assert!(score >= 1 && score <= 10, "score out of range: {score}");
    }

    /// EVID-05 — combined score is capped at 10 even with high issue count and churn.
    #[test]
    fn git_churn_combined_score_capped_at_10() {
        use rice_guard_core::issue::file_freq_with_churn;
        use rice_guard_core::scanner::parser::RawFinding;

        // 20 findings in the same file → issue_norm = min(20, 5) = 5.
        // Even if churn is very large, score must be capped at 10.
        let findings: Vec<RawFinding> = (0..20)
            .map(|i| RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: format!("rule-{i}"),
                severity: "warning".to_string(),
                file_path: "src/hotspot.py".to_string(),
                line: i + 1,
                message: "test".to_string(),
                matched_code: None,
                suggested_replacement: None,
            })
            .collect();
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = file_freq_with_churn(&findings, tmp.path());
        let score = result["src/hotspot.py"];
        assert!(score <= 10, "score must be capped at 10, got {score}");
        assert!(score >= 1, "score must be positive, got {score}");
    }

    /// EVID-05 — file with 3 findings in a non-git dir returns score in 1..=5.
    #[test]
    fn file_freq_with_churn_basic() {
        use rice_guard_core::issue::file_freq_with_churn;
        use rice_guard_core::scanner::parser::RawFinding;

        let findings: Vec<RawFinding> = (0..3)
            .map(|i| RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: format!("rule-{i}"),
                severity: "warning".to_string(),
                file_path: "src/app.py".to_string(),
                line: i + 1,
                message: "test".to_string(),
                matched_code: None,
                suggested_replacement: None,
            })
            .collect();
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = file_freq_with_churn(&findings, tmp.path());
        let score = result["src/app.py"];
        assert!(
            (1..=5).contains(&score),
            "3 findings in non-git dir must score between 1 and 5, got {score}"
        );
    }

    /// EVID-05 — existing file_freq_map() still works correctly.
    #[test]
    fn file_freq_map_still_works() {
        use rice_guard_core::issue::file_freq_map;
        use rice_guard_core::scanner::parser::RawFinding;

        let findings = vec![
            RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: "r".to_string(),
                severity: "warning".to_string(),
                file_path: "src/a.py".to_string(),
                line: 1,
                message: "t".to_string(),
                matched_code: None,
                suggested_replacement: None,
            },
            RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: "r".to_string(),
                severity: "warning".to_string(),
                file_path: "src/a.py".to_string(),
                line: 2,
                message: "t".to_string(),
                matched_code: None,
                suggested_replacement: None,
            },
            RawFinding {
                scanner: "semgrep".to_string(),
                rule_id: "r".to_string(),
                severity: "warning".to_string(),
                file_path: "src/b.py".to_string(),
                line: 1,
                message: "t".to_string(),
                matched_code: None,
                suggested_replacement: None,
            },
        ];
        let map = file_freq_map(&findings);
        assert_eq!(map["src/a.py"], 2);
        assert_eq!(map["src/b.py"], 1);
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
            fix_queue_by_category: HashMap::from([("linters".to_string(), 3usize)]),
        };

        assert_eq!(summary.total_issues, 10);
        assert_eq!(summary.fixable_count, 3);
        assert_eq!(summary.remaining_count, 7);
        assert_eq!(summary.scan_duration_ms, 1234);
    }
}
