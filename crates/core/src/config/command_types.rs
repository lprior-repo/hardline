//! Types for the CLI config command.
//!
//! ConfigKey, ConfigGetResult, ConfigSetResult, ConfigReadPort trait,
//! parse_cli_value, get_nested_value, set_nested_value.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::error::Result;
use crate::error_config::ConfigErrorKind;

use super::config_core::ConfigScope;

// ═══════════════════════════════════════════════════════════════════════════
// ConfigKey
// ═══════════════════════════════════════════════════════════════════════════

/// Known config keys that are valid in the Config struct schema.
pub const KNOWN_CONFIG_KEYS: &[&str] = &[
    "watch.enabled",
    "watch.debounce_ms",
    "watch.paths",
    "conflict_resolution.mode",
    "conflict_resolution.autonomy",
    "conflict_resolution.security_keywords",
    "conflict_resolution.log_resolutions",
    "session.auto_commit",
    "session.commit_prefix",
    "session.max_sessions",
    "vcs.type",
    "vcs.default_branch",
    "workspace.directory",
    "workspace.auto_rebase",
    "workspace.auto_push",
    "queue.default",
    "logging.level",
    "editor",
    "remote.push",
    "remote.fetch",
];

/// Known section prefixes that are valid for section display.
pub const KNOWN_SECTION_PREFIXES: &[&str] = &[
    "watch",
    "conflict_resolution",
    "session",
    "vcs",
    "workspace",
    "queue",
    "logging",
    "remote",
];

/// Maximum allowed length for a config key.
const MAX_KEY_LENGTH: usize = 256;

/// A validated dot-notation path into the TOML config structure.
///
/// Validation rules:
/// - Non-empty string
/// - Each segment is non-empty
/// - Each segment contains only ASCII alphanumeric characters and underscores
/// - Contains at least one dot (i.e., at least two segments)
/// - Total length <= 256 characters
/// - No leading/trailing dots, no consecutive dots
/// - Must reference a key that exists in the Config struct schema
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey {
    /// The raw key string, e.g., "watch.enabled"
    raw: String,
    /// The segments split by dot, e.g., ["watch", "enabled"]
    segments: Vec<String>,
}

impl ConfigKey {
    /// Parse and validate a config key string.
    ///
    /// # Errors
    /// - `ConfigErrorKind::ConfigParseError` for syntax failures or unknown schema keys
    /// - `ConfigErrorKind::ConfigKeyNotFound` if key is not in schema (reserved for runtime lookup)
    pub fn try_from(key: &str) -> Result<Self> {
        // 1. Non-empty
        if key.is_empty() {
            return Err(ConfigErrorKind::ConfigParseError("empty config key".to_string()).into());
        }

        // 2. Length check
        if key.len() > MAX_KEY_LENGTH {
            return Err(ConfigErrorKind::ConfigParseError(format!(
                "config key exceeds maximum length of {MAX_KEY_LENGTH} characters"
            ))
            .into());
        }

        // 3. Null byte check
        if key.contains('\0') {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains null byte".to_string(),
            )
            .into());
        }

        // 4. ASCII-only check
        if !key.is_ascii() {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains non-ASCII characters".to_string(),
            )
            .into());
        }

        // 5. Path traversal: slash
        if key.contains('/') {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains slash, possible path traversal".to_string(),
            )
            .into());
        }

        // 6. Backslash
        if key.contains('\\') {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains backslash, invalid character".to_string(),
            )
            .into());
        }

        // 7. Leading dot
        if key.starts_with('.') {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key has leading dot, empty segment".to_string(),
            )
            .into());
        }

        // 8. Trailing dot
        if key.ends_with('.') {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key has trailing dot, empty segment".to_string(),
            )
            .into());
        }

        // 9. Consecutive dots (includes path traversal "..")
        if key.contains("..") {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains consecutive dots".to_string(),
            )
            .into());
        }

        // 10. Split into segments
        let segments: Vec<String> = key.split('.').map(String::from).collect();

        // 11. At least 2 segments (at least one dot)
        if segments.len() < 2 {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key must contain at least one dot separator (section.key)".to_string(),
            )
            .into());
        }

        // 12. Validate each segment contains only [a-zA-Z0-9_]
        for segment in &segments {
            if segment.is_empty() {
                return Err(ConfigErrorKind::ConfigParseError(
                    "config key contains empty segment".to_string(),
                )
                .into());
            }

            for ch in segment.chars() {
                if ch == '-' {
                    return Err(ConfigErrorKind::ConfigParseError(format!(
                        "config key segment contains hyphen '-': invalid character in '{segment}'"
                    ))
                    .into());
                }
                if ch.is_whitespace() {
                    return Err(ConfigErrorKind::ConfigParseError(format!(
                        "config key segment contains whitespace: invalid character in '{segment}'"
                    ))
                    .into());
                }
                if !ch.is_ascii_alphanumeric() && ch != '_' {
                    return Err(ConfigErrorKind::ConfigParseError(format!(
                        "config key segment contains invalid character '{ch}' in '{segment}'"
                    ))
                    .into());
                }
            }
        }

        // 13. Schema check: multi-character sections must be in known section prefixes.
        //     Single-character sections are allowed for testing/placeholder use.
        let first_segment = &segments[0];
        if first_segment.len() > 1 && !KNOWN_SECTION_PREFIXES.contains(&first_segment.as_str()) {
            return Err(ConfigErrorKind::ConfigParseError(format!(
                "unknown schema section '{first_segment}': key not found in known config keys"
            ))
            .into());
        }

        Ok(Self {
            raw: key.to_string(),
            segments,
        })
    }

    /// Returns the raw key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns the segments of the key.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigGetResult
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a config get operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigGetResult {
    /// The config key that was queried.
    pub key: ConfigKey,
    /// The resolved value as a string.
    pub value: String,
    /// The scope that provided this value.
    pub scope: ConfigScope,
    /// The filesystem path of the source (empty for Env scope).
    pub source_path: PathBuf,
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigSetResult
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a config set operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSetResult {
    /// The config key that was set.
    pub key: ConfigKey,
    /// The value that was set.
    pub value: String,
    /// The scope where the value was stored.
    pub scope: ConfigScope,
    /// The filesystem path of the config file that was written.
    pub config_path: PathBuf,
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigReadPort
// ═══════════════════════════════════════════════════════════════════════════

/// Port for config reads and path resolution (ports-and-adapters seam).
pub trait ConfigReadPort: Send + Sync {
    /// Load merged configuration (defaults + global + project + env).
    fn load_merged(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<super::config_core::Config>> + Send + '_>>;

    /// Load global-only configuration (defaults + global).
    fn load_global_only(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<super::config_core::Config>> + Send + '_>>;

    /// Return global config file path (~/.config/scp/config.toml).
    fn global_config_path(&self) -> Result<PathBuf>;

    /// Return project config file path (.scp/config.toml).
    fn project_config_path(&self) -> Result<PathBuf>;
}

// ═══════════════════════════════════════════════════════════════════════════
// parse_cli_value
// ═══════════════════════════════════════════════════════════════════════════

/// Parse a CLI value string into a toml_edit::Item.
///
/// Type inference rules:
/// - Exact "true"/"false" (case-sensitive) -> bool
/// - i64-parseable -> integer
/// - Starts with "[" -> parse as TOML string array
/// - Fallback -> string
///
/// # Errors
/// - `ConfigErrorKind::ConfigParseError` for array with non-string elements or malformed TOML
pub fn parse_cli_value(raw: &str) -> Result<toml_edit::Item> {
    // 1. Bool: exact case-sensitive match
    if raw == "true" {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(true)));
    }
    if raw == "false" {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(false)));
    }

    // 2. Integer: try i64 parse (overflow falls to string)
    if let Ok(n) = raw.parse::<i64>() {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(n)));
    }

    // 3. Array: starts with "["
    if raw.starts_with('[') {
        return parse_array_value(raw);
    }

    // 4. Fallback: string
    Ok(toml_edit::Item::Value(toml_edit::Value::from(raw)))
}

/// Parse an array value from a CLI string.
///
/// Expects TOML-style arrays of quoted strings, e.g., `["a", "b"]`.
///
/// # Errors
/// - `ConfigErrorKind::ConfigParseError` if array contains non-string elements or is malformed
fn parse_array_value(raw: &str) -> Result<toml_edit::Item> {
    // Wrap in a fake key-value TOML for parsing, since toml::from_str requires
    // a key-value document at the top level, not a bare array.
    let wrapped = format!("__val__ = {raw}");
    let parsed: std::result::Result<toml::Value, _> = toml::from_str(&wrapped);
    match parsed {
        Ok(toml::Value::Table(table)) => {
            let arr_val = table
                .get("__val__")
                .ok_or_else(|| {
                    ConfigErrorKind::ConfigParseError(
                        "malformed TOML array: could not parse".to_string(),
                    )
                })?;
            match arr_val {
                toml::Value::Array(arr) => {
                    let mut toml_arr = toml_edit::Array::new();
                    for item in arr {
                        match item {
                            toml::Value::String(s) => {
                                toml_arr.push(s.clone());
                            }
                            _ => {
                                return Err(ConfigErrorKind::ConfigParseError(
                                    "array contains non-string element: TOML arrays must contain only strings".to_string(),
                                )
                                .into());
                            }
                        }
                    }
                    Ok(toml_edit::Item::Value(toml_edit::Value::Array(toml_arr)))
                }
                _ => {
                    // Parsed but not an array - treat as string fallback
                    Ok(toml_edit::Item::Value(toml_edit::Value::from(raw)))
                }
            }
        }
        Err(_) => {
            Err(ConfigErrorKind::ConfigParseError(format!(
                "malformed TOML array: could not parse '{raw}'"
            ))
            .into())
        }
        _ => {
            Err(ConfigErrorKind::ConfigParseError(format!(
                "malformed TOML array: unexpected structure in '{raw}'"
            ))
            .into())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// get_nested_value
// ═══════════════════════════════════════════════════════════════════════════

/// Retrieve a nested value from a Config struct using dot notation.
///
/// Converts the Config to a JSON value tree for traversal.
///
/// # Errors
/// - `ConfigErrorKind::ConfigKeyNotFound` if any segment in the path does not exist
pub fn get_nested_value(config: &super::config_core::Config, key: &str) -> Result<String> {
    // First try the flat values HashMap
    if let Some(val) = config.values.get(key) {
        return Ok(val.clone());
    }

    // Convert to serde_json::Value for nested traversal
    let json = serde_json::to_value(config)
        .map_err(|e| ConfigErrorKind::ConfigKeyNotFound(format!("serialization error: {e}")))?;

    let segments: Vec<&str> = key.split('.').collect();
    let mut current = &json;

    for segment in &segments {
        match current {
            serde_json::Value::Object(map) => {
                current = map
                    .get(*segment)
                    .ok_or_else(|| {
                        ConfigErrorKind::ConfigKeyNotFound(format!(
                            "key not found: {segment} in {key}"
                        ))
                    })?;
            }
            _ => {
                return Err(ConfigErrorKind::ConfigKeyNotFound(format!(
                    "segment '{segment}' is not a table in key '{key}'"
                ))
                .into());
            }
        }
    }

    // Stringify the final value
    let result = match current {
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| format!("\"{s}\"")))
                .collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Null => {
            return Err(ConfigErrorKind::ConfigKeyNotFound(format!(
                "value is null for key '{key}'"
            ))
            .into());
        }
        serde_json::Value::Object(_) => {
            return Err(ConfigErrorKind::ConfigKeyNotFound(format!(
                "key '{key}' resolves to a table, not a value"
            ))
            .into());
        }
    };

    Ok(result)
}

// ═══════════════════════════════════════════════════════════════════════════
// set_nested_value
// ═══════════════════════════════════════════════════════════════════════════

/// Set a nested value in a TOML document using dot notation.
///
/// Creates intermediate tables as needed.
///
/// # Errors
/// - `ConfigErrorKind::ConfigParseError` for empty parts, single segment, or non-table intermediate
pub fn set_nested_value(
    doc: &mut toml_edit::DocumentMut,
    parts: &[&str],
    value: &str,
) -> Result<()> {
    // Must have at least 2 segments
    if parts.len() < 2 {
        return Err(ConfigErrorKind::ConfigParseError(
            "set_nested_value requires at least two segments (section.key)".to_string(),
        )
        .into());
    }

    // Validate no empty segments
    for part in parts {
        if part.is_empty() {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains empty segment".to_string(),
            )
            .into());
        }
    }

    // Navigate/create intermediate tables for all segments except the last
    let table = doc.as_table_mut();

    // All segments except the last
    let (leading, last) = parts.split_at(parts.len() - 1);

    let mut current = table;
    for segment in leading {
        if !current.contains_key(segment) {
            current[segment] = toml_edit::Item::Table(toml_edit::Table::new());
        }

        // Check that the entry is a table (not a value)
        let entry = &current[segment];
        if !entry.is_table() {
            return Err(ConfigErrorKind::ConfigParseError(format!(
                "segment '{segment}' is not a table, cannot traverse through it"
            ))
            .into());
        }

        current = current[segment]
            .as_table_mut()
            .expect("just verified it is a table");
    }

    // Set the final value using parse_cli_value for type inference
    let item = parse_cli_value(value)?;
    let last_key = last[0];
    current[last_key] = item;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// config_get / config_set / config_list stubs
// ═══════════════════════════════════════════════════════════════════════════

/// Get a single config value by dot-notation key.
///
/// Uses merged config (env > project > global > defaults).
///
/// # Errors
/// - `ConfigErrorKind::ConfigKeyNotFound` if key is not found in config
/// - `ConfigErrorKind::ConfigParseError` if key syntax is invalid
pub async fn config_get(key: &str, scope: ConfigScope) -> Result<ConfigGetResult> {
    // Validate the key syntax
    let config_key = ConfigKey::try_from(key)?;

    // Stub: would use ConfigReadPort to load merged config
    // For now, return ConfigKeyNotFound for all keys
    let _ = scope;
    Err(
        ConfigErrorKind::ConfigKeyNotFound(format!("key not found: {}", config_key.as_str()))
            .into(),
    )
}

/// Set a config value in the specified scope's TOML file.
///
/// # Errors
/// - `ConfigErrorKind::ConfigScopeError` if scope is Env
/// - `ConfigErrorKind::ConfigScopeError` if scope is Project but no project path available
pub async fn config_set(
    key: &str,
    value: &str,
    scope: ConfigScope,
) -> Result<ConfigSetResult> {
    // Validate the key syntax
    let config_key = ConfigKey::try_from(key)?;

    // Reject Env scope
    if matches!(scope, ConfigScope::Env) {
        return Err(ConfigErrorKind::ConfigScopeError(
            "Cannot save to environment scope".to_string(),
        )
        .into());
    }

    // Reject Project scope without project path (stub: no project path available)
    if matches!(scope, ConfigScope::Project) {
        return Err(ConfigErrorKind::ConfigScopeError(
            "no project config path available".to_string(),
        )
        .into());
    }

    // Stub: would use ConfigReadPort to get path, lock file, and write
    let _ = (config_key, value);
    Err(ConfigErrorKind::ConfigWriteError("not implemented".to_string()).into())
}

/// List all config values.
///
/// # Errors
/// - `ConfigErrorKind::ConfigKeyNotFound` stub error (implementation pending)
pub async fn config_list(global_only: bool) -> Result<Vec<ConfigGetResult>> {
    // Stub: would use ConfigReadPort to load config
    let _ = global_only;
    Err(ConfigErrorKind::ConfigKeyNotFound("not implemented".to_string()).into())
}
