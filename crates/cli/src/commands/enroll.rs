/// Enroll subcommand handler — SonarQube project enrollment (Phase 6).
use std::path::PathBuf;

use anyhow::Context;

use crate::args::EnrollArgs;
use crate::output;

use super::sonar::client::SonarClient;
use super::sonar::converter::{convert_sarif_to_generic_issues, derive_project_key};
use super::sonar::models::{ProjectsSearchResponse, SonarGenericIssuesFile};

/// Run the enroll subcommand.
///
/// Registers the project in SonarQube, converts existing SARIF output to
/// Generic Issue Data JSON, and writes `sonar-project.properties` so the
/// user can run `sonar-scanner` immediately after.
pub async fn run(args: EnrollArgs) -> anyhow::Result<i32> {
    // ── Step 1: resolve target path ───────────────────────────────────────────
    let target = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    // ── Step 2: load config ───────────────────────────────────────────────────
    let config_path = target.join(".riceguard.yaml");
    let config = match rice_guard_core::config::load(&config_path) {
        Ok(c) => c,
        Err(rice_guard_core::errors::ConfigError::NotFound { .. }) => {
            output::print_error(&format!(
                "No .riceguard.yaml found in {}. Run `rice-guard init` first.",
                target.display()
            ));
            return Ok(2);
        }
        Err(e) => {
            output::print_error(&format!("Failed to load config: {e}"));
            return Ok(2);
        }
    };

    // ── Step 3: check sonarqube.enabled ──────────────────────────────────────
    if !config.scanners.sonarqube.enabled {
        output::print_info(
            "SonarQube integration disabled in .riceguard.yaml \
             (set sonarqube.enabled: true to use enroll)",
        );
        return Ok(0);
    }

    // ── Step 4: resolve token ─────────────────────────────────────────────────
    let token = match SonarClient::resolve_token(&config.scanners.sonarqube) {
        Ok(t) => t,
        Err(e) => {
            output::print_error(&format!("{e}"));
            return Ok(2);
        }
    };

    // ── Step 5: derive project key and name ───────────────────────────────────
    let dir_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let project_key = config
        .scanners
        .sonarqube
        .project_key
        .clone()
        .unwrap_or_else(|| derive_project_key(&dir_name));

    let project_name = config.project.name.clone();
    let host = config
        .scanners
        .sonarqube
        .host
        .trim_end_matches('/')
        .to_string();

    // ── Step 6: build client ──────────────────────────────────────────────────
    let client = SonarClient::new(&host, &token);

    // ── Step 7: check if project already enrolled ─────────────────────────────
    let search_path = format!("/api/projects/search?projects={project_key}");
    match client.get::<ProjectsSearchResponse>(&search_path).await {
        Ok(resp) if !resp.components.is_empty() => {
            output::print_info(&format!(
                "{project_name} already enrolled at {host}/dashboard?id={project_key}"
            ));
            // Still write properties so user can run sonar-scanner
            write_properties(&target, &project_key, &project_name, &host)?;
            return Ok(0);
        }
        Ok(_) => {
            // Project not found — continue to create it
        }
        Err(e) => {
            // Connection refused or other network error — graceful degrade
            let msg = e.to_string();
            if msg.contains("connection refused")
                || msg.contains("connect error")
                || msg.contains("Connection refused")
                || msg.contains("error sending request")
            {
                output::print_warning(
                    "SonarQube is not running. Start it with:\n  \
                     docker compose -f docker/docker-compose.yml up -d",
                );
                write_properties(&target, &project_key, &project_name, &host)?;
                convert_and_write_issues(&target, &project_name)?;
                return Ok(0);
            }
            output::print_warning(&format!("SonarQube project check failed: {e}"));
        }
    }

    // ── Step 8: create project ────────────────────────────────────────────────
    match client
        .post_form(
            "/api/projects/create",
            &[
                ("project", project_key.as_str()),
                ("name", project_name.as_str()),
            ],
        )
        .await
    {
        Ok(()) => {}
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("connection refused")
                || msg.contains("connect error")
                || msg.contains("error sending request")
            {
                output::print_warning(
                    "SonarQube is not running. Start it with:\n  \
                     docker compose -f docker/docker-compose.yml up -d",
                );
                write_properties(&target, &project_key, &project_name, &host)?;
                convert_and_write_issues(&target, &project_name)?;
                return Ok(0);
            }
            output::print_warning(&format!("Failed to create SonarQube project: {e}"));
        }
    }

    // ── Step 9: convert SARIF → sonar-issues.json ─────────────────────────────
    convert_and_write_issues(&target, &project_name)?;

    // ── Step 10: write sonar-project.properties ───────────────────────────────
    write_properties(&target, &project_key, &project_name, &host)?;

    // ── Step 11: success output ───────────────────────────────────────────────
    output::print_success(&format!(
        "Project enrolled: {host}/dashboard?id={project_key}"
    ));
    output::print_info("Next: run sonar-scanner to push analysis");

    Ok(0)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Find SARIF files from the most recent timestamp directory under `reports/{project_name}/`.
fn find_latest_sarif_files(target: &std::path::Path, project_name: &str) -> Vec<PathBuf> {
    let reports_dir = target.join("reports").join(project_name);
    if !reports_dir.exists() {
        return Vec::new();
    }

    // Collect timestamp subdirectories (ISO timestamps sort lexicographically)
    let mut timestamp_dirs: Vec<_> = std::fs::read_dir(&reports_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();

    timestamp_dirs.sort();

    let latest = match timestamp_dirs.last() {
        Some(d) => d,
        None => return Vec::new(),
    };

    // Collect *.sarif files in the latest timestamp dir
    std::fs::read_dir(latest)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "sarif").unwrap_or(false))
        .map(|e| e.path())
        .collect()
}

/// Convert SARIF files and write merged sonar-issues.json.
fn convert_and_write_issues(target: &std::path::Path, project_name: &str) -> anyhow::Result<()> {
    let sarif_files = find_latest_sarif_files(target, project_name);

    if sarif_files.is_empty() {
        output::print_warning("No SARIF files found — writing empty sonar-issues.json");
    }

    let mut all_issues = Vec::new();
    for sarif_path in &sarif_files {
        match convert_sarif_to_generic_issues(sarif_path, target) {
            Ok(f) => all_issues.extend(f.issues),
            Err(e) => {
                output::print_warning(&format!("Failed to convert {}: {e}", sarif_path.display()));
            }
        }
    }

    let merged = SonarGenericIssuesFile { issues: all_issues };
    let json = serde_json::to_string_pretty(&merged).context("serialize sonar-issues.json")?;

    let out_dir = target.join("reports").join(project_name);
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create reports/{project_name} dir"))?;

    std::fs::write(out_dir.join("sonar-issues.json"), json).context("write sonar-issues.json")?;

    Ok(())
}

/// Write `sonar-project.properties` to the project root.
pub fn write_properties(
    project_root: &std::path::Path,
    project_key: &str,
    project_name: &str,
    host: &str,
) -> anyhow::Result<()> {
    // Derive project_name for the report path from the project dir name
    // We use the same project_name (which is config.project.name)
    let issues_path = format!("reports/{project_name}/sonar-issues.json");

    let content = format!(
        "sonar.projectKey={project_key}\n\
         sonar.projectName={project_name}\n\
         sonar.host.url={host}\n\
         sonar.sources=.\n\
         sonar.externalIssuesReportPaths={issues_path}\n"
    );

    std::fs::write(project_root.join("sonar-project.properties"), content)
        .context("write sonar-project.properties")?;

    Ok(())
}
