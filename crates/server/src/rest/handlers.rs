use std::path::{Path, PathBuf};

use axum::extract::{Path as AxumPath, State};
use axum::Json;
use rice_guard_core::fixer::engine::FixerEngineConfig;
use rice_guard_core::fixer::{FixReport, FixerEngine, StageFilter};
use rice_guard_core::issue::{sort_issues, FixerDescriptorInfo, Issue, IssueBuilder};
use rice_guard_core::output::{OutputWriter, ScanSummary};
use rice_guard_core::registry::DescriptorRegistry;
use rice_guard_core::scanner::{OutputDir, ScanMode, ScannerEngine};
use serde_json::{json, Value};

use super::error::ApiError;
use super::request::{FixRequest, ScanRequest};
use super::state::AppState;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Find the lexicographically latest timestamped subdirectory under `base/reports/`.
///
/// Timestamp dirs sort correctly as strings (format: `%Y%m%dT%H%M%SZ`), so
/// alphabetical max == chronological max.
fn find_latest_reports_dir(base: &Path) -> anyhow::Result<PathBuf> {
    let reports = base.join("reports");

    // Walk one level of project subdirectories, then one level of timestamp dirs.
    // Pattern: reports/<project>/<timestamp>/
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(project_dirs) = std::fs::read_dir(&reports) {
        for project_entry in project_dirs.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }
            // Skip the "latest" symlink/junction.
            if project_path.file_name().and_then(|n| n.to_str()) == Some("latest") {
                continue;
            }
            if let Ok(ts_dirs) = std::fs::read_dir(&project_path) {
                for ts_entry in ts_dirs.flatten() {
                    let ts_path = ts_entry.path();
                    if ts_path.is_dir() {
                        candidates.push(ts_path);
                    }
                }
            }
        }
    }

    candidates
        .into_iter()
        .max_by(|a, b| {
            a.to_string_lossy()
                .as_ref()
                .cmp(b.to_string_lossy().as_ref())
        })
        .ok_or_else(|| anyhow::anyhow!("No scan results found in {}", reports.display()))
}

/// Load scanner descriptors for the given project path.
fn load_scanner_descriptors(
    project_path: &Path,
) -> Result<Vec<rice_guard_core::registry::ScannerDescriptor>, ApiError> {
    rice_guard_core::registry::loader::load_scanner_descriptors(project_path)
        .map_err(|e| ApiError::internal(&format!("Failed to load scanner descriptors: {e}")))
}

/// Load config for the given project path.
fn load_config(project_path: &Path) -> Result<rice_guard_core::config::RiceGuardConfig, ApiError> {
    let config_path = project_path.join(".riceguard.yaml");
    rice_guard_core::config::load(&config_path)
        .map_err(|e| ApiError::bad_request("CONFIG_NOT_FOUND", &e.to_string()))
}

/// Convert FixerDescriptors to FixerDescriptorInfo list.
fn to_fixer_descriptor_infos(
    fixer_descriptors: Vec<rice_guard_core::registry::FixerDescriptor>,
) -> Vec<FixerDescriptorInfo> {
    fixer_descriptors
        .into_iter()
        .map(|fd| {
            let mut stages: Vec<String> = Vec::new();
            if !fd.stages.format.is_empty() {
                stages.push("formatters".to_string());
            }
            if !fd.stages.lint.is_empty() {
                stages.push("linters".to_string());
            }
            if !fd.stages.security.is_empty() {
                stages.push("security".to_string());
            }
            if !fd.stages.ast.is_empty() {
                stages.push("ast".to_string());
            }
            if !fd.stages.deps.is_empty() {
                stages.push("deps".to_string());
            }
            if !fd.stages.import.is_empty() {
                stages.push("import".to_string());
            }
            let fix_tool = fd
                .stages
                .format
                .first()
                .or_else(|| fd.stages.lint.first())
                .or_else(|| fd.stages.security.first())
                .or_else(|| fd.stages.ast.first())
                .or_else(|| fd.stages.deps.first())
                .or_else(|| fd.stages.import.first())
                .map(|step| step.fix.clone());
            FixerDescriptorInfo {
                language: fd.language.clone(),
                stages,
                fix_tool,
            }
        })
        .collect()
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/v1/health — always returns 200 {"status": "ok"}.
pub async fn health_handler() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

/// POST /api/v1/scan — run the scanner engine and return a ScanSummary.
pub async fn scan_handler(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanSummary>, ApiError> {
    // Resolve project path.
    let project_path: PathBuf = req
        .path
        .map(PathBuf::from)
        .unwrap_or(state.working_dir.clone());

    // Load config.
    let config = load_config(&project_path)?;
    let project_name = config.project.name.clone();

    // Load scanner descriptors.
    let descriptors = load_scanner_descriptors(&project_path)?;

    // Parse scan mode.
    let mode = match req.mode.as_deref() {
        Some("quick") => ScanMode::Quick,
        Some("security") => ScanMode::Security,
        _ => ScanMode::Full,
    };

    // Create output directory.
    let reports_base = project_path.join("reports").to_string_lossy().to_string();
    let output_dir = OutputDir::new(&project_name, &reports_base)
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    // Run the scanner engine.
    let engine = ScannerEngine::new(descriptors, config);
    let findings = engine
        .run(&project_path, mode, &output_dir)
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    // Load fixer descriptors for build_batch.
    let fixer_descriptor_infos: Vec<FixerDescriptorInfo> =
        match rice_guard_core::registry::loader::load_fixer_descriptors(&project_path) {
            Ok(fds) => to_fixer_descriptor_infos(fds),
            Err(e) => {
                tracing::warn!("Failed to load fixer descriptors: {e}; using empty list");
                vec![]
            }
        };

    // Build issues.
    let mut issues = IssueBuilder::build_batch(&findings, &project_path, &fixer_descriptor_infos);
    sort_issues(&mut issues);

    // Build summary.
    let scanners_run: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        findings
            .iter()
            .filter(|f| seen.insert(f.scanner.clone()))
            .map(|f| f.scanner.clone())
            .collect()
    };
    let summary =
        ScanSummary::from_issues(&issues, scanners_run, &project_path.to_string_lossy(), 0);

    // Write output files.
    let writer = OutputWriter::new(output_dir.path());
    writer
        .write_all(&issues, &summary)
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    Ok(Json(summary))
}

/// GET /api/v1/issues — return all issues from the latest scan.
pub async fn issues_handler(State(state): State<AppState>) -> Result<Json<Vec<Issue>>, ApiError> {
    let reports_dir = find_latest_reports_dir(&state.working_dir)
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    let issues_path = reports_dir.join("issues.json");
    let contents = tokio::fs::read_to_string(&issues_path)
        .await
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    // issues.json is wrapped in an IssueOutput envelope: {"schema_version": "...", "issues": [...]}
    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| ApiError::internal(&format!("Failed to parse issues.json: {e}")))?;

    let issues_json = if let Some(arr) = parsed.get("issues") {
        arr.clone()
    } else {
        parsed
    };

    let issues: Vec<Issue> = serde_json::from_value(issues_json)
        .map_err(|e| ApiError::internal(&format!("Failed to deserialize issues: {e}")))?;

    Ok(Json(issues))
}

/// GET /api/v1/issues/{id} — return a single issue by fingerprint ID or 404.
pub async fn issue_by_id_handler(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Issue>, ApiError> {
    let reports_dir = find_latest_reports_dir(&state.working_dir)
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    let issues_path = reports_dir.join("issues.json");
    let contents = tokio::fs::read_to_string(&issues_path)
        .await
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| ApiError::internal(&format!("Failed to parse issues.json: {e}")))?;

    let issues_json = if let Some(arr) = parsed.get("issues") {
        arr.clone()
    } else {
        parsed
    };

    let issues: Vec<Issue> = serde_json::from_value(issues_json)
        .map_err(|e| ApiError::internal(&format!("Failed to deserialize issues: {e}")))?;

    let issue = issues
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| ApiError::not_found(&format!("Issue {id} not found")))?;

    Ok(Json(issue))
}

/// POST /api/v1/fix — run deterministic fixers and return a FixReport.
pub async fn fix_handler(
    State(state): State<AppState>,
    Json(req): Json<FixRequest>,
) -> Result<Json<FixReport>, ApiError> {
    let project_path: PathBuf = req
        .path
        .map(PathBuf::from)
        .unwrap_or(state.working_dir.clone());

    // Load config.
    let config = load_config(&project_path)?;

    // Load fixer descriptors.
    let fixer_descriptors =
        match rice_guard_core::registry::loader::load_fixer_descriptors(&project_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    "Failed to load fixer descriptors: {}; continuing with empty list",
                    e
                );
                vec![]
            }
        };
    let registry = DescriptorRegistry::new(fixer_descriptors);

    // Build stage filter from optional category.
    let stage_filter = match req.category.as_deref() {
        Some("formatters") => StageFilter::from_args(true, false, false, false, false, false),
        Some("linters") => StageFilter::from_args(false, true, false, false, false, false),
        Some("security") => StageFilter::from_args(false, false, true, false, false, false),
        Some("ast") => StageFilter::from_args(false, false, false, true, false, false),
        Some("deps") => StageFilter::from_args(false, false, false, false, true, false),
        Some("imports") => StageFilter::from_args(false, false, false, false, false, true),
        _ => StageFilter::from_args(false, false, false, false, false, false),
    };

    // Create fix output directories.
    let reports_base = project_path.join("reports").to_string_lossy().to_string();
    let fix_prefix = format!("fix-{}", config.project.name);
    let fix_output_dir = OutputDir::new(&fix_prefix, &reports_base)
        .map_err(|e| ApiError::internal(&e.to_string()))?;
    let latest_dir = project_path.join("reports").join("latest");
    let _ = std::fs::create_dir_all(&latest_dir);

    // Build engine config — API mode: no TTY prompt, unsafe auto-accepted if requested.
    let engine_config = FixerEngineConfig {
        project_root: project_path,
        file_targets: vec![],
        stage_filter,
        dry_run: req.dry_run.unwrap_or(false),
        include_unsafe: req.unsafe_fixes.unwrap_or(false),
        timeout_secs: 120,
        output_dir: fix_output_dir.path().to_path_buf(),
        latest_dir,
        issue_ids: req.issue_id.map(|id| vec![id]),
    };

    let engine = FixerEngine::new(config, registry);
    let fix_report = engine
        .run(engine_config)
        .await
        .map_err(|e| ApiError::internal(&e.to_string()))?;

    Ok(Json(fix_report))
}

/// GET /api/v1/status — return the ScanSummary from the latest scan.
pub async fn status_handler(State(state): State<AppState>) -> Result<Json<ScanSummary>, ApiError> {
    let reports_dir = find_latest_reports_dir(&state.working_dir)
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    let summary_path = reports_dir.join("summary.json");
    let contents = tokio::fs::read_to_string(&summary_path)
        .await
        .map_err(|_| ApiError::not_found("No scan results found"))?;

    let summary: ScanSummary = serde_json::from_str(&contents)
        .map_err(|e| ApiError::internal(&format!("Failed to parse summary.json: {e}")))?;

    Ok(Json(summary))
}
