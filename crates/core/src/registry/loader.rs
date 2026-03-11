// Descriptor loader — implemented in Task 2.
use std::path::Path;

use crate::errors::DescriptorError;
use crate::registry::fixer_descriptor::FixerDescriptor;
use crate::registry::scanner_descriptor::ScannerDescriptor;

/// Built-in scanner descriptor sources, embedded at compile time.
const SCANNER_YAMLS: &[(&str, &str)] = &[
    (
        "semgrep",
        include_str!("../../../../descriptors/scanners/semgrep.yaml"),
    ),
    (
        "trivy",
        include_str!("../../../../descriptors/scanners/trivy.yaml"),
    ),
    (
        "gitleaks",
        include_str!("../../../../descriptors/scanners/gitleaks.yaml"),
    ),
    (
        "jscpd",
        include_str!("../../../../descriptors/scanners/jscpd.yaml"),
    ),
    (
        "scc",
        include_str!("../../../../descriptors/scanners/scc.yaml"),
    ),
];

/// Built-in fixer descriptor sources, embedded at compile time.
const FIXER_YAMLS: &[(&str, &str)] = &[
    ("go", include_str!("../../../../descriptors/fixers/go.yaml")),
    (
        "rust",
        include_str!("../../../../descriptors/fixers/rust.yaml"),
    ),
    (
        "python",
        include_str!("../../../../descriptors/fixers/python.yaml"),
    ),
    (
        "dart",
        include_str!("../../../../descriptors/fixers/dart.yaml"),
    ),
    (
        "typescript",
        include_str!("../../../../descriptors/fixers/typescript.yaml"),
    ),
    (
        "javascript",
        include_str!("../../../../descriptors/fixers/javascript.yaml"),
    ),
    (
        "java",
        include_str!("../../../../descriptors/fixers/java.yaml"),
    ),
    (
        "kotlin",
        include_str!("../../../../descriptors/fixers/kotlin.yaml"),
    ),
    (
        "php",
        include_str!("../../../../descriptors/fixers/php.yaml"),
    ),
    (
        "csharp",
        include_str!("../../../../descriptors/fixers/csharp.yaml"),
    ),
    (
        "ruby",
        include_str!("../../../../descriptors/fixers/ruby.yaml"),
    ),
    (
        "shell",
        include_str!("../../../../descriptors/fixers/shell.yaml"),
    ),
];

/// Load all scanner descriptors: 5 built-in + any user YAML in
/// `project_dir/descriptors/scanners/`.
///
/// Returns `Err(DescriptorError::ParseError)` on first malformed YAML
/// encountered (built-in or user-provided).
pub fn load_scanner_descriptors(
    project_dir: &Path,
) -> Result<Vec<ScannerDescriptor>, DescriptorError> {
    let mut descriptors = Vec::with_capacity(8);

    // Parse built-in descriptors.
    for (name, yaml) in SCANNER_YAMLS {
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(yaml).map_err(|e| DescriptorError::ParseError {
                path: format!("<built-in:{name}>"),
                reason: e.to_string(),
            })?;
        desc.validate()?;
        descriptors.push(desc);
    }

    // Merge user-provided descriptors from project_dir/descriptors/scanners/.
    let user_dir = project_dir.join("descriptors").join("scanners");
    if user_dir.is_dir() {
        load_user_scanner_descriptors(&user_dir, &mut descriptors)?;
    }

    Ok(descriptors)
}

fn load_user_scanner_descriptors(
    dir: &Path,
    out: &mut Vec<ScannerDescriptor>,
) -> Result<(), DescriptorError> {
    let entries = std::fs::read_dir(dir).map_err(|e| DescriptorError::IoError {
        path: dir.display().to_string(),
        source: e,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let yaml = std::fs::read_to_string(&path).map_err(|e| DescriptorError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;
        let desc: ScannerDescriptor =
            serde_yaml_ng::from_str(&yaml).map_err(|e| DescriptorError::ParseError {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        desc.validate()?;
        out.push(desc);
    }
    Ok(())
}

/// Load all fixer descriptors: 12 built-in + any user YAML in
/// `project_dir/descriptors/fixers/`.
pub fn load_fixer_descriptors(project_dir: &Path) -> Result<Vec<FixerDescriptor>, DescriptorError> {
    let mut descriptors = Vec::with_capacity(16);

    for (name, yaml) in FIXER_YAMLS {
        let desc: FixerDescriptor =
            serde_yaml_ng::from_str(yaml).map_err(|e| DescriptorError::ParseError {
                path: format!("<built-in:{name}>"),
                reason: e.to_string(),
            })?;
        descriptors.push(desc);
    }

    let user_dir = project_dir.join("descriptors").join("fixers");
    if user_dir.is_dir() {
        load_user_fixer_descriptors(&user_dir, &mut descriptors)?;
    }

    Ok(descriptors)
}

fn load_user_fixer_descriptors(
    dir: &Path,
    out: &mut Vec<FixerDescriptor>,
) -> Result<(), DescriptorError> {
    let entries = std::fs::read_dir(dir).map_err(|e| DescriptorError::IoError {
        path: dir.display().to_string(),
        source: e,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let yaml = std::fs::read_to_string(&path).map_err(|e| DescriptorError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;
        let desc: FixerDescriptor =
            serde_yaml_ng::from_str(&yaml).map_err(|e| DescriptorError::ParseError {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        out.push(desc);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn load_scanner_descriptors_returns_five_builtins() {
        let tmp = TempDir::new().unwrap();
        let result = load_scanner_descriptors(tmp.path()).expect("built-in descriptors must load");
        assert_eq!(
            result.len(),
            5,
            "expected exactly 5 built-in scanner descriptors"
        );
        let names: Vec<&str> = result.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"semgrep"));
        assert!(names.contains(&"trivy"));
        assert!(names.contains(&"gitleaks"));
        assert!(names.contains(&"jscpd"));
        assert!(names.contains(&"scc"));
    }

    #[test]
    fn load_fixer_descriptors_returns_twelve_builtins() {
        let tmp = TempDir::new().unwrap();
        let result = load_fixer_descriptors(tmp.path()).expect("built-in descriptors must load");
        assert_eq!(
            result.len(),
            12,
            "expected exactly 12 built-in fixer descriptors"
        );
        let names: Vec<&str> = result.iter().map(|d| d.name.as_str()).collect();
        for expected in [
            "go",
            "rust",
            "python",
            "dart",
            "typescript",
            "javascript",
            "java",
            "kotlin",
            "php",
            "csharp",
            "ruby",
            "shell",
        ] {
            assert!(
                names.contains(&expected),
                "missing fixer descriptor: {expected}"
            );
        }
    }

    #[test]
    fn user_scanner_descriptor_is_merged() {
        let tmp = TempDir::new().unwrap();
        let scanners_dir = tmp.path().join("descriptors").join("scanners");
        std::fs::create_dir_all(&scanners_dir).unwrap();
        let yaml = r#"
name: custom-scanner
version: ">=1.0"
languages: [rust]
install:
  check: "custom-scanner --version"
  pip: "pip install custom-scanner"
commands:
  scan:
    cmd: "custom-scanner --json ."
    timeout: 60
output_format: json
severity_map: {}
"#;
        let mut f = std::fs::File::create(scanners_dir.join("custom.yaml")).unwrap();
        f.write_all(yaml.as_bytes()).unwrap();

        let result = load_scanner_descriptors(tmp.path()).unwrap();
        assert_eq!(result.len(), 6, "5 built-ins + 1 user descriptor");
        assert!(result.iter().any(|d| d.name == "custom-scanner"));
    }

    #[test]
    fn malformed_user_scanner_yaml_returns_parse_error() {
        let tmp = TempDir::new().unwrap();
        let scanners_dir = tmp.path().join("descriptors").join("scanners");
        std::fs::create_dir_all(&scanners_dir).unwrap();
        let mut f = std::fs::File::create(scanners_dir.join("bad.yaml")).unwrap();
        f.write_all(b"name: [bad yaml {{{{").unwrap();

        let result = load_scanner_descriptors(tmp.path());
        assert!(
            matches!(result, Err(DescriptorError::ParseError { .. })),
            "malformed YAML must return ParseError, got: {result:?}"
        );
    }

    #[test]
    fn user_fixer_descriptor_is_merged() {
        let tmp = TempDir::new().unwrap();
        let fixers_dir = tmp.path().join("descriptors").join("fixers");
        std::fs::create_dir_all(&fixers_dir).unwrap();
        let yaml = r#"
name: custom-lang
language: custom
detect: [custom.toml]
stages:
  format:
    - name: custom-fmt
      check: "custom-fmt --check ."
      fix: "custom-fmt ."
      scope: project
      safe: true
"#;
        let mut f = std::fs::File::create(fixers_dir.join("custom.yaml")).unwrap();
        f.write_all(yaml.as_bytes()).unwrap();

        let result = load_fixer_descriptors(tmp.path()).unwrap();
        assert_eq!(result.len(), 13, "12 built-ins + 1 user descriptor");
    }
}
