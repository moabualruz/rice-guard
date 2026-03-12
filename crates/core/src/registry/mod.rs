pub mod fixer_descriptor;
pub mod loader;
pub mod probe;
pub mod scanner_descriptor;

pub use fixer_descriptor::{FixerDescriptor, FixerStages, FixerStep};
pub use scanner_descriptor::{ScannerCommand, ScannerCommands, ScannerDescriptor, ScannerInstall};

/// Loaded and validated descriptor set used at fix-pipeline runtime.
///
/// Holds the full set of fixer descriptors available for a project.
/// Constructed once at startup and passed to [`crate::fixer::FixerEngine`].
#[derive(Debug, Clone, Default)]
pub struct DescriptorRegistry {
    /// All fixer descriptors (built-in + user-provided), keyed by language.
    pub fixers: Vec<FixerDescriptor>,
}

impl DescriptorRegistry {
    /// Create a registry from a pre-loaded fixer descriptor list.
    pub fn new(fixers: Vec<FixerDescriptor>) -> Self {
        Self { fixers }
    }
}
