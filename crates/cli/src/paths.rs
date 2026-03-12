/// Path utilities for Windows compatibility.
///
/// On Windows, `std::fs::canonicalize()` returns extended-length paths
/// prefixed with `\\?\`.  This prefix disables all path normalization,
/// meaning forward slashes in `{{output_dir}}/file.json` are NOT
/// translated to backslashes — causing "os error 123" when external
/// tools try to use those paths.
///
/// `safe_canonicalize` strips the prefix so downstream code gets a
/// normal absolute path that works with both Rust APIs and subprocess
/// arguments.
use std::path::{Path, PathBuf};

/// Canonicalize `path`, stripping the Windows `\\?\` prefix when present.
///
/// Falls back to the original path if canonicalization fails (e.g., the
/// path does not exist yet).
pub fn safe_canonicalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => strip_unc_prefix(p),
        Err(_) => path.to_path_buf(),
    }
}

/// Strip the `\\?\` extended-length path prefix on Windows.
/// No-op on other platforms.
#[cfg(windows)]
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix("\\\\?\\") {
        Some(stripped) => PathBuf::from(stripped),
        None => path,
    }
}

#[cfg(not(windows))]
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_current_dir_has_no_unc_prefix() {
        let p = safe_canonicalize(Path::new("."));
        let s = p.to_string_lossy();
        assert!(
            !s.starts_with("\\\\?\\"),
            "path should not have \\\\?\\ prefix: {s}"
        );
    }

    #[test]
    fn nonexistent_path_returns_original() {
        let orig = Path::new("this/does/not/exist/at/all");
        let result = safe_canonicalize(orig);
        assert_eq!(result, orig);
    }
}
