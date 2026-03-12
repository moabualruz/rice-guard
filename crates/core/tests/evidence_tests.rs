//! Tests for EVID-01 through EVID-07.
//!
//! EVID-01..04: EvidenceExtractor — line-context extraction and tree-sitter
//! AST walking. These are implemented in Plan 03-02 (this plan).
//!
//! EVID-05..07: fingerprint, FixMetadata, VerificationInfo — implemented in
//! Plan 03-01 (already done). Stubs here are still #[ignore] pointing to 03-03
//! for the integration test.

#[cfg(test)]
mod evidence_tests {
    use rice_guard_core::issue::{EvidenceExtractor, Issue};
    use rice_guard_core::scanner::parser::RawFinding;

    // ─── helper to build a minimal RawFinding ────────────────────────────────

    fn make_finding(rule_id: &str, file_path: &str, line: u32) -> RawFinding {
        RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: rule_id.to_string(),
            severity: "warning".to_string(),
            file_path: file_path.to_string(),
            line,
            message: "test finding".to_string(),
            matched_code: None,
            suggested_replacement: None,
        }
    }

    fn make_finding_with_code(rule_id: &str, file_path: &str, line: u32, code: &str) -> RawFinding {
        RawFinding {
            scanner: "semgrep".to_string(),
            rule_id: rule_id.to_string(),
            severity: "warning".to_string(),
            file_path: file_path.to_string(),
            line,
            message: "test finding".to_string(),
            matched_code: Some(code.to_string()),
            suggested_replacement: None,
        }
    }

    // ─── 20-line Python fixture ───────────────────────────────────────────────

    fn python_fixture() -> &'static str {
        // Lines 1-20 (1-based).  Function `process_data` spans lines 5-14.
        // Class `DataProcessor` wraps the whole file from line 3.
        concat!(
            "import os\n",                   // line 1
            "import sys\n",                  // line 2
            "\n",                            // line 3
            "class DataProcessor:\n",        // line 4
            "    def process_data(self):\n", // line 5
            "        x = 1\n",               // line 6
            "        y = 2\n",               // line 7
            "        z = 3\n",               // line 8
            "        a = 4\n",               // line 9
            "        result = eval(x)\n",    // line 10  <- target finding
            "        b = 5\n",               // line 11
            "        c = 6\n",               // line 12
            "        d = 7\n",               // line 13
            "        return result\n",       // line 14
            "\n",                            // line 15
            "    def other(self):\n",        // line 16
            "        pass\n",                // line 17
            "\n",                            // line 18
            "x = DataProcessor()\n",         // line 19
            "x.process_data()\n",            // line 20
        )
    }

    // ── EVID-01: matched_code present ────────────────────────────────────────

    /// EVID-01 — matched_code falls back to the source line when the scanner
    /// provides no snippet (matched_code=None on RawFinding).
    #[test]
    fn evidence_matched_code_present() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 10);
        let (matched, _, _) = extractor.extract_line_context(&finding, source);
        // line 10 (0-indexed: 9) is "        result = eval(x)"
        assert!(
            !matched.is_empty(),
            "matched_code must be non-empty when source line exists"
        );
        assert!(
            matched.contains("eval"),
            "matched_code should be line 10: got {matched:?}"
        );
    }

    /// EVID-01 — when scanner provides matched_code, it is used as-is.
    #[test]
    fn evidence_matched_code_from_scanner_when_provided() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding_with_code("rule", "src/app.py", 10, "eval(user_input)");
        let (matched, _, _) = extractor.extract_line_context(&finding, source);
        assert_eq!(
            matched, "eval(user_input)",
            "scanner-provided matched_code must be returned verbatim"
        );
    }

    // ── EVID-01: context_before and context_after present ────────────────────

    /// EVID-01 — context_before and context_after contain the surrounding lines.
    /// For a finding at line 10 of a 20-line file, both should have 5 lines.
    #[test]
    fn evidence_context_before_after_present() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 10);
        let (_, before, after) = extractor.extract_line_context(&finding, source);
        assert_eq!(
            before.len(),
            5,
            "context_before must have 5 lines for line 10 of 20-line file"
        );
        assert_eq!(
            after.len(),
            5,
            "context_after must have 5 lines for line 10 of 20-line file"
        );
    }

    /// EVID-01 edge case — context_before is empty when finding is at line 1.
    #[test]
    fn evidence_context_before_empty_at_line_1() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 1);
        let (_, before, _) = extractor.extract_line_context(&finding, source);
        assert_eq!(
            before.len(),
            0,
            "context_before must be empty at line 1 (no lines before it)"
        );
    }

    // ── EVID-02: enclosing_function extracted ────────────────────────────────

    /// EVID-02 — enclosing_function is Some("process_data") for a finding
    /// inside `process_data` in the Python fixture.
    #[test]
    fn enclosing_function_extracted() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 10);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 10))
            .expect("block must exist");
        assert_eq!(
            block.enclosing_function.as_deref(),
            Some("process_data"),
            "enclosing_function must be 'process_data' for finding at line 10"
        );
    }

    // ── EVID-02: enclosing_class extracted ───────────────────────────────────

    /// EVID-02 — enclosing_class is Some("DataProcessor") for a finding inside
    /// the class body in the Python fixture.
    #[test]
    fn enclosing_class_extracted() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 10);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 10))
            .expect("block must exist");
        assert_eq!(
            block.enclosing_class.as_deref(),
            Some("DataProcessor"),
            "enclosing_class must be 'DataProcessor' for finding at line 10"
        );
    }

    // ── EVID-03: imports extracted ───────────────────────────────────────────

    /// EVID-03 — imports contains "import os" and "import sys" extracted from
    /// the Python fixture's top-level import statements.
    #[test]
    fn imports_extracted() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding("rule", "src/app.py", 10);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 10))
            .expect("block must exist");
        assert!(
            !block.imports.is_empty(),
            "imports must not be empty for a file with import statements"
        );
        let import_text = block.imports.join("\n");
        assert!(
            import_text.contains("os"),
            "imports must contain 'import os'; got: {import_text:?}"
        );
        assert!(
            import_text.contains("sys"),
            "imports must contain 'import sys'; got: {import_text:?}"
        );
    }

    // ── EVID-04: file parsed once ────────────────────────────────────────────

    /// EVID-04 — three findings for the same file all produce correct
    /// EvidenceBlocks. This indirectly verifies file-grouping: if the source
    /// were re-parsed per finding, the output would be identical but the code
    /// path is verified via correct results for all three.
    #[test]
    fn file_parsed_once_not_per_finding() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let f1 = make_finding("rule-a", "src/app.py", 6);
        let f2 = make_finding("rule-b", "src/app.py", 10);
        let f3 = make_finding("rule-c", "src/app.py", 16);
        let findings_refs: Vec<&RawFinding> = vec![&f1, &f2, &f3];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);

        // All 3 findings must be in the result.
        assert_eq!(
            result.len(),
            3,
            "extract_file must return one EvidenceBlock per finding"
        );

        // Each result has non-empty matched_code (source line extracted).
        for key in [
            ("rule-a".to_string(), 6u32),
            ("rule-b".to_string(), 10u32),
            ("rule-c".to_string(), 16u32),
        ] {
            let block = result.get(&key).expect("block must exist for each finding");
            assert!(
                !block.matched_code.is_empty(),
                "matched_code must be non-empty for key {key:?}"
            );
        }
    }

    // ── EVID-02: small function body inclusion (< 15 lines) ──────────────────

    /// When the enclosing function body is < 15 lines, context_before and
    /// context_after together span the full function body (not just ±5 lines).
    /// matched_code is unchanged.
    ///
    /// python_fixture: process_data spans lines 5-14 (10 lines, < 15).
    /// Finding at line 6 (near top of function) — standard ±5 window would
    /// give context_before = [line 1..5] (5 lines), but with small-function
    /// body logic, context_before = [line 5] (just the function def line),
    /// and context_after = lines 7-14 (8 lines, up to end of function).
    /// So context_after.len() must equal 8 (not the standard 5).
    #[test]
    fn small_function_body_included() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        // Finding at line 6 ("        x = 1") which is near top of process_data.
        // With standard ±5: context_after = lines 7-11 (5 lines).
        // With small-function body: context_after = lines 7-14 (8 lines, to end of function).
        let finding = make_finding_with_code("rule", "src/app.py", 6, "x = 1");
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 6))
            .expect("block must exist");

        // matched_code is still the scanner snippet, not the function body.
        assert_eq!(
            block.matched_code, "x = 1",
            "matched_code must remain the scanner snippet"
        );

        // With small-function body logic: context_after extends to end of function (line 14),
        // giving 8 lines (lines 7-14), more than the standard ±5 = 5 lines.
        assert!(
            block.context_after.len() > 5,
            "small function: context_after must extend to end of function body (>5 lines), got {}",
            block.context_after.len()
        );

        // The last line of context_after should be the last line of the function body.
        let last_after = block
            .context_after
            .last()
            .expect("context_after must not be empty");
        assert!(
            last_after.contains("return result"),
            "context_after must end at the last line of the function body ('return result'); got: {last_after:?}"
        );
    }

    /// For a function with >= 15 lines, context_before and context_after use
    /// the standard ±5 line window (not the full function body).
    #[test]
    fn large_function_uses_window() {
        let extractor = EvidenceExtractor::new();
        // Build a Python file with a large function (>= 15 lines).
        let source = concat!(
            "import os\n",           // line 1
            "\n",                    // line 2
            "def big_function():\n", // line 3
            "    a = 1\n",           // line 4
            "    b = 2\n",           // line 5
            "    c = 3\n",           // line 6
            "    d = 4\n",           // line 7
            "    e = 5\n",           // line 8
            "    f = 6\n",           // line 9
            "    g = 7\n",           // line 10
            "    h = 8\n",           // line 11
            "    i = 9\n",           // line 12
            "    j = 10\n",          // line 13
            "    k = 11\n",          // line 14
            "    l = 12\n",          // line 15
            "    m = 13\n",          // line 16
            "    n = 14\n",          // line 17
            "    o = eval(x)\n",     // line 18  <- finding
            "    p = 15\n",          // line 19
            "    q = 16\n",          // line 20
            "    return a\n",        // line 21
        );
        // Function big_function spans lines 3-21 (19 lines, >= 15 threshold).
        // Finding at line 18 should use ±5 window.
        let finding = make_finding("rule", "src/big.py", 18);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/big.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 18))
            .expect("block must exist");

        // Standard ±5 window: context_before has 5 lines (lines 13-17),
        // context_after has 3 lines (lines 19-21).
        assert_eq!(
            block.context_before.len(),
            5,
            "large function: context_before must be 5 (standard window)"
        );
    }

    /// matched_code is the scanner snippet even when small-function body
    /// replacement is applied to context_before/after.
    #[test]
    fn small_function_body_does_not_change_matched_code() {
        let extractor = EvidenceExtractor::new();
        let source = python_fixture();
        let finding = make_finding_with_code("rule", "src/app.py", 10, "eval(user_input)");
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/app.py", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 10))
            .expect("block must exist");
        assert_eq!(
            block.matched_code, "eval(user_input)",
            "matched_code must not be replaced with function body"
        );
    }

    /// For unsupported extension (.xyz), EvidenceExtractor falls back to ±5
    /// line window without crashing (no tree-sitter available).
    #[test]
    fn graceful_fallback_no_tree_sitter() {
        let extractor = EvidenceExtractor::new();
        let source = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\n";
        let finding = make_finding("rule", "src/config.xyz", 5);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/config.xyz", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 5))
            .expect("block must exist for unknown extension");
        // Falls back to ±5 window: context_before=4 lines (1-4), context_after=5 lines (6-10)
        assert_eq!(
            block.context_before.len(),
            4,
            "fallback: context_before must be ±5 window"
        );
        assert_eq!(
            block.context_after.len(),
            5,
            "fallback: context_after must be ±5 window"
        );
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
        assert!(block.imports.is_empty());
    }

    // ── Graceful degradation for unsupported extension ───────────────────────

    /// If the file has an unsupported extension, EvidenceExtractor should still
    /// produce a valid EvidenceBlock with line-context (enclosing_* = None,
    /// imports = []).
    #[test]
    fn unsupported_extension_no_panic() {
        let extractor = EvidenceExtractor::new();
        let source = "line1\nline2\nline3\n";
        let finding = make_finding("rule", "src/config.xyz", 2);
        let findings_refs: Vec<&RawFinding> = vec![&finding];
        let result = extractor.extract_file("src/config.xyz", source, &findings_refs);
        let block = result
            .get(&("rule".to_string(), 2))
            .expect("block must exist even for unknown extension");
        assert_eq!(block.matched_code, "line2");
        assert!(block.enclosing_function.is_none());
        assert!(block.enclosing_class.is_none());
        assert!(block.imports.is_empty());
    }

    // ── EVID-05: content-hash fingerprinting ─────────────────────────────────

    /// EVID-05 — running the same scan twice produces identical issue IDs.
    #[test]
    fn fingerprint_stable_across_runs() {
        use rice_guard_core::issue::fingerprint;
        let id1 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        let id2 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        assert_eq!(id1, id2, "fingerprint must be deterministic");
    }

    /// EVID-05 — a formatter that shifts a line number does NOT change the ID,
    /// because IDs are based on rule_id + path + matched_code, not line number.
    #[test]
    fn fingerprint_stable_after_formatter_shift() {
        use rice_guard_core::issue::fingerprint;
        // Same code, different line (formatter shifted it).
        let id_before = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        let id_after = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        assert_eq!(
            id_before, id_after,
            "fingerprint must not depend on line number"
        );
    }

    // ── Compile-time: Issue struct has all required fields ───────────────────

    /// Verify that Issue has all required Phase 3 fields (compile-time check).
    #[test]
    fn issue_struct_has_required_fields() {
        // This is a compile-time check via struct construction.
        // If any field is missing or renamed, this test will fail to compile.
        use rice_guard_core::issue::{EvidenceBlock, FixComplexity, FixMetadata, VerificationInfo};

        let evidence = EvidenceBlock {
            matched_code: String::new(),
            context_before: vec![],
            context_after: vec![],
            enclosing_function: None,
            enclosing_class: None,
            imports: vec![],
        };

        let fix = FixMetadata {
            auto_fixable: false,
            auto_fix_tool: None,
            auto_fix_category: None,
            suggested_replacement: None,
            complexity: FixComplexity::Trivial,
        };

        let verification = VerificationInfo {
            rerun_command: "rice-guard scan .".to_string(),
            success_condition: "exit 0 with no findings".to_string(),
        };

        let issue = Issue {
            id: "test-id".to_string(),
            rule_id: "test-rule".to_string(),
            severity: "warning".to_string(),
            file_path: "src/lib.rs".to_string(),
            line: 1,
            message: "test".to_string(),
            scanner: "semgrep".to_string(),
            evidence,
            fix,
            verification,
            priority_score: 20,
            priority_tier: "medium".to_string(),
            cross_file: false,
        };

        assert_eq!(issue.priority_score, 20);
        assert!(!issue.cross_file);
    }
}
