#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(
    clippy::match_wild_err_arm,
    clippy::redundant_clone,
    clippy::uninlined_format_args,
    clippy::expect_used
)]

//! - Analyze conflicts and generate structured output

use std::io::Cursor;

use isolate_core::output::{
    ConflictAnalysis, ConflictDetail, ConflictType, JsonlWriter, OutputLine, ResolutionOption,
    ResolutionRisk, ResolutionStrategy,
};

// ============================================================================
// ConflictDetail Factory Tests
// ============================================================================

#[test]
fn test_conflict_detail_overlapping_has_correct_resolutions() {
    let detail = ConflictDetail::overlapping("src/lib.rs");

    assert_eq!(detail.file, "src/lib.rs");
    assert_eq!(detail.conflict_type, ConflictType::Overlapping);
    assert_eq!(detail.resolutions.len(), 4);
    assert_eq!(detail.recommended, ResolutionStrategy::JjResolve);

    // Verify resolution strategies are present
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::JjResolve));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::ManualMerge));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::AcceptOurs));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::AcceptTheirs));
}

#[test]
fn test_conflict_detail_existing_has_correct_resolutions() {
    let detail = ConflictDetail::existing("src/conflicted.rs");

    assert_eq!(detail.file, "src/conflicted.rs");
    assert_eq!(detail.conflict_type, ConflictType::Existing);
    assert_eq!(detail.resolutions.len(), 4);
    assert_eq!(detail.recommended, ResolutionStrategy::JjResolve);

    // Verify resolution strategies are present
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::JjResolve));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::ManualMerge));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::Rebase));
    assert!(detail
        .resolutions
        .iter()
        .any(|r| r.strategy == ResolutionStrategy::Abort));
}

#[test]
fn test_conflict_detail_optional_fields_are_none() {
    let detail = ConflictDetail::overlapping("test.rs");

    assert!(detail.workspace_additions.is_none());
    assert!(detail.workspace_deletions.is_none());
    assert!(detail.main_additions.is_none());
    assert!(detail.main_deletions.is_none());
}

// ============================================================================
// ResolutionOption Factory Tests
// ============================================================================

#[test]
fn test_resolution_option_accept_ours() {
    let option = ResolutionOption::accept_ours();

    assert_eq!(option.strategy, ResolutionStrategy::AcceptOurs);
    assert_eq!(option.description, "Accept workspace version");
    assert_eq!(option.risk, ResolutionRisk::Moderate);
    assert!(option.automatic);
    assert!(option.command.is_some());
}

#[test]
fn test_resolution_option_accept_theirs() {
    let option = ResolutionOption::accept_theirs();

    assert_eq!(option.strategy, ResolutionStrategy::AcceptTheirs);
    assert_eq!(option.description, "Accept main version");
    assert_eq!(option.risk, ResolutionRisk::Destructive);
    assert!(option.automatic);
    assert!(option.command.is_some());
    assert!(option.notes.is_some());
}

#[test]
fn test_resolution_option_manual_merge() {
    let option = ResolutionOption::manual_merge();

    assert_eq!(option.strategy, ResolutionStrategy::ManualMerge);
    assert_eq!(option.description, "Manually resolve conflicts");
    assert_eq!(option.risk, ResolutionRisk::Safe);
    assert!(!option.automatic);
    assert!(option.command.is_none());
    assert!(option.notes.is_some());
}

#[test]
fn test_resolution_option_jj_resolve() {
    let option = ResolutionOption::jj_resolve("src/test.rs");

    assert_eq!(option.strategy, ResolutionStrategy::JjResolve);
    assert_eq!(option.description, "Use jj resolve tool");
    assert_eq!(option.risk, ResolutionRisk::Safe);
    assert!(option.automatic);
    assert!(option.command.is_some());
    match option.command {
        Some(ref cmd) => assert!(cmd.contains("src/test.rs")),
        None => panic!("command should be present"),
    }
}

#[test]
fn test_resolution_option_rebase() {
    let option = ResolutionOption::rebase();

    assert_eq!(option.strategy, ResolutionStrategy::Rebase);
    assert_eq!(option.description, "Rebase onto fresh main");
    assert_eq!(option.risk, ResolutionRisk::Moderate);
    assert!(option.automatic);
    assert!(option.command.is_some());
}

#[test]
fn test_resolution_option_abort() {
    let option = ResolutionOption::abort();

    assert_eq!(option.strategy, ResolutionStrategy::Abort);
    assert_eq!(option.description, "Abort the operation");
    assert_eq!(option.risk, ResolutionRisk::Safe);
    assert!(option.automatic);
    assert!(option.command.is_some());
}

#[test]
fn test_resolution_option_skip() {
    let option = ResolutionOption::skip();

    assert_eq!(option.strategy, ResolutionStrategy::Skip);
    assert_eq!(option.description, "Skip this file");
    assert_eq!(option.risk, ResolutionRisk::Safe);
    assert!(option.automatic);
    assert!(option.command.is_none());
    assert!(option.notes.is_some());
}

// ============================================================================
// ConflictAnalysis Tests
// ============================================================================

#[test]
fn test_conflict_analysis_serialization() {
    let analysis = ConflictAnalysis {
        type_field: "conflict_analysis".to_string(),
        session: "test-session".to_string(),
        merge_safe: false,
        total_conflicts: 2,
        conflicts: vec![
            ConflictDetail::existing("src/a.rs"),
            ConflictDetail::overlapping("src/b.rs"),
        ],
        existing_conflicts: 1,
        overlapping_files: 1,
        merge_base: Some("abc123".to_string()),
        analysis_time_ms: Some(42),
        timestamp: chrono::Utc::now(),
    };

    let json_result = serde_json::to_string(&analysis);
    assert!(json_result.is_ok());

    let json = json_result;
    match json {
        Ok(ref j) => {
            assert!(j.contains("conflict_analysis"));
            assert!(j.contains("test-session"));
            assert!(j.contains("total_conflicts"));
            assert!(j.contains("abc123"));
            assert!(j.contains("src/a.rs"));
            assert!(j.contains("src/b.rs"));
        }
        Err(_) => panic!("serialization should succeed"),
    }
}

#[test]
fn test_conflict_analysis_round_trip() {
    let original = ConflictAnalysis {
        type_field: "conflict_analysis".to_string(),
        session: "round-trip-test".to_string(),
        merge_safe: true,
        total_conflicts: 0,
        conflicts: vec![],
        existing_conflicts: 0,
        overlapping_files: 0,
        merge_base: None,
        analysis_time_ms: Some(10),
        timestamp: chrono::Utc::now(),
    };

    let json_result = serde_json::to_string(&original);
    assert!(json_result.is_ok());

    match json_result {
        Ok(json) => {
            let deserialized_result: Result<ConflictAnalysis, _> = serde_json::from_str(&json);
            assert!(deserialized_result.is_ok());

            match deserialized_result {
                Ok(deserialized) => {
                    assert_eq!(original.session, deserialized.session);
                    assert_eq!(original.merge_safe, deserialized.merge_safe);
                    assert_eq!(original.total_conflicts, deserialized.total_conflicts);
                    assert_eq!(original.existing_conflicts, deserialized.existing_conflicts);
                    assert_eq!(original.overlapping_files, deserialized.overlapping_files);
                }
                Err(_) => panic!("deserialization should succeed"),
            }
        }
        Err(_) => panic!("serialization should succeed"),
    }
}

// ============================================================================
// OutputLine Integration Tests
// ============================================================================

#[test]
fn test_output_line_conflict_analysis_factory() {
    let conflicts = vec![
        ConflictDetail::existing("src/existing.rs"),
        ConflictDetail::overlapping("src/overlap.rs"),
    ];

    let line = OutputLine::conflict_analysis("my-session", false, conflicts.clone());

    // Verify it's the correct variant
    match &line {
        OutputLine::ConflictAnalysis(analysis) => {
            assert_eq!(analysis.session, "my-session");
            assert!(!analysis.merge_safe);
            assert_eq!(analysis.total_conflicts, 2);
            assert_eq!(analysis.existing_conflicts, 1);
            assert_eq!(analysis.overlapping_files, 1);
        }
        _ => panic!("Expected ConflictAnalysis variant"),
    }
}

#[test]
fn test_output_line_conflict_analysis_safe_merge() {
    let line = OutputLine::conflict_analysis("safe-session", true, vec![]);

    match &line {
        OutputLine::ConflictAnalysis(analysis) => {
            assert!(analysis.merge_safe);
            assert_eq!(analysis.total_conflicts, 0);
        }
        _ => panic!("Expected ConflictAnalysis variant"),
    }
}

#[test]
fn test_output_line_kind_for_conflict_analysis() {
    let line = OutputLine::conflict_analysis("test", true, vec![]);
    assert_eq!(line.kind(), "conflict_analysis");
}

// ============================================================================
// JSONL Emission Tests
// ============================================================================

#[test]
fn test_conflict_analysis_emits_valid_jsonl() {
    let conflicts = vec![ConflictDetail::existing("src/lib.rs")];
    let line = OutputLine::conflict_analysis("emit-test", false, conflicts);

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let emit_result = writer.emit(&line);
    assert!(emit_result.is_ok());

    let output_result = String::from_utf8(cursor.into_inner());
    assert!(output_result.is_ok());

    match output_result {
        Ok(output) => {
            // Each line should be valid JSON
            let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&output);
            assert!(parse_result.is_ok());

            match parse_result {
                Ok(json) => {
                    assert!(json.is_object());
                    // OutputLine serializes with tag for enum variant
                    // Check for the conflict_analysis variant
                    assert!(json.get("conflict_analysis").is_some() || json.get("type").is_some());
                }
                Err(_) => panic!("JSON parsing should succeed"),
            }
        }
        Err(_) => panic!("output should be valid UTF-8"),
    }
}

#[test]
fn test_multiple_conflict_details_emit_separately() {
    let conflicts = vec![
        ConflictDetail::existing("src/a.rs"),
        ConflictDetail::overlapping("src/b.rs"),
        ConflictDetail::overlapping("src/c.rs"),
    ];

    let line = OutputLine::conflict_analysis("multi-test", false, conflicts);

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = JsonlWriter::new(&mut cursor);

    let emit_result = writer.emit(&line);
    assert!(emit_result.is_ok());

    let output_result = String::from_utf8(cursor.into_inner());
    assert!(output_result.is_ok());

    match output_result {
        Ok(output) => {
            // Verify all conflicts are in the output
            assert!(output.contains("src/a.rs"));
            assert!(output.contains("src/b.rs"));
            assert!(output.contains("src/c.rs"));
        }
        Err(_) => panic!("output should be valid UTF-8"),
    }
}

// ============================================================================
// ConflictType Serialization Tests
// ============================================================================

#[test]
fn test_conflict_type_serialization() {
    let serialize_result = serde_json::to_string(&ConflictType::Existing);
    assert!(serialize_result.is_ok());
    match serialize_result {
        Ok(json) => assert_eq!(json, "\"existing\""),
        Err(_) => panic!("serialization should succeed"),
    }

    let serialize_result = serde_json::to_string(&ConflictType::Overlapping);
    assert!(serialize_result.is_ok());
    match serialize_result {
        Ok(json) => assert_eq!(json, "\"overlapping\""),
        Err(_) => panic!("serialization should succeed"),
    }

    let serialize_result = serde_json::to_string(&ConflictType::DeleteModify);
    assert!(serialize_result.is_ok());
    match serialize_result {
        Ok(json) => assert_eq!(json, "\"delete_modify\""),
        Err(_) => panic!("serialization should succeed"),
    }

    let serialize_result = serde_json::to_string(&ConflictType::RenameModify);
    assert!(serialize_result.is_ok());
    match serialize_result {
        Ok(json) => assert_eq!(json, "\"rename_modify\""),
        Err(_) => panic!("serialization should succeed"),
    }

    let serialize_result = serde_json::to_string(&ConflictType::Binary);
    assert!(serialize_result.is_ok());
    match serialize_result {
        Ok(json) => assert_eq!(json, "\"binary\""),
        Err(_) => panic!("serialization should succeed"),
    }
}

// ============================================================================
// ResolutionStrategy Serialization Tests
// ============================================================================

#[test]
fn test_resolution_strategy_serialization() {
    let strategies = [
        (ResolutionStrategy::AcceptOurs, "\"accept_ours\""),
        (ResolutionStrategy::AcceptTheirs, "\"accept_theirs\""),
        (ResolutionStrategy::JjResolve, "\"jj_resolve\""),
        (ResolutionStrategy::ManualMerge, "\"manual_merge\""),
        (ResolutionStrategy::Rebase, "\"rebase\""),
        (ResolutionStrategy::Abort, "\"abort\""),
        (ResolutionStrategy::Skip, "\"skip\""),
    ];

    for (strategy, expected) in strategies {
        let result = serde_json::to_string(&strategy);
        match result {
            Ok(json) => assert_eq!(json, expected),
            Err(_) => panic!("serialization of {:?} should succeed", strategy),
        }
    }
}

// ============================================================================
// ResolutionRisk Serialization Tests
// ============================================================================

#[test]
fn test_resolution_risk_serialization() {
    let risks = [
        (ResolutionRisk::Safe, "\"safe\""),
        (ResolutionRisk::Moderate, "\"moderate\""),
        (ResolutionRisk::Destructive, "\"destructive\""),
    ];

    for (risk, expected) in risks {
        let result = serde_json::to_string(&risk);
        match result {
            Ok(json) => assert_eq!(json, expected),
            Err(_) => panic!("serialization of {:?} should succeed", risk),
        }
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_conflict_analysis_with_empty_session() {
    let line = OutputLine::conflict_analysis("", true, vec![]);

    match &line {
        OutputLine::ConflictAnalysis(analysis) => {
            assert_eq!(analysis.session, "");
            assert!(analysis.merge_safe);
        }
        _ => panic!("Expected ConflictAnalysis variant"),
    }
}

#[test]
fn test_conflict_detail_with_special_characters_in_path() {
    let detail = ConflictDetail::overlapping("src/weird file name (1).rs");

    assert_eq!(detail.file, "src/weird file name (1).rs");

    // Verify serialization handles special characters
    let json_result = serde_json::to_string(&detail);
    assert!(json_result.is_ok());
}

#[test]
fn test_resolution_option_jj_resolve_with_special_path() {
    let option = ResolutionOption::jj_resolve("path/with spaces/file.rs");

    match option.command {
        Some(ref cmd) => assert!(cmd.contains("path/with spaces/file.rs")),
        None => panic!("command should be present"),
    }
}
