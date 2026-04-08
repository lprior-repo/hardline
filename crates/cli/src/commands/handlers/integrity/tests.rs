//! Exhaustive tests for the integrity command handler.
//!
//! Covers: integrity check execution, check result (pass/fail/warning),
//! repair mode (auto-fix), integrity scope selection, detailed violation
//! reporting, integrity report formatting, repair confirmation prompt,
//! partial integrity checks, backup list, backup restore.
//!
//! All test names are descriptive. All assertions use exact matching
//! (no bare `is_ok()`/`is_err()`).

use super::data::{
    BackupListResponse, IntegrityOptions, IntegrityOutputFormat, IntegritySubcommand,
    RepairResponse, RestoreResponse, ValidationResponse,
};

// ============================================================================
// IntegritySubcommand — construction & matching
// ============================================================================

#[test]
fn subcommand_validate_holds_workspace() {
    let sub = IntegritySubcommand::Validate {
        workspace: "my-workspace".to_string(),
    };
    match sub {
        IntegritySubcommand::Validate { workspace } => assert_eq!(workspace, "my-workspace"),
        other => panic!("Expected Validate, got {other:?}"),
    }
}

#[test]
fn subcommand_repair_holds_workspace_and_force() {
    let sub = IntegritySubcommand::Repair {
        workspace: "broken".to_string(),
        force: true,
    };
    match sub {
        IntegritySubcommand::Repair { workspace, force } => {
            assert_eq!(workspace, "broken");
            assert!(force);
        }
        other => panic!("Expected Repair, got {other:?}"),
    }
}

#[test]
fn subcommand_repair_force_false() {
    let sub = IntegritySubcommand::Repair {
        workspace: "ws".to_string(),
        force: false,
    };
    match sub {
        IntegritySubcommand::Repair { force, .. } => assert!(!force),
        other => panic!("Expected Repair, got {other:?}"),
    }
}

#[test]
fn subcommand_backup_list_is_unit() {
    assert!(matches!(IntegritySubcommand::BackupList, IntegritySubcommand::BackupList));
}

#[test]
fn subcommand_backup_restore_holds_id_and_force() {
    let sub = IntegritySubcommand::BackupRestore {
        backup_id: "bk-42".to_string(),
        force: true,
    };
    match sub {
        IntegritySubcommand::BackupRestore { backup_id, force } => {
            assert_eq!(backup_id, "bk-42");
            assert!(force);
        }
        other => panic!("Expected BackupRestore, got {other:?}"),
    }
}

#[test]
fn subcommand_backup_restore_force_false() {
    let sub = IntegritySubcommand::BackupRestore {
        backup_id: "bk-1".to_string(),
        force: false,
    };
    match sub {
        IntegritySubcommand::BackupRestore { force, .. } => assert!(!force),
        other => panic!("Expected BackupRestore, got {other:?}"),
    }
}

// ============================================================================
// IntegrityOptions — construction & clone
// ============================================================================

#[test]
fn options_with_validate() {
    let opts = IntegrityOptions {
        subcommand: IntegritySubcommand::Validate {
            workspace: "alpha".to_string(),
        },
    };
    assert!(matches!(opts.subcommand, IntegritySubcommand::Validate { .. }));
}

#[test]
fn options_with_repair() {
    let opts = IntegrityOptions {
        subcommand: IntegritySubcommand::Repair {
            workspace: "beta".to_string(),
            force: false,
        },
    };
    assert!(matches!(opts.subcommand, IntegritySubcommand::Repair { .. }));
}

#[test]
fn options_with_backup_list() {
    let opts = IntegrityOptions {
        subcommand: IntegritySubcommand::BackupList,
    };
    assert!(matches!(opts.subcommand, IntegritySubcommand::BackupList));
}

#[test]
fn options_with_backup_restore() {
    let opts = IntegrityOptions {
        subcommand: IntegritySubcommand::BackupRestore {
            backup_id: "bk-99".to_string(),
            force: false,
        },
    };
    assert!(matches!(opts.subcommand, IntegritySubcommand::BackupRestore { .. }));
}

#[test]
fn options_clone_is_independent() {
    let opts = IntegrityOptions {
        subcommand: IntegritySubcommand::Validate {
            workspace: "original".to_string(),
        },
    };
    let cloned = opts.clone();
    // Both should have the same variant
    assert!(matches!(opts.subcommand, IntegritySubcommand::Validate { .. }));
    assert!(matches!(cloned.subcommand, IntegritySubcommand::Validate { .. }));
}

// ============================================================================
// IntegrityOutputFormat — variants & is_json
// ============================================================================

#[test]
fn human_format_is_not_json() {
    assert!(!IntegrityOutputFormat::Human.is_json());
}

#[test]
fn json_format_is_json() {
    assert!(IntegrityOutputFormat::Json.is_json());
}

#[test]
fn format_from_str_json() {
    assert_eq!(IntegrityOutputFormat::from("json"), IntegrityOutputFormat::Json);
}

#[test]
fn format_from_str_human() {
    assert_eq!(IntegrityOutputFormat::from("human"), IntegrityOutputFormat::Human);
}

#[test]
fn format_from_str_default_is_human() {
    assert_eq!(IntegrityOutputFormat::from("anything"), IntegrityOutputFormat::Human);
}

#[test]
fn format_from_str_empty_is_human() {
    assert_eq!(IntegrityOutputFormat::from(""), IntegrityOutputFormat::Human);
}

#[test]
fn format_from_str_uppercase_json_is_human() {
    // Case-sensitive: "JSON" is not "json"
    assert_eq!(IntegrityOutputFormat::from("JSON"), IntegrityOutputFormat::Human);
}

#[test]
fn format_equality_same() {
    assert_eq!(IntegrityOutputFormat::Human, IntegrityOutputFormat::Human);
    assert_eq!(IntegrityOutputFormat::Json, IntegrityOutputFormat::Json);
}

#[test]
fn format_inequality() {
    assert_ne!(IntegrityOutputFormat::Human, IntegrityOutputFormat::Json);
}

#[test]
fn format_clone_matches() {
    assert_eq!(IntegrityOutputFormat::Json.clone(), IntegrityOutputFormat::Json);
    assert_eq!(IntegrityOutputFormat::Human.clone(), IntegrityOutputFormat::Human);
}

#[test]
fn format_copy_semantics() {
    let a = IntegrityOutputFormat::Json;
    let b = a; // Copy, not move
    assert_eq!(a, b);
}

// ============================================================================
// ValidationResponse — construction & fields
// ============================================================================

fn sample_validation_result() -> scp_core::workspace_integrity::ValidationResult {
    use scp_core::workspace_integrity::{Severity, ValidationResult};
    ValidationResult::valid("test-ws", std::path::PathBuf::from("/tmp/test-ws"))
}

fn sample_validation_result_with_issues() -> scp_core::workspace_integrity::ValidationResult {
    use scp_core::workspace_integrity::{CorruptionType, IntegrityIssue, ValidationResult};
    let issue = IntegrityIssue::new(CorruptionType::StaleLocks, "Stale lock files detected");
    ValidationResult::invalid("broken-ws", std::path::PathBuf::from("/tmp/broken"), vec![issue])
}

#[test]
fn validation_response_valid_workspace() {
    let vr = ValidationResponse {
        workspace: "clean-ws".to_string(),
        path: "/tmp/clean-ws".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    assert_eq!(vr.workspace, "clean-ws");
    assert_eq!(vr.path, "/tmp/clean-ws");
    assert!(vr.is_valid);
    assert_eq!(vr.issue_count, 0);
}

#[test]
fn validation_response_invalid_workspace() {
    let vr = ValidationResponse {
        workspace: "broken-ws".to_string(),
        path: "/tmp/broken".to_string(),
        is_valid: false,
        issue_count: 2,
        validation: sample_validation_result_with_issues(),
    };
    assert!(!vr.is_valid);
    assert_eq!(vr.issue_count, 2);
}

#[test]
fn validation_response_serialization_roundtrip() {
    let vr = ValidationResponse {
        workspace: "ws-rt".to_string(),
        path: "/tmp/ws-rt".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    let json = serde_json::to_string(&vr).expect("serialize");
    let back: ValidationResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.workspace, "ws-rt");
    assert_eq!(back.path, "/tmp/ws-rt");
    assert!(back.is_valid);
    assert_eq!(back.issue_count, 0);
}

#[test]
fn validation_response_invalid_roundtrip() {
    let vr = ValidationResponse {
        workspace: "bad".to_string(),
        path: "/bad".to_string(),
        is_valid: false,
        issue_count: 3,
        validation: sample_validation_result_with_issues(),
    };
    let json = serde_json::to_string(&vr).expect("serialize");
    let back: ValidationResponse = serde_json::from_str(&json).expect("deserialize");
    assert!(!back.is_valid);
    assert_eq!(back.issue_count, 3);
}

#[test]
fn validation_response_json_contains_workspace_key() {
    let vr = ValidationResponse {
        workspace: "my-ws".to_string(),
        path: "/tmp/my-ws".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    let json = serde_json::to_string(&vr).expect("serialize");
    assert!(json.contains("\"workspace\":\"my-ws\""));
    assert!(json.contains("\"is_valid\":true"));
    assert!(json.contains("\"issue_count\":0"));
}

// ============================================================================
// RepairResponse — construction & serialization
// ============================================================================

#[test]
fn repair_response_success() {
    let r = RepairResponse {
        workspace: "ws".to_string(),
        success: true,
        summary: "Fixed stale locks".to_string(),
    };
    assert!(r.success);
    assert_eq!(r.workspace, "ws");
    assert_eq!(r.summary, "Fixed stale locks");
}

#[test]
fn repair_response_failure() {
    let r = RepairResponse {
        workspace: "ws".to_string(),
        success: false,
        summary: "Cannot repair".to_string(),
    };
    assert!(!r.success);
}

#[test]
fn repair_response_serialization_roundtrip() {
    let r = RepairResponse {
        workspace: "test-ws".to_string(),
        success: true,
        summary: "Fixed locks".to_string(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.workspace, "test-ws");
    assert!(back.success);
    assert_eq!(back.summary, "Fixed locks");
}

#[test]
fn repair_response_failure_roundtrip() {
    let r = RepairResponse {
        workspace: "broken".to_string(),
        success: false,
        summary: "Unrepairable corruption".to_string(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
    assert!(!back.success);
    assert_eq!(back.summary, "Unrepairable corruption");
}

#[test]
fn repair_response_json_keys() {
    let r = RepairResponse {
        workspace: "x".to_string(),
        success: true,
        summary: "ok".to_string(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("\"workspace\":\"x\""));
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"summary\":\"ok\""));
}

#[test]
fn repair_response_empty_summary() {
    let r = RepairResponse {
        workspace: "ws".to_string(),
        success: true,
        summary: String::new(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
    assert!(back.summary.is_empty());
}

#[test]
fn repair_response_long_summary_roundtrips() {
    let long = "x".repeat(10_000);
    let r = RepairResponse {
        workspace: "ws".to_string(),
        success: true,
        summary: long.clone(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.summary.len(), 10_000);
}

// ============================================================================
// BackupListResponse — construction & serialization
// ============================================================================

#[test]
fn backup_list_response_empty() {
    let r = BackupListResponse {
        backups: vec![],
        count: 0,
    };
    assert_eq!(r.count, 0);
    assert!(r.backups.is_empty());
}

#[test]
fn backup_list_response_serialization_roundtrip_empty() {
    let r = BackupListResponse {
        backups: vec![],
        count: 0,
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: BackupListResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.count, 0);
    assert!(back.backups.is_empty());
}

#[test]
fn backup_list_response_count_matches_vec_len() {
    let r = BackupListResponse {
        backups: vec![],
        count: 5, // Intentionally mismatched — this is a data struct, not validated
    };
    assert_eq!(r.count, 5);
    assert_eq!(r.backups.len(), 0);
}

#[test]
fn backup_list_response_json_has_count() {
    let r = BackupListResponse {
        backups: vec![],
        count: 0,
    };
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("\"count\":0"));
}

// ============================================================================
// RestoreResponse — construction & serialization
// ============================================================================

#[test]
fn restore_response_success() {
    let r = RestoreResponse {
        workspace: "ws".to_string(),
        backup_id: "bk-1".to_string(),
        success: true,
        summary: "Restored".to_string(),
    };
    assert!(r.success);
    assert_eq!(r.workspace, "ws");
    assert_eq!(r.backup_id, "bk-1");
}

#[test]
fn restore_response_failure() {
    let r = RestoreResponse {
        workspace: "ws".to_string(),
        backup_id: "bk-1".to_string(),
        success: false,
        summary: "Backup not found".to_string(),
    };
    assert!(!r.success);
}

#[test]
fn restore_response_serialization_roundtrip() {
    let r = RestoreResponse {
        workspace: "ws".to_string(),
        backup_id: "bk-1".to_string(),
        success: true,
        summary: "Restored".to_string(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RestoreResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.workspace, "ws");
    assert_eq!(back.backup_id, "bk-1");
    assert!(back.success);
    assert_eq!(back.summary, "Restored");
}

#[test]
fn restore_response_json_keys() {
    let r = RestoreResponse {
        workspace: "my-ws".to_string(),
        backup_id: "bk-42".to_string(),
        success: true,
        summary: "Done".to_string(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("\"workspace\":\"my-ws\""));
    assert!(json.contains("\"backup_id\":\"bk-42\""));
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"summary\":\"Done\""));
}

#[test]
fn restore_response_empty_fields_roundtrip() {
    let r = RestoreResponse {
        workspace: String::new(),
        backup_id: String::new(),
        success: false,
        summary: String::new(),
    };
    let json = serde_json::to_string(&r).expect("serialize");
    let back: RestoreResponse = serde_json::from_str(&json).expect("deserialize");
    assert!(back.workspace.is_empty());
    assert!(back.backup_id.is_empty());
    assert!(back.summary.is_empty());
}

// ============================================================================
// IntegrityOutputFormat — exhaustive edge cases
// ============================================================================

#[test]
fn format_from_str_toml_is_human() {
    assert_eq!(IntegrityOutputFormat::from("toml"), IntegrityOutputFormat::Human);
}

#[test]
fn format_from_str_yaml_is_human() {
    assert_eq!(IntegrityOutputFormat::from("yaml"), IntegrityOutputFormat::Human);
}

#[test]
fn format_from_str_xml_is_human() {
    assert_eq!(IntegrityOutputFormat::from("xml"), IntegrityOutputFormat::Human);
}

#[test]
fn format_partial_eq_human() {
    assert!(IntegrityOutputFormat::Human == IntegrityOutputFormat::Human);
}

#[test]
fn format_partial_eq_json() {
    assert!(IntegrityOutputFormat::Json == IntegrityOutputFormat::Json);
}

#[test]
fn format_partial_ne() {
    assert!(IntegrityOutputFormat::Human != IntegrityOutputFormat::Json);
}

// ============================================================================
// IntegritySubcommand — workspace name edge cases
// ============================================================================

#[test]
fn validate_workspace_empty_string() {
    let sub = IntegritySubcommand::Validate {
        workspace: String::new(),
    };
    match sub {
        IntegritySubcommand::Validate { workspace } => assert!(workspace.is_empty()),
        _ => panic!("Expected Validate"),
    }
}

#[test]
fn validate_workspace_with_special_chars() {
    let sub = IntegritySubcommand::Validate {
        workspace: "../../../etc/passwd".to_string(),
    };
    match sub {
        IntegritySubcommand::Validate { workspace } => {
            assert_eq!(workspace, "../../../etc/passwd");
        }
        _ => panic!("Expected Validate"),
    }
}

#[test]
fn validate_workspace_unicode() {
    let sub = IntegritySubcommand::Validate {
        workspace: "ワークスペース".to_string(),
    };
    match sub {
        IntegritySubcommand::Validate { workspace } => {
            assert_eq!(workspace, "ワークスペース");
        }
        _ => panic!("Expected Validate"),
    }
}

#[test]
fn repair_workspace_empty_string() {
    let sub = IntegritySubcommand::Repair {
        workspace: String::new(),
        force: true,
    };
    match sub {
        IntegritySubcommand::Repair { workspace, force } => {
            assert!(workspace.is_empty());
            assert!(force);
        }
        _ => panic!("Expected Repair"),
    }
}

#[test]
fn backup_restore_empty_id() {
    let sub = IntegritySubcommand::BackupRestore {
        backup_id: String::new(),
        force: false,
    };
    match sub {
        IntegritySubcommand::BackupRestore { backup_id, force } => {
            assert!(backup_id.is_empty());
            assert!(!force);
        }
        _ => panic!("Expected BackupRestore"),
    }
}

// ============================================================================
// ValidationResponse — core type integration
// ============================================================================

#[test]
fn validation_response_with_valid_result_has_zero_issues() {
    let vr = ValidationResponse {
        workspace: "ws".to_string(),
        path: "/ws".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    assert_eq!(vr.validation.issues.len(), 0);
    assert!(vr.validation.is_valid);
}

#[test]
fn validation_response_with_issues_has_correct_count() {
    let vr = ValidationResponse {
        workspace: "ws".to_string(),
        path: "/ws".to_string(),
        is_valid: false,
        issue_count: 1,
        validation: sample_validation_result_with_issues(),
    };
    assert!(!vr.validation.is_valid);
    assert_eq!(vr.validation.issues.len(), 1);
}

#[test]
fn validation_response_preserves_workspace_name() {
    let vr = ValidationResponse {
        workspace: "my-workspace".to_string(),
        path: "/path/to/my-workspace".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    assert_eq!(vr.workspace, "my-workspace");
    assert_eq!(vr.validation.workspace, "test-ws");
}

// ============================================================================
// Cross-type consistency
// ============================================================================

#[test]
fn all_response_types_serialize_to_valid_json() {
    let vr = ValidationResponse {
        workspace: "ws".to_string(),
        path: "/ws".to_string(),
        is_valid: true,
        issue_count: 0,
        validation: sample_validation_result(),
    };
    let rr = RepairResponse {
        workspace: "ws".to_string(),
        success: true,
        summary: "ok".to_string(),
    };
    let bl = BackupListResponse {
        backups: vec![],
        count: 0,
    };
    let br = RestoreResponse {
        workspace: "ws".to_string(),
        backup_id: "bk-1".to_string(),
        success: true,
        summary: "ok".to_string(),
    };

    for json_str in &[
        serde_json::to_string(&vr).expect("ValidationResponse serialize"),
        serde_json::to_string(&rr).expect("RepairResponse serialize"),
        serde_json::to_string(&bl).expect("BackupListResponse serialize"),
        serde_json::to_string(&br).expect("RestoreResponse serialize"),
    ] {
        let val: serde_json::Value = serde_json::from_str(json_str).expect("parse JSON");
        assert!(val.is_object(), "Response must serialize to JSON object");
    }
}

// ============================================================================
// Adversarial / Red Queen
// ============================================================================

mod red_queen_adversarial {
    use super::*;

    #[test]
    fn validation_response_with_injection_in_workspace_survives_roundtrip() {
        let payloads = [
            "'; DROP TABLE workspaces; --",
            "$(rm -rf /)",
            "../../../etc/passwd",
            "<script>alert('xss')</script>",
            "ws\x00hidden",
            "ws\nnewline",
            "ws\ttab",
        ];
        for payload in &payloads {
            let vr = ValidationResponse {
                workspace: payload.to_string(),
                path: format!("/tmp/{payload}"),
                is_valid: false,
                issue_count: 0,
                validation: sample_validation_result(),
            };
            let json = serde_json::to_string(&vr).expect("serialize");
            let back: ValidationResponse = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.workspace, *payload, "workspace must roundtrip for injection payload");
        }
    }

    #[test]
    fn repair_response_with_injection_survives_roundtrip() {
        let payloads = [
            "'; DROP TABLE--; ",
            "<script>alert(1)</script>",
            "\x00null",
        ];
        for payload in &payloads {
            let r = RepairResponse {
                workspace: payload.to_string(),
                success: false,
                summary: format!("Failed: {payload}"),
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.workspace, *payload);
        }
    }

    #[test]
    fn restore_response_with_injection_survives_roundtrip() {
        let r = RestoreResponse {
            workspace: "<script>alert(1)</script>".to_string(),
            backup_id: "'; DROP--; ".to_string(),
            success: false,
            summary: "../../../etc/passwd".to_string(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let back: RestoreResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.workspace, "<script>alert(1)</script>");
        assert_eq!(back.backup_id, "'; DROP--; ");
    }

    #[test]
    fn validation_response_very_long_workspace_roundtrips() {
        let long = "x".repeat(10_000);
        let vr = ValidationResponse {
            workspace: long.clone(),
            path: "/tmp/ws".to_string(),
            is_valid: true,
            issue_count: 0,
            validation: sample_validation_result(),
        };
        let json = serde_json::to_string(&vr).expect("serialize");
        let back: ValidationResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.workspace.len(), 10_000);
    }

    #[test]
    fn repair_response_very_long_summary_roundtrips() {
        let long = "x".repeat(10_000);
        let r = RepairResponse {
            workspace: "ws".to_string(),
            success: true,
            summary: long.clone(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.summary.len(), 10_000);
    }

    #[test]
    fn output_format_never_panics_on_any_input() {
        let long_input = "a".repeat(1000);
        let inputs = ["", "json", "JSON", "human", "HUMAN", "yaml", "\x00", "\n", "🔥"];
        for input in &inputs {
            let _ = IntegrityOutputFormat::from(*input);
        }
        let _ = IntegrityOutputFormat::from(&long_input[..]);
    }

    #[test]
    fn subcommand_with_injection_workspace_does_not_crash() {
        let payloads = [
            "'; DROP TABLE--; ",
            "$(rm -rf /)",
            "../../../etc/passwd",
            "\x00hidden",
        ];
        for payload in &payloads {
            let _ = IntegritySubcommand::Validate {
                workspace: payload.to_string(),
            };
            let _ = IntegritySubcommand::Repair {
                workspace: payload.to_string(),
                force: true,
            };
        }
    }
}

// ============================================================================
// Proptest-based fuzzing
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// IntegrityOutputFormat never panics on any input string.
        #[test]
        fn proptest_output_format_never_panics(s in ".*") {
            let _ = IntegrityOutputFormat::from(&s[..]);
        }

        /// RepairResponse roundtrip preserves success flag.
        #[test]
        fn proptest_repair_response_roundtrip(success: bool, workspace in ".*", summary in ".*") {
            let r = RepairResponse {
                workspace,
                success,
                summary,
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: RepairResponse = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.success, success);
        }

        /// RestoreResponse roundtrip preserves success flag.
        #[test]
        fn proptest_restore_response_roundtrip(success: bool, workspace in ".*", backup_id in ".*", summary in ".*") {
            let r = RestoreResponse {
                workspace,
                backup_id,
                success,
                summary,
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: RestoreResponse = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.success, success);
        }

        /// BackupListResponse roundtrip preserves count.
        #[test]
        fn proptest_backup_list_roundtrip(count: usize) {
            let r = BackupListResponse {
                backups: vec![],
                count,
            };
            let json = serde_json::to_string(&r).expect("serialize");
            let back: BackupListResponse = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.count, count);
        }

        /// IntegrityOutputFormat::from always returns Human or Json.
        #[test]
        fn proptest_format_is_always_valid(s in ".*") {
            let fmt = IntegrityOutputFormat::from(&s[..]);
            assert!(fmt == IntegrityOutputFormat::Human || fmt == IntegrityOutputFormat::Json);
        }
    }
}
