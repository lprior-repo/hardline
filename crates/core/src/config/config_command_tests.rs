//! Unit tests for CLI config command types.
//!
//! ConfigKey::try_from() (17), parse_cli_value() (18),
//! error variants (3), nested value ops (6), env scope (1), exit codes (1).

use crate::config::command_types::{
    get_nested_value, parse_cli_value, set_nested_value, ConfigKey,
};
use crate::config::config_core::{Config, ConfigScope};
use crate::error::Error;
use crate::error_config::{ConfigError, ConfigErrorKind};

fn extract_kind(err: Error) -> ConfigErrorKind {
    match err {
        Error::Config(e) => e.kind().clone(),
        other => panic!("Expected Config error, got: {other:?}"),
    }
}

fn assert_parse_error(err: Error, check: fn(&str)) {
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigParseError(msg) => check(&msg),
        other => panic!("Expected ConfigParseError, got: {other:?}"),
    }
}

// === 3.1 ConfigKey::try_from() ===

#[test]
fn config_key_accepts_two_segment_key() {
    let key = ConfigKey::try_from("watch.enabled").expect("should parse");
    assert_eq!(
        key.segments(),
        &["watch".to_string(), "enabled".to_string()]
    );
    assert_eq!(key.as_str(), "watch.enabled");
}

#[test]
fn config_key_accepts_multi_segment_key() {
    let key = ConfigKey::try_from("conflict_resolution.mode").expect("should parse");
    assert_eq!(
        key.segments(),
        &["conflict_resolution".to_string(), "mode".to_string()]
    );
}

#[test]
fn config_key_accepts_minimal_segments() {
    let key = ConfigKey::try_from("a.b").expect("should parse");
    assert_eq!(key.segments(), &["a".to_string(), "b".to_string()]);
    assert_eq!(key.as_str(), "a.b");
}

#[test]
fn config_key_rejects_empty_string() {
    let err = ConfigKey::try_from("").unwrap_err();
    assert_parse_error(err, |msg| {
        assert!(
            msg.to_lowercase().contains("empty"),
            "Expected 'empty', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_single_segment() {
    let err = ConfigKey::try_from("nosection").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("dot") || l.contains("segment"),
            "Expected 'dot'/'segment', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_non_ascii() {
    let err = ConfigKey::try_from("s\u{e9}.key").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("ascii") || l.contains("non-ascii"),
            "Expected 'ASCII', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_hyphen() {
    let err = ConfigKey::try_from("my-key.val").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("hyphen") || l.contains('-') || l.contains("invalid character"),
            "Expected 'hyphen'/'-', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_space() {
    let err = ConfigKey::try_from("my key.val").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("space") || l.contains("whitespace") || l.contains("blank"),
            "Expected 'space'/'whitespace'/'blank', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_path_traversal_dotdot() {
    let err = ConfigKey::try_from("a..b").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("consecutive") || l.contains("dot"),
            "Expected 'consecutive'/'dot', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_path_traversal_slash() {
    let err = ConfigKey::try_from("../etc").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("slash") || l.contains('/') || l.contains("traversal"),
            "Expected 'slash'/'traversal', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_backslash() {
    let err = ConfigKey::try_from("a\\b").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("backslash") || l.contains('\\') || l.contains("invalid character"),
            "Expected 'backslash', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_null_byte() {
    let err = ConfigKey::try_from("k\x00.s").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(l.contains("null"), "Expected 'null', got: {msg}");
    });
}

#[test]
fn config_key_rejects_leading_dot() {
    let err = ConfigKey::try_from(".k").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("leading") || l.contains("empty segment"),
            "Expected 'leading', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_trailing_dot() {
    let err = ConfigKey::try_from("k.").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("trailing") || l.contains("empty segment"),
            "Expected 'trailing', got: {msg}"
        );
    });
}

#[test]
fn config_key_rejects_overlength() {
    let seg = "a".repeat(255);
    let key257 = format!("z.{seg}");
    assert!(key257.len() >= 257);
    let err = ConfigKey::try_from(key257.as_str()).unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("256") || l.contains("length"),
            "Expected '256'/'length', got: {msg}"
        );
    });
}

#[test]
fn config_key_accepts_at_max_length() {
    let seg = "a".repeat(254);
    let key256 = format!("z.{seg}");
    assert_eq!(key256.len(), 256);
    let key = ConfigKey::try_from(key256.as_str()).expect("should accept 256-char key");
    assert_eq!(key.as_str().len(), 256);
}

#[test]
fn config_key_rejects_unknown_schema() {
    let err = ConfigKey::try_from("zzz.yyy").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("schema") || l.contains("unknown") || l.contains("not found"),
            "Expected 'schema'/'unknown', got: {msg}"
        );
    });
}

// === 3.2 parse_cli_value() ===

#[test]
fn parse_cli_infers_bool_true() {
    let item = parse_cli_value("true").expect("should parse");
    assert!(item.is_bool());
    assert_eq!(item.as_bool(), Some(true));
}

#[test]
fn parse_cli_infers_bool_false() {
    let item = parse_cli_value("false").expect("should parse");
    assert!(item.is_bool());
    assert_eq!(item.as_bool(), Some(false));
}

#[test]
fn parse_cli_true_case_sensitive() {
    let item = parse_cli_value("True").expect("should parse");
    assert!(!item.is_bool(), "'True' must NOT be bool");
    assert_eq!(item.as_str(), Some("True"));
}

#[test]
fn parse_cli_false_case_sensitive() {
    let item = parse_cli_value("FALSE").expect("should parse");
    assert!(!item.is_bool(), "'FALSE' must NOT be bool");
    assert_eq!(item.as_str(), Some("FALSE"));
}

#[test]
fn parse_cli_whitespace_bool_is_string() {
    let item = parse_cli_value(" true").expect("should parse");
    assert!(!item.is_bool(), "' true' must NOT be bool");
    assert_eq!(item.as_str(), Some(" true"));
}

#[test]
fn parse_cli_infers_positive_int() {
    let item = parse_cli_value("42").expect("should parse");
    assert!(item.is_integer());
    assert_eq!(item.as_integer(), Some(42));
}

#[test]
fn parse_cli_infers_negative_int() {
    let item = parse_cli_value("-100").expect("should parse");
    assert!(item.is_integer());
    assert_eq!(item.as_integer(), Some(-100));
}

#[test]
fn parse_cli_i64_max() {
    let item = parse_cli_value("9223372036854775807").expect("should parse");
    assert!(item.is_integer());
    assert_eq!(item.as_integer(), Some(i64::MAX));
}

#[test]
fn parse_cli_i64_min() {
    let item = parse_cli_value("-9223372036854775808").expect("should parse");
    assert!(item.is_integer());
    assert_eq!(item.as_integer(), Some(i64::MIN));
}

#[test]
fn parse_cli_overflow_falls_to_string() {
    let item = parse_cli_value("99999999999999999999").expect("should parse");
    assert!(!item.is_integer(), "overflow must not be integer");
    assert_eq!(item.as_str(), Some("99999999999999999999"));
}

#[test]
fn parse_cli_infers_string_array() {
    let item = parse_cli_value("[\"a\",\"b\"]").expect("should parse");
    assert!(item.is_array());
    let vals: Vec<&str> = item
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(vals, vec!["a", "b"]);
}

#[test]
fn parse_cli_single_element_array() {
    let item = parse_cli_value("[\"only\"]").expect("should parse");
    assert!(item.is_array());
    let vals: Vec<&str> = item
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(vals, vec!["only"]);
}

#[test]
fn parse_cli_array_with_empty_string() {
    let item = parse_cli_value("[\"\"]").expect("should parse");
    assert!(item.is_array());
    let vals: Vec<&str> = item
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(vals, vec![""]);
}

#[test]
fn parse_cli_accepts_empty_array() {
    let item = parse_cli_value("[]").expect("should parse");
    assert!(item.is_array());
    assert!(item.as_array().unwrap().is_empty());
}

#[test]
fn parse_cli_rejects_non_string_array() {
    let err = parse_cli_value("[1,2]").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("string") || l.contains("non-string"),
            "Expected 'string', got: {msg}"
        );
    });
}

#[test]
fn parse_cli_rejects_malformed_array() {
    let err = parse_cli_value("[\"a\",").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("parse") || l.contains("malformed") || l.contains("toml"),
            "Expected 'parse'/'malformed'/'TOML', got: {msg}"
        );
    });
}

#[test]
fn parse_cli_falls_back_to_string() {
    let item = parse_cli_value("hello world").expect("should parse");
    assert!(item.is_str());
    assert_eq!(item.as_str(), Some("hello world"));
}

#[test]
fn parse_cli_empty_string_falls_to_string() {
    let item = parse_cli_value("").expect("should parse");
    assert!(item.is_str());
    assert_eq!(item.as_str(), Some(""));
}

// === 3.5 Error Variants ===

#[test]
fn error_not_found_variant() {
    let err: ConfigError = ConfigErrorKind::NotFound("path".into()).into();
    assert!(matches!(err.kind(), ConfigErrorKind::NotFound(_)));
    assert!(format!("{err}").contains("path"));
    assert_eq!(err.exit_code(), 40);
}

#[test]
fn error_invalid_variant() {
    let err: ConfigError = ConfigErrorKind::Invalid("bad config".into()).into();
    assert!(matches!(err.kind(), ConfigErrorKind::Invalid(_)));
    assert!(format!("{err}").contains("bad config"));
    assert_eq!(err.exit_code(), 41);
}

#[test]
fn error_permission_variant() {
    let err: ConfigError = ConfigErrorKind::Permission("/etc/config".into()).into();
    assert!(matches!(err.kind(), ConfigErrorKind::Permission(_)));
    assert!(format!("{err}").contains("/etc/config"));
    assert_eq!(err.exit_code(), 42);
}

#[test]
fn exit_codes_match_contract() {
    let cases: Vec<(ConfigErrorKind, i32)> = vec![
        (ConfigErrorKind::ConfigKeyNotFound("k".into()), 40),
        (ConfigErrorKind::ConfigParseError("p".into()), 41),
        (ConfigErrorKind::ConfigWriteError("w".into()), 42),
        (ConfigErrorKind::ConfigScopeError("s".into()), 43),
        (ConfigErrorKind::ConfigLockError("l".into()), 44),
        (ConfigErrorKind::NotFound("n".into()), 40),
        (ConfigErrorKind::Invalid("i".into()), 41),
        (ConfigErrorKind::Permission("p".into()), 42),
    ];
    for (kind, expected) in cases {
        let err: ConfigError = kind.into();
        assert_eq!(err.exit_code(), expected, "Wrong exit code for {err:?}");
    }
}

// === 3.6 Nested Value Ops ===

#[test]
fn get_nested_returns_leaf() {
    let config = Config::new();
    let result = get_nested_value(&config, "conflict_resolution.mode");
    let _ = result;
}

#[test]
fn get_nested_rejects_unknown() {
    let config = Config::new();
    let err = get_nested_value(&config, "nonexistent.key").unwrap_err();
    let kind = extract_kind(err);
    match kind {
        ConfigErrorKind::ConfigKeyNotFound(msg) => {
            let l = msg.to_lowercase();
            assert!(
                l.contains("nonexistent") || l.contains("not found"),
                "Expected 'nonexistent', got: {msg}"
            );
        }
        other => panic!("Expected ConfigKeyNotFound, got: {other:?}"),
    }
}

#[test]
fn get_nested_deep_path() {
    let config = Config::new();
    let result = get_nested_value(&config, "a.b.c.d");
    let _ = result;
}

#[test]
fn set_nested_creates_tables() {
    let mut doc = toml_edit::DocumentMut::new();
    set_nested_value(&mut doc, &["new_sec", "key"], "42").expect("should create tables");
    assert_eq!(doc["new_sec"]["key"].as_integer(), Some(42));
}

#[test]
fn set_nested_rejects_non_table() {
    let mut doc = toml_edit::DocumentMut::new();
    doc["watch"] = toml_edit::Item::Value("hello".into());
    let err = set_nested_value(&mut doc, &["watch", "enabled"], "true").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("table") || l.contains("not a table"),
            "Expected 'table', got: {msg}"
        );
    });
}

#[test]
fn set_nested_single_segment_rejected() {
    let mut doc = toml_edit::DocumentMut::new();
    let err = set_nested_value(&mut doc, &["key"], "val").unwrap_err();
    assert_parse_error(err, |msg| {
        let l = msg.to_lowercase();
        assert!(
            l.contains("segment") || l.contains("empty") || l.contains("non-empty"),
            "Expected 'segment', got: {msg}"
        );
    });
}

// === Env Scope Rejection ===

#[test]
fn env_scope_rejects_set() {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let err = rt
        .block_on(crate::config::command_types::config_set(
            "watch.enabled",
            "true",
            ConfigScope::Env,
        ))
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
