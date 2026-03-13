use std::path::PathBuf;

/// The primary project-level ignore file name.
pub const RICEGUARDIGNORE: &str = ".riceguardignore";

/// The secondary project-level ignore file name (fallback when `.riceguardignore` absent).
pub const RGIGNORE: &str = ".rgignore";

/// Returns the platform-appropriate path to the global ignore file.
///
/// On Linux/macOS: `~/.config/riceguard/ignore`
/// On Windows:     `%APPDATA%\riceguard\ignore`
///
/// Returns `None` if the platform config directory cannot be determined.
pub fn global_ignore_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("riceguard").join("ignore"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_ignore_path_contains_riceguard_and_ignore() {
        let path = global_ignore_path();
        // This may return None in unusual CI environments, so we only assert
        // when Some is returned.
        if let Some(p) = path {
            let display = p.to_string_lossy();
            assert!(
                display.contains("riceguard"),
                "global ignore path should contain 'riceguard', got: {display}"
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
        assert_eq!(RICEGUARDIGNORE, ".riceguardignore");
        assert_eq!(RGIGNORE, ".rgignore");
    }
}
