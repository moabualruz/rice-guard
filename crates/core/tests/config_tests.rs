/// Integration tests for config model round-trips.
///
/// These tests exercise the public serde interface of `rice_guard_core::config`.
use rice_guard_core::config::model::{
    Architecture, FiltersConfig, ProjectConfig, RiceGuardConfig, Topology,
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
