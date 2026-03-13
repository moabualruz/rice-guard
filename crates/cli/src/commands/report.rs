/// Report subcommand handler — SonarQube report retrieval (Phase 6).
use std::collections::HashMap;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::args::ReportArgs;
use crate::output;

use super::sonar::client::SonarClient;
use super::sonar::models::{
    MeasuresResponse, QualityGateStatus, SonarIssue, SonarIssuesSearchResponse,
};

/// Combined SonarQube status report written to `sonar-status.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonarStatusReport {
    /// Quality gate status string: "OK", "ERROR", or "NONE".
    pub quality_gate_status: String,
    /// Metrics map: metric key → value string.
    pub metrics: HashMap<String, String>,
    /// Critical/blocker issues (severity BLOCKER or CRITICAL).
    pub critical_issues: Vec<SonarIssue>,
    /// Total count of critical/blocker issues.
    pub total_critical: u32,
}

/// Run the report subcommand.
///
/// Pulls quality gate status, key metrics, and critical issues from SonarQube
/// and writes `sonar-status.json` to `reports/{project_name}/`.
pub async fn run(args: ReportArgs) -> anyhow::Result<i32> {
    // ── Step 1: resolve target path ───────────────────────────────────────────
    let target = crate::paths::safe_canonicalize(&args.path);

    // ── Step 2: load config ───────────────────────────────────────────────────
    let config_path = target.join(".rguard.yaml");
    let config = match rguard_core::config::load(&config_path) {
        Ok(c) => c,
        Err(rguard_core::errors::ConfigError::NotFound { .. }) => {
            output::print_error(&format!(
                "No .rguard.yaml found in {}. Run `rguard init` first.",
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
            "SonarQube integration disabled in .rguard.yaml \
             (set sonarqube.enabled: true to use report)",
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

    // ── Step 5: derive project key ────────────────────────────────────────────
    let dir_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let project_key = config
        .scanners
        .sonarqube
        .project_key
        .clone()
        .unwrap_or_else(|| super::sonar::converter::derive_project_key(&dir_name));

    let project_name = config.project.name.clone();
    let host = config
        .scanners
        .sonarqube
        .host
        .trim_end_matches('/')
        .to_string();

    // ── Step 6: build client ──────────────────────────────────────────────────
    let client = SonarClient::new(&host, &token);

    // ── Step 7: fetch quality gate ────────────────────────────────────────────
    let qg_path = format!("/api/qualitygates/project_status?projectKey={project_key}");
    let quality_gate_status = match client.get::<QualityGateStatus>(&qg_path).await {
        Ok(qg) => qg.project_status.status,
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
                return Ok(0);
            }
            if msg.contains("HTTP 404") || msg.contains("not found") || msg.contains("404") {
                output::print_warning("Project not found in SonarQube. Run `rguard enroll` first.");
                return Ok(0);
            }
            output::print_warning(&format!("Failed to fetch quality gate: {e}"));
            "UNKNOWN".to_string()
        }
    };

    // ── Step 8: fetch metrics ─────────────────────────────────────────────────
    let metrics_path = format!(
        "/api/measures/component?component={project_key}\
         &metricKeys=bugs,vulnerabilities,code_smells,duplicated_lines_density,coverage"
    );
    let metrics: HashMap<String, String> = match client.get::<MeasuresResponse>(&metrics_path).await
    {
        Ok(resp) => resp
            .component
            .measures
            .into_iter()
            .map(|m| (m.metric, m.value.unwrap_or_default()))
            .collect(),
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("connection refused") && !msg.contains("error sending request") {
                output::print_warning(&format!("Failed to fetch metrics: {e}"));
            }
            HashMap::new()
        }
    };

    // ── Step 9: fetch critical issues ─────────────────────────────────────────
    let issues_path =
        format!("/api/issues/search?componentKeys={project_key}&severities=BLOCKER,CRITICAL&ps=50");
    let (critical_issues, total_critical) =
        match client.get::<SonarIssuesSearchResponse>(&issues_path).await {
            Ok(resp) => {
                let total = resp.total;
                (resp.issues, total)
            }
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("connection refused") && !msg.contains("error sending request") {
                    output::print_warning(&format!("Failed to fetch issues: {e}"));
                }
                (Vec::new(), 0)
            }
        };

    // ── Step 10: build and write sonar-status.json ────────────────────────────
    let report = SonarStatusReport {
        quality_gate_status: quality_gate_status.clone(),
        metrics: metrics.clone(),
        critical_issues: critical_issues.clone(),
        total_critical,
    };

    let json = serde_json::to_string_pretty(&report).context("serialize sonar-status.json")?;

    let out_dir = target.join("reports").join(&project_name);
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create reports/{project_name} dir"))?;
    std::fs::write(out_dir.join("sonar-status.json"), &json).context("write sonar-status.json")?;

    // ── Step 11: print output ─────────────────────────────────────────────────
    use owo_colors::OwoColorize;
    use owo_colors::Stream::Stdout;

    let gate_colored = if quality_gate_status == "OK" {
        format!(
            "Quality Gate: {}",
            "PASSED".if_supports_color(Stdout, |t| t.green())
        )
    } else {
        format!(
            "Quality Gate: {}",
            quality_gate_status
                .as_str()
                .if_supports_color(Stdout, |t| t.red())
        )
    };

    println!("{gate_colored}");
    println!("Project: {host}/dashboard?id={project_key}");
    println!();

    if !metrics.is_empty() {
        println!("Metrics:");
        for (k, v) in &metrics {
            println!("  {k}: {v}");
        }
    }

    if total_critical > 0 {
        println!();
        println!(
            "Critical/Blocker issues: {}",
            total_critical
                .to_string()
                .if_supports_color(Stdout, |t| t.red())
        );
    }

    println!();
    println!(
        "Full report: {}",
        out_dir.join("sonar-status.json").display()
    );

    Ok(0)
}
