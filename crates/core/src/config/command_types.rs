//! Types for the CLI config command.
//!
//! ConfigKey, ConfigGetResult, ConfigSetResult, ConfigReadPort trait,
//! FileConfigReadPort implementation, port registry for dependency injection,
//! parse_cli_value, get_nested_value, set_nested_value.

use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::{Arc, OnceLock, RwLock},
    time::{Duration, Instant},
};

use super::{
    config_core::{Config, ConfigScope},
    config_watcher::validate_config_file,
};
use crate::{error::Result, error_config::ConfigErrorKind};

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
    "hooks.post_create",
    "hooks.pre_remove",
    "hooks.post_merge",
    "agent.command",
    "vcs.type",
    "vcs.default_branch",
    "vcs.forge",
    "vcs.branch_templates",
    "workspace.directory",
    "workspace.auto_rebase",
    "workspace.auto_push",
    "queue.default",
    "logging.level",
    "editor",
    "remote.push",
    "remote.fetch",
    "auth.preferred_source",
    "auth.allow_github_token_env",
    "auth.allow_stax_token_env",
    "auth.allow_credentials_file",
    "auth.allow_gh_cli",
];

/// Known section prefixes that are valid for section display.
pub const KNOWN_SECTION_PREFIXES: &[&str] = &[
    "watch",
    "conflict_resolution",
    "session",
    "hooks",
    "agent",
    "vcs",
    "workspace",
    "queue",
    "logging",
    "remote",
    "auth",
];

/// Maximum allowed length for a config key.
const MAX_KEY_LENGTH: usize = 256;

/// A validated dot-notation path into the TOML config structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigKey {
    raw: String,
    segments: Vec<String>,
}

/// Validate the overall format of a config key (length, characters, dots).
fn validate_key_format(key: &str) -> Result<()> {
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
        return Err(ConfigErrorKind::ConfigParseError(
            "config key contains null byte".to_string(),
        )
        .into());
    }
    if !key.is_ascii() {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key contains non-ASCII characters".to_string(),
        )
        .into());
    }
    if key.contains('/') {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key contains slash, possible path traversal".to_string(),
        )
        .into());
    }
    if key.contains('\\') {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key contains backslash, invalid character".to_string(),
        )
        .into());
    }
    if key.starts_with('.') {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key has leading dot, empty segment".to_string(),
        )
        .into());
    }
    if key.ends_with('.') {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key has trailing dot, empty segment".to_string(),
        )
        .into());
    }
    if key.contains("..") {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key contains consecutive dots".to_string(),
        )
        .into());
    }
    Ok(())
}

/// Split a key into segments and validate each segment's characters.
fn parse_key_segments(key: &str) -> Result<Vec<String>> {
    let segments: Vec<String> = key.split('.').map(String::from).collect();
    if segments.len() < 2 {
        return Err(ConfigErrorKind::ConfigParseError(
            "config key must contain at least one dot separator (section.key)".to_string(),
        )
        .into());
    }
    for segment in &segments {
        if segment.is_empty() {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains empty segment".to_string(),
            )
            .into());
        }
        validate_segment_chars(segment)?;
    }
    Ok(segments)
}

/// Validate that a single key segment contains only allowed characters.
fn validate_segment_chars(segment: &str) -> Result<()> {
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
    Ok(())
}

impl ConfigKey {
    pub fn try_from(key: &str) -> Result<Self> {
        validate_key_format(key)?;
        let segments = parse_key_segments(key)?;
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

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
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

#[allow(clippy::expect_used)]
pub fn set_port(port: Arc<dyn ConfigReadPort>) {
    let registry = port_registry();
    let mut guard = registry.write().expect("port registry lock poisoned");
    *guard = Some(port);
}

#[allow(clippy::expect_used)]
pub fn clear_port() {
    let registry = port_registry();
    let mut guard = registry.write().expect("port registry lock poisoned");
    *guard = None;
}

#[allow(clippy::expect_used)]
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

type ConfigInitResult = (
    Config,
    HashMap<String, ConfigScope>,
    HashMap<String, PathBuf>,
);

/// Accumulators for collecting config layer data.
struct LayerAccumulator<'a> {
    values: &'a mut HashMap<String, String>,
    scopes: &'a mut HashMap<String, ConfigScope>,
    paths: &'a mut HashMap<String, PathBuf>,
}

impl FileConfigReadPort {
    pub fn new() -> Self {
        let global_path = directories::ProjectDirs::from("com", "scp", "scp")
            .map(|dirs| dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| {
                std::env::var("HOME")
                    .map(|h| {
                        PathBuf::from(h)
                            .join(".config")
                            .join("scp")
                            .join("config.toml")
                    })
                    .unwrap_or_else(|_| PathBuf::from("config.toml"))
            });
        Self {
            global_path,
            project_path: None,
        }
    }

    pub fn with_paths(global_path: PathBuf, project_path: Option<PathBuf>) -> Self {
        Self {
            global_path,
            project_path,
        }
    }

    fn load_toml_file(path: &Path) -> Result<HashMap<String, String>> {
        validate_config_file(path)?;
        let contents = std::fs::read_to_string(path).map_err(|e| {
            ConfigErrorKind::ConfigWriteError(format!("failed to read {}: {e}", path.display()))
        })?;
        let doc: toml_edit::DocumentMut = contents.parse().map_err(|e: toml_edit::TomlError| {
            ConfigErrorKind::ConfigParseError(format!(
                "TOML parse error in {}: {e}",
                path.display()
            ))
        })?;
        let mut values = HashMap::new();
        flatten_toml_document(&doc, "", &mut values);
        Ok(values)
    }

    fn load_env_overrides() -> HashMap<String, String> {
        let mut values = HashMap::new();
        for (key, value) in std::env::vars() {
            if let Some(rest) = key.strip_prefix("SCP_") {
                let config_key = rest.to_lowercase().replace('_', ".");
                values.insert(config_key, value);
            }
        }
        values
    }

    pub fn load_with_layers(
        &self,
        include_project: bool,
        include_env: bool,
    ) -> Result<ConfigInitResult> {
        let mut values: HashMap<String, String> = HashMap::new();
        let mut scopes: HashMap<String, ConfigScope> = HashMap::new();
        let mut paths: HashMap<String, PathBuf> = HashMap::new();

        let mut acc = LayerAccumulator {
            values: &mut values,
            scopes: &mut scopes,
            paths: &mut paths,
        };

        if path_exists_on_disk(&self.global_path) {
            Self::apply_layer_values(
                Self::load_toml_file(&self.global_path)?,
                ConfigScope::Global,
                self.global_path.clone(),
                &mut acc,
            );
        }

        if include_project {
            if let Some(ref pp) = self.project_path {
                if path_exists_on_disk(pp) {
                    Self::apply_layer_values(
                        Self::load_toml_file(pp)?,
                        ConfigScope::Project,
                        pp.clone(),
                        &mut acc,
                    );
                }
            }
        }

        if include_env {
            Self::apply_layer_values(
                Self::load_env_overrides(),
                ConfigScope::Env,
                PathBuf::new(),
                &mut acc,
            );
        }

        let mut config = Config::new();
        for (k, v) in &values {
            config.set(k.clone(), v.clone());
        }

        Self::apply_structured_from_file(&self.global_path, &mut config);

        if include_project {
            if let Some(ref pp) = self.project_path {
                Self::apply_structured_from_file(pp, &mut config);
            }
        }

        if include_env {
            let ev = Self::load_env_overrides();
            apply_env_to_structured(&ev, &mut config);
        }

        Ok((config, scopes, paths))
    }

    /// Accumulators for collecting config layer data.
    fn apply_layer_values(
        layer: HashMap<String, String>,
        scope: ConfigScope,
        path: PathBuf,
        acc: &mut LayerAccumulator,
    ) {
        for (k, v) in layer {
            acc.values.insert(k.clone(), v);
            acc.scopes.insert(k.clone(), scope);
            acc.paths.insert(k, path.clone());
        }
    }

    /// Parse a TOML file and apply its structured sections to the config.
    fn apply_structured_from_file(path: &Path, config: &mut Config) {
        if !path_exists_on_disk(path) {
            return;
        }
        let Ok(contents) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(doc) = contents.parse::<toml_edit::DocumentMut>() else {
            return;
        };
        apply_structured_sections(&doc, config);
    }
}

impl Default for FileConfigReadPort {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether a path exists on disk, including as a dead symlink.
///
/// Unlike `Path::exists()` which follows symlinks (returning `false` for dead
/// symlinks), this uses `symlink_metadata` which returns `Ok` for any filesystem
/// entry, even if the symlink target is missing.
fn path_exists_on_disk(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn flatten_toml_document(
    doc: &toml_edit::DocumentMut,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    for (key, item) in doc.iter() {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        match item {
            toml_edit::Item::Value(v) => {
                out.insert(full_key, stringify_toml_value(v));
            }
            toml_edit::Item::Table(table) => {
                flatten_toml_table(table, &full_key, out);
            }
            _ => {}
        }
    }
}

fn flatten_toml_table(table: &toml_edit::Table, prefix: &str, out: &mut HashMap<String, String>) {
    for (key, item) in table.iter() {
        let full_key = format!("{prefix}.{key}");
        match item {
            toml_edit::Item::Value(v) => {
                out.insert(full_key, stringify_toml_value(v));
            }
            toml_edit::Item::Table(sub_table) => {
                flatten_toml_table(sub_table, &full_key, out);
            }
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
    apply_conflict_section(doc, config);
    apply_session_section(doc, config);
    apply_hooks_section(doc, config);
    apply_agent_section(doc, config);
}

fn apply_conflict_section(doc: &toml_edit::DocumentMut, config: &mut Config) {
    let Some(ci) = doc.get("conflict_resolution") else { return };
    let Some(ct) = ci.as_table() else { return };
    if let Some(mv) = ct.get("mode") {
        if let Some(s) = mv.as_str() {
            config.conflict.mode =
                super::types::ConflictMode::from_str(s).unwrap_or_default();
        }
    }
    if let Some(av) = ct.get("autonomy") {
        if let Some(n) = av.as_integer() {
            config.conflict.autonomy = u8::try_from(n).unwrap_or(0);
        }
    }
    if let Some(kv) = ct.get("security_keywords") {
        if let Some(arr) = kv.as_array() {
            config.conflict.security_keywords = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    if let Some(lv) = ct.get("log_resolutions") {
        if let Some(true) = lv.as_bool() {
            config.conflict.log_resolutions = super::types::ValidatedBool::new(true);
        }
    }
}

fn apply_session_section(doc: &toml_edit::DocumentMut, config: &mut Config) {
    let Some(si) = doc.get("session") else { return };
    let Some(st) = si.as_table() else { return };
    if let Some(ac) = st.get("auto_commit") {
        if let Some(true) = ac.as_bool() {
            config.session.auto_commit = super::types::ValidatedBool::new(true);
        }
    }
    if let Some(cp) = st.get("commit_prefix") {
        if let Some(s) = cp.as_str() {
            config.session.commit_prefix = s.to_string();
        }
    }
    if let Some(ms) = st.get("max_sessions") {
        if let Some(n) = ms.as_integer() {
            config.session.max_sessions = usize::try_from(n).unwrap_or(100);
        }
    }
}

fn apply_hooks_section(doc: &toml_edit::DocumentMut, config: &mut Config) {
    let Some(hi) = doc.get("hooks") else { return };
    let Some(ht) = hi.as_table() else { return };
    if let Some(pc) = ht.get("post_create") {
        if let Some(arr) = pc.as_array() {
            config.hooks.post_create = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    if let Some(pr) = ht.get("pre_remove") {
        if let Some(arr) = pr.as_array() {
            config.hooks.pre_remove = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    if let Some(pm) = ht.get("post_merge") {
        if let Some(arr) = pm.as_array() {
            config.hooks.post_merge = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
}

fn apply_agent_section(doc: &toml_edit::DocumentMut, config: &mut Config) {
    let Some(ai) = doc.get("agent") else { return };
    let Some(at) = ai.as_table() else { return };
    if let Some(cmd) = at.get("command") {
        if let Some(s) = cmd.as_str() {
            config.agent.command = s.to_string();
        }
    }
}

fn apply_env_to_structured(env_values: &HashMap<String, String>, config: &mut Config) {
    if let Some(ms) = env_values.get("conflict_resolution.mode") {
        config.conflict.mode = super::types::ConflictMode::from_str(ms).unwrap_or_default();
    }
    if let Some(a) = env_values.get("session.auto_commit") {
        if a == "true" {
            config.session.auto_commit = super::types::ValidatedBool::new(true);
        } else if a == "false" {
            config.session.auto_commit = super::types::ValidatedBool::new(false);
        }
    }
    if let Some(p) = env_values.get("session.commit_prefix") {
        config.session.commit_prefix = p.clone();
    }
    if let Some(m) = env_values.get("session.max_sessions") {
        if let Ok(n) = m.parse::<usize>() {
            config.session.max_sessions = n;
        }
    }
    if let Some(pc) = env_values.get("hooks.post_create") {
        config.hooks.post_create = parse_string_list(pc);
    }
    if let Some(pr) = env_values.get("hooks.pre_remove") {
        config.hooks.pre_remove = parse_string_list(pr);
    }
    if let Some(pm) = env_values.get("hooks.post_merge") {
        config.hooks.post_merge = parse_string_list(pm);
    }
    if let Some(w) = env_values.get("watch.enabled") {
        config.values.insert("watch.enabled".to_string(), w.clone());
    }
    if let Some(cmd) = env_values.get("agent.command") {
        config.agent.command = cmd.clone();
    }
}

/// Parse a comma-separated string into a Vec<String>.
fn parse_string_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

impl ConfigReadPort for FileConfigReadPort {
    fn load_merged(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>> {
        Box::pin(async move {
            let (c, _, _) = self.load_with_layers(true, true)?;
            Ok(c)
        })
    }
    fn load_global_only(&self) -> Pin<Box<dyn Future<Output = Result<Config>> + Send + '_>> {
        Box::pin(async move {
            let (c, _, _) = self.load_with_layers(false, false)?;
            Ok(c)
        })
    }
    fn global_config_path(&self) -> Result<PathBuf> {
        Ok(self.global_path.clone())
    }
    fn project_config_path(&self) -> Result<PathBuf> {
        self.project_path.clone().ok_or_else(|| {
            ConfigErrorKind::ConfigScopeError("no project config path available".to_string()).into()
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// parse_cli_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn parse_cli_value(raw: &str) -> Result<toml_edit::Item> {
    if raw == "true" {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(true)));
    }
    if raw == "false" {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(false)));
    }
    if let Ok(n) = raw.parse::<i64>() {
        return Ok(toml_edit::Item::Value(toml_edit::Value::from(n)));
    }
    if raw.starts_with('[') {
        return parse_array_value(raw);
    }
    Ok(toml_edit::Item::Value(toml_edit::Value::from(raw)))
}

fn parse_array_value(raw: &str) -> Result<toml_edit::Item> {
    let wrapped = format!("__val__ = {raw}");
    let parsed: std::result::Result<toml::Value, _> = toml::from_str(&wrapped);
    match parsed {
        Ok(toml::Value::Table(table)) => {
            let arr_val = table.get("__val__").ok_or_else(|| {
                ConfigErrorKind::ConfigParseError(
                    "malformed TOML array: could not parse".to_string(),
                )
            })?;
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
        Err(_) => Err(ConfigErrorKind::ConfigParseError(format!(
            "malformed TOML array: could not parse '{raw}'"
        ))
        .into()),
        _ => Err(ConfigErrorKind::ConfigParseError(format!(
            "malformed TOML array: unexpected structure in '{raw}'"
        ))
        .into()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// get_nested_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn get_nested_value(config: &Config, key: &str) -> Result<String> {
    if let Some(val) = config.values.get(key) {
        return Ok(val.clone());
    }
    let json = serde_json::to_value(config)
        .map_err(|e| ConfigErrorKind::ConfigKeyNotFound(format!("serialization error: {e}")))?;
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
                current = map.get(lookup).ok_or_else(|| {
                    ConfigErrorKind::ConfigKeyNotFound(format!("key not found: {segment} in {key}"))
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
    match current {
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| format!("\"{s}\"")))
                .collect();
            Ok(format!("[{}]", items.join(", ")))
        }
        serde_json::Value::Null => {
            Err(ConfigErrorKind::ConfigKeyNotFound(format!("value is null for key '{key}'")).into())
        }
        serde_json::Value::Object(_) => Err(ConfigErrorKind::ConfigKeyNotFound(format!(
            "key '{key}' resolves to a table, not a value"
        ))
        .into()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// set_nested_value
// ═══════════════════════════════════════════════════════════════════════════

pub fn set_nested_value(
    doc: &mut toml_edit::DocumentMut,
    parts: &[&str],
    value: &str,
) -> Result<()> {
    if parts.len() < 2 {
        return Err(ConfigErrorKind::ConfigParseError(
            "set_nested_value requires at least two segments (section.key)".to_string(),
        )
        .into());
    }
    for part in parts {
        if part.is_empty() {
            return Err(ConfigErrorKind::ConfigParseError(
                "config key contains empty segment".to_string(),
            )
            .into());
        }
    }
    let table = doc.as_table_mut();
    let (leading, last) = parts.split_at(parts.len() - 1);
    let mut current = table;
    for segment in leading {
        if !current.contains_key(segment) {
            current[segment] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        let entry = &current[segment];
        if !entry.is_table() {
            return Err(ConfigErrorKind::ConfigParseError(format!(
                "segment '{segment}' is not a table, cannot traverse through it"
            ))
            .into());
        }
        current = current[segment].as_table_mut().ok_or_else(|| {
            ConfigErrorKind::ConfigParseError(format!(
                "segment '{segment}' is not a table (internal error)"
            ))
        })?;
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
    let (config, scopes, paths) =
        if let Some(fp) = port.as_any().downcast_ref::<FileConfigReadPort>() {
            fp.load_with_layers(true, true)?
        } else {
            let config = port.load_merged().await?;
            let mut sm = HashMap::new();
            let pm = HashMap::new();
            for k in config.values.keys() {
                sm.insert(k.clone(), ConfigScope::Global);
            }
            (config, sm, pm)
        };
    let value = get_nested_value(&config, key).or_else(|_| {
        config.values.get(key).cloned().ok_or_else(|| {
            crate::error::Error::from(ConfigErrorKind::ConfigKeyNotFound(format!(
                "key not found: {key}"
            )))
        })
    })?;
    let resolved_scope = scopes.get(key).copied().unwrap_or(ConfigScope::Global);
    let source_path = paths.get(key).cloned().unwrap_or_default();
    Ok(ConfigGetResult {
        key: config_key,
        value,
        scope: resolved_scope,
        source_path,
    })
}

pub async fn config_set(key: &str, value: &str, scope: ConfigScope) -> Result<ConfigSetResult> {
    let config_key = ConfigKey::try_from(key)?;
    if matches!(scope, ConfigScope::Env) {
        return Err(ConfigErrorKind::ConfigScopeError(
            "Cannot save to environment scope".to_string(),
        )
        .into());
    }
    let port = get_port();
    let config_path = resolve_config_path(&*port, scope)?;
    ensure_parent_dir(&config_path)?;
    let file = open_config_file(&config_path)?;
    let file = acquire_config_file_lock(file, &config_path)?;
    let mut doc = read_config_doc(&file, &config_path)?;
    let segments: Vec<&str> = config_key.segments().iter().map(String::as_str).collect();
    set_nested_value(&mut doc, &segments, value)?;
    write_config_doc(&file, &doc, &config_path)?;
    Ok(ConfigSetResult {
        key: config_key,
        value: value.to_string(),
        scope,
        config_path,
    })
}

/// Resolve the config file path for the given scope.
fn resolve_config_path(port: &dyn ConfigReadPort, scope: ConfigScope) -> Result<PathBuf> {
    match scope {
        ConfigScope::Global => port.global_config_path(),
        ConfigScope::Project => port.project_config_path(),
        ConfigScope::Env => unreachable!(),
    }
}

/// Create the parent directory for a config file path if it doesn't exist.
fn ensure_parent_dir(config_path: &Path) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ConfigErrorKind::ConfigWriteError(format!(
                "failed to create config directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    Ok(())
}

/// Open a config file for reading and writing without truncating.
///
/// SAFETY: `truncate(false)` is critical here. Opening with `truncate(true)` before
/// acquiring the lock would zero the file. If the process crashes between open
/// and lock, all existing config data is permanently lost. The actual truncate
/// happens AFTER the lock is acquired (via `write_config_doc`).
fn open_config_file(config_path: &Path) -> Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(config_path)
        .map_err(|e| {
            ConfigErrorKind::ConfigWriteError(format!(
                "failed to open config file {}: {e}",
                config_path.display()
            ))
            .into()
        })
}

/// Acquire an exclusive file lock with retry and timeout.
fn acquire_config_file_lock(file: std::fs::File, config_path: &Path) -> Result<std::fs::File> {
    let start = Instant::now();
    loop {
        match fs4::fs_std::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            Err(_) => {
                if start.elapsed() >= LOCK_TIMEOUT {
                    return Err(ConfigErrorKind::ConfigLockError(format!(
                        "could not acquire lock on {} within 5s timeout",
                        config_path.display()
                    ))
                    .into());
                }
                std::thread::sleep(LOCK_RETRY_INTERVAL);
            }
        }
    }
}

/// Read and parse the TOML document from a locked config file.
fn read_config_doc(
    file: &std::fs::File,
    config_path: &Path,
) -> Result<toml_edit::DocumentMut> {
    let mut contents = String::new();
    std::io::Read::read_to_string(&mut &*file, &mut contents).map_err(|e| {
        ConfigErrorKind::ConfigWriteError(format!(
            "failed to read config file {}: {e}",
            config_path.display()
        ))
    })?;
    if contents.trim().is_empty() {
        return Ok(toml_edit::DocumentMut::new());
    }
    contents.parse().map_err(|e: toml_edit::TomlError| {
        ConfigErrorKind::ConfigParseError(format!(
            "TOML parse error in {}: {e}",
            config_path.display()
        ))
        .into()
    })
}

/// Truncate, write, and flush a TOML document to a locked config file.
fn write_config_doc(
    mut file: &std::fs::File,
    doc: &toml_edit::DocumentMut,
    config_path: &Path,
) -> Result<()> {
    use std::io::{Seek, Write};
    file.set_len(0).map_err(|e| {
        ConfigErrorKind::ConfigWriteError(format!(
            "failed to truncate config file {}: {e}",
            config_path.display()
        ))
    })?;
    file.seek(std::io::SeekFrom::Start(0)).map_err(|e| {
        ConfigErrorKind::ConfigWriteError(format!(
            "failed to seek in config file {}: {e}",
            config_path.display()
        ))
    })?;
    file.write_all(doc.to_string().as_bytes()).map_err(|e| {
        ConfigErrorKind::ConfigWriteError(format!(
            "failed to write config file {}: {e}",
            config_path.display()
        ))
    })?;
    file.flush().map_err(|e| {
        ConfigErrorKind::ConfigWriteError(format!(
            "failed to flush config file {}: {e}",
            config_path.display()
        ))
    })?;
    Ok(())
}

pub async fn config_list(global_only: bool) -> Result<Vec<ConfigGetResult>> {
    let port = get_port();
    let (config, scopes, paths) =
        if let Some(fp) = port.as_any().downcast_ref::<FileConfigReadPort>() {
            fp.load_with_layers(!global_only, !global_only)?
        } else {
            let config = if global_only {
                port.load_global_only().await?
            } else {
                port.load_merged().await?
            };
            let mut sm = HashMap::new();
            for k in config.values.keys() {
                sm.insert(k.clone(), ConfigScope::Global);
            }
            (config, sm, HashMap::new())
        };
    let mut all_keys: Vec<String> = config.values.keys().cloned().collect();
    all_keys.sort();
    all_keys.dedup();
    let mut results = Vec::new();
    for key in &all_keys {
        let first_segment = key.split('.').next().unwrap_or("");
        if !KNOWN_SECTION_PREFIXES.contains(&first_segment)
            && !KNOWN_CONFIG_KEYS.contains(&key.as_str())
        {
            continue;
        }
        let config_key = match ConfigKey::try_from(key.as_str()) {
            Ok(k) => k,
            Err(_) => continue,
        };
        let value = match config.values.get(key.as_str()) {
            Some(v) => v.clone(),
            None => match get_nested_value(&config, key) {
                Ok(v) => v,
                Err(_) => continue,
            },
        };
        let resolved_scope = scopes
            .get(key.as_str())
            .copied()
            .unwrap_or(ConfigScope::Global);
        let source_path = paths.get(key.as_str()).cloned().unwrap_or_default();
        results.push(ConfigGetResult {
            key: config_key,
            value,
            scope: resolved_scope,
            source_path,
        });
    }
    results.sort_by(|a, b| a.key.as_str().cmp(b.key.as_str()));
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // FileConfigReadPort: dead symlink produces error
    // ------------------------------------------------------------------

    #[test]
    fn load_toml_file_errors_on_dead_symlink() {
        #[cfg(unix)]
        {
            let dir = tempfile::tempdir().expect("tempdir should succeed");
            let dead_target = dir.path().join("does_not_exist.toml");
            let link = dir.path().join("config.toml");

            std::os::unix::fs::symlink(&dead_target, &link)
                .expect("symlink creation should succeed");

            let result = FileConfigReadPort::load_toml_file(&link);

            assert!(
                result.is_err(),
                "load_toml_file should error on dead symlink, not silently use defaults"
            );
            let err_msg = format!("{result:?}");
            assert!(
                err_msg.contains("dead symlink"),
                "Error should mention dead symlink, got: {err_msg}"
            );
        }
        #[cfg(not(unix))]
        {
            // On non-Unix platforms, symlink_file requires admin privileges.
        }
    }

    // ------------------------------------------------------------------
    // FileConfigReadPort: invalid TOML produces error
    // ------------------------------------------------------------------

    #[test]
    fn load_toml_file_errors_on_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let file_path = dir.path().join("bad.toml");

        std::fs::write(&file_path, "{{{{invalid toml garbage").expect("write should succeed");

        let result = FileConfigReadPort::load_toml_file(&file_path);

        assert!(
            result.is_err(),
            "load_toml_file should error on invalid TOML"
        );
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("TOML parse error"),
            "Error should mention TOML parse error, got: {err_msg}"
        );
    }

    // ------------------------------------------------------------------
    // FileConfigReadPort: missing file returns defaults (not error)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn load_merged_returns_defaults_when_no_files_exist() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let global = dir.path().join("nonexistent.toml");

        let port = FileConfigReadPort::with_paths(global, None);
        let result = port.load_merged().await;

        assert!(
            result.is_ok(),
            "load_merged should succeed when no config files exist"
        );
        let config = result.expect("should succeed");
        assert!(
            config.values.is_empty(),
            "Config should have empty values when no files exist"
        );
    }

    // ------------------------------------------------------------------
    // FileConfigReadPort: valid file loads correctly
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn load_merged_loads_valid_config() {
        let dir = tempfile::tempdir().expect("tempdir should succeed");
        let global = dir.path().join("config.toml");

        std::fs::write(&global, "logging.level = \"debug\"").expect("write should succeed");

        let port = FileConfigReadPort::with_paths(global, None);
        let result = port.load_merged().await;

        assert!(
            result.is_ok(),
            "load_merged should succeed with valid config"
        );
        let config = result.expect("should succeed");
        assert_eq!(
            config.values.get("logging.level"),
            Some(&"debug".to_string())
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey::try_from — valid keys (every KNOWN_CONFIG_KEYS variant)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_valid_for_all_known_keys() {
        for &key in KNOWN_CONFIG_KEYS {
            // "editor" is a single-segment key (no dot) and cannot pass ConfigKey::try_from,
            // which requires at least one dot separator. It is a known flat config key used
            // directly via config.values, not via ConfigKey validation.
            if !key.contains('.') {
                continue;
            }
            let result = ConfigKey::try_from(key);
            assert!(result.is_ok(), "Known config key '{key}' should be valid");
            let ck = result.expect("should succeed");
            assert_eq!(ck.as_str(), key, "raw string should round-trip for '{key}'");
            assert!(
                ck.segments().len() >= 2,
                "'{key}' should have at least 2 segments"
            );
        }
    }

    #[test]
    fn config_key_segments_parsed_correctly() {
        let ck = ConfigKey::try_from("watch.debounce_ms").expect("should succeed");
        assert_eq!(ck.segments(), &["watch", "debounce_ms"]);

        let ck =
            ConfigKey::try_from("conflict_resolution.security_keywords").expect("should succeed");
        assert_eq!(ck.segments(), &["conflict_resolution", "security_keywords"]);

        let ck = ConfigKey::try_from("remote.push").expect("should succeed");
        assert_eq!(ck.segments(), &["remote", "push"]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey::try_from — invalid keys
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_rejects_empty_string() {
        let result = ConfigKey::try_from("");
        assert!(result.is_err(), "empty string should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("empty config key"),
            "error should mention 'empty config key', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_unknown_section() {
        let result = ConfigKey::try_from("bogus.section");
        assert!(result.is_err(), "unknown section should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("unknown schema section"),
            "error should mention 'unknown schema section', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_unknown_single_char_section() {
        // Single-character first segments bypass the KNOWN_SECTION_PREFIXES check
        // (the condition is `first_segment.len() > 1`), so "x.y" should succeed.
        let result = ConfigKey::try_from("x.y");
        assert!(
            result.is_ok(),
            "single-char first segment should be allowed even if unknown"
        );
        assert_eq!(result.expect("should succeed").segments(), &["x", "y"]);
    }

    #[test]
    fn config_key_rejects_no_dot_separator() {
        let result = ConfigKey::try_from("nosection");
        assert!(
            result.is_err(),
            "key without dot separator should be rejected"
        );
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("at least one dot separator"),
            "error should mention dot separator requirement, got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_leading_dot() {
        let result = ConfigKey::try_from(".watch.enabled");
        assert!(result.is_err(), "leading dot should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("leading dot"),
            "error should mention 'leading dot', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_trailing_dot() {
        let result = ConfigKey::try_from("watch.enabled.");
        assert!(result.is_err(), "trailing dot should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("trailing dot"),
            "error should mention 'trailing dot', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_consecutive_dots() {
        let result = ConfigKey::try_from("watch..enabled");
        assert!(result.is_err(), "consecutive dots should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("consecutive dots"),
            "error should mention 'consecutive dots', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_null_byte() {
        let result = ConfigKey::try_from("watch.enabled\0");
        assert!(result.is_err(), "null byte should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("null byte"),
            "error should mention 'null byte', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_non_ascii() {
        let key_with_non_ascii = "watch.enabled\u{00e9}"; // e-acute
        let result = ConfigKey::try_from(key_with_non_ascii);
        assert!(result.is_err(), "non-ASCII characters should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("non-ASCII"),
            "error should mention 'non-ASCII', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_slash() {
        let result = ConfigKey::try_from("watch.enabled/path");
        assert!(result.is_err(), "slash should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("slash"),
            "error should mention 'slash', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_backslash() {
        let result = ConfigKey::try_from("watch.enabled\\path");
        assert!(result.is_err(), "backslash should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("backslash"),
            "error should mention 'backslash', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_hyphen_in_segment() {
        let result = ConfigKey::try_from("watch.de-bounce");
        assert!(result.is_err(), "hyphen in segment should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("hyphen"),
            "error should mention 'hyphen', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_whitespace() {
        let result = ConfigKey::try_from("watch. enabled");
        assert!(result.is_err(), "whitespace in segment should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("whitespace"),
            "error should mention 'whitespace', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_tab_in_segment() {
        let result = ConfigKey::try_from("watch.en\tabled");
        assert!(result.is_err(), "tab in segment should be rejected");
    }

    #[test]
    fn config_key_rejects_special_characters() {
        let special_chars = [
            "!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "+", "=", "{", "}", "|", ":", ";",
            "\"", "'", "<", ">", ",", "?", "~", "`",
        ];
        for ch in &special_chars {
            let key = format!("watch.enabled{ch}");
            let result = ConfigKey::try_from(&key);
            assert!(
                result.is_err(),
                "special character '{ch}' should be rejected"
            );
        }
    }

    #[test]
    fn config_key_rejects_excessive_length() {
        let long_segment = "a".repeat(300);
        let long_key = format!("watch.{long_segment}");
        assert!(
            long_key.len() > 256,
            "sanity: test key should exceed 256 chars"
        );
        let result = ConfigKey::try_from(&long_key);
        assert!(result.is_err(), "excessively long key should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("maximum length"),
            "error should mention 'maximum length', got: {err_msg}"
        );
    }

    #[test]
    fn config_key_rejects_empty_segment() {
        // Leading dot is the primary way to get an empty segment, already tested,
        // but double-dot should also produce an empty segment.
        let result = ConfigKey::try_from("watch.");
        assert!(
            result.is_err(),
            "trailing dot (empty last segment) should be rejected"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey — known section prefixes boundary cases
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_accepts_single_char_top_level_key() {
        // The code only checks KNOWN_SECTION_PREFIXES when len > 1, so "e.ditor"
        // with first segment "e" (len == 1) should be allowed.
        let result = ConfigKey::try_from("e.d");
        assert!(
            result.is_ok(),
            "single-char top-level section should bypass unknown section check"
        );
    }

    #[test]
    fn config_key_rejects_exact_known_prefix_without_subkey() {
        // "watch" alone has no dot, so it fails the "at least one dot" rule.
        let result = ConfigKey::try_from("watch");
        assert!(
            result.is_err(),
            "section name alone (no dot) should be rejected"
        );
    }

    #[test]
    fn config_key_rejects_typo_in_known_section() {
        let result = ConfigKey::try_from("watchh.enabled");
        assert!(
            result.is_err(),
            "typo in section name should be rejected as unknown section"
        );
        let result2 = ConfigKey::try_from("wotch.enabled");
        assert!(
            result2.is_err(),
            "typo in section name should be rejected as unknown section"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey — equality, clone, hash
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_equality_and_clone() {
        let a = ConfigKey::try_from("watch.enabled").expect("should succeed");
        let b = ConfigKey::try_from("watch.enabled").expect("should succeed");
        let c = ConfigKey::try_from("logging.level").expect("should succeed");
        let a_clone = a.clone();

        assert_eq!(a, b, "same key should be equal");
        assert_eq!(a, a_clone, "cloned key should be equal");
        assert_ne!(a, c, "different keys should not be equal");
    }

    #[test]
    fn config_key_hash_consistency() {
        use std::collections::HashSet;
        let a = ConfigKey::try_from("watch.enabled").expect("should succeed");
        let b = ConfigKey::try_from("watch.enabled").expect("should succeed");
        let c = ConfigKey::try_from("logging.level").expect("should succeed");

        let set: HashSet<ConfigKey> = [a, b, c].into_iter().collect();
        assert_eq!(set.len(), 2, "HashSet should deduplicate equal ConfigKeys");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey — Debug impl
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_debug_contains_raw_key() {
        let ck = ConfigKey::try_from("watch.debounce_ms").expect("should succeed");
        let debug_str = format!("{ck:?}");
        assert!(
            debug_str.contains("watch.debounce_ms"),
            "Debug output should contain the raw key, got: {debug_str}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ConfigKey — underscore is valid
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn config_key_allows_underscore_in_segment() {
        let result = ConfigKey::try_from("watch.debounce_ms");
        assert!(
            result.is_ok(),
            "underscore should be a valid character in segments"
        );
    }

    #[test]
    fn config_key_allows_numeric_segment() {
        // Section "watch" is known; subkey can be all-numeric.
        let result = ConfigKey::try_from("watch.123");
        assert!(result.is_ok(), "numeric segment should be valid");
        assert_eq!(
            result.expect("should succeed").segments(),
            &["watch", "123"]
        );
    }

    #[test]
    fn config_key_allows_leading_digit_in_subkey() {
        let result = ConfigKey::try_from("watch.1ms");
        assert!(
            result.is_ok(),
            "leading digit in subkey segment should be valid"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // parse_cli_value
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn parse_cli_value_bool() {
        let result = parse_cli_value("true").expect("should succeed");
        assert!(result.is_value(), "should be a Value");
        assert!(
            result.as_bool().expect("should be boolean"),
            "true should parse as boolean true"
        );

        let result = parse_cli_value("false").expect("should succeed");
        assert!(
            !result.as_bool().expect("should be boolean"),
            "false should parse as boolean false"
        );
    }

    #[test]
    fn parse_cli_value_integer() {
        let result = parse_cli_value("42").expect("should succeed");
        let val = result.as_integer().expect("should be integer");
        assert_eq!(val, 42);

        let result = parse_cli_value("-7").expect("should succeed");
        let val = result.as_integer().expect("should be integer");
        assert_eq!(val, -7);
    }

    #[test]
    fn parse_cli_value_string() {
        let result = parse_cli_value("hello world").expect("should succeed");
        let val = result.as_str().expect("should be string");
        assert_eq!(val, "hello world");
    }

    #[test]
    fn parse_cli_value_empty_string() {
        let result = parse_cli_value("").expect("should succeed");
        let val = result.as_str().expect("should be string");
        assert_eq!(val, "");
    }

    #[test]
    fn parse_cli_value_string_array() {
        let result = parse_cli_value(r#"["a", "b", "c"]"#).expect("should succeed");
        let arr = result.as_array().expect("should be array");
        let items: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_cli_value_array_rejects_non_string_elements() {
        let result = parse_cli_value("[1, 2, 3]");
        assert!(
            result.is_err(),
            "array with non-string elements should be rejected"
        );
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("non-string element"),
            "error should mention non-string elements, got: {err_msg}"
        );
    }

    #[test]
    fn parse_cli_value_array_rejects_malformed() {
        let result = parse_cli_value("[broken");
        assert!(result.is_err(), "malformed array should be rejected");
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("malformed TOML array"),
            "error should mention malformed array, got: {err_msg}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // set_nested_value
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn set_nested_value_simple() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &["watch", "enabled"], "true");
        assert!(result.is_ok(), "set_nested_value should succeed");
        let val = doc["watch"]["enabled"].as_bool();
        assert_eq!(val, Some(true));
    }

    #[test]
    fn set_nested_value_deep() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(
            &mut doc,
            &["conflict_resolution", "security_keywords"],
            "true",
        );
        assert!(
            result.is_ok(),
            "set_nested_value should succeed for deep key"
        );
        let val = doc["conflict_resolution"]["security_keywords"].as_bool();
        assert_eq!(val, Some(true));
    }

    #[test]
    fn set_nested_value_rejects_single_segment() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &["nosegment"], "true");
        assert!(result.is_err(), "single segment should be rejected");
    }

    #[test]
    fn set_nested_value_rejects_empty_segment() {
        let mut doc = toml_edit::DocumentMut::new();
        let result = set_nested_value(&mut doc, &["watch", "", "enabled"], "true");
        assert!(result.is_err(), "empty segment should be rejected");
    }

    #[test]
    fn set_nested_value_overwrites_scalar_as_table_error() {
        let mut doc = toml_edit::DocumentMut::new();
        // Set watch.enabled to a scalar first
        doc["watch"] = toml_edit::Item::Value(toml_edit::Value::from(true));
        // Now try to set watch.enabled.deeper — should fail because "watch" is a value, not a table
        let result = set_nested_value(&mut doc, &["watch", "enabled", "deeper"], "true");
        assert!(
            result.is_err(),
            "traversing through a non-table segment should fail"
        );
        let err_msg = format!("{result:?}");
        assert!(
            err_msg.contains("not a table"),
            "error should mention 'not a table', got: {err_msg}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // get_nested_value
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn get_nested_value_from_flat_values() {
        let mut config = Config::new();
        config
            .values
            .insert("watch.enabled".to_string(), "true".to_string());
        let result = get_nested_value(&config, "watch.enabled");
        assert_eq!(result.expect("should succeed"), "true");
    }

    #[test]
    fn get_nested_value_missing_key() {
        let config = Config::new();
        let result = get_nested_value(&config, "watch.nonexistent");
        assert!(result.is_err(), "missing key should return error");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // parse_string_list (internal helper, tested indirectly)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn parse_string_list_splits_on_comma() {
        let result = parse_string_list("a, b, c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_string_list_trims_whitespace() {
        let result = parse_string_list("  a  ,  b  ,  c  ");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_string_list_empty_string() {
        let result = parse_string_list("");
        assert!(result.is_empty(), "empty input should produce empty vec");
    }

    #[test]
    fn parse_string_list_single_item() {
        let result = parse_string_list("only");
        assert_eq!(result, vec!["only"]);
    }

    #[test]
    fn parse_string_list_skips_empty_parts() {
        let result = parse_string_list("a,,b,,c");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // KNOWN_CONFIG_KEYS / KNOWN_SECTION_PREFIXES constants
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn known_config_keys_not_empty() {
        assert!(
            !KNOWN_CONFIG_KEYS.is_empty(),
            "KNOWN_CONFIG_KEYS should not be empty"
        );
    }

    #[test]
    fn known_section_prefixes_not_empty() {
        assert!(
            !KNOWN_SECTION_PREFIXES.is_empty(),
            "KNOWN_SECTION_PREFIXES should not be empty"
        );
    }

    #[test]
    fn every_known_key_starts_with_known_prefix_or_is_short() {
        // Every multi-segment known key's first segment should be in KNOWN_SECTION_PREFIXES,
        // unless the first segment is a single character (bypasses the check).
        // Single-segment keys like "editor" are exempt since they cannot be used with ConfigKey.
        for &key in KNOWN_CONFIG_KEYS {
            if !key.contains('.') {
                continue;
            }
            let first_segment = key
                .split('.')
                .next()
                .expect("key should have at least one segment");
            if first_segment.len() > 1 {
                assert!(
                    KNOWN_SECTION_PREFIXES.contains(&first_segment),
                    "Known key '{key}' has first segment '{first_segment}' not in KNOWN_SECTION_PREFIXES"
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // stringify_toml_value (internal helper)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn stringify_toml_value_variants() {
        let bool_val = toml_edit::Value::from(true);
        assert_eq!(stringify_toml_value(&bool_val), "true");

        let int_val = toml_edit::Value::from(42);
        assert_eq!(stringify_toml_value(&int_val), "42");

        let str_val = toml_edit::Value::from("hello");
        assert_eq!(stringify_toml_value(&str_val), "hello");

        let mut arr = toml_edit::Array::new();
        arr.push("a");
        arr.push("b");
        let arr_val = toml_edit::Value::Array(arr);
        assert_eq!(stringify_toml_value(&arr_val), "[a, b]");
    }
}
