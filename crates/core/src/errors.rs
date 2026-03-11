use thiserror::Error;

/// Errors when loading or validating the `.riceguard.yaml` config file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Config file does not exist at the given path.
    #[error("Config file not found at {path}")]
    NotFound { path: String },

    /// I/O error while reading the config file.
    #[error("Failed to read config at {path}: {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// YAML parse error.
    #[error("Failed to parse config at {path}: {reason}")]
    ParseError { path: String, reason: String },

    /// Semantic validation error (e.g., missing required field).
    #[error("Invalid config: {message}")]
    ValidationError { message: String },
}

/// Errors when loading or validating scanner/fixer descriptor YAML files.
#[derive(Debug, Error)]
pub enum DescriptorError {
    /// I/O error while reading a descriptor file.
    #[error("Failed to read descriptor at {path}: {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// YAML parse error in a descriptor file.
    #[error("Failed to parse descriptor at {path}: {reason}")]
    ParseError { path: String, reason: String },

    /// Semantic validation error in a descriptor.
    #[error("Invalid descriptor '{name}': {reason}")]
    ValidationError { name: String, reason: String },

    /// A command template string is syntactically invalid.
    #[error("Invalid command template: {0}")]
    InvalidCommandTemplate(String),

    /// A descriptor defines an empty command (no executable).
    #[error("Empty command in descriptor")]
    EmptyCommand,
}

/// Errors that occur during the `init` command flow.
#[derive(Debug, Error)]
pub enum InitError {
    /// `scc` executable not found in PATH.
    #[error("scc not found in PATH — install scc to enable language detection")]
    SccNotFound,

    /// `scc` exited with non-zero status or produced unparseable output.
    #[error("scc failed: {0}")]
    SccFailed(String),

    /// Caller tried to run interactive prompts but stdin is not a TTY.
    #[error("Not an interactive terminal — use --yes for non-interactive mode")]
    NotInteractive { hint: String },

    /// An `inquire` prompt returned an error.
    #[error("Wizard prompt failed: {0}")]
    WizardFailed(String),

    /// Failed to write a generated file to disk.
    #[error("Failed to generate file at {path}: {reason}")]
    GenerationFailed { path: String, reason: String },

    /// Descriptor error encountered during init.
    #[error("Descriptor error during init: {0}")]
    DescriptorError(#[from] DescriptorError),
}

/// Errors when probing whether an external tool is available.
#[derive(Debug, Error)]
pub enum ProbeError {
    /// Tool binary not found in PATH.
    #[error("Tool '{tool}' not found in PATH")]
    NotFound { tool: String },

    /// Tool found but its version does not satisfy the constraint.
    #[error("Tool '{tool}' version {found} does not satisfy {required}")]
    VersionMismatch {
        tool: String,
        found: String,
        required: String,
    },

    /// Tool probe command failed to execute or produced unparseable output.
    #[error("Failed to probe '{tool}': {reason}")]
    ProbeFailed { tool: String, reason: String },
}
