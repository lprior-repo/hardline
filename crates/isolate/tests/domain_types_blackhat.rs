//! Black-hat tests for isolate domain types: WorkspaceId, BeadId, BeadWorkspaceMapping.
//!
//! Covers:
//! - Parse validation (empty, whitespace, special chars, unicode)
//! - Generation uniqueness
//! - Display, Debug, Serde roundtrips
//! - BeadWorkspaceMapping field preservation
//! - Hash/Eq consistency
//! - Proptests for arbitrary inputs

use std::collections::{HashMap, HashSet};

use scp_isolate::{BeadId, BeadWorkspaceMapping, WorkspaceId, WorkspaceState};

// =============================================================================
// WorkspaceId
// =============================================================================

#[test]
fn workspace_id_generate_starts_with_iso() {
    let id = WorkspaceId::generate();
    assert!(id.as_str().starts_with("iso-"), "generated ID should start with 'iso-'");
}

#[test]
fn workspace_id_generate_produces_unique_ids() {
    let ids: HashSet<String> = (0..100)
        .map(|_| WorkspaceId::generate().as_str().to_string())
        .collect();
    assert_eq!(ids.len(), 100, "all generated IDs should be unique");
}

#[test]
fn workspace_id_parse_rejects_empty() {
    let result = WorkspaceId::parse(String::new());
    assert!(result.is_err());
}

#[test]
fn workspace_id_parse_accepts_non_empty() {
    let id = WorkspaceId::parse("custom-id-123".into()).unwrap();
    assert_eq!(id.as_str(), "custom-id-123");
}

#[test]
fn workspace_id_parse_preserves_value() {
    let id = WorkspaceId::parse("ws-abc/DEF_!@#".into()).unwrap();
    assert_eq!(id.as_str(), "ws-abc/DEF_!@#");
}

#[test]
fn workspace_id_display_matches_as_str() {
    let id = WorkspaceId::parse("display-test".into()).unwrap();
    assert_eq!(format!("{id}"), id.as_str());
}

#[test]
fn workspace_id_equality() {
    let a = WorkspaceId::parse("same".into()).unwrap();
    let b = WorkspaceId::parse("same".into()).unwrap();
    let c = WorkspaceId::parse("different".into()).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn workspace_id_hash_consistency() {
    let a = WorkspaceId::parse("hash-me".into()).unwrap();
    let b = WorkspaceId::parse("hash-me".into()).unwrap();
    let mut set = HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
}

#[test]
fn workspace_id_hash_map_key() {
    let mut map = HashMap::new();
    let key = WorkspaceId::parse("map-key".into()).unwrap();
    map.insert(key.clone(), "value");
    assert_eq!(map.get(&key), Some(&"value"));
    let same = WorkspaceId::parse("map-key".into()).unwrap();
    assert_eq!(map.get(&same), Some(&"value"));
}

#[test]
fn workspace_id_serde_roundtrip() {
    let id = WorkspaceId::parse("serde-test".into()).unwrap();
    let json = serde_json::to_string(&id).unwrap();
    let parsed: WorkspaceId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn workspace_id_debug_contains_value() {
    let id = WorkspaceId::parse("debug-visible".into()).unwrap();
    let debug = format!("{id:?}");
    assert!(debug.contains("debug-visible"));
}

#[test]
fn workspace_id_clone_preserves_value() {
    let id = WorkspaceId::parse("clone-me".into()).unwrap();
    let cloned = id.clone();
    assert_eq!(id, cloned);
    assert_eq!(id.as_str(), cloned.as_str());
}

#[test]
fn workspace_id_parse_whitespace_only_succeeds() {
    // Only empty is rejected; whitespace is non-empty
    let result = WorkspaceId::parse("   ".into());
    assert!(result.is_ok());
}

#[test]
fn workspace_id_parse_unicode_succeeds() {
    let result = WorkspaceId::parse("日本語-id".into());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str(), "日本語-id");
}

#[test]
fn workspace_id_parse_very_long_succeeds() {
    let long = "a".repeat(10_000);
    let result = WorkspaceId::parse(long.clone());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_str().len(), 10_000);
}

#[test]
fn workspace_id_parse_special_chars_succeeds() {
    let special = "!@#$%^&*()+=[]{}|;':\",./<>?\\`~";
    let result = WorkspaceId::parse(special.into());
    assert!(result.is_ok());
}

#[test]
fn workspace_id_parse_null_byte_succeeds() {
    // Only empty is rejected
    let result = WorkspaceId::parse("test\0id".into());
    assert!(result.is_ok());
}

// =============================================================================
// BeadId
// =============================================================================

#[test]
fn bead_id_parse_rejects_empty() {
    let result = BeadId::parse(String::new());
    assert!(result.is_err());
}

#[test]
fn bead_id_parse_accepts_non_empty() {
    let id = BeadId::parse("bead-123".into()).unwrap();
    assert_eq!(id.as_str(), "bead-123");
}

#[test]
fn bead_id_display_matches_as_str() {
    let id = BeadId::parse("display-bead".into()).unwrap();
    assert_eq!(format!("{id}"), id.as_str());
}

#[test]
fn bead_id_equality() {
    let a = BeadId::parse("same".into()).unwrap();
    let b = BeadId::parse("same".into()).unwrap();
    let c = BeadId::parse("other".into()).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn bead_id_hash_consistency() {
    let a = BeadId::parse("hash".into()).unwrap();
    let b = BeadId::parse("hash".into()).unwrap();
    let mut set = HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
}

#[test]
fn bead_id_serde_roundtrip() {
    let id = BeadId::parse("serde-bead".into()).unwrap();
    let json = serde_json::to_string(&id).unwrap();
    let parsed: BeadId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn bead_id_debug_contains_value() {
    let id = BeadId::parse("debug-bead".into()).unwrap();
    let debug = format!("{id:?}");
    assert!(debug.contains("debug-bead"));
}

#[test]
fn bead_id_clone_preserves_value() {
    let id = BeadId::parse("clone-bead".into()).unwrap();
    let cloned = id.clone();
    assert_eq!(id, cloned);
    assert_eq!(id.as_str(), cloned.as_str());
}

#[test]
fn bead_id_parse_unicode_succeeds() {
    let result = BeadId::parse("ビーズ-123".into());
    assert!(result.is_ok());
}

#[test]
fn bead_id_parse_very_long_succeeds() {
    let long = "b".repeat(10_000);
    let result = BeadId::parse(long.clone());
    assert!(result.is_ok());
}

#[test]
fn bead_id_hash_map_key() {
    let mut map = HashMap::new();
    let key = BeadId::parse("map-bead".into()).unwrap();
    map.insert(key.clone(), 42);
    assert_eq!(map.get(&key), Some(&42));
    let same = BeadId::parse("map-bead".into()).unwrap();
    assert_eq!(map.get(&same), Some(&42));
}

// =============================================================================
// BeadWorkspaceMapping
// =============================================================================

#[test]
fn mapping_new_creates_correct_association() {
    let ws_id = WorkspaceId::generate();
    let bead_id = BeadId::parse("bead-new".into()).unwrap();
    let mapping = BeadWorkspaceMapping::new(bead_id.clone(), ws_id.clone());

    assert_eq!(mapping.bead_id(), &bead_id);
    assert_eq!(mapping.workspace_id(), &ws_id);
}

#[test]
fn mapping_assigned_at_is_recent() {
    let before = chrono::Utc::now();
    let mapping = BeadWorkspaceMapping::new(
        BeadId::parse("bead-ts".into()).unwrap(),
        WorkspaceId::generate(),
    );
    let after = chrono::Utc::now();
    let assigned = mapping.assigned_at();
    assert!(assigned >= before);
    assert!(assigned <= after);
}

#[test]
fn mapping_different_beads_have_different_ids() {
    let ws_id = WorkspaceId::generate();
    let bead1 = BeadId::parse("bead-1".into()).unwrap();
    let bead2 = BeadId::parse("bead-2".into()).unwrap();
    let m1 = BeadWorkspaceMapping::new(bead1, ws_id.clone());
    let m2 = BeadWorkspaceMapping::new(bead2, ws_id);
    assert_ne!(m1.bead_id(), m2.bead_id());
    assert_eq!(m1.workspace_id(), m2.workspace_id());
}

#[test]
fn mapping_same_bead_different_workspaces() {
    let bead = BeadId::parse("same-bead".into()).unwrap();
    let ws1 = WorkspaceId::generate();
    let ws2 = WorkspaceId::generate();
    let m1 = BeadWorkspaceMapping::new(bead.clone(), ws1);
    let m2 = BeadWorkspaceMapping::new(bead, ws2);
    assert_eq!(m1.bead_id(), m2.bead_id());
    assert_ne!(m1.workspace_id(), m2.workspace_id());
}

#[test]
fn mapping_serde_roundtrip() {
    let mapping = BeadWorkspaceMapping::new(
        BeadId::parse("serde-bead".into()).unwrap(),
        WorkspaceId::generate(),
    );
    let json = serde_json::to_string(&mapping).unwrap();
    let parsed: BeadWorkspaceMapping = serde_json::from_str(&json).unwrap();
    assert_eq!(mapping.bead_id(), parsed.bead_id());
    assert_eq!(mapping.workspace_id(), parsed.workspace_id());
    assert_eq!(mapping.assigned_at(), parsed.assigned_at());
}

#[test]
fn mapping_debug_contains_fields() {
    let mapping = BeadWorkspaceMapping::new(
        BeadId::parse("debug-bead".into()).unwrap(),
        WorkspaceId::parse("debug-ws".into()).unwrap(),
    );
    let debug = format!("{mapping:?}");
    assert!(debug.contains("debug-bead"));
    assert!(debug.contains("debug-ws"));
}

#[test]
fn mapping_clone_preserves_fields() {
    let mapping = BeadWorkspaceMapping::new(
        BeadId::parse("clone-bead".into()).unwrap(),
        WorkspaceId::generate(),
    );
    let cloned = mapping.clone();
    assert_eq!(mapping.bead_id(), cloned.bead_id());
    assert_eq!(mapping.workspace_id(), cloned.workspace_id());
    assert_eq!(mapping.assigned_at(), cloned.assigned_at());
}

// =============================================================================
// WorkspaceState FromStr edge cases
// =============================================================================

#[test]
fn workspace_state_from_str_whitespace_fails() {
    let result: Result<WorkspaceState, _> = "  created  ".parse();
    assert!(result.is_err());
}

#[test]
fn workspace_state_from_str_with_trailing_newline_fails() {
    let result: Result<WorkspaceState, _> = "created\n".parse();
    assert!(result.is_err());
}

#[test]
fn workspace_state_from_str_mixed_case() {
    // Case-insensitive: "Created", "CREATED", "cReAtEd" all work
    assert!("Created".parse::<WorkspaceState>().is_ok());
    assert!("CREATED".parse::<WorkspaceState>().is_ok());
    assert!("cReAtEd".parse::<WorkspaceState>().is_ok());
}

#[test]
fn workspace_state_from_str_all_variants() {
    assert!("created".parse::<WorkspaceState>().is_ok());
    assert!("working".parse::<WorkspaceState>().is_ok());
    assert!("ready".parse::<WorkspaceState>().is_ok());
    assert!("merged".parse::<WorkspaceState>().is_ok());
    assert!("abandoned".parse::<WorkspaceState>().is_ok());
    assert!("conflict".parse::<WorkspaceState>().is_ok());
}

// =============================================================================
// Proptests
// =============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::{prop_assert, prop_assert_eq};

    proptest! {
        #[test]
        fn workspace_id_parse_non_empty(s in ".+") {
            let result = WorkspaceId::parse(s);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn workspace_id_parse_empty_fails(s in "") {
            let result = WorkspaceId::parse(s);
            prop_assert!(result.is_err());
        }

        #[test]
        fn workspace_id_generate_unique_batch(count in 2usize..50) {
            let ids: HashSet<String> = (0..count)
                .map(|_| WorkspaceId::generate().as_str().to_string())
                .collect();
            prop_assert_eq!(ids.len(), count);
        }

        #[test]
        fn workspace_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,50}") {
            let id = WorkspaceId::parse(s.clone()).unwrap();
            let json = serde_json::to_string(&id).unwrap();
            let parsed: WorkspaceId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(id.as_str(), parsed.as_str());
            prop_assert_eq!(parsed.as_str(), &s);
        }

        #[test]
        fn bead_id_parse_non_empty(s in ".+") {
            let result = BeadId::parse(s);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn bead_id_parse_empty_fails(s in "") {
            let result = BeadId::parse(s);
            prop_assert!(result.is_err());
        }

        #[test]
        fn bead_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,50}") {
            let id = BeadId::parse(s.clone()).unwrap();
            let json = serde_json::to_string(&id).unwrap();
            let parsed: BeadId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(id, parsed);
        }

        #[test]
        fn mapping_preserves_ids(bead_suffix in "[a-z0-9]{1,10}", ws_suffix in "[a-z0-9]{1,10}") {
            let bead = BeadId::parse(format!("bead-{}", bead_suffix)).unwrap();
            let ws = WorkspaceId::parse(format!("ws-{}", ws_suffix)).unwrap();
            let mapping = BeadWorkspaceMapping::new(bead.clone(), ws.clone());
            prop_assert_eq!(mapping.bead_id(), &bead);
            prop_assert_eq!(mapping.workspace_id(), &ws);
        }

        #[test]
        fn mapping_serde_roundtrip(bead in "[a-zA-Z0-9_-]{1,30}", ws in "[a-zA-Z0-9_-]{1,30}") {
            let mapping = BeadWorkspaceMapping::new(
                BeadId::parse(bead).unwrap(),
                WorkspaceId::parse(ws).unwrap(),
            );
            let json = serde_json::to_string(&mapping).unwrap();
            let parsed: BeadWorkspaceMapping = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(mapping.bead_id(), parsed.bead_id());
            prop_assert_eq!(mapping.workspace_id(), parsed.workspace_id());
        }

        #[test]
        fn workspace_state_roundtrip(idx in 0usize..6) {
            let states = [
                WorkspaceState::Created,
                WorkspaceState::Working,
                WorkspaceState::Ready,
                WorkspaceState::Merged,
                WorkspaceState::Abandoned,
                WorkspaceState::Conflict,
            ];
            let state = states[idx];
            let display = state.to_string();
            let parsed: WorkspaceState = display.parse().unwrap();
            prop_assert_eq!(state, parsed);
        }
    }
}
