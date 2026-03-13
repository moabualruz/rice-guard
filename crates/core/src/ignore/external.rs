use crate::config::model::RiceGuardConfig;

/// Per-tool exclude flag arguments derived from active ignore patterns.
///
/// Maps gitignore-style patterns to the native CLI exclude flags for each
/// external scanner tool.
///
/// # Gitleaks note
/// Gitleaks uses fingerprint-based ignoring (`.gitleaksignore`), not path-based
/// CLI flags. Path-based exclusion via Gitleaks CLI flags is unreliable.
/// Therefore, `ExcludeArgs` does NOT produce Gitleaks flags. If needed,
/// post-filter Gitleaks results by file path as a future enhancement.
#[derive(Debug, Clone, Default)]
pub struct ExcludeArgs {
    /// `--exclude <pattern>` pairs for semgrep.
    pub semgrep: Vec<String>,
    /// `--skip-dirs <pattern>` args for trivy (no glob wildcards).
    pub trivy: Vec<String>,
    /// `--ignore <pattern>` args for jscpd.
    pub jscpd: Vec<String>,
    /// `--exclude-dir <pattern>` args for scc (no glob wildcards).
    pub scc: Vec<String>,
}

impl ExcludeArgs {
    /// Build `ExcludeArgs` from a slice of gitignore-style patterns.
    ///
    /// Rules:
    /// - Patterns starting with `!` (negations / rescues) are skipped.
    /// - Trailing `/` is stripped before passing to tool flags.
    /// - Patterns containing `*` are excluded from trivy and scc flags
    ///   (those tools don't support glob wildcards in their exclude args).
    pub fn from_patterns(patterns: &[String]) -> Self {
        let mut args = Self::default();

        for pattern in patterns {
            // Skip negation patterns — they are rescues, not excludes.
            if pattern.starts_with('!') {
                continue;
            }

            // Strip trailing slash for flag values.
            let clean = pattern.trim_end_matches('/');
            if clean.is_empty() {
                continue;
            }

            let has_glob = clean.contains('*');

            // Semgrep: accepts all patterns including globs.
            args.semgrep.push("--exclude".to_string());
            args.semgrep.push(clean.to_string());

            // jscpd: accepts all patterns including globs.
            args.jscpd.push("--ignore".to_string());
            args.jscpd.push(clean.to_string());

            // Trivy and scc: only plain directory/file names (no globs).
            if !has_glob {
                args.trivy.push("--skip-dirs".to_string());
                args.trivy.push(clean.to_string());

                args.scc.push("--exclude-dir".to_string());
                args.scc.push(clean.to_string());
            }
        }

        args
    }

    /// Collect the active exclude patterns by merging `FiltersConfig::default_excludes()`
    /// with `config.filters.exclude` (deduped, config values appended after defaults).
    ///
    /// Does NOT include `filters.include` — those are rescues, not excludes.
    pub fn collect_active_patterns(config: &RiceGuardConfig) -> Vec<String> {
        use crate::config::model::FiltersConfig;

        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for pattern in FiltersConfig::default_excludes()
            .into_iter()
            .chain(config.filters.exclude.iter().cloned())
        {
            if seen.insert(pattern.clone()) {
                result.push(pattern);
            }
        }

        result
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::FiltersConfig;

    fn cfg_with_excludes(excludes: Vec<&str>) -> RiceGuardConfig {
        let mut c = RiceGuardConfig::default();
        c.filters.exclude = excludes.into_iter().map(String::from).collect();
        c
    }

    #[test]
    fn from_patterns_empty_returns_empty() {
        let args = ExcludeArgs::from_patterns(&[]);
        assert!(args.semgrep.is_empty());
        assert!(args.trivy.is_empty());
        assert!(args.jscpd.is_empty());
        assert!(args.scc.is_empty());
    }

    #[test]
    fn from_patterns_dir_patterns_produce_correct_flags() {
        let patterns = vec!["vendor/".to_string(), "node_modules/".to_string()];
        let args = ExcludeArgs::from_patterns(&patterns);

        // Semgrep should have 2 pairs: --exclude vendor --exclude node_modules
        assert_eq!(
            args.semgrep,
            vec!["--exclude", "vendor", "--exclude", "node_modules"]
        );

        // Trivy: --skip-dirs vendor --skip-dirs node_modules
        assert_eq!(
            args.trivy,
            vec!["--skip-dirs", "vendor", "--skip-dirs", "node_modules"]
        );

        // jscpd: --ignore vendor --ignore node_modules
        assert_eq!(
            args.jscpd,
            vec!["--ignore", "vendor", "--ignore", "node_modules"]
        );

        // scc: --exclude-dir vendor --exclude-dir node_modules
        assert_eq!(
            args.scc,
            vec!["--exclude-dir", "vendor", "--exclude-dir", "node_modules"]
        );
    }

    #[test]
    fn from_patterns_glob_excluded_from_trivy_and_scc() {
        let patterns = vec!["*.generated.*".to_string()];
        let args = ExcludeArgs::from_patterns(&patterns);

        // Semgrep and jscpd accept globs
        assert!(!args.semgrep.is_empty(), "semgrep should get glob pattern");
        assert!(!args.jscpd.is_empty(), "jscpd should get glob pattern");

        // Trivy and scc do NOT accept globs
        assert!(
            args.trivy.is_empty(),
            "trivy should NOT receive glob patterns"
        );
        assert!(args.scc.is_empty(), "scc should NOT receive glob patterns");
    }

    #[test]
    fn from_patterns_mixed_glob_and_plain() {
        let patterns = vec!["vendor/".to_string(), "*.min.js".to_string()];
        let args = ExcludeArgs::from_patterns(&patterns);

        // jscpd should have both
        assert!(args.jscpd.contains(&"vendor".to_string()));
        assert!(args.jscpd.contains(&"*.min.js".to_string()));

        // trivy should only have vendor (no glob)
        assert!(args.trivy.contains(&"vendor".to_string()));
        assert!(!args.trivy.contains(&"*.min.js".to_string()));
    }

    #[test]
    fn from_patterns_negation_patterns_skipped() {
        let patterns = vec!["!vendor/important.go".to_string(), "logs/".to_string()];
        let args = ExcludeArgs::from_patterns(&patterns);

        // negation should not appear
        assert!(
            !args.semgrep.iter().any(|s| s.contains("important")),
            "negation pattern should be skipped"
        );
        // plain pattern should appear
        assert!(args.semgrep.contains(&"logs".to_string()));
    }

    #[test]
    fn collect_active_patterns_merges_defaults_and_config() {
        let config = cfg_with_excludes(vec!["custom-dir/"]);
        let patterns = ExcludeArgs::collect_active_patterns(&config);

        // Should contain all defaults
        for default in FiltersConfig::default_excludes() {
            assert!(
                patterns.contains(&default),
                "should contain default: {default}"
            );
        }

        // Should contain config excludes
        assert!(
            patterns.contains(&"custom-dir/".to_string()),
            "should contain config exclude: custom-dir/"
        );
    }

    #[test]
    fn collect_active_patterns_deduplicates() {
        // Add a pattern that's already a default
        let config = cfg_with_excludes(vec!["vendor/", "custom/"]);
        let patterns = ExcludeArgs::collect_active_patterns(&config);

        let vendor_count = patterns.iter().filter(|p| *p == "vendor/").count();
        assert_eq!(
            vendor_count, 1,
            "vendor/ should appear exactly once (deduplicated)"
        );
    }

    #[test]
    fn collect_active_patterns_does_not_include_filters_include() {
        let mut config = RiceGuardConfig::default();
        config.filters.include = vec!["src/**".to_string()];
        config.filters.exclude = vec![];
        let patterns = ExcludeArgs::collect_active_patterns(&config);

        assert!(
            !patterns.contains(&"src/**".to_string()),
            "filters.include patterns should NOT appear in active exclude patterns"
        );
    }
}
