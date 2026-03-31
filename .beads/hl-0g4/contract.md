bead_id: hl-0g4
bead_title: Port CLI: config command
phase: state-1-contract
updated_at: 2026-03-31T12:00:00Z

---

# Contract Specification: CLI Config Command

## Context

- **Feature**: Port the isolate `config` command to the hardline SCP CLI, replacing the current flat-key-value implementation with dot-notation, typed values, file locking, JSON envelope output, and scoped config (global/project/env).
- **Source**: `/home/lewis/src/isolate/crates/isolate/src/commands/config.rs`
- **Domain terms**:
  - `ConfigKey`: A dot-separated path into the TOML config structure (e.g., `watch.enabled`, `conflict_resolution.mode`).
  - `ConfigValue`: A typed TOML primitive (bool, integer, string, or string array).
  - `ConfigScope`: The layer a config entry originates from: Global, Project, or Env.
  - `ConfigGetResult`: Value object returned by a get operation, carrying the resolved value and its source scope.
  - `ConfigSetResult`: Value object returned by a set operation, confirming the key, value, and target scope.
  - `ConfigReadPort`: Trait abstracting config loading and path resolution (ports-and-adapters seam).
- **Assumptions**:
  - The hardline `Config` struct (in `crates/core/src/config/config_core.rs`) is the authoritative config shape. Its fields (keyed struct members, not a flat HashMap) are the valid config key namespace.
  - TOML files are the sole persistence format. Global: `~/.config/scp/config.toml`. Project: `.scp/config.toml` in the repo root. No `.scp/config` (KDL) support.
  - Environment variable prefix is `SCP_` (e.g., `SCP_WATCH_ENABLED=true`).
  - The existing `ConfigManager::load()` precedence (global -> project -> env) is correct and must be preserved.
  - File locking uses `fs4` exclusive advisory locks with a 5-second timeout to prevent data loss under concurrent writes.
  - `toml_edit` is used for surgical, round-trip-safe TOML modifications (preserving comments and formatting).

---

## Value Objects

### ConfigGetResult

```rust
/// Result of a config get operation.
///
/// Invariants:
/// - key is a non-empty ASCII dot-notation path
/// - value is the stringified TOML representation
/// - scope is the highest-precedence source that provided this key
/// - source_path is the filesystem path of that source (empty for Env scope)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigGetResult {
    pub key: ConfigKey,
    pub value: String,
    pub scope: ConfigScope,
    pub source_path: PathBuf,
}
```

### ConfigSetResult

```rust
/// Result of a config set operation.
///
/// Invariants:
/// - key is a non-empty ASCII dot-notation path
/// - value matches one of the TOML types: bool, i64, String, Vec<String>
/// - scope is Global or Project (never Env -- cannot write to env vars)
/// - config_path is the file that was written
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSetResult {
    pub key: ConfigKey,
    pub value: String,
    pub scope: ConfigScope,
    pub config_path: PathBuf,
}
```

---

## Types

### ConfigScope

```rust
/// Configuration source scope with strict precedence ordering.
///
/// Precedence (highest first): Env > Project > Global > Defaults
///
/// Invariants:
/// - Save operations are only valid for Global and Project scopes
/// - Env scope is read-only (sourced from process environment at load time)
/// - Defaults are built into the Config struct and never persisted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ConfigScope {
    #[default]
    Global,   // ~/.config/scp/config.toml
    Project,  // .scp/config.toml
    Env,      // SCP_* environment variables (read-only)
}
```

**Precedence rules**:
1. `Env` (highest): `SCP_<SECTION>_<KEY>` environment variables override everything.
2. `Project`: `.scp/config.toml` in the working directory.
3. `Global`: `~/.config/scp/config.toml`.
4. Built-in defaults (lowest): hardcoded in `Config::default()`.

When loading merged config, each layer overrides the previous. The resulting `ConfigGetResult` reports the highest-precedence scope that contributed that key's value.

### ConfigKey

```rust
/// A validated dot-notation path into the TOML config structure.
///
/// Validation rules:
/// - Non-empty string
/// - Each segment is non-empty
/// - Each segment contains only ASCII alphanumeric characters and underscores ([a-zA-Z0-9_]+)
/// - Contains at least one dot (i.e., at least two segments: "section.key")
/// - Total length <= 256 characters
/// - No leading/trailing dots, no consecutive dots
/// - Must reference a key that exists in the Config struct schema
///   (either a leaf field or a table prefix for section display)
///
/// Construction is fallible: use ConfigKey::try_from(&str) -> Result<ConfigKey, ConfigError>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey {
    raw: String,       // e.g., "watch.enabled"
    segments: Vec<String>,  // e.g., ["watch", "enabled"]
}
```

**Validation function signature**:

```rust
impl ConfigKey {
    /// Parse and validate a config key string.
    ///
    /// Errors:
    /// - ConfigKeyNotFound: key does not match any known key or section prefix
    /// - ConfigParseError: empty string, non-ASCII characters, invalid segment characters,
    ///   missing dot separator, consecutive dots, exceeds 256 chars, contains null bytes
    pub fn try_from(key: &str) -> Result<Self>;
}
```

### ConfigValue (TOML type support)

The CLI accepts raw string input from the user and infers the TOML type:

| Input                | Parsed Type  | TOML Representation   |
|----------------------|-------------|-----------------------|
| `"true"` / `"false"` | `bool`      | `true` / `false`      |
| `"42"`               | `i64`       | `42`                  |
| `"["a", "b"]"`       | `Vec<String>` | `["a", "b"]`        |
| any other string     | `String`    | `"the string"`        |

**Parsing rules**:
- Boolean: exact match `"true"` or `"false"` (case-sensitive).
- Integer: string must parse as `i64` without overflow. Values that overflow `i64` are stored as strings.
- Array: must start with `[` and end with `]`. Inner items must be quoted strings. Non-string-array TOML arrays are rejected.
- String (fallback): all other inputs are stored as TOML basic strings.

```rust
/// Parse a CLI value string into a toml_edit::Item.
///
/// Errors:
/// - ConfigParseError: array contains non-string elements or malformed TOML
pub fn parse_cli_value(raw: &str) -> Result<toml_edit::Item>;
```

---

## Function Signatures

### ConfigCommandHandler

```rust
/// Execute the config command.
///
/// Dispatches based on (key, value) combination:
/// - (None, None): list all config
/// - (Some(key), None): get a specific value
/// - (Some(key), Some(value)): set a value
/// - (None, Some(_)): error -- cannot set without a key
///
/// Preconditions:
/// - If key is provided, it must pass ConfigKey validation
///
/// Postconditions:
/// - On get: ConfigGetResult.key matches the requested key
/// - On set: ConfigSetResult.key matches the requested key,
///   ConfigSetResult.config_path exists and contains valid TOML,
///   ConfigSetResult.scope is Global or Project (never Env)
pub async fn run(options: ConfigOptions) -> Result<()>;
```

### Core Operations

```rust
/// Get a single config value by dot-notation key.
///
/// Uses merged config (env > project > global > defaults).
///
/// Preconditions:
/// - key passes ConfigKey::try_from validation
///
/// Returns:
/// - Ok(ConfigGetResult) with the resolved value and source scope
/// - Err(ConfigKeyNotFound) if key is valid syntax but not present in config
/// - Err(ConfigParseError) if key syntax is invalid
pub async fn config_get(key: &str, scope: ConfigScope) -> Result<ConfigGetResult>;

/// Set a config value in the specified scope's TOML file.
///
/// Preconditions:
/// - key passes ConfigKey::try_from validation
/// - scope is Global or Project (not Env)
/// - value is a valid CLI value string
/// - config file path is writable
///
/// Postconditions:
/// - The TOML file at the target path is valid TOML after the write
/// - The key's value in the file equals the requested value
/// - All other keys in the file are unchanged
/// - File lock is acquired exclusively before write and released after
/// - Parent directories are created if they do not exist
///
/// Returns:
/// - Ok(ConfigSetResult) confirming the write
/// - Err(ConfigScopeError) if scope == Env
/// - Err(ConfigWriteError) if file cannot be written
/// - Err(ConfigLockError) if file lock cannot be acquired within timeout
/// - Err(ConfigParseError) if resulting TOML would be invalid
pub async fn config_set(
    key: &str,
    value: &str,
    scope: ConfigScope,
) -> Result<ConfigSetResult>;

/// List all config values.
///
/// Returns all keys from the merged config (env > project > global > defaults).
/// If `global_only` is true, returns only keys from global scope.
///
/// Postconditions:
/// - All returned keys are valid ConfigKey instances
/// - Keys are sorted alphabetically
pub async fn config_list(global_only: bool) -> Result<Vec<ConfigGetResult>>;
```

### ConfigReadPort (Trait)

```rust
/// Port for config reads and path resolution (ports-and-adapters seam).
pub trait ConfigReadPort: Send + Sync {
    /// Load merged configuration (defaults + global + project + env).
    fn load_merged(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>>;

    /// Load global-only configuration (defaults + global).
    fn load_global_only(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>>;

    /// Return global config file path (~/.config/scp/config.toml).
    fn global_config_path(&self) -> Result<PathBuf>;

    /// Return project config file path (.scp/config.toml).
    fn project_config_path(&self) -> Result<PathBuf>;
}
```

### Nested Value Operations

```rust
/// Retrieve a nested value from a Config struct using dot notation.
///
/// Converts the Config to a JSON value tree for traversal.
///
/// Preconditions:
/// - key passes ConfigKey validation
///
/// Returns:
/// - Ok(String) with the value formatted for display:
///   bool -> "true"/"false", int -> "42", string -> "hello",
///   array -> "[\"a\", \"b\"]"
/// - Err(ConfigKeyNotFound) if any segment in the path does not exist
pub fn get_nested_value(config: &Config, key: &str) -> Result<String>;

/// Set a nested value in a TOML document using dot notation.
///
/// Creates intermediate tables as needed.
///
/// Preconditions:
/// - parts is non-empty
/// - each segment in parts is non-empty
///
/// Postconditions:
/// - The document is valid TOML after modification
/// - Intermediate tables are created if they did not exist
/// - If a segment is not a table (e.g., it is a value), returns ConfigParseError
///
/// Errors:
/// - ConfigParseError: empty parts, empty segment, or non-table intermediate
pub fn set_nested_value(
    doc: &mut toml_edit::DocumentMut,
    parts: &[&str],
    value: &str,
) -> Result<()>;
```

---

## File Locking Requirements

All write operations to config TOML files MUST use exclusive advisory file locking to prevent data loss from concurrent writes.

### Protocol

1. **Open** the config file with `read + write + create` (no truncate).
2. **Create parent directories** if they do not exist (`create_dir_all`).
3. **Acquire exclusive lock** via `fs4::AsyncFileExt::try_lock_exclusive()` with retry loop.
   - Timeout: 5 seconds.
   - Retry interval: 100ms.
   - If lock cannot be acquired within timeout, return `ConfigLockError`.
4. **Read** the full file contents (seek to start, read to end).
5. **Parse** as `toml_edit::DocumentMut` (empty file produces empty document).
6. **Modify** the document via `set_nested_value()`.
7. **Validate** the resulting TOML by round-tripping through `toml::from_str::<PartialConfig>()` to catch invalid configurations before persisting.
8. **Truncate** the file (`set_len(0)`).
9. **Write** the serialized document.
10. **Flush** to ensure durability.
11. **Release** lock by dropping the file handle.

### Guarantees

- At most one process holds the lock on a config file at any time.
- Lock is released even on panic (RAII via `Drop`).
- Lock timeout prevents indefinite blocking.
- No TOCTTOU: lock is held for the entire read-modify-write cycle.

---

## Error Taxonomy

All config errors are variants of `ConfigErrorKind` (mapped through `Error::Config`). Each variant has a dedicated exit code in the 4xxx range.

```rust
#[derive(Error, Debug, Clone)]
pub enum ConfigErrorKind {
    /// A config key was not found in the loaded configuration.
    /// The key syntax is valid, but no value exists at that path.
    /// Exit code: 40
    #[error("Config key not found: {0}")]
    ConfigKeyNotFound(String),

    /// An operation was attempted against an invalid or unsupported scope.
    /// E.g., attempting to save to Env scope, or project scope when no
    /// project config path is available.
    /// Exit code: 43
    #[error("Config scope error: {0}")]
    ConfigScopeError(String),

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

    /// A file lock could not be acquired within the timeout.
    /// Indicates another process is holding the lock on the config file.
    /// Exit code: 44
    #[error("Config lock error: {0}")]
    ConfigLockError(String),

    // Existing variants (preserved for backward compatibility):
    /// Generic not found (config file or directory).
    #[error("Configuration not found: {0}")]
    NotFound(String),

    /// Generic invalid configuration.
    #[error("Configuration invalid: {0}")]
    Invalid(String),

    /// Permission denied on config file or directory.
    #[error("Configuration permission denied: {0}")]
    Permission(String),
}
```

### Error Code Mapping

| Variant | Exit Code | Trigger |
|---------|-----------|---------|
| `ConfigKeyNotFound` | 40 | `config_get` on a key that does not exist in any loaded scope |
| `ConfigParseError` | 41 | Invalid key syntax, non-ASCII, overflow int, malformed array |
| `ConfigWriteError` | 42 | File open/write/flush failure, permission denied |
| `ConfigScopeError` | 43 | Save to Env scope, no project path available |
| `ConfigLockError` | 44 | Lock acquisition timeout (5s exceeded) |
| `NotFound` | 40 | Config directory cannot be determined |
| `Invalid` | 41 | Generic config validation failure |
| `Permission` | 42 | Permission denied on config file/directory |

### Error Construction Rules

- `ConfigKeyNotFound`: Constructed when `get_nested_value()` or JSON traversal fails to find a segment in the config tree. NOT used for syntax-invalid keys (those produce `ConfigParseError`).
- `ConfigScopeError`: Constructed when `config_set` is called with `ConfigScope::Env`, or when `project_config_path()` fails in a save context.
- `ConfigParseError`: Constructed by `ConfigKey::try_from()` for syntax failures, by `parse_cli_value()` for value parse failures, or by `set_nested_value()` when a TOML segment is not a table.
- `ConfigWriteError`: Constructed by wrapping `std::io::Error` from any file operation (open, seek, read, write, flush, create_dir_all) with context about the failing path.
- `ConfigLockError`: Constructed when the retry loop exhausts the 5-second timeout without acquiring the exclusive lock.

---

## Invariants

1. **Key syntax**: Every `ConfigKey` is a non-empty ASCII string with segments matching `[a-zA-Z0-9_]+`, separated by dots, containing at least one dot, and total length <= 256 characters. No null bytes, no newlines, no Unicode, no path traversal characters (`/`, `\`, `..`).

2. **Value type consistency**: Every value stored in a TOML config file matches one of the four supported TOML types: bool, integer (i64), string, or array of strings. The `parse_cli_value()` function is the sole arbiter of type inference.

3. **Scope precedence**: When loading merged config, the resolution order is strictly `env > project > global > defaults`. The `ConfigGetResult.scope` field always reports the highest-precedence source that contributed the value. A key overridden by env will report `ConfigScope::Env`.

4. **Write scope restriction**: Save operations are only valid for `ConfigScope::Global` and `ConfigScope::Project`. Attempting to write to `ConfigScope::Env` must fail with `ConfigScopeError`.

5. **TOML validity preservation**: After any set operation, the config file must parse as valid TOML. This is verified by round-tripping through `toml::from_str::<PartialConfig>()` before persisting.

6. **Atomic write-under-lock**: The read-modify-write cycle for config files must occur entirely while holding an exclusive file lock. No external observer should see a partially-written or invalid TOML file.

7. **Key existence in schema**: Valid keys must correspond to a field in the hardline `Config` struct (or be a section prefix of such a field). Unknown keys are rejected at validation time with `ConfigParseError`, not silently stored.

8. **ConfigGetResult key stability**: The `key` field in a `ConfigGetResult` must exactly equal the requested key string. No normalization, trimming, or transformation.

9. **ConfigSetResult confirmation**: The `key` and `value` fields in a `ConfigSetResult` must exactly match the caller's input. The `config_path` must be an existing, readable file after the operation completes.

---

## Non-goals

- SchemaEnvelope JSON wrapping (covered by a separate bead if needed; the contract specifies raw JSON output without envelope requirements).
- Config file hot-reload / file watching for the config command itself (the command is fire-and-forget).
- KDL config format support (TOML only).
- Config migration / versioning between releases.
- Nested table display for `config list` (flat key=value output is sufficient).
- Config unset / delete operations (not in the source; out of scope for this port).
- Config edit (opening an editor; not in the source).
