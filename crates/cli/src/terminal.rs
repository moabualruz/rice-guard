/// Terminal output helpers for scan progress and summary reporting.
///
/// Provides per-scanner progress bars (suppressed in non-TTY environments),
/// a colored summary table printed after scan completes, and TTY detection.
use std::collections::HashMap;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use owo_colors::Stream;

/// Returns `true` when stdout is a TTY (interactive terminal).
///
/// Returns `false` in CI pipelines, when stdout is piped, or when
/// the `NO_COLOR` env var is set. Progress bars and ANSI codes are
/// suppressed when this returns `false`.
pub fn is_tty() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

/// Per-scanner outcome collected after the scan engine completes.
///
/// Used to populate the summary table rows.
#[derive(Debug, Clone)]
pub struct ScannerResult {
    /// Scanner name (e.g. "semgrep", "trivy").
    pub name: String,
    /// Whether the scanner succeeded, failed, or was skipped.
    pub status: ScannerStatus,
    /// Total number of findings produced by this scanner.
    pub findings: usize,
    /// Findings with severity "error".
    pub high: usize,
    /// Findings with severity "warning".
    pub medium: usize,
    /// Findings with severity "info".
    pub low: usize,
    /// Findings with severity other than error/warning/info.
    pub info: usize,
    /// Wall-clock time for this scanner in milliseconds.
    pub duration_ms: u64,
    /// Brief error description for failed scanners (shown in table row).
    pub error_msg: Option<String>,
}

/// Status of a scanner run.
#[derive(Debug, Clone, PartialEq)]
pub enum ScannerStatus {
    /// Scanner ran and returned results (possibly zero findings).
    Success,
    /// Scanner ran but encountered an error.
    Failed,
    /// Scanner was not in the selected mode subset or was unavailable.
    // Constructed by callers that have richer scanner metadata (future use).
    #[allow(dead_code)]
    Skipped,
}

/// Manages a set of per-scanner `indicatif` progress bars during parallel scan
/// execution.
///
/// When stdout is not a TTY, `new()` still constructs the struct but no bars
/// are rendered — all methods become no-ops.
pub struct ScanProgressReporter {
    mp: MultiProgress,
    bars: HashMap<String, ProgressBar>,
    enabled: bool,
}

impl ScanProgressReporter {
    /// Create a new reporter with one spinner bar per scanner name.
    ///
    /// If `is_tty()` is `false`, the bars are created but immediately hidden.
    pub fn new(scanner_names: &[String]) -> Self {
        let enabled = is_tty();
        let mp = MultiProgress::new();
        let mut bars = HashMap::new();
        if enabled {
            for name in scanner_names {
                let pb = mp.add(ProgressBar::new_spinner());
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template(&format!("  {{spinner:.cyan}} {} {{msg}}", name))
                        .unwrap_or_else(|_| ProgressStyle::default_spinner()),
                );
                pb.set_message("waiting...");
                pb.enable_steady_tick(std::time::Duration::from_millis(80));
                bars.insert(name.clone(), pb);
            }
        }
        Self { mp, bars, enabled }
    }

    /// Mark a scanner as actively running (updates its spinner message).
    pub fn set_running(&self, name: &str) {
        if let Some(pb) = self.bars.get(name) {
            pb.set_message("running...");
        }
    }

    /// Finish a scanner's spinner with a completion message.
    // Used by callers with per-scanner completion callbacks (future use).
    #[allow(dead_code)]
    pub fn finish_scanner(&self, name: &str, msg: &str) {
        if let Some(pb) = self.bars.get(name) {
            pb.finish_with_message(msg.to_string());
        }
    }

    /// Clear all progress bars from the terminal.
    ///
    /// Call this before printing the summary table so the bars don't
    /// interfere with the table layout.
    pub fn clear(&self) {
        if self.enabled {
            let _ = self.mp.clear();
        }
    }
}

/// Print the post-scan summary table to stdout.
///
/// If `is_tty()` is `false` (CI, pipe, `NO_COLOR`), output is plain text
/// with no ANSI escape codes.
///
/// # Arguments
///
/// * `mode_label` — human-readable mode name, e.g. `"Full scan"` or `"Quick scan"`
/// * `results` — one `ScannerResult` per scanner that was attempted
/// * `output_dir_path` — path to the scan output directory (printed at footer)
pub fn print_scan_summary_table(
    mode_label: &str,
    results: &[ScannerResult],
    output_dir_path: &str,
) {
    let tty = is_tty();

    // Header line: "Full scan: 5 scanner(s)"
    if tty {
        println!("\n{}: {} scanner(s)\n", mode_label.bold(), results.len());
    } else {
        println!("\n{}: {} scanner(s)\n", mode_label, results.len());
    }

    // Column header
    let header = format!(
        "  {:<15} {:<8} {:<10} {:<28} {}",
        "Scanner", "Status", "Findings", "Severity", "Duration"
    );
    println!(
        "{}",
        if tty {
            header
                .if_supports_color(Stream::Stdout, |t| t.dimmed())
                .to_string()
        } else {
            header
        }
    );
    println!("  {}", "\u{2500}".repeat(70));

    let mut total_findings = 0usize;
    let mut total_high = 0usize;
    let mut total_medium = 0usize;
    let mut total_low = 0usize;
    let mut total_info = 0usize;

    for r in results {
        total_findings += r.findings;
        total_high += r.high;
        total_medium += r.medium;
        total_low += r.low;
        total_info += r.info;

        let status_str = match r.status {
            ScannerStatus::Success => {
                if tty {
                    "\u{2713}"
                        .if_supports_color(Stream::Stdout, |t| t.green())
                        .to_string()
                } else {
                    "ok".to_string()
                }
            }
            ScannerStatus::Failed => {
                if tty {
                    "\u{2717}"
                        .if_supports_color(Stream::Stdout, |t| t.red())
                        .to_string()
                } else {
                    "FAIL".to_string()
                }
            }
            ScannerStatus::Skipped => {
                if tty {
                    "\u{2013}"
                        .if_supports_color(Stream::Stdout, |t| t.dimmed())
                        .to_string()
                } else {
                    "skip".to_string()
                }
            }
        };

        // Severity breakdown: "5H 12M 3L 2I" (color-coded in TTY)
        let sev = if tty {
            let h = format!("{}H", r.high)
                .if_supports_color(Stream::Stdout, |t| t.red())
                .to_string();
            let m = format!("{}M", r.medium)
                .if_supports_color(Stream::Stdout, |t| t.yellow())
                .to_string();
            let l = format!("{}L", r.low)
                .if_supports_color(Stream::Stdout, |t| t.blue())
                .to_string();
            let i = format!("{}I", r.info)
                .if_supports_color(Stream::Stdout, |t| t.dimmed())
                .to_string();
            format!("{h} {m} {l} {i}")
        } else {
            format!("{}H {}M {}L {}I", r.high, r.medium, r.low, r.info)
        };

        let error_suffix = r
            .error_msg
            .as_deref()
            .map(|e| format!(" ({})", e))
            .unwrap_or_default();
        let duration = format!("{:.1}s", r.duration_ms as f64 / 1000.0);

        println!(
            "  {:<15} {:<8} {:<10} {:<28} {}{}",
            r.name, status_str, r.findings, sev, duration, error_suffix
        );
    }

    // Totals row
    println!("  {}", "\u{2500}".repeat(70));
    let total_sev = if tty {
        let h = format!("{}H", total_high)
            .if_supports_color(Stream::Stdout, |t| t.red())
            .to_string();
        let m = format!("{}M", total_medium)
            .if_supports_color(Stream::Stdout, |t| t.yellow())
            .to_string();
        let l = format!("{}L", total_low)
            .if_supports_color(Stream::Stdout, |t| t.blue())
            .to_string();
        let i = format!("{}I", total_info)
            .if_supports_color(Stream::Stdout, |t| t.dimmed())
            .to_string();
        format!("{h} {m} {l} {i}")
    } else {
        format!(
            "{}H {}M {}L {}I",
            total_high, total_medium, total_low, total_info
        )
    };

    let total_label = if tty {
        "Total".bold().to_string()
    } else {
        "Total".to_string()
    };
    println!(
        "  {:<15} {:<8} {:<10} {:<28}",
        total_label, "", total_findings, total_sev
    );
    println!();
    println!("  Output: {}", output_dir_path);
    println!();
}
