// Probe implementation — implemented in Task 2.
use crate::errors::DescriptorError;
use crate::registry::fixer_descriptor::FixerStep;
use crate::registry::scanner_descriptor::ScannerDescriptor;

/// Result of probing whether a tool is installed and meets version requirements.
#[derive(Debug, Clone)]
pub enum ProbeResult {
    /// Tool is present and meets the version constraint.
    Available { version: String },
    /// Tool is present but its version is older than required.
    Outdated { found: String, required: String },
    /// Tool binary not found in PATH.
    Missing,
    /// Tool was found but the probe command failed or produced unrecognisable output.
    Error(String),
}

/// Build a `tokio::process::Command` that correctly handles Windows `.cmd`/`.bat`
/// wrappers (e.g. npm-installed tools like `jscpd`).
///
/// On Windows, `CreateProcessW` does not consult `PATHEXT` so bare names like
/// `"jscpd"` fail to resolve to `jscpd.cmd`. We use `which::which` (which does
/// respect `PATHEXT`) to find the real path, then route `.cmd`/`.bat` through
/// `cmd /c` so Windows can execute them.
pub fn resolve_command(exe: &str) -> tokio::process::Command {
    #[cfg(windows)]
    {
        if let Ok(resolved) = which::which(exe) {
            let ext = resolved
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if ext == "cmd" || ext == "bat" {
                let mut cmd = tokio::process::Command::new("cmd");
                cmd.arg("/c").arg(&resolved);
                return cmd;
            }
            return tokio::process::Command::new(resolved);
        }
        // which failed — fall through to bare name so the caller gets NotFound
        tokio::process::Command::new(exe)
    }
    #[cfg(not(windows))]
    {
        tokio::process::Command::new(exe)
    }
}

/// Build an argument vector from a command template by substituting `{{placeholder}}`
/// tokens with their values and then splitting the result via `shlex`.
///
/// The substitution is performed **before** shell-word splitting so that values
/// containing spaces or shell metacharacters are treated as single opaque tokens —
/// preventing shell injection.
///
/// # Example
/// ```
/// # use rice_guard_core::registry::probe::build_command;
/// let args = build_command(
///     "scc --format json {{output_dir}}",
///     &[("output_dir", "my dir/out")],
/// ).unwrap();
/// assert_eq!(args, ["scc", "--format", "json", "my dir/out"]);
/// ```
pub fn build_command(
    cmd_template: &str,
    vars: &[(&str, &str)],
) -> Result<Vec<String>, DescriptorError> {
    if cmd_template.trim().is_empty() {
        return Err(DescriptorError::EmptyCommand);
    }

    // Split the template into tokens first.  Each token may contain a
    // {{placeholder}} which we then substitute in-place.  This means a value
    // like "my dir/out" remains a single token — it is never re-split.
    let tokens = shlex::split(cmd_template).ok_or_else(|| {
        DescriptorError::InvalidCommandTemplate(format!("shlex could not parse: {cmd_template}"))
    })?;

    if tokens.is_empty() {
        return Err(DescriptorError::EmptyCommand);
    }

    let result: Vec<String> = tokens
        .into_iter()
        .map(|token| {
            let mut out = token;
            for (key, value) in vars {
                let placeholder = format!("{{{{{key}}}}}");
                out = out.replace(&placeholder, value);
            }
            out
        })
        .collect();

    Ok(result)
}

/// Probe whether a scanner tool is installed and meets its version requirement.
///
/// Executes `descriptor.install.check` via the OS (not a shell) and parses
/// the version from its output. Returns `ProbeResult::Missing` rather than
/// panicking when the binary is not found.
pub async fn probe_scanner(descriptor: &ScannerDescriptor) -> ProbeResult {
    let tokens = match shlex::split(&descriptor.install.check) {
        Some(t) if !t.is_empty() => t,
        _ => {
            return ProbeResult::Error(format!(
                "invalid check command: {}",
                descriptor.install.check
            ))
        }
    };

    let output = match resolve_command(&tokens[0])
        .args(&tokens[1..])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ProbeResult::Missing,
        Err(e) => return ProbeResult::Error(e.to_string()),
    };

    let raw = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);

    parse_version_result(raw.trim(), &descriptor.version)
}

/// Probe whether the tool referenced by a fixer step's `check` command exists
/// in PATH.  Uses `which` for a lightweight existence check.
pub async fn probe_fixer_step(step: &FixerStep) -> ProbeResult {
    let tokens = match shlex::split(&step.check) {
        Some(t) if !t.is_empty() => t,
        _ => return ProbeResult::Error(format!("invalid check command: {}", step.check)),
    };

    match which::which(&tokens[0]) {
        Ok(_) => ProbeResult::Available {
            version: String::new(),
        },
        Err(_) => ProbeResult::Missing,
    }
}

/// Extract a semver version from raw command output and compare it against a
/// version requirement string (e.g. `">=1.60"`).
fn parse_version_result(output: &str, version_req: &str) -> ProbeResult {
    // Find the first semver-like token in the output.
    let version_str = output
        .split_whitespace()
        .find(|token| {
            let cleaned = token.trim_start_matches('v');
            semver::Version::parse(cleaned).is_ok()
        })
        .map(|t| t.trim_start_matches('v').to_string());

    let Some(ver_str) = version_str else {
        // Output present but no parseable version — treat as available (unknown version).
        return ProbeResult::Available {
            version: output.lines().next().unwrap_or("unknown").to_string(),
        };
    };

    let Ok(ver) = semver::Version::parse(&ver_str) else {
        return ProbeResult::Available { version: ver_str };
    };

    let req = match semver::VersionReq::parse(version_req) {
        Ok(r) => r,
        Err(_) => {
            // Version requirement unparseable — just report available.
            return ProbeResult::Available {
                version: ver.to_string(),
            };
        }
    };

    if req.matches(&ver) {
        ProbeResult::Available {
            version: ver.to_string(),
        }
    } else {
        ProbeResult::Outdated {
            found: ver.to_string(),
            required: version_req.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── build_command tests ──────────────────────────────────────────────────

    #[test]
    fn build_command_basic_substitution() {
        let args = build_command(
            "scc --format json {{output_dir}}",
            &[("output_dir", "reports/out")],
        )
        .unwrap();
        assert_eq!(args, ["scc", "--format", "json", "reports/out"]);
    }

    #[test]
    fn build_command_spaces_in_value_stay_single_token() {
        let args = build_command(
            "scc --format json {{output_dir}}",
            &[("output_dir", "my dir/out")],
        )
        .unwrap();
        // "my dir/out" must not be split into two tokens.
        assert_eq!(
            args,
            ["scc", "--format", "json", "my dir/out"],
            "path with space must remain one token"
        );
    }

    #[test]
    fn build_command_injection_attempt_is_verbatim_token() {
        let args = build_command("scc {{output_dir}}", &[("output_dir", "dir; rm -rf /")]).unwrap();
        // The injection payload must be treated as a literal argument, not shell-interpreted.
        assert_eq!(
            args,
            ["scc", "dir; rm -rf /"],
            "injection payload must be a single verbatim token"
        );
    }

    #[test]
    fn build_command_no_vars() {
        let args = build_command("semgrep --version", &[]).unwrap();
        assert_eq!(args, ["semgrep", "--version"]);
    }

    #[test]
    fn build_command_empty_template_returns_empty_command_error() {
        let result = build_command("", &[]);
        assert!(
            matches!(result, Err(DescriptorError::EmptyCommand)),
            "empty template must return EmptyCommand"
        );
    }

    #[test]
    fn build_command_multiple_placeholders() {
        let args = build_command(
            "scanner --out {{output_dir}} --target {{target}}",
            &[("output_dir", "out/dir"), ("target", "src/")],
        )
        .unwrap();
        assert_eq!(args, ["scanner", "--out", "out/dir", "--target", "src/"]);
    }

    // ── probe_scanner tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn probe_scanner_missing_tool_returns_missing() {
        use crate::registry::scanner_descriptor::{
            ScannerCommand, ScannerCommands, ScannerInstall,
        };
        use std::collections::HashMap;

        let desc = ScannerDescriptor {
            name: "nonexistent-tool-xyz".to_string(),
            version: ">=1.0".to_string(),
            languages: vec!["rust".to_string()],
            install: ScannerInstall {
                check: "nonexistent-tool-xyz-abc123 --version".to_string(),
                methods: HashMap::new(),
            },
            commands: ScannerCommands {
                scan: ScannerCommand {
                    cmd: "nonexistent-tool-xyz-abc123 scan .".to_string(),
                    timeout: None,
                },
                security: None,
                quick: None,
            },
            output_format: "json".to_string(),
            severity_map: HashMap::new(),
        };

        let result = probe_scanner(&desc).await;
        assert!(
            matches!(result, ProbeResult::Missing),
            "non-existent tool must return Missing, got: {result:?}"
        );
    }

    // ── probe_fixer_step tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn probe_fixer_step_missing_tool_returns_missing() {
        let step = FixerStep {
            name: "nonexistent-fixer".to_string(),
            check: "nonexistent-fixer-xyz-abc123 --check .".to_string(),
            fix: "nonexistent-fixer-xyz-abc123 .".to_string(),
            scope: "project".to_string(),
            safe: true,
            unsafe_flag: None,
        };

        let result = probe_fixer_step(&step).await;
        assert!(
            matches!(result, ProbeResult::Missing),
            "non-existent fixer must return Missing, got: {result:?}"
        );
    }
}
