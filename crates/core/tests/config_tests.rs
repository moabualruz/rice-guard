/// Integration tests for config model round-trips.
///
/// These tests exercise the public serde interface of `rice_guard_core::config`.
use rice_guard_core::config::model::{
    Architecture, FiltersConfig, ProjectConfig, RiceGuardConfig, SonarqubeConfig, Topology,
};

// ── round-trip ────────────────────────────────────────────────────────────────

#[test]
fn config_roundtrip() {
    let config = RiceGuardConfig {
        version: "1".to_string(),
        project: ProjectConfig {
            name: "test-project".to_string(),
            languages: vec!["rust".to_string()],
            topology: Topology::Cli,
            architecture: Architecture::None,
        },
        filters: FiltersConfig {
            include: vec![],
            exclude: FiltersConfig::default_excludes(),
        },
        ..Default::default()
    };
    let yaml = serde_yaml_ng::to_string(&config).expect("serialize failed");
    let reparsed: RiceGuardConfig = serde_yaml_ng::from_str(&yaml).expect("deserialize failed");
    assert_eq!(config.version, reparsed.version);
    assert_eq!(config.project.name, reparsed.project.name);
    assert_eq!(config.project.topology, reparsed.project.topology);
    assert_eq!(config.project.architecture, reparsed.project.architecture);
    assert_eq!(config.filters.include, reparsed.filters.include);
    assert_eq!(config.filters.exclude, reparsed.filters.exclude);
}

#[test]
fn default_excludes_present_after_roundtrip() {
    let config = RiceGuardConfig {
        version: "1".to_string(),
        project: ProjectConfig {
            name: "roundtrip-check".to_string(),
            languages: vec![],
            topology: Topology::Monolith,
            architecture: Architecture::None,
        },
        filters: FiltersConfig {
            include: vec![],
            exclude: FiltersConfig::default_excludes(),
        },
        ..Default::default()
    };
    let yaml = serde_yaml_ng::to_string(&config).unwrap();
    let reparsed: RiceGuardConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    for expected in ["vendor/", "node_modules/", "target/", ".git/"] {
        assert!(
            reparsed.filters.exclude.contains(&expected.to_string()),
            "default exclude '{expected}' must survive round-trip"
        );
    }
}

// ── SonarqubeConfig optional fields ──────────────────────────────────────────

/// Parsing a sonarqube block without token/project_key fields yields None.
#[test]
fn sonarqube_config_token_optional() {
    let yaml = r#"
version: "1"
project:
  name: test
  languages: []
  topology: monolith
  architecture: none
scanners:
  sonarqube:
    enabled: true
    host: "http://localhost:9000"
"#;
    let config: RiceGuardConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.scanners.sonarqube.token, None);
    assert_eq!(config.scanners.sonarqube.project_key, None);
    assert!(config.scanners.sonarqube.enabled);
}

/// Parsing a sonarqube block with token present yields Some("abc").
#[test]
fn sonarqube_config_token_present() {
    let yaml = r#"
version: "1"
project:
  name: test
  languages: []
  topology: monolith
  architecture: none
scanners:
  sonarqube:
    enabled: true
    host: "http://localhost:9000"
    token: "abc"
    project_key: "my-project"
"#;
    let config: RiceGuardConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.scanners.sonarqube.token, Some("abc".to_string()));
    assert_eq!(
        config.scanners.sonarqube.project_key,
        Some("my-project".to_string()),
    );
}

/// Token and project_key are omitted from serialized YAML when None
/// (skip_serializing_if = "Option::is_none").
#[test]
fn sonarqube_config_none_fields_not_serialized() {
    let config = SonarqubeConfig {
        enabled: true,
        host: "http://localhost:9000".to_string(),
        token: None,
        project_key: None,
    };
    let yaml = serde_yaml_ng::to_string(&config).unwrap();
    assert!(
        !yaml.contains("token"),
        "token: None should not appear in YAML output"
    );
    assert!(
        !yaml.contains("project_key"),
        "project_key: None should not appear in YAML output"
    );
}
