/// Subcommand handler modules.
///
/// Each module contains a single `run` function that accepts the
/// subcommand's `*Args` struct and returns `anyhow::Result<i32>`
/// where the `i32` is the process exit code (0, 1, or 2).
pub mod enroll;
pub mod fix;
pub mod init;
pub mod mcp;
pub mod report;
pub mod scan;
pub mod serve;
pub mod status;
pub mod version;
