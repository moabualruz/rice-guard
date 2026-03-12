/// SonarQube integration submodule.
///
/// Provides the HTTP client, API response models, and SARIF converter used
/// by the `enroll` and `report` commands.
pub mod client;
pub mod converter;
pub mod models;

// Re-exported for use by enroll/report commands in Plan 02.
#[allow(unused_imports)]
pub use client::SonarClient;
