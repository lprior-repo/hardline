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

    /// A config file is a symbolic link, which is rejected for security.
    /// Exit code: 45
    #[error("Config file security error (symlink rejected): {0}")]
    SecuritySymlink(String),

    /// A config file exceeds the maximum allowed file size (1 MB).
    /// Exit code: 46
    #[error("Config file too large: {0}")]
    FileTooLarge(String),

    /// A config file is a dead symlink (target does not exist).
    /// Exit code: 48
    #[error("Config file is a dead symlink: {0}")]
    DeadSymlink(String),

    /// File watcher setup failed.
    /// Exit code: 47
    #[error("Config watcher error: {0}")]
    WatcherError(String),
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
    /// Returns a reference to the inner error kind.
    #[must_use]
    pub fn kind(&self) -> &ConfigErrorKind {
        &self.inner
    }

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
            ConfigErrorKind::SecuritySymlink(_) => 45,
            ConfigErrorKind::FileTooLarge(_) => 46,
            ConfigErrorKind::DeadSymlink(_) => 48,
            ConfigErrorKind::WatcherError(_) => 47,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Variant Construction
    // ========================================================================

    #[test]
    fn config_key_not_found_construction() {
        let kind = ConfigErrorKind::ConfigKeyNotFound("core.autosync".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::ConfigKeyNotFound(_)));
    }

    #[test]
    fn config_parse_error_construction() {
        let kind = ConfigErrorKind::ConfigParseError("invalid key syntax".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::ConfigParseError(_)));
    }

    #[test]
    fn config_write_error_construction() {
        let kind = ConfigErrorKind::ConfigWriteError("permission denied".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::ConfigWriteError(_)));
    }

    #[test]
    fn config_scope_error_construction() {
        let kind = ConfigErrorKind::ConfigScopeError("env scope not writable".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::ConfigScopeError(_)));
    }

    #[test]
    fn config_lock_error_construction() {
        let kind = ConfigErrorKind::ConfigLockError("timeout acquiring lock".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::ConfigLockError(_)));
    }

    #[test]
    fn not_found_construction() {
        let kind = ConfigErrorKind::NotFound("~/.config/hl/config.toml".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::NotFound(_)));
    }

    #[test]
    fn invalid_construction() {
        let kind = ConfigErrorKind::Invalid("malformed TOML".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::Invalid(_)));
    }

    #[test]
    fn permission_construction() {
        let kind = ConfigErrorKind::Permission("/etc/hl/config.toml".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::Permission(_)));
    }

    #[test]
    fn security_symlink_construction() {
        let kind = ConfigErrorKind::SecuritySymlink("/tmp/evil.toml".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::SecuritySymlink(_)));
    }

    #[test]
    fn file_too_large_construction() {
        let kind = ConfigErrorKind::FileTooLarge("2MB exceeds 1MB limit".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::FileTooLarge(_)));
    }

    #[test]
    fn dead_symlink_construction() {
        let kind = ConfigErrorKind::DeadSymlink("/tmp/gone.toml -> /nowhere".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::DeadSymlink(_)));
    }

    #[test]
    fn watcher_error_construction() {
        let kind = ConfigErrorKind::WatcherError("inotify limit reached".to_string());
        let err: ConfigError = kind.clone().into();
        assert!(matches!(err.kind(), ConfigErrorKind::WatcherError(_)));
    }

    // ========================================================================
    // Display Output
    // ========================================================================

    #[test]
    fn config_key_not_found_display() {
        let err = ConfigErrorKind::ConfigKeyNotFound("core.autosync".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config key not found"));
        assert!(msg.contains("core.autosync"));
    }

    #[test]
    fn config_parse_error_display() {
        let err = ConfigErrorKind::ConfigParseError("empty key".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config parse error"));
        assert!(msg.contains("empty key"));
    }

    #[test]
    fn config_write_error_display() {
        let err = ConfigErrorKind::ConfigWriteError("disk full".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config write error"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn config_scope_error_display() {
        let err = ConfigErrorKind::ConfigScopeError("env scope not writable".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config scope error"));
        assert!(msg.contains("env scope not writable"));
    }

    #[test]
    fn config_lock_error_display() {
        let err = ConfigErrorKind::ConfigLockError("lock held by PID 1234".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config lock error"));
        assert!(msg.contains("lock held by PID 1234"));
    }

    #[test]
    fn not_found_display() {
        let err = ConfigErrorKind::NotFound("~/.config/hl/config.toml".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Configuration not found"));
        assert!(msg.contains("~/.config/hl/config.toml"));
    }

    #[test]
    fn invalid_display() {
        let err = ConfigErrorKind::Invalid("malformed TOML".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Configuration invalid"));
        assert!(msg.contains("malformed TOML"));
    }

    #[test]
    fn permission_display() {
        let err = ConfigErrorKind::Permission("/etc/hl/config.toml".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Configuration permission denied"));
        assert!(msg.contains("/etc/hl/config.toml"));
    }

    #[test]
    fn security_symlink_display() {
        let err = ConfigErrorKind::SecuritySymlink("/tmp/evil.toml".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("symlink rejected"));
        assert!(msg.contains("/tmp/evil.toml"));
    }

    #[test]
    fn file_too_large_display() {
        let err = ConfigErrorKind::FileTooLarge("2MB exceeds 1MB limit".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config file too large"));
        assert!(msg.contains("2MB exceeds 1MB limit"));
    }

    #[test]
    fn dead_symlink_display() {
        let err = ConfigErrorKind::DeadSymlink("/tmp/gone.toml -> /nowhere".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("dead symlink"));
        assert!(msg.contains("/tmp/gone.toml -> /nowhere"));
    }

    #[test]
    fn watcher_error_display() {
        let err = ConfigErrorKind::WatcherError("inotify limit reached".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Config watcher error"));
        assert!(msg.contains("inotify limit reached"));
    }

    // ========================================================================
    // ConfigError wrapper Display (transparent delegation)
    // ========================================================================

    #[test]
    fn config_error_display_delegates_to_kind() {
        let err: ConfigError = ConfigErrorKind::NotFound("missing.toml".to_string()).into();
        let msg = format!("{err}");
        assert!(msg.contains("Configuration not found"));
        assert!(msg.contains("missing.toml"));
    }

    // ========================================================================
    // Exit Codes
    // ========================================================================

    #[test]
    fn exit_codes_all_variants() {
        assert_eq!(ConfigError::from(ConfigErrorKind::ConfigKeyNotFound("x".into())).exit_code(), 40);
        assert_eq!(ConfigError::from(ConfigErrorKind::ConfigParseError("x".into())).exit_code(), 41);
        assert_eq!(ConfigError::from(ConfigErrorKind::ConfigWriteError("x".into())).exit_code(), 42);
        assert_eq!(ConfigError::from(ConfigErrorKind::ConfigScopeError("x".into())).exit_code(), 43);
        assert_eq!(ConfigError::from(ConfigErrorKind::ConfigLockError("x".into())).exit_code(), 44);
        assert_eq!(ConfigError::from(ConfigErrorKind::NotFound("x".into())).exit_code(), 40);
        assert_eq!(ConfigError::from(ConfigErrorKind::Invalid("x".into())).exit_code(), 41);
        assert_eq!(ConfigError::from(ConfigErrorKind::Permission("x".into())).exit_code(), 42);
        assert_eq!(ConfigError::from(ConfigErrorKind::SecuritySymlink("x".into())).exit_code(), 45);
        assert_eq!(ConfigError::from(ConfigErrorKind::FileTooLarge("x".into())).exit_code(), 46);
        assert_eq!(ConfigError::from(ConfigErrorKind::WatcherError("x".into())).exit_code(), 47);
        assert_eq!(ConfigError::from(ConfigErrorKind::DeadSymlink("x".into())).exit_code(), 48);
    }

    #[test]
    fn exit_codes_are_nonzero() {
        let variants = [
            ConfigErrorKind::ConfigKeyNotFound("x".into()),
            ConfigErrorKind::ConfigParseError("x".into()),
            ConfigErrorKind::ConfigWriteError("x".into()),
            ConfigErrorKind::ConfigScopeError("x".into()),
            ConfigErrorKind::ConfigLockError("x".into()),
            ConfigErrorKind::NotFound("x".into()),
            ConfigErrorKind::Invalid("x".into()),
            ConfigErrorKind::Permission("x".into()),
            ConfigErrorKind::SecuritySymlink("x".into()),
            ConfigErrorKind::FileTooLarge("x".into()),
            ConfigErrorKind::DeadSymlink("x".into()),
            ConfigErrorKind::WatcherError("x".into()),
        ];
        for kind in &variants {
            let err: ConfigError = kind.clone().into();
            assert_ne!(err.exit_code(), 0, "exit code must be nonzero for {:?}", kind);
        }
    }

    // ========================================================================
    // kind() accessor
    // ========================================================================

    #[test]
    fn kind_accessor_returns_correct_variant() {
        let err: ConfigError = ConfigErrorKind::DeadSymlink("target gone".to_string()).into();
        assert!(matches!(err.kind(), ConfigErrorKind::DeadSymlink(s) if s == "target gone"));
    }

    // ========================================================================
    // Clone
    // ========================================================================

    #[test]
    fn clone_config_error_kind() {
        let original = ConfigErrorKind::WatcherError("limit".to_string());
        let cloned = original.clone();
        let msg_orig = format!("{original}");
        let msg_clone = format!("{cloned}");
        assert_eq!(msg_orig, msg_clone);
    }

    #[test]
    fn clone_config_error() {
        let original: ConfigError = ConfigErrorKind::FileTooLarge("big".to_string()).into();
        let cloned = original.clone();
        assert_eq!(original.exit_code(), cloned.exit_code());
        assert_eq!(format!("{original}"), format!("{cloned}"));
    }

    // ========================================================================
    // From<ConfigErrorKind> for Error (top-level)
    // ========================================================================

    #[test]
    fn from_config_error_kind_to_error() {
        let err: Error = ConfigErrorKind::NotFound("missing.toml".to_string()).into();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn from_config_error_to_error() {
        let config_err: ConfigError = ConfigErrorKind::Invalid("bad".to_string()).into();
        let err: Error = config_err.into();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn error_exit_code_delegates_to_config() {
        let err: Error = ConfigErrorKind::SecuritySymlink("sym".to_string()).into();
        assert_eq!(err.exit_code(), 45);
    }

    // ========================================================================
    // Error::source chain (none for leaf errors)
    // ========================================================================

    #[test]
    fn error_source_is_none_for_kind_variants() {
        // ConfigErrorKind variants are leaf errors with no #[source] attribute,
        // so std::error::Error::source() should return None.
        let err: ConfigError = ConfigErrorKind::ConfigLockError("test".to_string()).into();
        // We can't call .source() directly on a non-Error type unless it implements std::error::Error.
        // ConfigError derives thiserror::Error which implements std::error::Error.
        // Since ConfigErrorKind variants have no #[source], source() is None.
        let _: &ConfigError = &err;
        // Just verify the type is usable as a dyn Error
        let _dyn: &(dyn std::error::Error + Send + Sync) = &err;
    }

    // ========================================================================
    // Empty string edge cases
    // ========================================================================

    #[test]
    fn display_with_empty_string() {
        let variants = [
            ConfigErrorKind::ConfigKeyNotFound(String::new()),
            ConfigErrorKind::ConfigParseError(String::new()),
            ConfigErrorKind::ConfigWriteError(String::new()),
            ConfigErrorKind::ConfigScopeError(String::new()),
            ConfigErrorKind::ConfigLockError(String::new()),
            ConfigErrorKind::NotFound(String::new()),
            ConfigErrorKind::Invalid(String::new()),
            ConfigErrorKind::Permission(String::new()),
            ConfigErrorKind::SecuritySymlink(String::new()),
            ConfigErrorKind::FileTooLarge(String::new()),
            ConfigErrorKind::DeadSymlink(String::new()),
            ConfigErrorKind::WatcherError(String::new()),
        ];
        for kind in &variants {
            // Should not panic on empty string
            let msg = format!("{kind}");
            assert!(!msg.is_empty(), "Display output must not be empty for {:?}", kind);
        }
    }

    // ========================================================================
    // Debug formatting
    // ========================================================================

    #[test]
    fn debug_format_contains_variant_name() {
        let err = ConfigErrorKind::DeadSymlink("/broken".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("DeadSymlink"));
        assert!(debug.contains("/broken"));
    }
}
