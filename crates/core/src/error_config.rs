//! Configuration-related errors.
//!
//! Error codes: 4xxx

use thiserror::Error;
use crate::error::Error;

/// Configuration-related errors
#[derive(Error, Debug, Clone)]
#[error(transparent)]
pub struct ConfigError {
    #[from]
    inner: ConfigErrorKind,
}

#[derive(Error, Debug, Clone)]
pub enum ConfigErrorKind {
    /// A config key was not found in the loaded configuration.
    /// The key syntax is valid, but no value exists at that path.
    /// Exit code: 40
    #[error("Config key not found: {0}")]
    ConfigKeyNotFound(String),

    /// A config key or value failed parsing/validation.
    /// Covers: empty keys, non-ASCII characters, invalid segment characters,
    /// missing dot separator, overflow integers, malformed arrays,
    /// non-string array elements, resulting TOML would be invalid.
    /// Exit code: 41
    #[error("Config parse error: {0}")]
    ConfigParseError(String),

    /// A config file could not be written.
    /// Covers: permission denied, disk full, parent directory creation failure,
    /// file open failure, seek failure, write failure, flush failure.
    /// Exit code: 42
    #[error("Config write error: {0}")]
    ConfigWriteError(String),

    /// An operation was attempted against an invalid or unsupported scope.
    /// E.g., attempting to save to Env scope, or project scope when no
    /// project config path is available.
    /// Exit code: 43
    #[error("Config scope error: {0}")]
    ConfigScopeError(String),

    /// A file lock could not be acquired within the timeout.
    /// Indicates another process is holding the lock on the config file.
    /// Exit code: 44
    #[error("Config lock error: {0}")]
    ConfigLockError(String),

    /// Generic not found (config file or directory).
    /// Preserved for backward compatibility.
    /// Exit code: 40
    #[error("Configuration not found: {0}")]
    NotFound(String),

    /// Generic invalid configuration.
    /// Preserved for backward compatibility.
    /// Exit code: 41
    #[error("Configuration invalid: {0}")]
    Invalid(String),

    /// Permission denied on config file or directory.
    /// Preserved for backward compatibility.
    /// Exit code: 42
    #[error("Configuration permission denied: {0}")]
    Permission(String),
}

impl From<ConfigErrorKind> for Error {
    fn from(e: ConfigErrorKind) -> Self {
        Error::Config(e.into())
    }
}

// ========================================================================
// Exit Code
// ========================================================================

impl ConfigError {
    /// Returns exit code for CLI.
    pub fn exit_code(&self) -> i32 {
        match self.inner {
            ConfigErrorKind::ConfigKeyNotFound(_) => 40,
            ConfigErrorKind::ConfigParseError(_) => 41,
            ConfigErrorKind::ConfigWriteError(_) => 42,
            ConfigErrorKind::ConfigScopeError(_) => 43,
            ConfigErrorKind::ConfigLockError(_) => 44,
            ConfigErrorKind::NotFound(_) => 40,
            ConfigErrorKind::Invalid(_) => 41,
            ConfigErrorKind::Permission(_) => 42,
        }
    }

    /// Returns a reference to the inner error kind.
    pub fn kind(&self) -> &ConfigErrorKind {
        &self.inner
    }
}
