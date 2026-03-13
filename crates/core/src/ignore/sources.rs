use std::path::PathBuf;

/// The primary project-level ignore file name.
pub const RGUARDIGNORE: &str = ".rguardignore";

/// The secondary project-level ignore file name (fallback when `.rguardignore` absent).
pub const RGIGNORE: &str = ".rgignore";

/// Returns the platform-appropriate path to the global ignore file.
///
/// On Linux/macOS: `~/.config/rguard/ignore`
/// On Windows:     `%APPDATA%\rguard\ignore`
///
/// Returns `None` if the platform config directory cannot be determined.
pub fn global_ignore_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rguard").join("ignore"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_ignore_path_contains_rguard_and_ignore() {
        let path = global_ignore_path();
        // This may return None in unusual CI environments, so we only assert
        // when Some is returned.
        if let Some(p) = path {
            let display = p.to_string_lossy();
            assert!(
                display.contains("rguard"),
                "global ignore path should contain 'rguard', got: {display}"
            );
            // The final component should be "ignore"
            assert_eq!(
                p.file_name().and_then(|n| n.to_str()),
                Some("ignore"),
                "global ignore path final component should be 'ignore'"
            );
        }
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(RGUARDIGNORE, ".rguardignore");
        assert_eq!(RGIGNORE, ".rgignore");
    }
}
