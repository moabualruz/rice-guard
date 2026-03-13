use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::config::model::RiceGuardConfig;

use super::sources::{global_ignore_path, RGIGNORE, RICEGUARDIGNORE};

/// Compiled ignore engine built from all 5 sources in priority order:
///
/// 1. Built-in defaults (vendor/, node_modules/, target/, .git/, etc.)
/// 2. Global ignore file (`~/.config/riceguard/ignore`)
/// 3. `.gitignore` (only if `respect_gitignore = true`)
/// 4. `.riceguardignore` or `.rgignore` (project-level, riceguardignore takes precedence)
/// 5. `config.filters.exclude` patterns from YAML config
/// 6. `config.filters.include` patterns as `!` negations (rescues)
pub struct IgnoreEngine {
    matcher: Gitignore,
    root: PathBuf,
}

impl IgnoreEngine {
    /// Build an `IgnoreEngine` for the given project root.
    ///
    /// The `respect_gitignore` parameter controls whether `.gitignore` is loaded.
    /// Normally this is taken from `config.filters.respect_gitignore`, but it is
    /// passed explicitly here to make callers clear about the override.
    pub fn build(
        root: &Path,
        config: &RiceGuardConfig,
        respect_gitignore: bool,
    ) -> Result<Self, ignore::Error> {
        let mut builder = GitignoreBuilder::new(root);

        // ── Layer 1: Built-in defaults ──────────────────────────────────────
        for pattern in built_in_defaults() {
            builder.add_line(None, pattern)?;
        }

        // ── Layer 2: Global ignore file ─────────────────────────────────────
        if let Some(global_path) = global_ignore_path() {
            if global_path.exists() {
                if let Some(err) = builder.add(&global_path) {
                    tracing::warn!(
                        path = %global_path.display(),
                        error = %err,
                        "failed to load global ignore file"
                    );
                }
            }
        }

        // ── Layer 3: .gitignore (opt-in) ────────────────────────────────────
        if respect_gitignore {
            let gitignore_path = root.join(".gitignore");
            if gitignore_path.exists() {
                if let Some(err) = builder.add(&gitignore_path) {
                    tracing::warn!(
                        path = %gitignore_path.display(),
                        error = %err,
                        "failed to load .gitignore"
                    );
                }
            }
        }

        // ── Layer 4: .riceguardignore or .rgignore ──────────────────────────
        // .riceguardignore takes precedence: if it exists, skip .rgignore.
        let riceguardignore_path = root.join(RICEGUARDIGNORE);
        let rgignore_path = root.join(RGIGNORE);

        if riceguardignore_path.exists() {
            if let Some(err) = builder.add(&riceguardignore_path) {
                tracing::warn!(
                    path = %riceguardignore_path.display(),
                    error = %err,
                    "failed to load .riceguardignore"
                );
            }
        } else if rgignore_path.exists() {
            if let Some(err) = builder.add(&rgignore_path) {
                tracing::warn!(
                    path = %rgignore_path.display(),
                    error = %err,
                    "failed to load .rgignore"
                );
            }
        }

        // ── Layer 5: config.filters.exclude ─────────────────────────────────
        for pattern in &config.filters.exclude {
            builder.add_line(None, pattern)?;
        }

        // ── Layer 6: config.filters.include as negations (rescues) ──────────
        for pattern in &config.filters.include {
            // Only add as negation if it doesn't already start with '!'
            let negation = if pattern.starts_with('!') {
                pattern.clone()
            } else {
                format!("!{pattern}")
            };
            builder.add_line(None, &negation)?;
        }

        let matcher = builder.build()?;
        Ok(Self {
            matcher,
            root: root.to_path_buf(),
        })
    }

    /// Returns `true` if the given path should be ignored.
    ///
    /// `is_dir` controls whether the path is matched as a directory entry.
    /// Pass `true` for directories, `false` for files.
    ///
    /// Uses `matched_path_or_any_parents` so that files inside an ignored
    /// directory (e.g. `logs/app.log` when `logs/` is excluded) are also
    /// correctly reported as ignored.
    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let rel = match path.strip_prefix(&self.root) {
            Ok(r) => r,
            Err(_) => path,
        };
        matches!(
            self.matcher.matched_path_or_any_parents(rel, is_dir),
            ignore::Match::Ignore(_)
        )
    }
}

/// Returns the built-in default exclude patterns applied to every project.
fn built_in_defaults() -> &'static [&'static str] {
    &[
        "vendor/",
        "node_modules/",
        "target/",
        ".git/",
        "*.generated.*",
        "dist/",
    ]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn empty_config() -> RiceGuardConfig {
        let mut c = RiceGuardConfig::default();
        c.filters.exclude = vec![];
        c.filters.include = vec![];
        c
    }

    /// Create a temp dir with optional ignore files and return the dir.
    fn make_project(files: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(&path, content).expect("write file");
        }
        dir
    }

    #[test]
    fn empty_config_does_not_ignore_arbitrary_paths() {
        let dir = make_project(&[]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            !engine.is_ignored(&dir.path().join("src/main.rs"), false),
            "src/main.rs should not be ignored"
        );
        assert!(
            !engine.is_ignored(&dir.path().join("README.md"), false),
            "README.md should not be ignored"
        );
    }

    #[test]
    fn built_in_defaults_are_ignored() {
        let dir = make_project(&[]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            engine.is_ignored(&dir.path().join("vendor"), true),
            "vendor/ dir should be ignored"
        );
        assert!(
            engine.is_ignored(&dir.path().join("node_modules"), true),
            "node_modules/ dir should be ignored"
        );
        assert!(
            engine.is_ignored(&dir.path().join("target"), true),
            "target/ dir should be ignored"
        );
        assert!(
            engine.is_ignored(&dir.path().join(".git"), true),
            ".git/ dir should be ignored"
        );
    }

    #[test]
    fn riceguardignore_patterns_are_applied() {
        let dir = make_project(&[(".riceguardignore", "logs/\n")]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            engine.is_ignored(&dir.path().join("logs"), true),
            "logs/ should be ignored via .riceguardignore"
        );
        assert!(
            engine.is_ignored(&dir.path().join("logs/app.log"), false),
            "logs/app.log should be ignored via .riceguardignore"
        );
        assert!(
            !engine.is_ignored(&dir.path().join("src/main.rs"), false),
            "src/main.rs should not be ignored"
        );
    }

    #[test]
    fn rgignore_loaded_when_no_riceguardignore() {
        let dir = make_project(&[(".rgignore", "coverage/\n")]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            engine.is_ignored(&dir.path().join("coverage"), true),
            "coverage/ should be ignored via .rgignore"
        );
    }

    #[test]
    fn riceguardignore_takes_precedence_over_rgignore() {
        // .riceguardignore excludes "logs/"; .rgignore excludes "coverage/".
        // When both exist, only .riceguardignore should be loaded.
        let dir = make_project(&[
            (".riceguardignore", "logs/\n"),
            (".rgignore", "coverage/\n"),
        ]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            engine.is_ignored(&dir.path().join("logs"), true),
            "logs/ should be ignored (from .riceguardignore)"
        );
        assert!(
            !engine.is_ignored(&dir.path().join("coverage"), true),
            "coverage/ should NOT be ignored (.rgignore skipped when .riceguardignore exists)"
        );
    }

    #[test]
    fn respect_gitignore_false_does_not_load_gitignore() {
        let dir = make_project(&[(".gitignore", "build/\n")]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            !engine.is_ignored(&dir.path().join("build"), true),
            "build/ should NOT be ignored when respect_gitignore=false"
        );
    }

    #[test]
    fn respect_gitignore_true_loads_gitignore() {
        let dir = make_project(&[(".gitignore", "build/\n")]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, true).expect("build");

        assert!(
            engine.is_ignored(&dir.path().join("build"), true),
            "build/ should be ignored when respect_gitignore=true"
        );
    }

    #[test]
    fn negation_pattern_rescues_file_from_built_in_defaults() {
        let dir = make_project(&[(".riceguardignore", "!vendor/important.go\n")]);
        let config = empty_config();
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        // vendor/ is ignored by default, but vendor/important.go is rescued
        assert!(
            engine.is_ignored(&dir.path().join("vendor"), true),
            "vendor/ itself should still be ignored"
        );
        assert!(
            !engine.is_ignored(&dir.path().join("vendor/important.go"), false),
            "vendor/important.go should be rescued by negation pattern"
        );
    }

    #[test]
    fn filters_include_rescues_files_from_exclusion() {
        let dir = make_project(&[]);
        let mut config = empty_config();
        // vendor/ is excluded by built-in defaults; include vendor/keep.go to rescue it
        config.filters.include = vec!["vendor/keep.go".to_string()];
        let engine = IgnoreEngine::build(dir.path(), &config, false).expect("build");

        assert!(
            !engine.is_ignored(&dir.path().join("vendor/keep.go"), false),
            "vendor/keep.go should be rescued by filters.include"
        );
    }
}
