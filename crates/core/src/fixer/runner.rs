/// Single-fixer execution: check → fix → verify cycle.
///
/// `run_one_fixer` implements the check-fix-verify cycle for a single
/// fixer tool descriptor entry. The full signature and implementation
/// will be completed in Plan 03.
use crate::fixer::report::FixToolResult;

/// Execute a single fixer tool and return the result.
///
/// Full signature and implementation are defined in Plan 03.
/// This stub establishes the return type so engine.rs can reference it.
#[allow(dead_code)]
pub async fn run_one_fixer() -> FixToolResult {
    todo!("run_one_fixer() — signature + implementation in Plan 03")
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 03"]
    fn check_fix_verify_cycle() {
        todo!("RED stub — verify check runs first, fix applied only when check finds issues, verify runs after fix");
    }

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 03"]
    fn dry_run_no_file_changes() {
        todo!("RED stub — verify no files are modified when dry_run=true");
    }

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 03"]
    fn unavailable_tool_skips() {
        todo!("RED stub — verify unavailable fixer tools produce Skipped status");
    }

    #[test]
    #[ignore = "RED stub — will turn GREEN in Plan 03"]
    fn file_targets_substituted() {
        todo!("RED stub — verify {{files}} placeholder in fixer commands is substituted with actual file targets");
    }
}
