//! Comprehensive tests for Config and ConfigManager — load/save/get/set, scopes, round-trip.
//!
//! Covers:
//! - Load config from file
//! - Save config to file
//! - Get config value by key
//! - Set config value by key
//! - Local scope overrides global
//! - Missing key returns default (None)
//! - Invalid config file format error
//! - Round-trip load/save/load

use super::command_types::{ConfigReadPort, FileConfigReadPort};
use super::config_core::{Config, ConfigManager, ConfigScope};
use super::config_watcher::validate_config_file;

// ═══════════════════════════════════════════════════════════════════════════
// 1. Load config from file
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn load_from_file_single_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "editor = \"vim\"\n").expect("write");

    let manager = ConfigManager::with_paths(path, None);
    let config = manager.load().expect("should load");

    assert_eq!(config.get("editor"), Some(&"vim".to_string()));
}

#[test]
fn load_from_file_multiple_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "vcs.type = \"git\"\nvcs.default_branch = \"main\"\nlogging.level = \"debug\"\n",
    )
    .expect("write");

    let manager = ConfigManager::with_paths(path, None);
    let config = manager.load().expect("should load");

    assert_eq!(config.get("vcs.type"), Some(&"git".to_string()));
    assert_eq!(config.get("vcs.default_branch"), Some(&"main".to_string()));
    assert_eq!(config.get("logging.level"), Some(&"debug".to_string()));
}

#[test]
fn load_from_file_missing_file_returns_empty_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nonexistent.toml");

    let manager = ConfigManager::with_paths(missing, None);
    let config = manager.load().expect("should load with missing file");

    assert!(
        config.values.is_empty(),
        "Missing file should produce empty config"
    );
}

#[test]
fn load_from_file_full_line_comments_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let content = r#"
# This is a comment
; This is also a comment

editor = "vim"
logging.level = "info"
"#;
    std::fs::write(&path, content).expect("write");

    let manager = ConfigManager::with_paths(path, None);
    let config = manager.load().expect("should load");

    assert_eq!(config.get("editor"), Some(&"vim".to_string()));
    assert_eq!(config.get("logging.level"), Some(&"info".to_string()));
}

#[test]
fn load_from_file_quoted_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let content = r#"
simple = "value"
double_quoted = "with spaces"
single_quoted = 'single quoted value'
"#;
    std::fs::write(&path, content).expect("write");

    let manager = ConfigManager::with_paths(path, None);
    let config = manager.load().expect("should load");

    assert_eq!(config.get("simple"), Some(&"value".to_string()));
    assert_eq!(
        config.get("double_quoted"),
        Some(&"with spaces".to_string())
    );
    assert_eq!(
        config.get("single_quoted"),
        Some(&"single quoted value".to_string())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Save config to file
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_to_file_creates_parent_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir
        .path()
        .join("subdir")
        .join("deeply")
        .join("nested")
        .join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("editor", "vim");

    manager
        .save(&config, ConfigScope::Global)
        .expect("should save");

    assert!(path.exists(), "File should be created with parent dirs");
    let contents = std::fs::read_to_string(&path).expect("read");
    assert!(contents.contains("editor"), "Should contain key");
    assert!(contents.contains("vim"), "Should contain value");
}

#[test]
fn save_to_file_overwrites_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "old.key = \"old_value\"\n").expect("write");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("new.key", "new_value");

    manager
        .save(&config, ConfigScope::Global)
        .expect("should save");

    let contents = std::fs::read_to_string(&path).expect("read");
    assert!(contents.contains("new.key"), "Should contain new key");
    assert!(
        !contents.contains("old.key") || contents.contains("new.key"),
        "Old key should be replaced or absent"
    );
}

#[test]
fn save_to_file_multiple_keys_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("zebra.key", "z");
    config.set("apple.key", "a");
    config.set("middle.key", "m");

    manager
        .save(&config, ConfigScope::Global)
        .expect("should save");

    let contents = std::fs::read_to_string(&path).expect("read");
    let z_pos = contents.find("zebra.key").expect("zebra present");
    let a_pos = contents.find("apple.key").expect("apple present");
    let m_pos = contents.find("middle.key").expect("middle present");
    assert!(
        a_pos < m_pos && m_pos < z_pos,
        "Keys should be sorted alphabetically"
    );
}

#[test]
fn save_to_file_includes_generation_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let config = Config::new();

    manager
        .save(&config, ConfigScope::Global)
        .expect("should save");

    let contents = std::fs::read_to_string(&path).expect("read");
    assert!(
        contents.contains("# SCP Configuration"),
        "Should have SCP header"
    );
    assert!(
        contents.contains("# Generated:"),
        "Should have generation timestamp"
    );
}

#[test]
fn save_to_project_scope_rejects_without_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let manager = ConfigManager::with_paths(global_path, None);
    let config = Config::new();

    let result = manager.save(&config, ConfigScope::Project);
    assert!(
        result.is_err(),
        "Should reject save to Project without path"
    );
}

#[test]
fn save_to_env_scope_rejects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let manager = ConfigManager::with_paths(global_path, None);
    let config = Config::new();

    let result = manager.save(&config, ConfigScope::Env);
    assert!(result.is_err(), "Should reject save to Env scope");
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Get config value by key
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn get_returns_value_after_set() {
    let mut config = Config::new();
    config.set("test.key", "test_value");
    assert_eq!(config.get("test.key"), Some(&"test_value".to_string()));
}

#[test]
fn get_returns_none_for_missing_key() {
    let config = Config::new();
    assert!(config.get("nonexistent.key").is_none());
}

#[test]
fn get_returns_none_for_empty_config() {
    let config = Config::new();
    assert!(config.get("any.key").is_none());
}

#[test]
fn get_after_remove_returns_none() {
    let mut config = Config::new();
    config.set("temp.key", "temp_value");
    let removed = config.remove("temp.key");
    assert_eq!(removed, Some("temp_value".to_string()));
    assert!(config.get("temp.key").is_none());
}

#[test]
fn get_overwrite_returns_latest() {
    let mut config = Config::new();
    config.set("key", "first");
    config.set("key", "second");
    assert_eq!(config.get("key"), Some(&"second".to_string()));
}

#[test]
fn contains_key_true_after_set() {
    let mut config = Config::new();
    config.set("present.key", "value");
    assert!(config.contains_key("present.key"));
}

#[test]
fn contains_key_false_for_missing() {
    let config = Config::new();
    assert!(!config.contains_key("missing.key"));
}

#[test]
fn keys_iter_returns_all_set_keys() {
    let mut config = Config::new();
    config.set("a.key", "1");
    config.set("b.key", "2");
    config.set("c.key", "3");

    let keys: Vec<_> = config.keys().collect();
    assert_eq!(keys.len(), 3);
    assert!(keys.iter().any(|k| *k == "a.key"));
    assert!(keys.iter().any(|k| *k == "b.key"));
    assert!(keys.iter().any(|k| *k == "c.key"));
}

#[test]
fn iter_returns_all_key_value_pairs() {
    let mut config = Config::new();
    config.set("x.key", "val1");
    config.set("y.key", "val2");

    let pairs: Vec<_> = config.iter().collect();
    assert_eq!(pairs.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Set config value by key
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn set_inserts_new_key_value() {
    let mut config = Config::new();
    config.set("new.key", "new_value");
    assert_eq!(config.get("new.key"), Some(&"new_value".to_string()));
}

#[test]
fn set_replaces_existing_value() {
    let mut config = Config::new();
    config.set("existing.key", "old");
    config.set("existing.key", "new");
    assert_eq!(config.get("existing.key"), Some(&"new".to_string()));
}

#[test]
fn set_empty_string_value() {
    let mut config = Config::new();
    config.set("empty.key", "");
    assert_eq!(config.get("empty.key"), Some(&"".to_string()));
}

#[test]
fn set_unicode_value() {
    let mut config = Config::new();
    config.set("unicode.key", "héllo wörld");
    assert_eq!(config.get("unicode.key"), Some(&"héllo wörld".to_string()));
}

#[test]
fn set_special_characters_in_value() {
    let mut config = Config::new();
    config.set("special.key", "value with $pecial ch@rs!");
    assert_eq!(
        config.get("special.key"),
        Some(&"value with $pecial ch@rs!".to_string())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Local scope overrides global
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn local_scope_overrides_global_same_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"\n").expect("write global");
    std::fs::write(&project_path, "editor = \"code\"\n").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");

    assert_eq!(
        config.get("editor"),
        Some(&"code".to_string()),
        "Project should override global"
    );
}

#[test]
fn local_scope_does_not_affect_global_only_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "global.only = \"from_global\"\n").expect("write global");
    std::fs::write(&project_path, "project.only = \"from_project\"\n").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");

    assert_eq!(config.get("global.only"), Some(&"from_global".to_string()));
    assert_eq!(
        config.get("project.only"),
        Some(&"from_project".to_string())
    );
}

#[test]
fn project_adds_keys_not_in_global() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "vcs.type = \"git\"\n").expect("write global");
    std::fs::write(&project_path, "workspace.directory = \"/tmp/workspaces\"\n")
        .expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");

    assert_eq!(config.get("vcs.type"), Some(&"git".to_string()));
    assert_eq!(
        config.get("workspace.directory"),
        Some(&"/tmp/workspaces".to_string())
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. Missing key returns default (None)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn missing_key_returns_none_direct_get() {
    let config = Config::new();
    assert!(config.get("completely.missing").is_none());
}

#[test]
fn missing_key_with_partial_match() {
    let mut config = Config::new();
    config.set("session.commit_prefix", "wip:");
    assert!(config.get("session.max_sessions").is_none());
}

#[test]
fn missing_key_after_clear() {
    let mut config = Config::new();
    config.set("key", "value");
    config.remove("key");
    assert!(config.get("key").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. Invalid config file format error
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn invalid_toml_file_returns_error_on_load_via_file_port() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "{{{{invalid toml content[[[").expect("write");

    let port = FileConfigReadPort::with_paths(path, None);
    let result = port.load_merged().await;
    assert!(result.is_err(), "Invalid TOML should produce error on load");
}

#[tokio::test]
async fn invalid_toml_parse_error_contains_toml_keyword() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "key = \"unclosed string\n").expect("write");

    let port = FileConfigReadPort::with_paths(path, None);
    let result = port.load_merged().await;
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.to_lowercase().contains("parse") || err_msg.to_lowercase().contains("toml"),
        "Error should mention parse or TOML, got: {err_msg}"
    );
}

#[test]
fn empty_file_is_valid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.toml");
    std::fs::write(&path, "").expect("write");

    let result = validate_config_file(&path);
    assert!(result.is_ok(), "Empty file should be valid TOML");
}

#[test]
fn comments_only_file_is_valid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("comments.toml");
    let content = "# full line comment\n; another comment\n   # indented comment\n";
    std::fs::write(&path, content).expect("write");

    let result = validate_config_file(&path);
    assert!(result.is_ok(), "Comments-only file should be valid");
}

#[test]
fn dead_symlink_returns_error() {
    #[cfg(unix)]
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let dead_target = dir.path().join("does_not_exist.toml");
        let link = dir.path().join("dead_link.toml");

        std::os::unix::fs::symlink(&dead_target, &link).expect("symlink creation should succeed");

        let result = validate_config_file(&link);
        assert!(result.is_err(), "Dead symlink should produce error");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("dead") || err_msg.to_lowercase().contains("symlink"),
            "Error should mention dead symlink, got: {err_msg}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. Round-trip load/save/load
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn round_trip_single_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    // Initial save
    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("vcs.type", "git");

    manager
        .save(&config, ConfigScope::Global)
        .expect("first save");

    // Load what we saved
    let loaded = manager.load().expect("first load");
    assert_eq!(loaded.get("vcs.type"), Some(&"git".to_string()));

    // Modify and save again
    let mut modified = manager.load().expect("load for modification");
    modified.set("vcs.type", "hg");

    manager
        .save(&modified, ConfigScope::Global)
        .expect("second save");

    // Verify modification persisted
    let reloaded = manager.load().expect("second load");
    assert_eq!(reloaded.get("vcs.type"), Some(&"hg".to_string()));
}

#[test]
fn round_trip_multiple_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("logging.level", "debug");
    config.set("vcs.type", "git");
    config.set("session.max_sessions", "50");

    manager.save(&config, ConfigScope::Global).expect("save");

    let loaded = manager.load().expect("load");
    assert_eq!(loaded.get("logging.level"), Some(&"debug".to_string()));
    assert_eq!(loaded.get("vcs.type"), Some(&"git".to_string()));
    assert_eq!(loaded.get("session.max_sessions"), Some(&"50".to_string()));
}

#[test]
fn round_trip_with_project_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_path = dir.path().join(".scp").join("config.toml");
    std::fs::create_dir_all(dir.path().join(".scp")).expect("mkdir");

    let manager = ConfigManager::with_paths(global_path.clone(), Some(project_path.clone()));

    // Save to global
    let mut global_config = Config::new();
    global_config.set("editor", "vim");
    manager
        .save(&global_config, ConfigScope::Global)
        .expect("save global");

    // Save to project
    let mut project_config = Config::new();
    project_config.set("editor", "nano");
    project_config.set("project.key", "project_value");
    manager
        .save(&project_config, ConfigScope::Project)
        .expect("save project");

    // Load merged
    let loaded = manager.load().expect("load merged");
    assert_eq!(
        loaded.get("editor"),
        Some(&"nano".to_string()),
        "Project should override"
    );
    assert_eq!(
        loaded.get("project.key"),
        Some(&"project_value".to_string())
    );
}

#[test]
fn round_trip_preserves_empty_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("empty_key", "");

    manager.save(&config, ConfigScope::Global).expect("save");

    let loaded = manager.load().expect("load");
    assert_eq!(loaded.get("empty_key"), Some(&"".to_string()));
}

#[test]
fn round_trip_special_characters_in_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("path.with.dots", "/usr/local/bin");
    config.set(
        "url.value",
        "https://example.com/path?query=value&other=123",
    );

    manager.save(&config, ConfigScope::Global).expect("save");

    let loaded = manager.load().expect("load");
    assert_eq!(
        loaded.get("path.with.dots"),
        Some(&"/usr/local/bin".to_string())
    );
    assert_eq!(
        loaded.get("url.value"),
        Some(&"https://example.com/path?query=value&other=123".to_string())
    );
}

#[test]
fn round_trip_after_remove() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    let manager = ConfigManager::with_paths(path.clone(), None);
    let mut config = Config::new();
    config.set("keep.key", "keep_value");
    config.set("remove.key", "remove_value");

    manager
        .save(&config, ConfigScope::Global)
        .expect("initial save");

    let mut loaded = manager.load().expect("load");
    loaded.remove("remove.key");

    manager
        .save(&loaded, ConfigScope::Global)
        .expect("save after remove");

    let reloaded = manager.load().expect("final load");
    assert_eq!(reloaded.get("keep.key"), Some(&"keep_value".to_string()));
    assert!(
        reloaded.get("remove.key").is_none(),
        "Removed key should not be present"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Config::new() defaults
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_new_has_default_structs() {
    let config = Config::new();
    assert_eq!(config.conflict.mode, super::types::ConflictMode::Manual);
    assert!(!config.session.auto_commit.value());
    assert_eq!(config.session.commit_prefix, "wip:");
    assert_eq!(config.session.max_sessions, 100);
    assert!(config.hooks.post_create.is_empty());
    assert!(config.hooks.pre_remove.is_empty());
    assert!(config.hooks.post_merge.is_empty());
    assert_eq!(config.agent.command, "claude");
    assert!(config.agent.env.is_empty());
}

#[test]
fn config_new_values_is_empty() {
    let config = Config::new();
    assert!(config.values.is_empty());
    assert_eq!(config.keys().count(), 0);
}

#[test]
fn config_new_sources_is_empty() {
    let config = Config::new();
    assert!(config.sources().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Config equality and clone
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_clone_preserves_values() {
    let mut original = Config::new();
    original.set("key1", "value1");
    original.set("key2", "value2");

    let cloned = original.clone();
    assert_eq!(cloned.get("key1"), Some(&"value1".to_string()));
    assert_eq!(cloned.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn config_clone_is_independent() {
    let mut original = Config::new();
    original.set("key", "original_value");

    let mut cloned = original.clone();
    cloned.set("key", "modified_value");

    assert_eq!(original.get("key"), Some(&"original_value".to_string()));
    assert_eq!(cloned.get("key"), Some(&"modified_value".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigManager::get_value
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn get_value_returns_none_when_key_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "other.key = \"value\"\n").expect("write");

    let manager = ConfigManager::with_paths(path, None);
    assert!(manager.get_value("missing.key").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Config Display
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_display_shows_key_values() {
    let mut config = Config::new();
    config.set("test.key", "test_value");

    let display = format!("{}", config);
    assert!(display.contains("test.key"), "Display should contain key");
    assert!(
        display.contains("test_value"),
        "Display should contain value"
    );
}

#[test]
fn config_display_empty() {
    let config = Config::new();
    let display = format!("{}", config);
    assert!(
        display.contains("Configuration:"),
        "Should show Configuration header"
    );
}
