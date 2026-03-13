use crate::config::model::RGuardConfig;
use crate::errors::ConfigError;

/// Validate the semantic constraints on a loaded config.
///
/// Returns `ConfigError::ValidationError` if any constraint is violated.
pub fn validate(config: &RGuardConfig) -> Result<(), ConfigError> {
    if config.version != "1" {
        return Err(ConfigError::ValidationError {
            message: format!(
                "unsupported config version '{}' — expected '1'",
                config.version
            ),
        });
    }

    if config.project.name.trim().is_empty() {
        return Err(ConfigError::ValidationError {
            message: "project.name must not be empty".into(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::model::{Architecture, ProjectConfig, Topology};

    use super::*;

    fn make_config(version: &str, name: &str) -> RGuardConfig {
        RGuardConfig {
            version: version.into(),
            project: ProjectConfig {
                name: name.into(),
                languages: vec![],
                topology: Topology::default(),
                architecture: Architecture::default(),
            },
            scanners: Default::default(),
            fixers: Default::default(),
            filters: Default::default(),
            tools: Default::default(),
            output: Default::default(),
            hooks: Default::default(),
            ci: Default::default(),
        }
    }

    #[test]
    fn valid_config_passes() {
        let config = make_config("1", "my-app");
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn wrong_version_fails() {
        let config = make_config("2", "my-app");
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError { .. }));
    }

    #[test]
    fn empty_project_name_fails() {
        let config = make_config("1", "");
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError { .. }));
    }

    #[test]
    fn whitespace_project_name_fails() {
        let config = make_config("1", "   ");
        let err = validate(&config).unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError { .. }));
    }
}
