//! Types for the CLI config command.
//!
//! ConfigKey, ConfigGetResult, ConfigSetResult, ConfigReadPort trait,
//! FileConfigReadPort implementation, port registry for dependency injection,
//! parse_cli_value, get_nested_value, set_nested_value.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::error_config::ConfigErrorKind;

use super::config_core::{Config, ConfigScope};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey {
    raw: String,
    segments: Vec<String>,
}

impl ConfigKey {
    pub fn try_from(key: &str) -> Result<Self> {
        if key.is_empty() {
            return Err(ConfigErrorKind::ConfigParseError("empty config key".to_string()).into());
        }
        if key.len() > MAX_KEY_LENGTH {
            return Err(ConfigErrorKind::ConfigParseError(format!(
                "config key exceeds maximum length of {MAX_KEY_LENGTH} characters"
            ))
            .into());
        }
        if key.contains('\0') {
            return Err(ConfigErrorKind::ConfigParseError("config key contains null byte".to_string()).into());
        }
        if !key.is_ascii() {
            return Err(ConfigErrorKind::ConfigParseError("config key contains non-ASCII characters".to_string()).into());
        }
        if key.contains('/') {
            return Err(ConfigErrorKind::ConfigParseError("config key contains slash, possible path traversal".to_string()).into());
        }
        if key.contains('\\') {
            return Err(ConfigErrorKind::ConfigParseError("config key contains backslash, invalid character".to_string()).into());
        }
        if key.starts_with('.') {
            return Err(ConfigErrorKind::ConfigParseError("config key has leading dot, empty segment".to_string()).into());
        }
        if key.ends_with('.') {
            return Err(ConfigErrorKind::ConfigParseError("config key has trailing dot, empty segment".to_string()).into());
        }
        if key.contains("..") {
            return Err(ConfigErrorKind::ConfigParseError("config key contains consecutive dots".to_string()).into());
        }
        let segments: Vec<String> = key.split('.').map(String::from).collect();
        if segments.len() < 2 {
            return Err(ConfigErrorKind::ConfigParseError("config key must contain at least one dot separator (section.key)".to_string()).into());
        }
        for segment in &segments {
            if segment.is_empty() {
                return Err(ConfigErrorKind::ConfigParseError("config key contains empty segment".to_string()).into());
            }
            for ch in segment.chars() {
                if ch == '-' {
                    return Err(ConfigErrorKind::ConfigParseError(format!("config key segment contains hyphen '-': invalid character in '{segment}'")).into());
                }
                if ch.is_whitespace() {
                    return Err(ConfigErrorKind::ConfigParseError(format!("config key segment contains whitespace: invalid character in '{segment}'")).into());
                }
                if !ch.is_ascii_alphanumeric() && ch != '_' {
                    return Err(ConfigErrorKind::ConfigParseError(format!("config key segment contains invalid character '{ch}' in '{segment}'")).into());
                }
            }
        }
        let first_segment = &segments[0];
        if first_segment.len() > 1 && !KNOWN_SECTION_PREFIXES.contains(&first_segment.as_str()) {
            return Err(ConfigErrorKind::ConfigParseError(format!("unknown schema section '{first_segment}': key not found in known config keys")).into());
        }
        Ok(Self { raw: key.to_string(), segments })
    }

    #[must_use]
    pub fn as_str(&self) -> &str { &self.raw }

    #[must_use]
    pub fn segments(&self) -> &[String] { &self.segments }
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigGetResult
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigGetResult {
    pub key: ConfigKey,
    pub value: String,
    pub scope: ConfigScope,
    pub source_path: PathBuf,
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigSetResult
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSetResult {
    pub key: ConfigKey,
    pub value: String,
    pub scope: ConfigScope,
    pub config_path: PathBuf,
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigReadPort
// ═══════════════════════════════════════════════════════════════════════════

pub trait ConfigReadPort: Send + Sync {
    fn load_merged(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>>;
    fn load_global_only(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>>;
    fn global_config_path(&self) -> Result<PathBuf>;
    fn project_config_path(&self) -> Result<PathBuf>;
    fn as_any(&self) -> &dyn std::any::Any;
}

// ═══════════════════════════════════════════════════════════════════════════
// Port Registry
// ═══════════════════════════════════════════════════════════════════════════

static PORT_REGISTRY: OnceLock<RwLock<Option<Arc<dyn ConfigReadPort>>>> = OnceLock::new();

fn port_registry() -> &'static RwLock<Option<Arc<dyn ConfigReadPort>>> {
    PORT_REGISTRY.get_or_init(|| RwLock::new(None))
}

pub fn set_port(port: Arc<dyn ConfigReadPort>) {
    let registry = port_registry();
    let mut guard = registry.write().expect("port registry lock poisoned");
    *guard = Some(port);
}

pub fn clear_port() {
    let registry = port_registry();
    let mut guard = registry.write().expect("port registry lock poisoned");
    *guard = None;
}

pub(crate) fn get_port() -> Arc<dyn ConfigReadPort> {
    let registry = port_registry();
    let guard = registry.read().expect("port registry lock poisoned");
    if let Some(port) = guard.as_ref() {
        Arc::clone(port)
    } else {
        Arc::new(FileConfigReadPort::new())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FileConfigReadPort
// ═══════════════════════════════════════════════════════════════════════════

pub struct FileConfigReadPort {
    global_path: PathBuf,
    project_path: Option<PathBuf>,
}

impl FileConfigReadPort {
    pub fn new() -> Self {
        let global_path = directories::ProjectDirs::from("com", "scp", "scp")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config").join("scp").join("config.toml"))
                    .unwrap_or_else(|_| PathBuf::from("config.toml"))
            });
        Self { global_path, project_path: None }
    }

    pub fn with_paths(global_path: PathBuf, project_path: Option<PathBuf>) -> Self {
        Self { global_path, project_path }
    }

    fn load_toml_file(path: &Path) -> Result<HashMap<String, String>> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            ConfigErrorKind::ConfigWriteError(format!("failed to read {}: {e}", path.display()))
        })?;
        let doc: toml_edit::DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| {
            ConfigErrorKind::ConfigParseError(format!("TOML parse error in {}: {e}", path.display()))
        })?;
        let mut values = HashMap::new();
        flatten_toml_document(&doc, "", &mut values);
        Ok(values)
    }

    fn load_env_overrides() -> HashMap<String, String> {
        let mut values = HashMap::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("SCP_") {
                let config_key = key["SCP_".len()..].to_lowercase().replace('_', ".");
                values.insert(config_key, value);
            }
        }
        values
    }

    pub fn load_with_layers(
        &self,
        include_project: bool,
        include_env: bool,
    ) -> Result<(Config, HashMap<String, ConfigScope>, HashMap<String, PathBuf>)> {
        let mut values: HashMap<String, String> = HashMap::new();
        let mut scopes: HashMap<String, ConfigScope> = HashMap::new();
        let mut paths: HashMap<String, PathBuf> = HashMap::new();

        if self.global_path.exists() {
            let gv = Self::load_toml_file(&self.global_path)?;
            for (k, v) in gv {
                values.insert(k.clone(), v);
                scopes.insert(k.clone(), ConfigScope::Global);
                paths.insert(k, self.global_path.clone());
            }
        }

        if include_project {
            if let Some(ref pp) = self.project_path {
                if pp.exists() {
                    let pv = Self::load_toml_file(pp)?;
                    for (k, v) in pv {
                        values.insert(k.clone(), v);
                        scopes.insert(k.clone(), ConfigScope::Project);
                        paths.insert(k, pp.clone());
                    }
                }
            }
        }

        if include_env {
            let ev = Self::load_env_overrides();
            for (k, v) in ev {
                values.insert(k.clone(), v);
                scopes.insert(k.clone(), ConfigScope::Env);
                paths.insert(k, PathBuf::new());
            }
        }

        let mut config = Config::new();
        for (k, v) in &values {
            config.set(k.clone(), v.clone());
        }

        if self.global_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&self.global_path) {
                if let Ok(doc) = contents.parse::<toml_edit::DocumentMut>() {
                    apply_structured_sections(&doc, &mut config);
                }
            }
        }

        if include_project {
            if let Some(ref pp) = self.project_path {
                if pp.exists() {
                    if let Ok(contents) = std::fs::read_to_string(pp) {
                        if let Ok(doc) = contents.parse::<toml_edit::DocumentMut>() {
                            apply_structured_sections(&doc, &mut config);
                        }
                    }
                }
            }
        }

        if include_env {
            let ev = Self::load_env_overrides();
            apply_env_to_structured(&ev, &mut config);
        }

        Ok((config, scopes, paths))
    }
}

impl Default for FileConfigReadPort {
    fn default() -> Self { Self::new() }
}

fn flatten_toml_document(doc: &toml_edit::DocumentMut, prefix: &str, out: &mut HashMap<String, String>) {
    for (key, item) in doc.iter() {
        let full_key = if prefix.is_empty() { key.to_string() } else { format!("{prefix}.{key}") };
        match item {
            toml_edit::Item::Value(v) => { out.insert(full_key, stringify_toml_value(v)); }
            toml_edit::Item::Table(table) => { flatten_toml_table(table, &full_key, out); }
            _ => {}
        }
    }
}

fn flatten_toml_table(table: &toml_edit::Table, prefix: &str, out: &mut HashMap<String, String>) {
    for (key, item) in table.iter() {
        let full_key = format!("{prefix}.{key}");
        match item {
            toml_edit::Item::Value(v) => { out.insert(full_key, stringify_toml_value(v)); }
            toml_edit::Item::Table(sub_table) => { flatten_toml_table(sub_table, &full_key, out); }
            _ => {}
        }
    }
}

fn stringify_toml_value(v: &toml_edit::Value) -> String {
    match v {
        toml_edit::Value::Boolean(b) => b.value().to_string(),
        toml_edit::Value::Integer(n) => n.to_string(),
        toml_edit::Value::Float(f) => f.to_string(),
        toml_edit::Value::String(s) => s.value().to_string(),
        toml_edit::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(stringify_toml_value).collect();
            format!("[{}]", items.join(", "))
        }
        toml_edit::Value::Datetime(dt) => dt.to_string(),
        toml_edit::Value::InlineTable(_) => "{}".to_string(),
    }
}

fn apply_structured_sections(doc: &toml_edit::DocumentMut, config: &mut Config) {
    if let Some(ci) = doc.get("conflict_resolution") {
        if let Some(ct) = ci.as_table() {
            if let Some(mv) = ct.get("mode") {
                if let Some(s) = mv.as_str() {
                    config.conflict.mode = super::types::ConflictMode::from_str(s).unwrap_or_default();
                }
            }
            if let Some(av) = ct.get("autonomy") {
                if let Some(n) = av.as_integer() {
                    config.conflict.autonomy = u8::try_from(n).unwrap_or(0);
                }
            }
            if let Some(kv) = ct.get("security_keywords") {
                if let Some(arr) = kv.as_array() {
                    config.conflict.security_keywords = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                }
            }
            if let Some(lv) = ct.get("log_resolutions") {
                if let Some(true) = lv.as_bool() {
                    config.conflict.log_resolutions = super::types::ValidatedBool::new(true);
                }
            }
        }
    }
    if let Some(si) = doc.get("session") {
        if let Some(st) = si.as_table() {
            if let Some(ac) = st.get("auto_commit") {
                if let Some(true) = ac.as_bool() {
                    config.session.auto_commit = super::types::ValidatedBool::new(true);
                }
            }
            if let Some(cp) = st.get("commit_prefix") {
                if let Some(s) = cp.as_str() { config.session.commit_prefix = s.to_string(); }
            }
            if let Some(ms) = st.get("max_sessions") {
                if let Some(n) = ms.as_integer() {
                    config.session.max_sessions = usize::try_from(n).unwrap_or(10);
                }
            }
        }
    }
}

fn apply_env_to_structured(env_values: &HashMap<String, String>, config: &mut Config) {
    if let Some(ms) = env_values.get("conflict_resolution.mode") {
        config.conflict.mode = super::types::ConflictMode::from_str(ms).unwrap_or_default();
    }
    if let Some(a) = env_values.get("session.auto_commit") {
        if a == "true" { config.session.auto_commit = super::types::ValidatedBool::new(true); }
        else if a == "false" { config.session.auto_commit = super::types::ValidatedBool::new(false); }
    }
    if let Some(p) = env_values.get("session.commit_prefix") { config.session.commit_prefix = p.clone(); }
    if let Some(m) = env_values.get("session.max_sessions") {
        if let Ok(n) = m.parse::<usize>() { config.session.max_sessions = n; }
    }
    if let Some(w) = env_values.get("watch.enabled") {
        config.values.insert("watch.enabled".to_string(), w.clone());
    }
}

impl ConfigReadPort for FileConfigReadPort {
    fn load_merged(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>> {
        Box::pin(async move { let (c, _, _) = self.load_with_layers(true, true)?; Ok(c) })
    }
    fn load_global_only(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>> {
        Box::pin(async move { let (c, _, _) = self.load_with_layers(false, false)?; Ok(c) })
    }
    fn global_config_path(&self) -> Result<PathBuf> { Ok(self.global_path.clone()) }
    fn project_config_path(&self) -> Result<PathBuf> {
        self.project_path.clone().ok_or_else(|| ConfigErrorKind::ConfigScopeError("no project config path available".to_string()).into())
    }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

// ═══════════════════════════════════════════════════════════════════════════
// parse_cli_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn parse_cli_value(raw: &str) -> Result<toml_edit::Item> {
    if raw == "true" { return Ok(toml_edit::Item::Value(toml_edit::Value::from(true))); }
    if raw == "false" { return Ok(toml_edit::Item::Value(toml_edit::Value::from(false))); }
    if let Ok(n) = raw.parse::<i64>() { return Ok(toml_edit::Item::Value(toml_edit::Value::from(n))); }
    if raw.starts_with('[') { return parse_array_value(raw); }
    Ok(toml_edit::Item::Value(toml_edit::Value::from(raw)))
}

fn parse_array_value(raw: &str) -> Result<toml_edit::Item> {
    let wrapped = format!("__val__ = {raw}");
    let parsed: std::result::Result<toml::Value, _> = toml::from_str(&wrapped);
    match parsed {
        Ok(toml::Value::Table(table)) => {
            let arr_val = table.get("__val__").ok_or_else(|| ConfigErrorKind::ConfigParseError("malformed TOML array: could not parse".to_string()))?;
            match arr_val {
                toml::Value::Array(arr) => {
                    let mut toml_arr = toml_edit::Array::new();
                    for item in arr {
                        match item {
                            toml::Value::String(s) => toml_arr.push(s.clone()),
                            _ => return Err(ConfigErrorKind::ConfigParseError("array contains non-string element: TOML arrays must contain only strings".to_string()).into()),
                        }
                    }
                    Ok(toml_edit::Item::Value(toml_edit::Value::Array(toml_arr)))
                }
                _ => Ok(toml_edit::Item::Value(toml_edit::Value::from(raw))),
            }
        }
        Err(_) => Err(ConfigErrorKind::ConfigParseError(format!("malformed TOML array: could not parse '{raw}'")).into()),
        _ => Err(ConfigErrorKind::ConfigParseError(format!("malformed TOML array: unexpected structure in '{raw}'")).into()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// get_nested_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn get_nested_value(config: &Config, key: &str) -> Result<String> {
    if let Some(val) = config.values.get(key) { return Ok(val.clone()); }
    let json = serde_json::to_value(config).map_err(|e| ConfigErrorKind::ConfigKeyNotFound(format!("serialization error: {e}")))?;
    let segments: Vec<&str> = key.split('.').collect();
    let mut current = &json;
    for (i, segment) in segments.iter().enumerate() {
        match current {
            serde_json::Value::Object(map) => {
                let lookup = if i == 0 {
                    match *segment {
                        "conflict_resolution" => "conflict",
                        s => s,
                    }
                } else {
                    *segment
                };
                current = map.get(lookup).ok_or_else(|| ConfigErrorKind::ConfigKeyNotFound(format!("key not found: {segment} in {key}")))?;
            }
            _ => { return Err(ConfigErrorKind::ConfigKeyNotFound(format!("segment '{segment}' is not a table in key '{key}'")).into()); }
        }
    }
    match current {
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| format!("\"{s}\""))).collect();
            Ok(format!("[{}]", items.join(", ")))
        }
        serde_json::Value::Null => Err(ConfigErrorKind::ConfigKeyNotFound(format!("value is null for key '{key}'")).into()),
        serde_json::Value::Object(_) => Err(ConfigErrorKind::ConfigKeyNotFound(format!("key '{key}' resolves to a table, not a value")).into()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// set_nested_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn set_nested_value(doc: &mut toml_edit::DocumentMut, parts: &[&str], value: &str) -> Result<()> {
    if parts.len() < 2 { return Err(ConfigErrorKind::ConfigParseError("set_nested_value requires at least two segments (section.key)".to_string()).into()); }
    for part in parts { if part.is_empty() { return Err(ConfigErrorKind::ConfigParseError("config key contains empty segment".to_string()).into()); } }
    let table = doc.as_table_mut();
    let (leading, last) = parts.split_at(parts.len() - 1);
    let mut current = table;
    for segment in leading {
        if !current.contains_key(segment) { current[segment] = toml_edit::Item::Table(toml_edit::Table::new()); }
        let entry = &current[segment];
        if !entry.is_table() { return Err(ConfigErrorKind::ConfigParseError(format!("segment '{segment}' is not a table, cannot traverse through it")).into()); }
        current = current[segment].as_table_mut().expect("just verified it is a table");
    }
    let item = parse_cli_value(value)?;
    current[last[0]] = item;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// config_get / config_set / config_list
// ═══════════════════════════════════════════════════════════════════════════

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub async fn config_get(key: &str, _scope: ConfigScope) -> Result<ConfigGetResult> {
    let config_key = ConfigKey::try_from(key)?;
    let port = get_port();
    let (config, scopes, paths) = if let Some(fp) = port.as_any().downcast_ref::<FileConfigReadPort>() {
        fp.load_with_layers(true, true)?
    } else {
        let config = port.load_merged().await?;
        let mut sm = HashMap::new();
        let pm = HashMap::new();
        for k in config.values.keys() { sm.insert(k.clone(), ConfigScope::Global); }
        (config, sm, pm)
    };
    let value = get_nested_value(&config, key).or_else(|_| {
        config.values.get(key).cloned().ok_or_else(|| crate::error::Error::from(ConfigErrorKind::ConfigKeyNotFound(format!("key not found: {key}"))))
    })?;
    let resolved_scope = scopes.get(key).copied().unwrap_or(ConfigScope::Global);
    let source_path = paths.get(key).cloned().unwrap_or_default();
    Ok(ConfigGetResult { key: config_key, value, scope: resolved_scope, source_path })
}

pub async fn config_set(key: &str, value: &str, scope: ConfigScope) -> Result<ConfigSetResult> {
    let config_key = ConfigKey::try_from(key)?;
    if matches!(scope, ConfigScope::Env) {
        return Err(ConfigErrorKind::ConfigScopeError("Cannot save to environment scope".to_string()).into());
    }
    let port = get_port();
    let config_path = match scope {
        ConfigScope::Global => port.global_config_path()?,
        ConfigScope::Project => port.project_config_path()?,
        ConfigScope::Env => unreachable!(),
    };
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to create config directory {}: {e}", parent.display())))?;
    }
    let file = std::fs::OpenOptions::new().read(true).write(true).create(true).open(&config_path)
        .map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to open config file {}: {e}", config_path.display())))?;
    let start = Instant::now();
    let file = loop {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => break file,
            Err(_) => {
                if start.elapsed() >= LOCK_TIMEOUT {
                    return Err(ConfigErrorKind::ConfigLockError(format!("could not acquire lock on {} within 5s timeout", config_path.display())).into());
                }
                std::thread::sleep(LOCK_RETRY_INTERVAL);
            }
        }
    };
    let mut contents = String::new();
    let mut file = &file;
    std::io::Read::read_to_string(&mut file, &mut contents).map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to read config file {}: {e}", config_path.display())))?;
    let mut doc: toml_edit::DocumentMut = if contents.trim().is_empty() { toml_edit::DocumentMut::new() } else {
        contents.parse().map_err(|e: toml_edit::TomlError| ConfigErrorKind::ConfigParseError(format!("TOML parse error in {}: {e}", config_path.display())))?
    };
    let segments: Vec<&str> = config_key.segments().iter().map(String::as_str).collect();
    set_nested_value(&mut doc, &segments, value)?;
    use std::io::{Seek, Write};
    file.set_len(0).map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to truncate config file {}: {e}", config_path.display())))?;
    file.seek(std::io::SeekFrom::Start(0)).map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to seek in config file {}: {e}", config_path.display())))?;
    file.write_all(doc.to_string().as_bytes()).map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to write config file {}: {e}", config_path.display())))?;
    file.flush().map_err(|e| ConfigErrorKind::ConfigWriteError(format!("failed to flush config file {}: {e}", config_path.display())))?;
    Ok(ConfigSetResult { key: config_key, value: value.to_string(), scope, config_path })
}

pub async fn config_list(global_only: bool) -> Result<Vec<ConfigGetResult>> {
    let port = get_port();
    let (config, scopes, paths) = if let Some(fp) = port.as_any().downcast_ref::<FileConfigReadPort>() {
        fp.load_with_layers(!global_only, !global_only)?
    } else {
        let config = if global_only { port.load_global_only().await? } else { port.load_merged().await? };
        let mut sm = HashMap::new();
        for k in config.values.keys() { sm.insert(k.clone(), ConfigScope::Global); }
        (config, sm, HashMap::new())
    };
    let mut all_keys: Vec<String> = config.values.keys().cloned().collect();
    all_keys.sort();
    all_keys.dedup();
    let mut results = Vec::new();
    for key in &all_keys {
        let first_segment = key.split('.').next().unwrap_or("");
        if !KNOWN_SECTION_PREFIXES.contains(&first_segment) && !KNOWN_CONFIG_KEYS.contains(&key.as_str()) { continue; }
        let config_key = match ConfigKey::try_from(key.as_str()) { Ok(k) => k, Err(_) => continue };
        let value = match config.values.get(key.as_str()) {
            Some(v) => v.clone(),
            None => match get_nested_value(&config, key) {
                Ok(v) => v,
                Err(_) => continue,
            },
        };
        let resolved_scope = scopes.get(key.as_str()).copied().unwrap_or(ConfigScope::Global);
        let source_path = paths.get(key.as_str()).cloned().unwrap_or_default();
        results.push(ConfigGetResult { key: config_key, value, scope: resolved_scope, source_path });
    }
    results.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
    Ok(results)
}
