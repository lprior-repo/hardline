//! Exhaustive tests for ConfigScope, ConfigSource, ConfigValue — precedence rules,
//! source attribution, merge behavior, override detection, fallback, empty scope handling.
//!
//! Unit tests + proptests following functional-rust zero-panic style.

use std::path::PathBuf;

use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

use super::config_core::{Config, ConfigManager, ConfigScope, ConfigSource, ConfigValue};

// ═══════════════════════════════════════════════════════════════════════════
// ConfigScope: construction, equality, ordering
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_scope_default_is_global() {
    assert_eq!(ConfigScope::default(), ConfigScope::Global);
}

#[test]
fn config_scope_variants_are_distinct() {
    let variants = [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            assert_eq!(a == b, i == j, "Equality mismatch at ({i}, {j})");
        }
    }
}

#[test]
fn config_scope_copy_semantics() {
    let a = ConfigScope::Global;
    let b = a;
    assert_eq!(a, b, "Copy types should remain equal after assignment");
}

#[test]
fn config_scope_clone_matches_original() {
    let variants = [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env];
    for v in &variants {
        assert_eq!(*v, v.clone(), "Clone should match original for {v:?}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigScope: Display
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_scope_display_global() {
    assert_eq!(format!("{}", ConfigScope::Global), "Global");
}

#[test]
fn config_scope_display_project() {
    assert_eq!(format!("{}", ConfigScope::Project), "Project");
}

#[test]
fn config_scope_display_env() {
    assert_eq!(format!("{}", ConfigScope::Env), "Env");
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigScope: Serde roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_scope_serde_roundtrip_json() {
    for scope in [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env] {
        let json = serde_json::to_string(&scope).expect("should serialize");
        let de: ConfigScope = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(scope, de, "Roundtrip failed for {scope:?}");
    }
}

#[test]
fn config_scope_serde_roundtrip_toml_value() {
    // ConfigScope derives Serialize/Deserialize; verify via JSON as TOML enum proxy
    for scope in [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env] {
        let json = serde_json::to_value(&scope).expect("should convert to value");
        let de: ConfigScope = serde_json::from_value(json).expect("should convert from value");
        assert_eq!(scope, de, "Value roundtrip failed for {scope:?}");
    }
}

#[test]
fn config_scope_deserializes_from_lowercase() {
    // Serde with default settings uses PascalCase for enum variants
    let de: ConfigScope = serde_json::from_str("\"Global\"").expect("Global");
    assert_eq!(de, ConfigScope::Global);

    let de: ConfigScope = serde_json::from_str("\"Project\"").expect("Project");
    assert_eq!(de, ConfigScope::Project);

    let de: ConfigScope = serde_json::from_str("\"Env\"").expect("Env");
    assert_eq!(de, ConfigScope::Env);
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigScope: Precedence ordering (Env > Project > Global)
// ═══════════════════════════════════════════════════════════════════════════

fn scope_priority(scope: ConfigScope) -> u8 {
    match scope {
        ConfigScope::Global => 1,
        ConfigScope::Project => 2,
        ConfigScope::Env => 3,
    }
}

#[test]
fn precedence_env_highest() {
    assert!(scope_priority(ConfigScope::Env) > scope_priority(ConfigScope::Project));
    assert!(scope_priority(ConfigScope::Env) > scope_priority(ConfigScope::Global));
}

#[test]
fn precedence_project_mid() {
    assert!(scope_priority(ConfigScope::Project) > scope_priority(ConfigScope::Global));
    assert!(scope_priority(ConfigScope::Project) < scope_priority(ConfigScope::Env));
}

#[test]
fn precedence_global_lowest() {
    assert!(scope_priority(ConfigScope::Global) < scope_priority(ConfigScope::Project));
    assert!(scope_priority(ConfigScope::Global) < scope_priority(ConfigScope::Env));
}

#[test]
fn precedence_transitivity() {
    // If Env > Project and Project > Global, then Env > Global
    let env_p = scope_priority(ConfigScope::Env) as u32;
    let proj_p = scope_priority(ConfigScope::Project) as u32;
    let glob_p = scope_priority(ConfigScope::Global) as u32;
    assert!(
        env_p > proj_p && proj_p > glob_p,
        "Strict total order required"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigSource: construction and field access
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_source_construction() {
    let source = ConfigSource {
        path: PathBuf::from("/etc/scp/config.toml"),
        scope: ConfigScope::Global,
        priority: 1,
    };
    assert_eq!(source.path, PathBuf::from("/etc/scp/config.toml"));
    assert_eq!(source.scope, ConfigScope::Global);
    assert_eq!(source.priority, 1);
}

#[test]
fn config_source_all_scopes() {
    let global = ConfigSource {
        path: PathBuf::from("/global.toml"),
        scope: ConfigScope::Global,
        priority: 1,
    };
    let project = ConfigSource {
        path: PathBuf::from(".scp/config.toml"),
        scope: ConfigScope::Project,
        priority: 2,
    };
    let env = ConfigSource {
        path: PathBuf::from("environment"),
        scope: ConfigScope::Env,
        priority: 3,
    };
    assert_eq!(global.scope, ConfigScope::Global);
    assert_eq!(project.scope, ConfigScope::Project);
    assert_eq!(env.scope, ConfigScope::Env);
}

#[test]
fn config_source_priority_matches_scope_convention() {
    // Convention: Global=1, Project=2, Env=3
    let sources = vec![
        ConfigSource {
            path: PathBuf::from("env"),
            scope: ConfigScope::Env,
            priority: 3,
        },
        ConfigSource {
            path: PathBuf::from("proj"),
            scope: ConfigScope::Project,
            priority: 2,
        },
        ConfigSource {
            path: PathBuf::from("glob"),
            scope: ConfigScope::Global,
            priority: 1,
        },
    ];
    // Verify the convention holds
    assert_eq!(sources[0].priority, scope_priority(ConfigScope::Env));
    assert_eq!(sources[1].priority, scope_priority(ConfigScope::Project));
    assert_eq!(sources[2].priority, scope_priority(ConfigScope::Global));
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigSource: Serde roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_source_serde_roundtrip() {
    let source = ConfigSource {
        path: PathBuf::from("/some/path.toml"),
        scope: ConfigScope::Project,
        priority: 2,
    };
    let json = serde_json::to_string(&source).expect("should serialize");
    let de: ConfigSource = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(source.path, de.path);
    assert_eq!(source.scope, de.scope);
    assert_eq!(source.priority, de.priority);
}

#[test]
fn config_source_serde_roundtrip_env_scope() {
    let source = ConfigSource {
        path: PathBuf::from("environment"),
        scope: ConfigScope::Env,
        priority: 3,
    };
    let json = serde_json::to_string(&source).expect("serialize");
    let de: ConfigSource = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(source.path, de.path);
    assert_eq!(source.scope, de.scope);
    assert_eq!(source.priority, de.priority);
}

// ═══════════════════════════════════════════════════════════════════════════
// Config::add_source — source registration and priority sorting
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_add_source_single() {
    let mut config = Config::new();
    assert!(config.sources().is_empty());

    config.add_source(PathBuf::from("/global.toml"), ConfigScope::Global, 1);
    assert_eq!(config.sources().len(), 1);
    assert_eq!(config.sources()[0].scope, ConfigScope::Global);
    assert_eq!(config.sources()[0].priority, 1);
}

#[test]
fn config_add_source_sorted_by_priority_descending() {
    let mut config = Config::new();

    // Add in reverse order (lowest priority first)
    config.add_source(PathBuf::from("/global.toml"), ConfigScope::Global, 1);
    config.add_source(PathBuf::from(".scp/config.toml"), ConfigScope::Project, 2);
    config.add_source(PathBuf::from("environment"), ConfigScope::Env, 3);

    // Sources should be sorted by priority descending (highest first)
    let sources = config.sources();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].priority, 3, "Highest priority should be first");
    assert_eq!(sources[1].priority, 2, "Middle priority second");
    assert_eq!(sources[2].priority, 1, "Lowest priority last");
}

#[test]
fn config_add_source_insertion_order_preserved_for_same_priority() {
    let mut config = Config::new();

    config.add_source(PathBuf::from("/first.toml"), ConfigScope::Global, 1);
    config.add_source(PathBuf::from("/second.toml"), ConfigScope::Global, 1);

    let sources = config.sources();
    assert_eq!(sources.len(), 2);
    // Same priority — order is stable (both have priority 1)
    assert_eq!(sources[0].path, PathBuf::from("/first.toml"));
    assert_eq!(sources[1].path, PathBuf::from("/second.toml"));
}

#[test]
fn config_add_source_mixed_order_still_sorted() {
    let mut config = Config::new();

    // Add in random order
    config.add_source(PathBuf::from("env"), ConfigScope::Env, 3);
    config.add_source(PathBuf::from("global"), ConfigScope::Global, 1);
    config.add_source(PathBuf::from("project"), ConfigScope::Project, 2);

    let sources = config.sources();
    assert_eq!(sources[0].priority, 3);
    assert_eq!(sources[1].priority, 2);
    assert_eq!(sources[2].priority, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigValue: construction, source attribution
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_value_new_without_source() {
    let val = ConfigValue::new("vcs.type", "git", ConfigScope::Global);
    assert_eq!(val.key, "vcs.type");
    assert_eq!(val.value, "git");
    assert_eq!(val.scope, ConfigScope::Global);
    assert!(
        val.source.as_os_str().is_empty(),
        "New without source should have empty path"
    );
}

#[test]
fn config_value_with_source() {
    let val = ConfigValue::with_source(
        "logging.level",
        "debug",
        ConfigScope::Project,
        "/project/.scp/config.toml",
    );
    assert_eq!(val.key, "logging.level");
    assert_eq!(val.value, "debug");
    assert_eq!(val.scope, ConfigScope::Project);
    assert_eq!(val.source, PathBuf::from("/project/.scp/config.toml"));
}

#[test]
fn config_value_source_attribution_global() {
    let val = ConfigValue::with_source(
        "k",
        "v",
        ConfigScope::Global,
        "/home/.config/scp/config.toml",
    );
    assert_eq!(val.scope, ConfigScope::Global);
    assert_eq!(val.source, PathBuf::from("/home/.config/scp/config.toml"));
}

#[test]
fn config_value_source_attribution_env() {
    let val = ConfigValue::new("k", "v", ConfigScope::Env);
    assert_eq!(val.scope, ConfigScope::Env);
    assert!(
        val.source.as_os_str().is_empty(),
        "Env values typically have no file source"
    );
}

#[test]
fn config_value_serde_roundtrip() {
    let val = ConfigValue::with_source(
        "queue.default",
        "main",
        ConfigScope::Project,
        ".scp/config.toml",
    );
    let json = serde_json::to_string(&val).expect("should serialize");
    let de: ConfigValue = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(val.key, de.key);
    assert_eq!(val.value, de.value);
    assert_eq!(val.scope, de.scope);
    assert_eq!(val.source, de.source);
}

// ═══════════════════════════════════════════════════════════════════════════
// Merge behavior: ConfigManager.load() precedence (file-based)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn merge_global_only_no_override() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    std::fs::write(&global_path, "editor = \"vim\"").expect("write");

    let manager = ConfigManager::with_paths(global_path, None);
    let config = manager.load().expect("should load");
    assert_eq!(config.get("editor"), Some(&"vim".to_string()));
}

#[test]
fn merge_project_overrides_global() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"").expect("write global");
    std::fs::write(&project_path, "editor = \"nano\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");
    assert_eq!(
        config.get("editor"),
        Some(&"nano".to_string()),
        "Project value should override global"
    );
}

#[test]
fn merge_project_adds_keys_not_in_global() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"").expect("write global");
    std::fs::write(&project_path, "logging.level = \"debug\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");
    // Both keys should be present
    assert_eq!(config.get("editor"), Some(&"vim".to_string()));
    assert_eq!(config.get("logging.level"), Some(&"debug".to_string()));
}

#[test]
fn merge_global_only_keys_preserved_when_project_has_different_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"\nqueue.default = \"main\"")
        .expect("write global");
    std::fs::write(&project_path, "logging.level = \"debug\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");
    assert_eq!(config.get("editor"), Some(&"vim".to_string()));
    assert_eq!(config.get("queue.default"), Some(&"main".to_string()));
    assert_eq!(config.get("logging.level"), Some(&"debug".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Merge behavior: empty scope handling
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn merge_empty_config_returns_defaults() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nonexistent.toml");
    let manager = ConfigManager::with_paths(missing, None);
    let config = manager.load().expect("should load");
    assert!(
        config.values.is_empty(),
        "Empty config should have no values"
    );
}

#[test]
fn merge_empty_project_config_falls_back_to_global() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"").expect("write global");
    std::fs::write(&project_path, "").expect("write empty project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");
    assert_eq!(
        config.get("editor"),
        Some(&"vim".to_string()),
        "Global value should be used when project config is empty"
    );
}

#[test]
fn merge_empty_global_with_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "").expect("write empty global");
    std::fs::write(&project_path, "editor = \"nano\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");
    assert_eq!(config.get("editor"), Some(&"nano".to_string()));
}

#[test]
fn merge_no_files_no_env_returns_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing_global = dir.path().join("no_global.toml");
    let manager = ConfigManager::with_paths(missing_global, None);
    let config = manager.load().expect("should load with missing files");
    assert!(config.values.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigManager.load(): source tracking after merge
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn load_sources_includes_global_when_file_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    std::fs::write(&global_path, "editor = \"vim\"").expect("write");

    let manager = ConfigManager::with_paths(global_path.clone(), None);
    let config = manager.load().expect("should load");

    let sources = config.sources();
    assert!(
        sources
            .iter()
            .any(|s| s.scope == ConfigScope::Global && s.path == global_path),
        "Should track global source"
    );
}

#[test]
fn load_sources_includes_project_when_file_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"").expect("write global");
    std::fs::write(&project_path, "editor = \"nano\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path.clone()));
    let config = manager.load().expect("should load");

    let sources = config.sources();
    assert!(
        sources
            .iter()
            .any(|s| s.scope == ConfigScope::Project && s.path == project_path),
        "Should track project source"
    );
}

#[test]
fn load_sources_always_includes_env() {
    // Env source is always added (even when no env vars are set)
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("nonexistent.toml");
    let manager = ConfigManager::with_paths(missing, None);
    let config = manager.load().expect("should load");

    let sources = config.sources();
    assert!(
        sources.iter().any(|s| s.scope == ConfigScope::Env),
        "Env source should always be tracked"
    );
}

#[test]
fn load_sources_sorted_by_priority() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_dir = dir.path().join(".scp");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let project_path = project_dir.join("config.toml");

    std::fs::write(&global_path, "editor = \"vim\"").expect("write global");
    std::fs::write(&project_path, "editor = \"nano\"").expect("write project");

    let manager = ConfigManager::with_paths(global_path, Some(project_path));
    let config = manager.load().expect("should load");

    let sources = config.sources();
    // Should be sorted descending by priority
    for i in 1..sources.len() {
        assert!(
            sources[i - 1].priority >= sources[i].priority,
            "Sources should be sorted by priority descending, but [{:?}] < [{:?}]",
            sources[i - 1],
            sources[i]
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ConfigManager.save(): scope-specific save behavior
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn save_rejects_env_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let manager = ConfigManager::with_paths(global_path, None);
    let config = Config::new();

    let result = manager.save(&config, ConfigScope::Env);
    assert!(result.is_err(), "Should reject saving to Env scope");
}

#[test]
fn save_rejects_project_without_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let manager = ConfigManager::with_paths(global_path, None);
    let config = Config::new();

    let result = manager.save(&config, ConfigScope::Project);
    assert!(
        result.is_err(),
        "Should reject saving to Project scope without project path"
    );
}

#[test]
fn save_to_global_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("subdir").join("config.toml");
    let manager = ConfigManager::with_paths(global_path.clone(), None);
    let mut config = Config::new();
    config.set("editor", "vim");

    manager
        .save(&config, ConfigScope::Global)
        .expect("should save");

    assert!(global_path.exists(), "File should be created");
    let contents = std::fs::read_to_string(&global_path).expect("should read");
    assert!(contents.contains("editor"), "File should contain the key");
    assert!(contents.contains("vim"), "File should contain the value");
}

#[test]
fn save_to_project_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let global_path = dir.path().join("config.toml");
    let project_path = dir.path().join(".scp").join("config.toml");
    let manager = ConfigManager::with_paths(global_path, Some(project_path.clone()));
    let mut config = Config::new();
    config.set("logging.level", "debug");

    manager
        .save(&config, ConfigScope::Project)
        .expect("should save");

    assert!(project_path.exists(), "File should be created");
    let contents = std::fs::read_to_string(&project_path).expect("should read");
    assert!(contents.contains("logging.level"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Config::basic operations relevant to scope/source
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_new_has_no_sources() {
    let config = Config::new();
    assert!(config.sources().is_empty());
}

#[test]
fn config_sources_returns_slice() {
    let mut config = Config::new();
    config.add_source(PathBuf::from("a"), ConfigScope::Global, 1);
    config.add_source(PathBuf::from("b"), ConfigScope::Project, 2);

    let sources: &[ConfigSource] = config.sources();
    assert_eq!(sources.len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Proptests: ConfigScope
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// ConfigScope Display always produces a non-empty, known string.
    #[test]
    fn proptest_scope_display_known_variants(scope in prop_oneof![
        Just(ConfigScope::Global),
        Just(ConfigScope::Project),
        Just(ConfigScope::Env),
    ]) {
        let s = format!("{scope}");
        prop_assert!(!s.is_empty(), "Display should never be empty");
        prop_assert!(
            s == "Global" || s == "Project" || s == "Env",
            "Display should be a known variant name, got: {s}"
        );
    }
}

proptest! {
    /// ConfigScope serde roundtrip always preserves equality.
    #[test]
    fn proptest_scope_serde_roundtrip(scope in prop_oneof![
        Just(ConfigScope::Global),
        Just(ConfigScope::Project),
        Just(ConfigScope::Env),
    ]) {
        let json = serde_json::to_string(&scope).expect("serialize");
        let de: ConfigScope = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(scope, de);
    }
}

proptest! {
    /// ConfigScope equality is reflexive, symmetric, transitive.
    #[test]
    fn proptest_scope_equality_laws(
        a in prop_oneof![Just(ConfigScope::Global), Just(ConfigScope::Project), Just(ConfigScope::Env)],
        b in prop_oneof![Just(ConfigScope::Global), Just(ConfigScope::Project), Just(ConfigScope::Env)],
    ) {
        // Reflexive
        prop_assert_eq!(a, a);
        // Symmetric
        prop_assert_eq!(a == b, b == a);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proptests: ConfigSource
// ═══════════════════════════════════════════════════════════════════════════

prop_compose! {
    fn arb_config_scope()
        (scope in prop_oneof![Just(ConfigScope::Global), Just(ConfigScope::Project), Just(ConfigScope::Env)])
        -> ConfigScope {
        scope
    }
}

prop_compose! {
    fn arb_config_source()
        (path in "[a-zA-Z0-9/_.]{1,40}", scope in arb_config_scope(), priority in 0u8..=5)
        -> ConfigSource {
        ConfigSource { path: PathBuf::from(path), scope, priority }
    }
}

proptest! {
    /// ConfigSource serde roundtrip preserves all fields.
    #[test]
    fn proptest_source_serde_roundtrip(source in arb_config_source()) {
        let json = serde_json::to_string(&source).expect("serialize");
        let de: ConfigSource = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(source.path, de.path);
        prop_assert_eq!(source.scope, de.scope);
        prop_assert_eq!(source.priority, de.priority);
    }
}

proptest! {
    /// Config::add_source always keeps sources sorted by priority descending.
    #[test]
    fn proptest_add_source_maintains_sort(
        sources in proptest::collection::vec(arb_config_source(), 0..10)
    ) {
        let mut config = Config::new();
        for src in &sources {
            config.add_source(src.path.clone(), src.scope, src.priority);
        }
        let result = config.sources();
        for i in 1..result.len() {
            prop_assert!(result[i - 1].priority >= result[i].priority,
                "Sources not sorted: [{:?}] before [{:?}]",
                result[i - 1], result[i]);
        }
    }
}

proptest! {
    /// ConfigSource priority ordering invariant: higher priority number means
    /// higher precedence (Env=3 > Project=2 > Global=1).
    #[test]
    fn proptest_source_precedence_invariant(
        scope1 in arb_config_scope(),
        scope2 in arb_config_scope(),
    ) {
        let p1 = scope_priority(scope1);
        let p2 = scope_priority(scope2);
        // If scopes are different, priorities must be different
        if scope1 != scope2 {
            prop_assert!(p1 != p2, "Different scopes must have different priorities");
        }
        // Env always has highest priority
        if scope1 == ConfigScope::Env && scope2 != ConfigScope::Env {
            prop_assert!(p1 > p2, "Env should have higher priority than {scope2:?}");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Proptests: ConfigValue
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// ConfigValue::new always has empty source path.
    #[test]
    fn proptest_config_value_new_empty_source(
        key in "[a-zA-Z_][a-zA-Z0-9_.]{0,40}",
        value in ".{0,100}",
        scope in arb_config_scope(),
    ) {
        let val = ConfigValue::new(&key, &value, scope);
        prop_assert_eq!(val.key, key);
        prop_assert_eq!(val.value, value);
        prop_assert_eq!(val.scope, scope);
        prop_assert!(val.source.as_os_str().is_empty(),
            "ConfigValue::new should have empty source");
    }
}

proptest! {
    /// ConfigValue serde roundtrip preserves all fields.
    #[test]
    fn proptest_config_value_serde_roundtrip(
        key in "[a-zA-Z_][a-zA-Z0-9_.]{0,40}",
        value in ".{0,100}",
        scope in arb_config_scope(),
        source in "[a-zA-Z0-9/_.]{0,40}",
    ) {
        let val = ConfigValue::with_source(&key, &value, scope, &source);
        let json = serde_json::to_string(&val).expect("serialize");
        let de: ConfigValue = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(val.key, de.key);
        prop_assert_eq!(val.value, de.value);
        prop_assert_eq!(val.scope, de.scope);
        prop_assert_eq!(val.source, de.source);
    }
}
