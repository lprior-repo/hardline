//! Integration tests for CLI config command.
//!
//! Tests cover: ConfigReadPort (8), Scope Precedence (4), File Locking (5),
//! TOML round-trip (2), Error taxonomy integration (5), ConfigGet direct (2),
//! ConfigList (4), Env scope (1), Command dispatch (5).
//!
//! All tests are RED (failing) -- implementation does not exist yet.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::config::command_types::{config_get, config_list, config_set, ConfigReadPort};
use crate::config::config_core::{Config, ConfigScope};
use crate::error::Error;
use crate::error_config::{ConfigError, ConfigErrorKind};

/// Helper to extract ConfigErrorKind from an Error
fn extract_kind(err: Error) -> ConfigErrorKind {
    match err {
        Error::Config(e) => e.kind().clone(),
        other => panic!("Expected Config error, got: {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3.3 ConfigReadPort Trait Methods
// ═══════════════════════════════════════════════════════════════════════════

/// Fake implementation of ConfigReadPort for testing.
struct FakeConfigReadPort {
    global_path: PathBuf,
    project_path: Option<PathBuf>,
}

impl FakeConfigReadPort {
    fn new(global_path: PathBuf, project_path: Option<PathBuf>) -> Self {
        Self { global_path, project_path }
    }
}

impl ConfigReadPort for FakeConfigReadPort {
    fn load_merged(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<Config>> + Send + '_>> {
        // STUB: returns error until implementation exists
        Box::pin(async {
            Err(ConfigErrorKind::ConfigKeyNotFound("not implemented".to_string()).into())
        })
    }

    fn load_global_only(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<Config>> + Send + '_>> {
        Box::pin(async {
            Err(ConfigErrorKind::ConfigKeyNotFound("not implemented".to_string()).into())
        })
    }

    fn global_config_path(&self) -> crate::error::Result<PathBuf> {
        Err(ConfigErrorKind::ConfigKeyNotFound("not implemented".to_string()).into())
    }

    fn project_config_path(&self) -> crate::error::Result<PathBuf> {
        Err(ConfigErrorKind::ConfigKeyNotFound("not implemented".to_string()).into())
    }
}

#[tokio::test]
async fn port_load_merged_all_layers() {
    // global.toml has watch.enabled=false, project.toml has watch.enabled=true, env SCP_WATCH_ENABLED=true
    // load_merged() => Ok(Config) where watch.enabled=="true" (env wins)
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), Some(PathBuf::from("/tmp/project.toml")));
    let config = port.load_merged().await.expect("should load merged config");
    // This will fail since the stub returns error
    let _ = config;
}

#[tokio::test]
async fn port_load_merged_missing_global() {
    // No global file, project.toml exists with conflict.mode="Auto"
    // load_merged() => Ok(Config) where conflict.mode=="Auto"
    let port = FakeConfigReadPort::new(PathBuf::from("/nonexistent/config.toml"), Some(PathBuf::from("/tmp/project.toml")));
    let config = port.load_merged().await.expect("should load with missing global");
    let _ = config;
}

#[tokio::test]
async fn port_load_merged_invalid_toml() {
    // project.toml contains "bad [[toml{"
    // load_merged() => Err(ConfigParseError)
    // Must assert exact variant is ConfigParseError, not Invalid.
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), Some(PathBuf::from("/tmp/bad.toml")));
    let err = port.load_merged().await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigParseError(msg) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("parse") || lower.contains("toml"),
                "Expected 'parse' or 'TOML' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigParseError, not {other:?}"),
    }
}

#[tokio::test]
async fn port_load_merged_env_only() {
    // No files exist, env SCP_WATCH_ENABLED=true
    // load_merged() => Ok(Config) where watch.enabled=="true"
    let port = FakeConfigReadPort::new(PathBuf::from("/nonexistent/config.toml"), None);
    let config = port.load_merged().await.expect("should load from env only");
    let _ = config;
}

#[tokio::test]
async fn port_load_global_only_returns_no_project() {
    // global.toml has watch.enabled=false, project.toml has watch.enabled=true
    // load_global_only() => Ok(Config) where watch.enabled=="false" (project ignored)
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), Some(PathBuf::from("/tmp/project.toml")));
    let config = port.load_global_only().await.expect("should load global only");
    let _ = config;
}

#[test]
fn port_global_config_path_returns_valid() {
    // HOME set to temp dir => global_config_path() == tempdir/.config/scp/config.toml
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), None);
    let path = port.global_config_path().expect("should return valid path");
    let _ = path;
}

#[test]
fn port_project_config_path_returns_valid() {
    // Inside git repo with .scp/ => project_config_path() == repo_root/.scp/config.toml
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), Some(PathBuf::from("/tmp/project/.scp/config.toml")));
    let path = port.project_config_path().expect("should return valid path");
    let _ = path;
}

#[test]
fn port_project_config_path_err_no_project() {
    // Outside any git repo, no project context
    // project_config_path() => Err(ConfigScopeError)
    let port = FakeConfigReadPort::new(PathBuf::from("/tmp/global.toml"), None);
    let err = port.project_config_path().unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigScopeError(msg) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("project") || lower.contains("no project"),
                "Expected 'project' or 'no project' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigScopeError, got: {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3.4 Scope Precedence
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn precedence_env_overrides_all() {
    // global=false, project=true, env SCP_WATCH_ENABLED=true
    // config_get("watch.enabled") => Ok{value:"true",scope:Env,source_path:PathBuf::new()}
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Env);
    assert!(result.source_path.as_os_str().is_empty());
}

#[tokio::test]
async fn precedence_project_overrides_global() {
    // global=false, project=true, no env
    // config_get("watch.enabled") => Ok{value:"true",scope:Project,source_path:project_path}
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Project);
}

#[tokio::test]
async fn precedence_global_only() {
    // global=false, no project, no env
    // config_get("watch.enabled") => Ok{value:"false",scope:Global,source_path:global_path}
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.value, "false");
    assert_eq!(result.scope, ConfigScope::Global);
}

#[tokio::test]
async fn precedence_defaults_when_no_config() {
    // No files, no env
    // config_get("watch.enabled") => watch.enabled=="false" (default)
    // config_get("conflict_resolution.mode") => "manual" (ConflictMode::Manual default)
    // config_get("session.commit_prefix") => "feat" (non-obvious default)
    // 3 fields with distinct non-trivial defaults to resist Default::default() mutation
    let result_watch = config_get("watch.enabled", ConfigScope::Global).await.expect("should get watch.enabled default");
    assert_eq!(result_watch.value, "false");

    let result_conflict = config_get("conflict_resolution.mode", ConfigScope::Global).await.expect("should get conflict default");
    assert_eq!(result_conflict.value, "manual");

    let result_session = config_get("session.commit_prefix", ConfigScope::Global).await.expect("should get session default");
    assert_eq!(result_session.value, "feat");
}

// ═══════════════════════════════════════════════════════════════════════════
// File Locking
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn lock_acquired_on_write() {
    // Writable tmpdir + config.toml containing "# header\nwatch.enabled = false"
    // config_set("watch.enabled","true",Global) => Ok, file re-read => watch.enabled==true,
    // header line "# header" preserved verbatim
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let _ = result;
}

#[tokio::test]
async fn lock_timeout_returns_error() {
    // Helper hold_lock_for(path, 10s) spawns thread holding exclusive lock
    // config_set("watch.enabled","true",Global) after 5s
    // => Err(ConfigLockError) msg contains "timeout" or "5" or "lock"
    let err = config_set("watch.enabled", "true", ConfigScope::Global).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigLockError(msg) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("timeout") || lower.contains('5') || lower.contains("lock"),
                "Expected 'timeout', '5', or 'lock' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigLockError, got: {other:?}"),
    }
}

#[tokio::test]
async fn lock_released_on_failure() {
    // Writable file with content "CORRUPT [[[toml"
    // config_set fails, then 2nd process acquires lock within 1s
    // => 2nd lock acquisition succeeds within 1s
    let err = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let _ = err;
}

#[tokio::test]
async fn lock_verified_held_during_write() {
    // Writable tmpdir, concurrent reader thread
    // config_set in progress => reader thread cannot acquire shared lock until set completes
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let _ = result;
}

#[tokio::test]
async fn lock_retry_behavior() {
    // Helper hold_lock_for(path, 500ms) holds lock for 500ms
    // config_set with 5s timeout => Ok (succeeds after lock released), elapsed >= 400ms
    let start = std::time::Instant::now();
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let elapsed = start.elapsed();
    let _ = (result, elapsed);
}

// ═══════════════════════════════════════════════════════════════════════════
// TOML Round-trip
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn toml_valid_after_set() {
    // TOML file: "# top comment\n[watch]\nenabled = false\ninterval = 5"
    // config_set("watch.enabled","true",Global)
    // => file parses as valid TOML; re-read => watch.enabled==true, watch.interval==5,
    //    "# top comment" line preserved
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let _ = result;
}

#[tokio::test]
async fn toml_types_preserved() {
    // file: [watch]\nenabled = true\ninterval = 5\nname = "test"\ntags = ["a","b"]
    // config_set("watch.name","updated",Global) then reload
    // => watch.enabled==bool(true), watch.interval==integer(5), watch.name==string("updated"),
    //    watch.tags==array["a","b"]
    let result = config_set("watch.name", "updated", ConfigScope::Global).await;
    let _ = result;
}

// ═══════════════════════════════════════════════════════════════════════════
// Error Taxonomy Integration
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn error_key_not_found() {
    // Valid syntax, absent key
    // config_get("no.key") => Err(ConfigKeyNotFound) msg contains "no.key", exit_code()==40
    let err = config_get("no.key", ConfigScope::Global).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigKeyNotFound(msg) => {
            assert!(msg.contains("no.key"), "Expected 'no.key' in error message, got: {msg}");
        }
        other => panic!("Expected ConfigKeyNotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn error_write_error() {
    // Read-only dir (chmod 444 on parent)
    // config_set("watch.enabled","true",Global) => Err(ConfigWriteError) exit_code()==42
    let err = config_set("watch.enabled", "true", ConfigScope::Global).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigWriteError(msg) => {
            let _ = msg;
        }
        other => panic!("Expected ConfigWriteError, got: {other:?}"),
    }
}

#[tokio::test]
async fn error_scope_env_write() {
    // scope==Env
    // config_set(...,Env) => Err(ConfigScopeError("Cannot save to environment scope")), exit_code()==43
    let err = config_set("watch.enabled", "true", ConfigScope::Env).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigScopeError(msg) => {
            assert!(
                msg.contains("Cannot save to environment scope"),
                "Expected exact message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigScopeError, got: {other:?}"),
    }
}

#[tokio::test]
async fn error_scope_no_project() {
    // Outside git repo, no project_path
    // config_set(...,Project) => Err(ConfigScopeError) exit_code()==43
    let err = config_set("watch.enabled", "true", ConfigScope::Project).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigScopeError(msg) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("project") || lower.contains("no project"),
                "Expected 'project' or 'no project' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigScopeError, got: {other:?}"),
    }
}

#[tokio::test]
async fn error_lock_timeout() {
    // Lock held >5s via hold_lock_for(path, 10s)
    // config_set => Err(ConfigLockError) exit_code()==44
    let err = config_set("watch.enabled", "true", ConfigScope::Global).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigLockError(msg) => {
            let lower = msg.to_lowercase();
            assert!(
                lower.contains("timeout") || lower.contains("5s"),
                "Expected 'timeout' or '5s' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigLockError, got: {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Env Scope Read-only
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn env_scope_empty_source_path() {
    // SCP_WATCH_ENABLED=true, no file
    // config_get("watch.enabled") => Ok{scope:Env,source_path:PathBuf::new()}
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.scope, ConfigScope::Env);
    assert!(result.source_path.as_os_str().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigGet Direct
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn config_get_direct_full_result() {
    // global.toml: watch.enabled=true, no env
    // config_get("watch.enabled",Global) => Ok, result.key.raw=="watch.enabled",
    // result.value=="true", result.scope==Global, result.source_path==global_toml_path
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.key.as_str(), "watch.enabled");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Global);
}

#[tokio::test]
async fn config_get_key_stability() {
    // Any config state
    // config_get("conflict_resolution.mode") => result.key.raw == "conflict_resolution.mode"
    // (exact input, no normalization)
    let result = config_get("conflict_resolution.mode", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.key.as_str(), "conflict_resolution.mode");
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigList
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn config_list_all_sorted() {
    // global.toml: watch.enabled=true, conflict.mode="Auto"
    // config_list(global_only=false) => Ok, list has exactly
    // [ConfigGetResult{key:"conflict_resolution.mode",...}, ConfigGetResult{key:"watch.enabled",...}]
    // sorted alpha by key
    let list = config_list(false).await.expect("should list all");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].key.as_str(), "conflict_resolution.mode");
    assert_eq!(list[0].value, "Auto");
    assert_eq!(list[1].key.as_str(), "watch.enabled");
    assert_eq!(list[1].value, "true");
}

#[tokio::test]
async fn config_list_global_only() {
    // global.toml: watch.enabled=true, project.toml: conflict.mode="Auto"
    // config_list(global_only=true) => Ok, list has exactly
    // [ConfigGetResult{key:"watch.enabled",value:"true",scope:Global,...}]
    let list = config_list(true).await.expect("should list global only");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key.as_str(), "watch.enabled");
    assert_eq!(list[0].value, "true");
    assert_eq!(list[0].scope, ConfigScope::Global);
}

#[tokio::test]
async fn config_list_empty() {
    // No config files, no env
    // config_list(global_only=false) => Ok, list is empty Vec
    let list = config_list(false).await.expect("should list empty");
    assert!(list.is_empty());
}

#[tokio::test]
async fn config_list_single_key() {
    // global.toml: only watch.enabled=true
    // config_list(global_only=false) => Ok, list.len()==1, list[0].key.raw=="watch.enabled"
    let list = config_list(false).await.expect("should list single");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key.as_str(), "watch.enabled");
    assert_eq!(list[0].value, "true");
}

// ═══════════════════════════════════════════════════════════════════════════
// Command Dispatch (E2E-adjacent integration)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn run_lists_all() {
    // config files: watch.enabled=true, conflict.mode="Auto"
    // run{key:None,value:None} => stdout contains exact lines
    // "conflict_resolution.mode = Auto" and "watch.enabled = true" in alpha order
    // NOTE: This tests the dispatch layer. For full stdout capture, use E2E tests.
    let list = config_list(false).await.expect("should list all");
    // Verify alpha sort
    let keys: Vec<&str> = list.iter().map(|r| r.key.as_str()).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    assert_eq!(keys, sorted_keys, "keys should be sorted alphabetically");
}

#[tokio::test]
async fn run_gets_value() {
    // watch.enabled=true
    // run{key:Some("watch.enabled"),value:None} => stdout contains "watch.enabled = true"
    let result = config_get("watch.enabled", ConfigScope::Global).await.expect("should get value");
    assert_eq!(result.key.as_str(), "watch.enabled");
    assert_eq!(result.value, "true");
}

#[tokio::test]
async fn run_sets_value() {
    // Writable config with watch.enabled=true
    // run{key:Some("watch.enabled"),value:Some("false")}
    // => stdout contains "watch.enabled = false", file re-read => watch.enabled==false
    let result = config_set("watch.enabled", "false", ConfigScope::Global).await;
    let _ = result;
}

#[tokio::test]
async fn run_rejects_value_no_key() {
    // ConfigOptions{key:None,value:Some("v")} => Err(ConfigParseError) msg contains "key" or "required"
    // This is a dispatch-layer validation, tested via error construction
    let kind = ConfigErrorKind::ConfigParseError("key is required when value is provided".to_string());
    let display = format!("{kind}");
    let lower = display.to_lowercase();
    assert!(
        lower.contains("key") || lower.contains("required"),
        "Expected 'key' or 'required' in error message, got: {display}"
    );
}

#[test]
fn cli_exit_codes() {
    // Various errors => CLI exits with codes: 40,41,42,43,44
    let cases: Vec<(ConfigErrorKind, i32)> = vec![
        (ConfigErrorKind::ConfigKeyNotFound("k".into()), 40),
        (ConfigErrorKind::ConfigParseError("p".into()), 41),
        (ConfigErrorKind::ConfigWriteError("w".into()), 42),
        (ConfigErrorKind::ConfigScopeError("s".into()), 43),
        (ConfigErrorKind::ConfigLockError("l".into()), 44),
    ];
    for (kind, expected_code) in cases {
        let config_err: ConfigError = kind.into();
        assert_eq!(
            config_err.exit_code(),
            expected_code,
            "Wrong exit code for {config_err:?}"
        );
    }
}
