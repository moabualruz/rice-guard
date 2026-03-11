/// Init subcommand handler.
///
/// Orchestrates the 3-step detection pipeline from `rice_guard_core::init`:
///   1. detect() — scc language detection + tool probing
///   2. wizard() / noninteractive_choices() — interactive or default choices
///   3. generate() — write .riceguard.yaml and optional CI/pre-commit configs
use crate::args::InitArgs;
use crate::output;
use rice_guard_core::init::{DetectionResult, GeneratorInput};
use std::path::PathBuf;

/// Run the init subcommand.
///
/// Returns exit code 0 on success. Any error propagates as exit code 2
/// via the main.rs error handler.
pub async fn run(args: InitArgs) -> anyhow::Result<i32> {
    let project_path = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    output::print_info("Detecting project languages and available tools...");

    let detection = rice_guard_core::init::detect(&project_path)
        .await
        .map_err(|e| anyhow::anyhow!("Detection failed: {e}"))?;

    let choices = if args.yes {
        rice_guard_core::init::noninteractive_choices(&detection)
    } else {
        rice_guard_core::init::wizard(&detection).map_err(|e| anyhow::anyhow!("{e}"))?
    };

    let project_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let input = GeneratorInput {
        project_name,
        project_path: project_path.clone(),
        detection: detection.clone(),
        choices,
    };

    if args.dry_run {
        output::print_info("Dry run — files that would be generated:");
        output::print_info("  .riceguard.yaml");
        return Ok(0);
    }

    let created = rice_guard_core::init::generate(&input)
        .await
        .map_err(|e| anyhow::anyhow!("Generation failed: {e}"))?;

    print_init_summary(&detection, &created);
    Ok(0)
}

/// Print a summary table after successful init.
fn print_init_summary(detection: &DetectionResult, created: &[PathBuf]) {
    output::print_success("\nrice-guard initialized successfully!\n");

    // Languages detected
    if detection.languages.is_empty() {
        output::print_info("  Languages: none detected");
    } else {
        output::print_info("  Languages detected:");
        for lang in &detection.languages {
            output::print_info(&format!("    - {} ({} files)", lang.name, lang.files));
        }
    }

    // Scanners available
    let available_scanners: Vec<&str> = detection
        .scanner_probes
        .iter()
        .filter(|(_, result)| {
            matches!(
                result,
                rice_guard_core::registry::probe::ProbeResult::Available { .. }
            )
        })
        .map(|(name, _)| name.as_str())
        .collect();

    let missing_scanners: Vec<&str> = detection
        .scanner_probes
        .iter()
        .filter(|(_, result)| {
            !matches!(
                result,
                rice_guard_core::registry::probe::ProbeResult::Available { .. }
            )
        })
        .map(|(name, _)| name.as_str())
        .collect();

    if !available_scanners.is_empty() {
        output::print_info(&format!(
            "  Scanners available: {}",
            available_scanners.join(", ")
        ));
    }
    if !missing_scanners.is_empty() {
        output::print_warning(&format!(
            "  Scanners missing (install to enable): {}",
            missing_scanners.join(", ")
        ));
    }

    // Files generated
    if !created.is_empty() {
        output::print_info("\n  Files generated:");
        for path in created {
            output::print_success(&format!("    + {}", path.display()));
        }
    }

    output::print_info("\nRun `rice-guard scan .` to scan your project.");
}
