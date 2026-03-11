use std::path::Path;

use crate::config::model::RiceGuardConfig;
use crate::errors::ConfigError;

/// Load and parse a `.riceguard.yaml` file from the given path.
///
/// Returns `ConfigError::NotFound` if the file does not exist,
/// `ConfigError::IoError` on read failure, and `ConfigError::ParseError`
/// if the YAML is malformed.
pub fn load(path: &Path) -> Result<RiceGuardConfig, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::NotFound {
            path: path.display().to_string(),
        });
    }

    let contents = std::fs::read_to_string(path).map_err(|e| ConfigError::IoError {
        path: path.display().to_string(),
        source: e,
    })?;

    let config: RiceGuardConfig =
        serde_yaml_ng::from_str(&contents).map_err(|e| ConfigError::ParseError {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn load_returns_not_found_for_missing_file() {
        let path = Path::new("/nonexistent/path/.riceguard.yaml");
        let result = load(path);
        assert!(matches!(result, Err(ConfigError::NotFound { .. })));
    }

    #[test]
    fn load_returns_parse_error_for_invalid_yaml() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, ": invalid: yaml: {{{{").unwrap();
        let result = load(f.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn load_succeeds_for_valid_yaml() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"version: "1"
project:
  name: my-app
  languages: [rust]
  topology: cli
  architecture: none
"#
        )
        .unwrap();
        let config = load(f.path()).expect("should load valid yaml");
        assert_eq!(config.version, "1");
        assert_eq!(config.project.name, "my-app");
    }
}
