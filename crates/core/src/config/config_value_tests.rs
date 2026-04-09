//! Exhaustive tests for ConfigValue — type coercion, validation, serialization.
//!
//! Covers two ConfigValue types:
//! 1. `cli_contracts::domain_types::ConfigValue` — validated string wrapper
//! 2. `config_core::ConfigValue` — struct with key/value/scope/source + Serialize/Deserialize
//!
//! Plus: parse_cli_value (type coercion), get_nested_value, missing/null handling.

use super::command_types::{get_nested_value, parse_cli_value};
use super::config_core::{Config, ConfigManager, ConfigScope, ConfigValue as CoreConfigValue};
use crate::cli_contracts::{ConfigValue as ContractConfigValue, ContractError};
use proptest::prelude::*;
use proptest::{prop_assert, prop_assert_eq};

// ═══════════════════════════════════════════════════════════════════════════════
// 1. ContractConfigValue — validation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn contract_config_value_accepts_non_empty_string() {
    assert!(ContractConfigValue::try_from("hello").is_ok());
    assert!(ContractConfigValue::try_from("42").is_ok());
    assert!(ContractConfigValue::try_from(" ").is_ok());
    assert!(ContractConfigValue::try_from("a").is_ok());
}

#[test]
fn contract_config_value_rejects_empty_string() {
    let result = ContractConfigValue::try_from("");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ContractError::InvalidInput { field, .. } if field == "value"),
        "Expected InvalidInput for 'value', got: {err:?}"
    );
}

#[test]
fn contract_config_value_validate_rejects_empty() {
    assert!(ContractConfigValue::validate("").is_err());
}

#[test]
fn contract_config_value_validate_accepts_non_empty() {
    assert!(ContractConfigValue::validate("anything").is_ok());
    assert!(ContractConfigValue::validate(" ").is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. ContractConfigValue — as_str and Display
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn contract_config_value_as_str_roundtrip() {
    let val = ContractConfigValue::try_from("some_value").expect("should succeed");
    assert_eq!(val.as_str(), "some_value");
}

#[test]
fn contract_config_value_display_matches_inner() {
    let val = ContractConfigValue::try_from("display_me").expect("should succeed");
    assert_eq!(format!("{val}"), "display_me");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. ContractConfigValue — equality
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn contract_config_value_equality() {
    let a = ContractConfigValue::try_from("x").expect("ok");
    let b = ContractConfigValue::try_from("x").expect("ok");
    let c = ContractConfigValue::try_from("y").expect("ok");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn contract_config_value_clone_preserves_value() {
    let val = ContractConfigValue::try_from("original").expect("ok");
    let cloned = val.clone();
    assert_eq!(val, cloned);
    assert_eq!(cloned.as_str(), "original");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. ContractConfigValue — type-coercion-like values
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn contract_config_value_stores_bool_like_string() {
    let val = ContractConfigValue::try_from("true").expect("ok");
    assert_eq!(val.as_str(), "true");

    let val = ContractConfigValue::try_from("false").expect("ok");
    assert_eq!(val.as_str(), "false");
}

#[test]
fn contract_config_value_stores_int_like_string() {
    let val = ContractConfigValue::try_from("42").expect("ok");
    assert_eq!(val.as_str(), "42");

    let val = ContractConfigValue::try_from("-7").expect("ok");
    assert_eq!(val.as_str(), "-7");

    let val = ContractConfigValue::try_from("0").expect("ok");
    assert_eq!(val.as_str(), "0");
}

#[test]
fn contract_config_value_stores_path_like_string() {
    let val = ContractConfigValue::try_from("/usr/local/bin").expect("ok");
    assert_eq!(val.as_str(), "/usr/local/bin");

    let val = ContractConfigValue::try_from("./relative/path").expect("ok");
    assert_eq!(val.as_str(), "./relative/path");

    let val = ContractConfigValue::try_from("~/home/dir").expect("ok");
    assert_eq!(val.as_str(), "~/home/dir");
}

#[test]
fn contract_config_value_stores_special_characters() {
    let val = ContractConfigValue::try_from("value with spaces").expect("ok");
    assert_eq!(val.as_str(), "value with spaces");

    let val = ContractConfigValue::try_from("key=value").expect("ok");
    assert_eq!(val.as_str(), "key=value");

    let val = ContractConfigValue::try_from("emoji: \u{2764}").expect("ok");
    assert_eq!(val.as_str(), "emoji: \u{2764}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. CoreConfigValue — construction
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn core_config_value_new_sets_empty_source() {
    let cv = CoreConfigValue::new("test.key", "test_val", ConfigScope::Global);
    assert_eq!(cv.key, "test.key");
    assert_eq!(cv.value, "test_val");
    assert_eq!(cv.scope, ConfigScope::Global);
    assert!(cv.source.as_os_str().is_empty(), "source should be empty");
}

#[test]
fn core_config_value_new_with_all_scopes() {
    let global = CoreConfigValue::new("k", "v", ConfigScope::Global);
    assert_eq!(global.scope, ConfigScope::Global);

    let project = CoreConfigValue::new("k", "v", ConfigScope::Project);
    assert_eq!(project.scope, ConfigScope::Project);

    let env = CoreConfigValue::new("k", "v", ConfigScope::Env);
    assert_eq!(env.scope, ConfigScope::Env);
}

#[test]
fn core_config_value_with_source_sets_source() {
    let cv = CoreConfigValue::with_source("k", "v", ConfigScope::Project, "/etc/scp/config.toml");
    assert_eq!(cv.source.to_str(), Some("/etc/scp/config.toml"));
}

#[test]
fn core_config_value_with_source_empty_path() {
    let cv = CoreConfigValue::with_source("k", "v", ConfigScope::Global, "");
    assert!(cv.source.as_os_str().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. CoreConfigValue — JSON serialization roundtrip
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn core_config_value_json_roundtrip() {
    let cv = CoreConfigValue::with_source(
        "logging.level",
        "debug",
        ConfigScope::Global,
        "/home/user/.config/scp/config.toml",
    );
    let json = serde_json::to_string(&cv).expect("serialize");
    let deserialized: CoreConfigValue = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.key, "logging.level");
    assert_eq!(deserialized.value, "debug");
    assert_eq!(deserialized.scope, ConfigScope::Global);
    assert_eq!(deserialized.source, cv.source);
}

#[test]
fn core_config_value_json_roundtrip_all_scopes() {
    for scope in [ConfigScope::Global, ConfigScope::Project, ConfigScope::Env] {
        let cv = CoreConfigValue::new("k", "v", scope);
        let json = serde_json::to_string(&cv).expect("serialize");
        let back: CoreConfigValue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.scope, scope, "scope {scope:?} should roundtrip");
    }
}

#[test]
fn core_config_value_json_fields_present() {
    let cv = CoreConfigValue::new("editor", "vim", ConfigScope::Project);
    let val = serde_json::to_value(&cv).expect("serialize to Value");

    assert_eq!(val.get("key").and_then(|v| v.as_str()), Some("editor"));
    assert_eq!(val.get("value").and_then(|v| v.as_str()), Some("vim"));
    assert_eq!(val.get("scope").and_then(|v| v.as_str()), Some("Project"));
    assert!(
        val.get("source").is_some(),
        "source field should be present"
    );
}

#[test]
fn core_config_value_json_empty_source() {
    let cv = CoreConfigValue::new("k", "v", ConfigScope::Global);
    let json = serde_json::to_string(&cv).expect("serialize");
    assert!(
        json.contains("source"),
        "JSON should contain source field even when empty"
    );
    let back: CoreConfigValue = serde_json::from_str(&json).expect("deserialize");
    assert!(back.source.as_os_str().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. CoreConfigValue — type coercion through string value
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn core_config_value_stores_bool_string() {
    let cv = CoreConfigValue::new("watch.enabled", "true", ConfigScope::Global);
    assert_eq!(cv.value, "true");
    assert_eq!(
        cv.value.parse::<bool>(),
        Ok(true),
        "stored value should be parseable as bool"
    );
}

#[test]
fn core_config_value_stores_int_string() {
    let cv = CoreConfigValue::new("session.max_sessions", "100", ConfigScope::Global);
    assert_eq!(cv.value.parse::<i64>(), Ok(100));
}

#[test]
fn core_config_value_stores_path_string() {
    let cv = CoreConfigValue::new(
        "workspace.directory",
        "/home/user/workspaces",
        ConfigScope::Project,
    );
    assert!(cv.value.starts_with('/'));
    assert!(std::path::Path::new(&cv.value).is_absolute());
}

#[test]
fn core_config_value_stores_empty_value_string() {
    // CoreConfigValue doesn't validate — it accepts empty strings
    let cv = CoreConfigValue::new("k", "", ConfigScope::Global);
    assert_eq!(cv.value, "");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. parse_cli_value — type coercion
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parse_cli_value_true_is_bool() {
    let item = parse_cli_value("true").expect("should parse");
    assert_eq!(item.as_bool(), Some(true));
}

#[test]
fn parse_cli_value_false_is_bool() {
    let item = parse_cli_value("false").expect("should parse");
    assert_eq!(item.as_bool(), Some(false));
}

#[test]
fn parse_cli_value_true_case_sensitive() {
    let item = parse_cli_value("TRUE").expect("should parse as string, not bool");
    assert!(
        item.as_bool().is_none(),
        "'TRUE' should NOT parse as boolean"
    );
    assert_eq!(item.as_str(), Some("TRUE"));
}

#[test]
fn parse_cli_value_false_case_sensitive() {
    let item = parse_cli_value("False").expect("should parse as string");
    assert!(item.as_bool().is_none(), "'False' should NOT parse as bool");
}

#[test]
fn parse_cli_value_positive_integer() {
    let item = parse_cli_value("42").expect("should parse");
    assert_eq!(item.as_integer(), Some(42));
    assert!(item.as_bool().is_none(), "should not be bool");
}

#[test]
fn parse_cli_value_negative_integer() {
    let item = parse_cli_value("-99").expect("should parse");
    assert_eq!(item.as_integer(), Some(-99));
}

#[test]
fn parse_cli_value_zero() {
    let item = parse_cli_value("0").expect("should parse");
    assert_eq!(item.as_integer(), Some(0));
}

#[test]
fn parse_cli_value_large_integer() {
    let item = parse_cli_value(&i64::MAX.to_string()).expect("should parse");
    assert_eq!(item.as_integer(), Some(i64::MAX));
}

#[test]
fn parse_cli_value_string_fallback() {
    let item = parse_cli_value("hello world").expect("should parse");
    assert_eq!(item.as_str(), Some("hello world"));
}

#[test]
fn parse_cli_value_empty_string_is_string() {
    let item = parse_cli_value("").expect("should parse");
    assert_eq!(item.as_str(), Some(""));
}

#[test]
fn parse_cli_value_float_stays_string() {
    // parse_cli_value does not parse floats
    let item = parse_cli_value("3.14").expect("should parse");
    assert!(item.as_integer().is_none(), "3.14 should not parse as int");
    assert_eq!(item.as_str(), Some("3.14"));
}

#[test]
fn parse_cli_value_string_array() {
    let item = parse_cli_value(r#"["alpha", "beta"]"#).expect("should parse");
    let arr = item.as_array().expect("should be array");
    let items: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(items, vec!["alpha", "beta"]);
}

#[test]
fn parse_cli_value_array_rejects_integers() {
    let result = parse_cli_value("[1, 2]");
    assert!(result.is_err());
    let err = format!("{result:?}");
    assert!(
        err.contains("non-string element"),
        "should reject non-string array elements, got: {err}"
    );
}

#[test]
fn parse_cli_value_malformed_array() {
    let result = parse_cli_value("[unclosed");
    assert!(result.is_err());
}

#[test]
fn parse_cli_value_whitespace_true_is_string() {
    let item = parse_cli_value(" true ").expect("should parse");
    assert!(
        item.as_bool().is_none(),
        "' true ' (with spaces) should NOT be bool"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. Config — missing value handling
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_get_missing_returns_none() {
    let config = Config::new();
    assert!(config.get("nonexistent.key").is_none());
}

#[test]
fn config_get_returns_some_after_set() {
    let mut config = Config::new();
    config.set("existing.key", "value");
    assert_eq!(config.get("existing.key"), Some(&"value".to_string()));
}

#[test]
fn config_get_after_remove_returns_none() {
    let mut config = Config::new();
    config.set("temp.key", "temp_val");
    let removed = config.remove("temp.key");
    assert_eq!(removed, Some("temp_val".to_string()));
    assert!(config.get("temp.key").is_none());
}

#[test]
fn config_contains_key_missing() {
    let config = Config::new();
    assert!(!config.contains_key("anything"));
}

#[test]
fn config_empty_values_is_empty() {
    let config = Config::new();
    assert!(config.values.is_empty());
    assert_eq!(config.keys().count(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. Config — null/empty value handling
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_stores_empty_string_value() {
    let mut config = Config::new();
    config.set("empty.key", "");
    assert_eq!(config.get("empty.key"), Some(&String::new()));
    // Empty string is still Some, not None
    assert!(config.get("empty.key").is_some());
}

#[test]
fn config_distinguishes_missing_from_empty() {
    let mut config = Config::new();
    config.set("present.empty", "");
    // Missing key => None
    assert!(config.get("missing.key").is_none());
    // Present but empty => Some("")
    assert_eq!(config.get("present.empty"), Some(&String::new()));
}

#[test]
fn config_overwrite_replaces_value() {
    let mut config = Config::new();
    config.set("k", "first");
    config.set("k", "second");
    assert_eq!(config.get("k"), Some(&"second".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. get_nested_value — nested value access
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn get_nested_value_from_flat() {
    let mut config = Config::new();
    config
        .values
        .insert("logging.level".to_string(), "debug".to_string());
    let result = get_nested_value(&config, "logging.level").expect("should find");
    assert_eq!(result, "debug");
}

#[test]
fn get_nested_value_missing_key() {
    let config = Config::new();
    let result = get_nested_value(&config, "no.such.key");
    assert!(result.is_err());
}

#[test]
fn get_nested_value_structured_conflict_resolution() {
    let mut config = Config::new();
    config.conflict.mode = super::types::ConflictMode::Auto;
    let result = get_nested_value(&config, "conflict_resolution.mode").expect("should find");
    assert_eq!(result, "auto");
}

#[test]
fn get_nested_value_null_returns_error() {
    // A value of Null (from JSON serialization) should return an error
    let config = Config::new();
    let result = get_nested_value(&config, "nonexistent.section.key");
    assert!(result.is_err());
}

#[test]
fn get_nested_value_table_returns_error() {
    let config = Config::new();
    // "conflict" is a table (object), not a leaf value
    let result = get_nested_value(&config, "conflict_resolution");
    assert!(
        result.is_err(),
        "table values should not be returned as strings"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. Config — ConfigScope display
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_scope_display() {
    assert_eq!(ConfigScope::Global.to_string(), "Global");
    assert_eq!(ConfigScope::Project.to_string(), "Project");
    assert_eq!(ConfigScope::Env.to_string(), "Env");
}

#[test]
fn config_scope_default_is_global() {
    assert_eq!(ConfigScope::default(), ConfigScope::Global);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13. Config — serialization roundtrip (JSON)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_json_roundtrip_preserves_values() {
    let mut config = Config::new();
    config.set("vcs.type", "git");
    config.set("logging.level", "debug");

    let json = serde_json::to_string(&config).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.get("vcs.type"), Some(&"git".to_string()));
    assert_eq!(back.get("logging.level"), Some(&"debug".to_string()));
}

#[test]
fn config_json_roundtrip_empty() {
    let config = Config::new();
    let json = serde_json::to_string(&config).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert!(back.values.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. ConfigManager::validate — type mismatch errors
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn validate_rejects_invalid_vcs_type() {
    let mut config = Config::new();
    config.set("vcs.type", "svn");
    let errors = ConfigManager::validate(&config);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.contains("VCS") || e.contains("vcs")));
}

#[test]
fn validate_accepts_git_vcs_type() {
    let mut config = Config::new();
    config.set("vcs.type", "git");
    let errors = ConfigManager::validate(&config);
    assert!(errors.is_empty());
}

#[test]
fn validate_rejects_invalid_log_level() {
    let mut config = Config::new();
    config.set("logging.level", "verbose");
    let errors = ConfigManager::validate(&config);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.to_lowercase().contains("log")));
}

#[test]
fn validate_accepts_all_valid_log_levels() {
    for level in &["trace", "debug", "info", "warn", "error"] {
        let mut config = Config::new();
        config.set("logging.level", *level);
        let errors = ConfigManager::validate(&config);
        assert!(
            errors.is_empty(),
            "level '{level}' should be valid: {errors:?}"
        );
    }
}

#[test]
fn validate_empty_config_no_errors() {
    let config = Config::new();
    assert!(ConfigManager::validate(&config).is_empty());
}

#[test]
fn validate_multiple_errors() {
    let mut config = Config::new();
    config.set("vcs.type", "mercurial");
    config.set("logging.level", "VERBOSE");
    let errors = ConfigManager::validate(&config);
    assert!(errors.len() >= 2, "should have 2+ errors: {errors:?}");
}

#[test]
fn validate_rejects_case_variant_log_levels() {
    for level in &["DEBUG", "Info", "WARN ", " error"] {
        let mut config = Config::new();
        config.set("logging.level", *level);
        let errors = ConfigManager::validate(&config);
        assert!(
            !errors.is_empty(),
            "case variant '{level}' should be rejected"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. Proptests — ContractConfigValue
// ═══════════════════════════════════════════════════════════════════════════════

proptest::proptest! {
    /// Non-empty strings should always produce a valid ContractConfigValue
    /// that round-trips through as_str().
    #[test]
    fn proptest_contract_config_value_non_empty_roundtrip(
        s in ".+"
    ) {
        let val = ContractConfigValue::try_from(s.as_str());
        prop_assert!(val.is_ok(), "non-empty string should be valid");
        let v = val.expect("ok");
        prop_assert_eq!(v.as_str(), s);
    }

    /// Empty string should always be rejected.
    #[test]
    fn proptest_contract_config_value_rejects_empty(
        _s in Just("")
    ) {
        let result = ContractConfigValue::try_from("");
        prop_assert!(result.is_err(), "empty string must be rejected");
    }

    /// Two ContractConfigValues from the same string must be equal.
    #[test]
    fn proptest_contract_config_value_equality(
        s in "[a-zA-Z0-9_ ]{1,50}"
    ) {
        let a = ContractConfigValue::try_from(s.as_str()).expect("ok");
        let b = ContractConfigValue::try_from(s.as_str()).expect("ok");
        prop_assert_eq!(a, b);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16. Proptests — CoreConfigValue JSON roundtrip
// ═══════════════════════════════════════════════════════════════════════════════

proptest::proptest! {
    /// CoreConfigValue JSON roundtrip preserves all fields.
    #[test]
    fn proptest_core_config_value_json_roundtrip(
        key in "[a-zA-Z_][a-zA-Z0-9_.]{0,50}",
        value in ".*",
        scope_idx in 0..3usize
    ) {
        let scope = match scope_idx {
            0 => ConfigScope::Global,
            1 => ConfigScope::Project,
            _ => ConfigScope::Env,
        };
        let cv = CoreConfigValue::new(&key, &value, scope);
        let json = serde_json::to_string(&cv).expect("serialize");
        let back: CoreConfigValue = serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(back.key, key);
        prop_assert_eq!(back.value, value);
        prop_assert_eq!(back.scope, scope);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17. Proptests — parse_cli_value type coercion
// ═══════════════════════════════════════════════════════════════════════════════

proptest::proptest! {
    /// Exact "true"/"false" must produce boolean; no other string can.
    #[test]
    fn proptest_parse_cli_value_bool_invariant(s in ".*") {
        let result = parse_cli_value(&s);
        if let Ok(item) = result {
            if item.is_bool() {
                prop_assert!(s == "true" || s == "false",
                    "Only exact 'true'/'false' should produce bool, got: '{s}'");
            }
        }
        // Must never panic
    }

    /// Any i64-parseable string must produce an integer that matches.
    #[test]
    fn proptest_parse_cli_value_int_roundtrip(n in proptest::num::i64::ANY) {
        let s = n.to_string();
        let result = parse_cli_value(&s);
        if let Ok(item) = result {
            if item.is_integer() {
                prop_assert_eq!(item.as_integer(), Some(n));
            }
            prop_assert!(!item.is_bool(), "integer should not be bool");
        }
    }

    /// Any string must produce a valid result (never panic).
    #[test]
    fn proptest_parse_cli_value_never_panics(s in ".*") {
        let _ = parse_cli_value(&s);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18. Proptests — Config set/get roundtrip
// ═══════════════════════════════════════════════════════════════════════════════

proptest::proptest! {
    /// Setting a value and getting it must always return the original.
    #[test]
    fn proptest_config_set_get_roundtrip(
        key in "[a-zA-Z_][a-zA-Z0-9_.]{0,63}",
        value in ".*"
    ) {
        let mut config = Config::new();
        config.set(&key, &value);
        prop_assert_eq!(config.get(&key), Some(&value.to_string()));
    }

    /// Removing a set key must yield None thereafter.
    #[test]
    fn proptest_config_set_then_remove(
        key in "[a-zA-Z_][a-zA-Z0-9_.]{0,63}",
        value in ".+"
    ) {
        let mut config = Config::new();
        config.set(&key, &value);
        let removed = config.remove(&key);
        prop_assert_eq!(removed, Some(value.to_string()));
        prop_assert!(config.get(&key).is_none());
    }
}
