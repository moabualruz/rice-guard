//! Wave 0 test stubs for EVID-01 through EVID-07.
//!
//! All tests are `#[ignore]` — they fail to compile until the implementation
//! crates provide the new type hierarchy (EvidenceBlock, FixMetadata with
//! auto_fixable, VerificationInfo). Wave 1 plans un-ignore tests as they
//! implement each requirement.

#[cfg(test)]
mod evidence_tests {
    use rice_guard_core::issue::{EvidenceBlock, Issue};

    // ── EVID-01: evidence.matched_code, context_before, context_after present ──

    /// EVID-01 — every issue includes evidence.matched_code (may be empty string,
    /// but the field must exist and be populated for scanner-provided findings).
    #[test]
    #[ignore = "Wave 0 stub: implement EvidenceExtractor in Plan 03-02"]
    fn evidence_matched_code_present() {
        let block = EvidenceBlock {
            matched_code: "eval(user_input)".to_string(),
            context_before: vec!["def handler(req):".to_string()],
            context_after: vec!["    return result".to_string()],
            enclosing_function: None,
            enclosing_class: None,
            imports: vec![],
        };
        assert!(
            !block.matched_code.is_empty(),
            "matched_code must be non-empty"
        );
    }

    /// EVID-01 — context_before and context_after are Vec<String> (may be empty
    /// when there are no lines before/after, but the fields must exist).
    #[test]
    #[ignore = "Wave 0 stub: implement EvidenceExtractor in Plan 03-02"]
    fn evidence_context_before_after_present() {
        let block = EvidenceBlock {
            matched_code: "x = eval(y)".to_string(),
            context_before: vec!["# line before".to_string()],
            context_after: vec!["# line after".to_string()],
            enclosing_function: None,
            enclosing_class: None,
            imports: vec![],
        };
        assert_eq!(block.context_before.len(), 1);
        assert_eq!(block.context_after.len(), 1);
    }

    // ── EVID-02: enclosing_function and enclosing_class extracted via tree-sitter ──

    /// EVID-02 — enclosing_function is Some(name) when the finding is inside a
    /// function, None when at module level.
    #[test]
    #[ignore = "Wave 0 stub: implement tree-sitter AST walking in Plan 03-02"]
    fn enclosing_function_extracted() {
        let block = EvidenceBlock {
            matched_code: "eval(x)".to_string(),
            context_before: vec![],
            context_after: vec![],
            enclosing_function: Some("process_request".to_string()),
            enclosing_class: None,
            imports: vec![],
        };
        assert_eq!(block.enclosing_function.as_deref(), Some("process_request"));
    }

    /// EVID-02 — enclosing_class is Some(name) when inside a class/struct.
    #[test]
    #[ignore = "Wave 0 stub: implement tree-sitter AST walking in Plan 03-02"]
    fn enclosing_class_extracted() {
        let block = EvidenceBlock {
            matched_code: "self.db.execute(query)".to_string(),
            context_before: vec![],
            context_after: vec![],
            enclosing_function: Some("query".to_string()),
            enclosing_class: Some("UserRepository".to_string()),
            imports: vec![],
        };
        assert_eq!(block.enclosing_class.as_deref(), Some("UserRepository"));
    }

    // ── EVID-03: evidence.imports extracted via tree-sitter ──────────────────

    /// EVID-03 — imports contains file-level import statements extracted via
    /// tree-sitter. May be empty for files with no imports.
    #[test]
    #[ignore = "Wave 0 stub: implement tree-sitter import extraction in Plan 03-02"]
    fn imports_extracted() {
        let block = EvidenceBlock {
            matched_code: "eval(x)".to_string(),
            context_before: vec![],
            context_after: vec![],
            enclosing_function: None,
            enclosing_class: None,
            imports: vec!["import os".to_string(), "import sys".to_string()],
        };
        assert_eq!(block.imports.len(), 2);
        assert!(block.imports.contains(&"import os".to_string()));
    }

    // ── EVID-04: file parsed once per file, not per finding ──────────────────

    /// EVID-04 — EvidenceExtractor groups findings by file so each source file
    /// is parsed exactly once (parse tree reuse across findings in the same file).
    ///
    /// Verified by: passing two findings for the same file and asserting that
    /// the extractor calls the parser exactly once (checked via a parse counter).
    #[test]
    #[ignore = "Wave 0 stub: implement file-grouping in EvidenceExtractor (Plan 03-02)"]
    fn file_parsed_once_not_per_finding() {
        // This test will use a mock or instrumented extractor to count parses.
        // Implementation in Plan 03-02.
        todo!("implement after EvidenceExtractor exists");
    }

    // ── EVID-05: content-hash fingerprinting ─────────────────────────────────

    /// EVID-05 — running the same scan twice produces identical issue IDs.
    #[test]
    #[ignore = "Wave 0 stub: implement content-hash ID in Plan 03-03"]
    fn fingerprint_stable_across_runs() {
        use rice_guard_core::issue::fingerprint;
        let id1 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        let id2 = fingerprint("semgrep.eval", "src/app.py", "eval(x)");
        assert_eq!(id1, id2, "fingerprint must be deterministic");
    }

    /// EVID-05 — a formatter that shifts a line number does NOT change the ID,
    /// because IDs are based on rule_id + path + matched_code, not line number.
    #[test]
    #[ignore = "Wave 0 stub: implement content-hash ID in Plan 03-03"]
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
            cross_file: false,
        };

        assert_eq!(issue.priority_score, 20);
        assert!(!issue.cross_file);
    }
}
