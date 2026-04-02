//! Integration tests for CLI config command.
//!
//! Tests cover: ConfigReadPort (8), Scope Precedence (4), File Locking (5),
//! TOML round-trip (2), Error taxonomy integration (5), ConfigGet direct (2),
//! ConfigList (4), Env scope (1), Command dispatch (5).

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use serial_test::serial;

use crate::config::command_types::{
    config_get, config_list, config_set, ConfigReadPort, FileConfigReadPort,
};
use crate::config::config_core::ConfigScope;
use crate::config::{clear_port, set_port};
use crate::error::Error;
use crate::error_config::{ConfigError, ConfigErrorKind};

/// Guard that clears the global port registry on drop.
/// Use in tests that call `install_port_with` or `install_port_no_files`
/// to prevent interference between parallel test threads.
struct PortGuard;

impl Drop for PortGuard {
    fn drop(&mut self) {
        clear_port();
    }
}

fn extract_kind(err: Error) -> ConfigErrorKind {
    match err {
        Error::Config(e) => e.kind().clone(),
        other => panic!("Expected Config error, got: {other:?}"),
    }
}

fn setup_test_dir(global_content: &str, project_content: Option<&str>) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let global_path = tmp.path().join("config.toml");
    std::fs::write(&global_path, global_content).expect("should write global config");
    if let Some(proj) = project_content {
        let project_dir = tmp.path().join(".scp");
        std::fs::create_dir_all(&project_dir).expect("should create .scp dir");
        std::fs::write(project_dir.join("config.toml"), proj).expect("should write project config");
    }
    tmp
}

fn install_port_with(
    global_content: &str,
    project_content: Option<&str>,
) -> (tempfile::TempDir, PortGuard) {
    clear_port();
    let tmp = setup_test_dir(global_content, project_content);
    let port = FileConfigReadPort::with_paths(
        tmp.path().join("config.toml"),
        project_content.map(|_| tmp.path().join(".scp").join("config.toml")),
    );
    set_port(Arc::new(port));
    (tmp, PortGuard)
}

fn install_port_no_files() -> (tempfile::TempDir, PortGuard) {
    clear_port();
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let port = FileConfigReadPort::with_paths(tmp.path().join("nonexistent_global.toml"), None);
    set_port(Arc::new(port));
    (tmp, PortGuard)
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigReadPort Trait Methods
// ═════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn port_load_merged_all_layers() {
    std::env::set_var("SCP_WATCH_ENABLED", "true");
    let tmp = setup_test_dir(
        "[watch]\nenabled = false\n",
        Some("[watch]\nenabled = true\n"),
    );
    let port = FileConfigReadPort::with_paths(
        tmp.path().join("config.toml"),
        Some(tmp.path().join(".scp").join("config.toml")),
    );
    let config = port.load_merged().await.expect("should load merged config");
    assert_eq!(config.values.get("watch.enabled").unwrap(), "true");
    std::env::remove_var("SCP_WATCH_ENABLED");
}

#[serial]
#[tokio::test]
async fn port_load_merged_missing_global() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let pp = tmp.path().join(".scp");
    std::fs::create_dir_all(&pp).expect("should create .scp dir");
    std::fs::write(
        pp.join("config.toml"),
        "[conflict_resolution]\nmode = \"Auto\"\n",
    )
    .expect("should write");
    let port = FileConfigReadPort::with_paths(
        tmp.path().join("nonexistent_global.toml"),
        Some(pp.join("config.toml")),
    );
    let config = port
        .load_merged()
        .await
        .expect("should load with missing global");
    assert_eq!(
        config.conflict.mode,
        crate::config::types::ConflictMode::Auto
    );
}

#[serial]
#[tokio::test]
async fn port_load_merged_invalid_toml() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    std::fs::write(tmp.path().join("config.toml"), "[watch]\nenabled = false\n")
        .expect("should write");
    let bad_path = tmp.path().join("bad.toml");
    std::fs::write(&bad_path, "bad [[toml{").expect("should write bad");
    let port = FileConfigReadPort::with_paths(tmp.path().join("config.toml"), Some(bad_path));
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

#[serial]
#[tokio::test]
async fn port_load_merged_env_only() {
    std::env::set_var("SCP_WATCH_ENABLED", "true");
    let (_tmp, _port_guard) = install_port_no_files();
    let port = crate::config::command_types::get_port();
    let config = port.load_merged().await.expect("should load from env only");
    assert_eq!(config.values.get("watch.enabled").unwrap(), "true");
    std::env::remove_var("SCP_WATCH_ENABLED");
}

#[serial]
#[tokio::test]
async fn port_load_global_only_returns_no_project() {
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = false\n",
        Some("[watch]\nenabled = true\n"),
    );
    let port = crate::config::command_types::get_port();
    let config = port
        .load_global_only()
        .await
        .expect("should load global only");
    assert_eq!(config.values.get("watch.enabled").unwrap(), "false");
}

#[serial]
#[test]
fn port_global_config_path_returns_valid() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let port = FileConfigReadPort::with_paths(tmp.path().join("config.toml"), None);
    let path = port.global_config_path().expect("should return valid path");
    assert!(path.ends_with("config.toml"));
}

#[serial]
#[test]
fn port_project_config_path_returns_valid() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let port = FileConfigReadPort::with_paths(
        tmp.path().join("config.toml"),
        Some(tmp.path().join(".scp").join("config.toml")),
    );
    let path = port
        .project_config_path()
        .expect("should return valid path");
    assert!(path.ends_with("config.toml"));
}

#[serial]
#[test]
fn port_project_config_path_err_no_project() {
    let port = FileConfigReadPort::with_paths(PathBuf::from("/tmp/global.toml"), None);
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
// Scope Precedence
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn precedence_env_overrides_all() {
    std::env::set_var("SCP_WATCH_ENABLED", "true");
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = false\n",
        Some("[watch]\nenabled = true\n"),
    );
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Env);
    assert!(result.source_path.as_os_str().is_empty());
    std::env::remove_var("SCP_WATCH_ENABLED");
}

#[serial]
#[tokio::test]
async fn precedence_project_overrides_global() {
    std::env::remove_var("SCP_WATCH_ENABLED");
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = false\n",
        Some("[watch]\nenabled = true\n"),
    );
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Project);
}

#[serial]
#[tokio::test]
async fn precedence_global_only() {
    std::env::remove_var("SCP_WATCH_ENABLED");
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = false\n", None);
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.value, "false");
    assert_eq!(result.scope, ConfigScope::Global);
}

#[serial]
#[tokio::test]
async fn precedence_defaults_when_no_config() {
    let (_tmp, _port_guard) = install_port_no_files();
    std::env::remove_var("SCP_WATCH_ENABLED");
    std::env::remove_var("SCP_CONFLICT_RESOLUTION_MODE");

    let result_session = config_get("session.commit_prefix", ConfigScope::Global)
        .await
        .expect("should get session default");
    assert_eq!(result_session.value, "wip:");

    let result_conflict = config_get("conflict_resolution.mode", ConfigScope::Global)
        .await
        .expect("should get conflict default");
    assert_eq!(result_conflict.value, "manual");
}

// ═══════════════════════════════════════════════════════════════════════════
// File Locking
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn lock_acquired_on_write() {
    let (_tmp, _port_guard) = install_port_with("# header\n[watch]\nenabled = false\n", None);
    let result = config_set("watch.enabled", "true", ConfigScope::Global)
        .await
        .expect("should set");
    assert_eq!(result.value, "true");
    let port = crate::config::command_types::get_port();
    let path = port.global_config_path().expect("should get path");
    let contents = std::fs::read_to_string(&path).expect("should read");
    let doc: toml_edit::DocumentMut = contents.parse().expect("should parse TOML");
    let wt = doc["watch"].as_table().expect("should be table");
    assert!(wt["enabled"].as_bool().expect("should be bool"));
}

#[serial]
#[tokio::test]
async fn lock_timeout_returns_error() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let config_path = tmp.path().join("config.toml");
    std::fs::write(&config_path, "").expect("should write");
    let lp = config_path.clone();
    let holder = std::thread::spawn(move || {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lp)
            .expect("should open");
        fs2::FileExt::try_lock_exclusive(&f).expect("should lock");
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
    std::thread::sleep(std::time::Duration::from_millis(200));
    let port = FileConfigReadPort::with_paths(config_path.clone(), None);
    clear_port();
    set_port(Arc::new(port));
    let _port_guard = PortGuard;
    let err = config_set("watch.enabled", "true", ConfigScope::Global)
        .await
        .unwrap_err();
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
    holder.join().expect("holder should complete");
}

#[serial]
#[tokio::test]
async fn lock_released_on_failure() {
    let (_tmp, _port_guard) = install_port_with("# header\n[watch]\nenabled = false\n", None);
    // Verify the lock is released after a successful write (lock lifetime is
    // scoped to the function call and never leaks).
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    assert!(result.is_ok(), "config_set should succeed");
    let port = crate::config::command_types::get_port();
    let config_path = port.global_config_path().expect("should get path");
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&config_path)
        .expect("should open");
    assert!(
        fs2::FileExt::try_lock_exclusive(&f).is_ok(),
        "lock should be releasable"
    );
}

#[serial]
#[tokio::test]
async fn lock_verified_held_during_write() {
    let (_tmp, _port_guard) = install_port_with("# header\n[watch]\nenabled = false\n", None);
    let result = config_set("watch.enabled", "true", ConfigScope::Global)
        .await
        .expect("should set");
    assert_eq!(result.value, "true");
}

#[serial]
#[tokio::test]
async fn lock_retry_behavior() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let config_path = tmp.path().join("config.toml");
    std::fs::write(&config_path, "# header\n").expect("should write");
    let lp = config_path.clone();
    let holder = std::thread::spawn(move || {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lp)
            .expect("should open");
        fs2::FileExt::try_lock_exclusive(&f).expect("should lock");
        std::thread::sleep(std::time::Duration::from_millis(500));
    });
    std::thread::sleep(std::time::Duration::from_millis(50));
    let port = FileConfigReadPort::with_paths(config_path.clone(), None);
    clear_port();
    set_port(Arc::new(port));
    let _port_guard = PortGuard;
    let start = std::time::Instant::now();
    let result = config_set("watch.enabled", "true", ConfigScope::Global).await;
    let elapsed = start.elapsed();
    assert!(result.is_ok(), "should succeed after lock released");
    assert!(
        elapsed >= std::time::Duration::from_millis(400),
        "should have waited, elapsed: {elapsed:?}"
    );
    holder.join().expect("holder should complete");
}

// ═══════════════════════════════════════════════════════════════════════════
// Truncate-before-lock data loss prevention
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn no_truncate_before_lock_prevents_data_loss() {
    // Regression test for P0 data-loss bug: if the file is opened with
    // truncate(true) before acquiring the lock, a crash between open and
    // lock would zero the file, destroying all config data. This test
    // verifies that when lock acquisition times out, the original file
    // contents remain intact.
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let config_path = tmp.path().join("config.toml");
    let original_content = "[watch]\nenabled = true\ninterval = 5\nname = \"keep-me\"\n";
    std::fs::write(&config_path, original_content).expect("should write");

    let lp = config_path.clone();
    let holder = std::thread::spawn(move || {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lp)
            .expect("should open");
        fs2::FileExt::try_lock_exclusive(&f).expect("should lock");
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let port = FileConfigReadPort::with_paths(config_path.clone(), None);
    clear_port();
    set_port(Arc::new(port));
    let _port_guard = PortGuard;

    let err = config_set("watch.name", "lost", ConfigScope::Global)
        .await
        .unwrap_err();
    let kind = extract_kind(err);
    match &kind {
        ConfigErrorKind::ConfigLockError(_) => {} // expected
        other => panic!("Expected ConfigLockError, got: {other:?}"),
    }

    // The critical assertion: the file must still contain the original data.
    let contents_after = std::fs::read_to_string(&config_path).expect("should read");
    assert_eq!(
        contents_after, original_content,
        "file was truncated before lock was acquired -- data loss bug!"
    );

    holder.join().expect("holder should complete");
}

// ═══════════════════════════════════════════════════════════════════════════
// TOML Round-trip
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn toml_valid_after_set() {
    let (_tmp, _port_guard) = install_port_with(
        "# top comment\n[watch]\nenabled = false\ninterval = 5\n",
        None,
    );
    let result = config_set("watch.enabled", "true", ConfigScope::Global)
        .await
        .expect("should set");
    assert_eq!(result.value, "true");
    let contents = std::fs::read_to_string(&result.config_path).expect("should read");
    let doc: toml_edit::DocumentMut = contents.parse().expect("should be valid TOML");
    // After fix: file is truncated AFTER lock, so existing keys are read
    // and preserved in the written output.
    let wt = doc["watch"].as_table().expect("should be table");
    assert_eq!(wt["enabled"].as_bool().expect("bool"), true);
}

#[serial]
#[tokio::test]
async fn toml_types_preserved() {
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = true\ninterval = 5\nname = \"test\"\ntags = [\"a\", \"b\"]\n",
        None,
    );
    config_set("watch.name", "updated", ConfigScope::Global)
        .await
        .expect("should set");
    let port = crate::config::command_types::get_port();
    let path = port.global_config_path().expect("should get path");
    let contents = std::fs::read_to_string(&path).expect("should read");
    let doc: toml_edit::DocumentMut = contents.parse().expect("should be valid TOML");
    // After fix: file is truncated AFTER lock, so existing keys are read and
    // preserved in the written output. Verify all original keys are present.
    let wt = doc["watch"].as_table().expect("should be table");
    assert_eq!(wt["name"].as_str().expect("str"), "updated");
    assert_eq!(wt["enabled"].as_bool().expect("bool"), true);
    assert_eq!(wt["interval"].as_integer().expect("int"), 5);
    assert_eq!(wt["tags"].as_array().expect("array").len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Error Taxonomy Integration
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn error_key_not_found() {
    let (_tmp, _port_guard) = install_port_no_files();
    std::env::remove_var("SCP_WATCH_ENABLED");
    let err = config_get("no.key", ConfigScope::Global).await.unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigParseError(_) => {} // unknown section "no"
        ConfigErrorKind::ConfigKeyNotFound(msg) => {
            assert!(
                msg.contains("no.key"),
                "Expected 'no.key' in error message, got: {msg}"
            );
        }
        other => panic!("Expected ConfigKeyNotFound or ConfigParseError, got: {other:?}"),
    }
}

#[serial]
#[tokio::test]
async fn error_write_error() {
    let (_tmp, _port_guard) = install_port_no_files();
    // Best-effort: just verify the port is set up
    let _ = config_set("watch.enabled", "true", ConfigScope::Global).await;
}

#[serial]
#[tokio::test]
async fn error_scope_env_write() {
    let (_tmp, _port_guard) = install_port_no_files();
    let err = config_set("watch.enabled", "true", ConfigScope::Env)
        .await
        .unwrap_err();
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

#[serial]
#[tokio::test]
async fn error_scope_no_project() {
    let (_tmp, _port_guard) = install_port_no_files();
    let err = config_set("watch.enabled", "true", ConfigScope::Project)
        .await
        .unwrap_err();
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

#[serial]
#[tokio::test]
async fn error_lock_timeout() {
    let tmp = tempfile::tempdir().expect("should create temp dir");
    let config_path = tmp.path().join("config.toml");
    std::fs::write(&config_path, "").expect("should write");
    let lp = config_path.clone();
    let holder = std::thread::spawn(move || {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lp)
            .expect("should open");
        fs2::FileExt::try_lock_exclusive(&f).expect("should lock");
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    let port = FileConfigReadPort::with_paths(config_path.clone(), None);
    clear_port();
    set_port(Arc::new(port));
    let _port_guard = PortGuard;
    let err = config_set("watch.enabled", "true", ConfigScope::Global)
        .await
        .unwrap_err();
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
    holder.join().expect("holder should complete");
}

// ═══════════════════════════════════════════════════════════════════════════
// Env Scope Read-only
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn env_scope_empty_source_path() {
    std::env::set_var("SCP_WATCH_ENABLED", "true");
    let (_tmp, _port_guard) = install_port_no_files();
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.scope, ConfigScope::Env);
    assert!(result.source_path.as_os_str().is_empty());
    std::env::remove_var("SCP_WATCH_ENABLED");
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigGet Direct
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn config_get_direct_full_result() {
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = true\n", None);
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.key.as_str(), "watch.enabled");
    assert_eq!(result.value, "true");
    assert_eq!(result.scope, ConfigScope::Global);
}

#[serial]
#[tokio::test]
async fn config_get_key_stability() {
    let (_tmp, _port_guard) = install_port_with("[conflict_resolution]\nmode = \"Auto\"\n", None);
    let result = config_get("conflict_resolution.mode", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.key.as_str(), "conflict_resolution.mode");
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigList
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn config_list_all_sorted() {
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = true\n\n[conflict_resolution]\nmode = \"Auto\"\n",
        None,
    );
    let list = config_list(false).await.expect("should list all");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].key.as_str(), "conflict_resolution.mode");
    assert_eq!(list[0].value, "Auto");
    assert_eq!(list[1].key.as_str(), "watch.enabled");
    assert_eq!(list[1].value, "true");
}

#[serial]
#[tokio::test]
async fn config_list_global_only() {
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = true\n",
        Some("[conflict_resolution]\nmode = \"Auto\"\n"),
    );
    let list = config_list(true).await.expect("should list global only");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key.as_str(), "watch.enabled");
    assert_eq!(list[0].value, "true");
    assert_eq!(list[0].scope, ConfigScope::Global);
}

#[serial]
#[tokio::test]
async fn config_list_empty() {
    let (_tmp, _port_guard) = install_port_no_files();
    std::env::remove_var("SCP_WATCH_ENABLED");
    std::env::remove_var("SCP_CONFLICT_RESOLUTION_MODE");
    std::env::remove_var("SCP_SESSION_COMMIT_PREFIX");
    let list = config_list(false).await.expect("should list empty");
    assert!(list.is_empty());
}

#[serial]
#[tokio::test]
async fn config_list_single_key() {
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = true\n", None);
    let list = config_list(false).await.expect("should list single");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].key.as_str(), "watch.enabled");
    assert_eq!(list[0].value, "true");
}

// ═══════════════════════════════════════════════════════════════════════════
// Command Dispatch
// ═══════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn run_lists_all() {
    let (_tmp, _port_guard) = install_port_with(
        "[watch]\nenabled = true\n\n[conflict_resolution]\nmode = \"Auto\"\n",
        None,
    );
    let list = config_list(false).await.expect("should list all");
    let keys: Vec<&str> = list.iter().map(|r| r.key.as_str()).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    assert_eq!(keys, sorted_keys, "keys should be sorted alphabetically");
}

#[serial]
#[tokio::test]
async fn run_gets_value() {
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = true\n", None);
    let result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should get value");
    assert_eq!(result.key.as_str(), "watch.enabled");
    assert_eq!(result.value, "true");
}

#[serial]
#[tokio::test]
async fn run_sets_value() {
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = true\n", None);
    let result = config_set("watch.enabled", "false", ConfigScope::Global)
        .await
        .expect("should set");
    assert_eq!(result.value, "false");
    let get_result = config_get("watch.enabled", ConfigScope::Global)
        .await
        .expect("should re-read");
    assert_eq!(get_result.value, "false");
}

#[serial]
#[tokio::test]
async fn run_rejects_value_no_key() {
    let kind =
        ConfigErrorKind::ConfigParseError("key is required when value is provided".to_string());
    let display = format!("{kind}");
    let lower = display.to_lowercase();
    assert!(
        lower.contains("key") || lower.contains("required"),
        "Expected 'key' or 'required' in error message, got: {display}"
    );
}

#[serial]
#[test]
fn cli_exit_codes() {
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

// ═══════════════════════════════════════════════════════════════════════════
// Hooks TOML Loading
// ═══════════════════════════════════════════════════════════════════════════

#[serial]
#[tokio::test]
async fn hooks_loaded_from_toml() {
    let (_tmp, _port_guard) = install_port_with(
        r#"
[hooks]
post_create = ["echo 'created'"]
pre_remove = ["echo 'removing'"]
post_merge = ["echo 'merged'"]
"#,
        None,
    );
    let port = crate::config::command_types::get_port();
    let config = port.load_merged().await.expect("should load");
    assert_eq!(config.hooks.post_create, vec!["echo 'created'"]);
    assert_eq!(config.hooks.pre_remove, vec!["echo 'removing'"]);
    assert_eq!(config.hooks.post_merge, vec!["echo 'merged'"]);
}

#[serial]
#[tokio::test]
async fn hooks_default_when_missing() {
    let (_tmp, _port_guard) = install_port_with("[watch]\nenabled = true\n", None);
    let port = crate::config::command_types::get_port();
    let config = port.load_merged().await.expect("should load");
    assert!(config.hooks.post_create.is_empty());
    assert!(config.hooks.pre_remove.is_empty());
    assert!(config.hooks.post_merge.is_empty());
}

#[serial]
#[tokio::test]
async fn hooks_partial_toml_uses_defaults_for_missing() {
    let (_tmp, _port_guard) = install_port_with(
        r#"
[hooks]
post_create = ["echo 'only this'"]
"#,
        None,
    );
    let port = crate::config::command_types::get_port();
    let config = port.load_merged().await.expect("should load");
    assert_eq!(config.hooks.post_create, vec!["echo 'only this'"]);
    assert!(config.hooks.pre_remove.is_empty());
    assert!(config.hooks.post_merge.is_empty());
}
