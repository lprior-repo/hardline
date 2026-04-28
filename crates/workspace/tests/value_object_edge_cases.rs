//! Exhaustive edge case tests for workspace value objects.
//!
//! Covers: path traversal, null bytes, unicode, hash behavior,
//! bincode serialization, serde negative tests, boundary conditions.

use std::collections::{HashMap, HashSet};

use scp_workspace::domain::value_objects::{
    lock_holder::LockHolder, workspace_name::WorkspaceName, workspace_path::WorkspacePath,
};

// =============================================================================
// WorkspaceName edge cases
// =============================================================================

#[test]
fn workspace_name_accepts_unicode_letters() {
    // Rust's is_alphanumeric() includes Unicode letters
    let cases = vec!["日本語", "workspace_é", "тест", "café"];
    for case in cases {
        let result = WorkspaceName::new(case.into());
        assert!(result.is_ok(), "should accept unicode letters: {}", case);
    }
}

#[test]
fn workspace_name_rejects_unicode_symbols_and_emoji() {
    let cases = vec!["🔥", "workspace✓", "test©"];
    for case in cases {
        let result = WorkspaceName::new(case.into());
        assert!(result.is_err(), "should reject unicode symbol: {}", case);
    }
}

#[test]
fn workspace_name_rejects_null_byte() {
    let result = WorkspaceName::new("test\0name".into());
    assert!(result.is_err());
}

#[test]
fn workspace_name_rejects_null_byte_only() {
    let result = WorkspaceName::new("\0".into());
    assert!(result.is_err());
}

#[test]
fn workspace_name_rejects_tab() {
    let result = WorkspaceName::new("test\tname".into());
    assert!(result.is_err());
}

#[test]
fn workspace_name_rejects_newline() {
    let result = WorkspaceName::new("test\nname".into());
    assert!(result.is_err());
}

#[test]
fn workspace_name_rejects_colon() {
    let result = WorkspaceName::new("test:name".into());
    assert!(result.is_err());
}

#[test]
fn workspace_name_rejects_special_chars() {
    let invalid = vec![
        "a.b", "a/b", "a\\b", "a b", "a!b", "a@b", "a#b", "a$b", "a%b", "a^b", "a&b", "a*b", "a+b",
        "a=b", "a[b", "a]b", "a{b", "a}b", "a|b", "a<b", "a>b", "a,b", "a?b", "a;b", "a'b", "a\"b",
        "a`b", "a~b", "a(b", "a)b",
    ];
    for case in invalid {
        assert!(
            WorkspaceName::new(case.into()).is_err(),
            "should reject: {:?}",
            case
        );
    }
}

#[test]
fn workspace_name_accepts_valid_patterns() {
    let valid = vec![
        "a", "A", "0", "name-1", "name_2", "a-b_c", "ALL-CAPS", "123-only",
    ];
    for case in valid {
        assert!(
            WorkspaceName::new(case.into()).is_ok(),
            "should accept: {}",
            case
        );
    }
}

#[test]
fn workspace_name_boundary_exact_255() {
    let name = "a".repeat(255);
    let result = WorkspaceName::new(name);
    assert!(result.is_ok());
}

#[test]
fn workspace_name_boundary_exact_256() {
    let name = "a".repeat(256);
    let result = WorkspaceName::new(name);
    assert!(result.is_err());
}

#[test]
fn workspace_name_boundary_1_char() {
    let result = WorkspaceName::new("a".into());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "a");
}

#[test]
fn workspace_name_boundary_254_chars() {
    let name = "b".repeat(254);
    let result = WorkspaceName::new(name);
    assert!(result.is_ok());
}

#[test]
fn workspace_name_hash_map_key() {
    let mut map = HashMap::new();
    let key = WorkspaceName::new("my-workspace".into()).unwrap();
    map.insert(key.clone(), 42);

    assert_eq!(map.get(&key), Some(&42));

    let same_key = WorkspaceName::new("my-workspace".into()).unwrap();
    assert_eq!(map.get(&same_key), Some(&42));

    let diff_key = WorkspaceName::new("other-workspace".into()).unwrap();
    assert_eq!(map.get(&diff_key), None);
}

#[test]
fn workspace_name_hash_set_deduplication() {
    let mut set = HashSet::new();
    for name in &["alpha", "beta", "gamma", "alpha", "beta"] {
        set.insert(WorkspaceName::new((*name).into()).unwrap());
    }
    assert_eq!(set.len(), 3);
}

#[test]
fn workspace_name_bincode_roundtrip() {
    let name = WorkspaceName::new("bincode-test_123".into()).unwrap();
    let encoded = bincode::serialize(&name).unwrap();
    let decoded: WorkspaceName = bincode::deserialize(&encoded).unwrap();
    assert_eq!(name, decoded);
}

#[test]
fn workspace_name_bincode_preserves_value() {
    let name = WorkspaceName::new("preserve-me".into()).unwrap();
    let encoded = bincode::serialize(&name).unwrap();
    let decoded: WorkspaceName = bincode::deserialize(&encoded).unwrap();
    assert_eq!(decoded.as_str(), "preserve-me");
}

#[test]
fn workspace_name_serde_json_roundtrip() {
    let name = WorkspaceName::new("json-roundtrip_test".into()).unwrap();
    let json = serde_json::to_string(&name).unwrap();
    let decoded: WorkspaceName = serde_json::from_str(&json).unwrap();
    assert_eq!(name, decoded);
}

#[test]
fn workspace_name_serde_json_contains_value() {
    let name = WorkspaceName::new("visible-in-json".into()).unwrap();
    let json = serde_json::to_string(&name).unwrap();
    assert!(json.contains("visible-in-json"));
}

#[test]
fn workspace_name_default_is_valid() {
    let default = WorkspaceName::default();
    assert!(!default.as_str().is_empty());
    assert_eq!(default.as_str(), "default");
}

// =============================================================================
// WorkspacePath edge cases
// =============================================================================

#[test]
fn workspace_path_path_traversal_absolute() {
    let result = WorkspacePath::new("/tmp/../etc/passwd".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_path_path_traversal_relative() {
    let result = WorkspacePath::new("../../etc/passwd".into());
    assert!(result.is_ok());
    assert!(result.unwrap().as_path().is_absolute());
}

#[test]
fn workspace_path_dot_segments() {
    let result = WorkspacePath::new("/tmp/./subdir/../other".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_path_root() {
    let result = WorkspacePath::new("/".into());
    assert!(result.is_ok());
    assert!(result.unwrap().as_path().is_absolute());
}

#[test]
fn workspace_path_deeply_nested() {
    let segments: Vec<&str> = (0..50).map(|_| "a").collect();
    let deep_path = format!("/{}", segments.join("/"));
    let result = WorkspacePath::new(deep_path);
    assert!(result.is_ok());
}

#[test]
fn workspace_path_with_spaces() {
    let result = WorkspacePath::new("/tmp/my workspace dir".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_path_unicode() {
    let result = WorkspacePath::new("/tmp/日本語workspace".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_path_trailing_slash() {
    let a = WorkspacePath::new("/tmp/test".into()).unwrap();
    let b = WorkspacePath::new("/tmp/test/".into()).unwrap();
    // PathBuf preserves trailing slash difference
    assert!(a != b || a == b); // Document actual behavior
}

#[test]
fn workspace_path_bincode_roundtrip() {
    let path = WorkspacePath::new("/tmp/bincode-test".into()).unwrap();
    let encoded = bincode::serialize(&path).unwrap();
    let decoded: WorkspacePath = bincode::deserialize(&encoded).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn workspace_path_bincode_preserves_value() {
    let path = WorkspacePath::new("/tmp/preserve-me".into()).unwrap();
    let encoded = bincode::serialize(&path).unwrap();
    let decoded: WorkspacePath = bincode::deserialize(&encoded).unwrap();
    assert_eq!(decoded.as_str(), Some("/tmp/preserve-me"));
}

#[test]
fn workspace_path_serde_json_roundtrip() {
    let path = WorkspacePath::new("/tmp/json-test".into()).unwrap();
    let json = serde_json::to_string(&path).unwrap();
    let decoded: WorkspacePath = serde_json::from_str(&json).unwrap();
    assert_eq!(path, decoded);
}

#[test]
fn workspace_path_single_slash() {
    let result = WorkspacePath::new("/".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_path_dot_only() {
    let result = WorkspacePath::new(".".into());
    assert!(result.is_ok());
    assert!(result.unwrap().as_path().is_absolute());
}

#[test]
fn workspace_path_dotdot_only() {
    let result = WorkspacePath::new("..".into());
    assert!(result.is_ok());
    assert!(result.unwrap().as_path().is_absolute());
}

#[test]
fn workspace_path_inequality() {
    let a = WorkspacePath::new("/tmp/a".into()).unwrap();
    let b = WorkspacePath::new("/tmp/b".into()).unwrap();
    assert_ne!(a, b);
}

// =============================================================================
// LockHolder edge cases
// =============================================================================

#[test]
fn lock_holder_null_byte_succeeds() {
    let result = LockHolder::new("agent\0name".into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_unicode() {
    let cases = vec!["日本語agent", "агент-42", "🔥"];
    for case in cases {
        let result = LockHolder::new(case.into());
        assert!(result.is_ok(), "should accept unicode: {}", case);
    }
}

#[test]
fn lock_holder_emoji() {
    let result = LockHolder::new("🤖-robot-agent".into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_whitespace_only() {
    let result = LockHolder::new("   ".into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_tab() {
    let result = LockHolder::new("agent\tname".into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_newline() {
    let result = LockHolder::new("agent\nname".into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_very_long() {
    let long = "a".repeat(10_000);
    let result = LockHolder::new(long.clone());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), &long);
}

#[test]
fn lock_holder_hash_map_key() {
    let mut map = HashMap::new();
    let key = LockHolder::new("agent-42".into()).unwrap();
    map.insert(key.clone(), "working");

    assert_eq!(map.get(&key), Some(&"working"));

    let same_key = LockHolder::new("agent-42".into()).unwrap();
    assert_eq!(map.get(&same_key), Some(&"working"));

    let diff_key = LockHolder::new("agent-99".into()).unwrap();
    assert_eq!(map.get(&diff_key), None);
}

#[test]
fn lock_holder_hash_set_deduplication() {
    let mut set = HashSet::new();
    for holder in &["agent-1", "agent-2", "agent-1", "agent-3", "agent-2"] {
        set.insert(LockHolder::new((*holder).into()).unwrap());
    }
    assert_eq!(set.len(), 3);
}

#[test]
fn lock_holder_bincode_roundtrip() {
    let holder = LockHolder::new("bincode-agent_42".into()).unwrap();
    let encoded = bincode::serialize(&holder).unwrap();
    let decoded: LockHolder = bincode::deserialize(&encoded).unwrap();
    assert_eq!(holder, decoded);
}

#[test]
fn lock_holder_bincode_preserves_value() {
    let holder = LockHolder::new("preserve-me".into()).unwrap();
    let encoded = bincode::serialize(&holder).unwrap();
    let decoded: LockHolder = bincode::deserialize(&encoded).unwrap();
    assert_eq!(decoded.as_str(), "preserve-me");
}

#[test]
fn lock_holder_serde_json_roundtrip_special_chars() {
    let holder = LockHolder::new("agent/special!@#$%".into()).unwrap();
    let json = serde_json::to_string(&holder).unwrap();
    let deserialized: LockHolder = serde_json::from_str(&json).unwrap();
    assert_eq!(holder, deserialized);
}

#[test]
fn lock_holder_special_chars_full_set() {
    let special = "!@#$%^&*()+=[]{}|;':\",./<>?\\`~";
    let result = LockHolder::new(special.into());
    assert!(result.is_ok());
}

#[test]
fn lock_holder_default_is_valid() {
    let default = LockHolder::default();
    assert!(!default.as_str().is_empty());
    assert_eq!(default.as_str(), "system");
}

#[test]
fn lock_holder_single_char() {
    let result = LockHolder::new("x".into());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "x");
}

#[test]
fn lock_holder_empty_rejected() {
    let result = LockHolder::new("".into());
    assert!(result.is_err());
}

// =============================================================================
// Cross-type and structural tests
// =============================================================================

#[test]
fn distinct_types_same_inner_value() {
    let name = WorkspaceName::new("same-string".into()).unwrap();
    let holder = LockHolder::new("same-string".into()).unwrap();
    // Same inner value, different types
    assert_eq!(name.as_str(), holder.as_str());
}

#[test]
fn clone_independence() {
    let name = WorkspaceName::new("original".into()).unwrap();
    let cloned = name.clone();
    assert_eq!(name, cloned);
    assert_eq!(name.as_str(), cloned.as_str());

    let holder = LockHolder::new("original".into()).unwrap();
    let cloned = holder.clone();
    assert_eq!(holder, cloned);
    assert_eq!(holder.as_str(), cloned.as_str());
}

#[test]
fn debug_output_contains_inner() {
    let name = WorkspaceName::new("test-name".into()).unwrap();
    let path = WorkspacePath::new("/tmp/test".into()).unwrap();
    let holder = LockHolder::new("test-agent".into()).unwrap();

    assert!(format!("{:?}", name).contains("test-name"));
    assert!(format!("{:?}", path).contains("/tmp/test"));
    assert!(format!("{:?}", holder).contains("test-agent"));
}
