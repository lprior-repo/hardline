//! Snapshot tests for contract type JSON serialization.
//!
//! These tests verify that type contract system types serialize correctly
//! to JSON for schema generation and AI-first design tools.

use im::HashMap;
use scp_core::contracts::types::{
    Constraint, ContextualHint, FieldContract, HintType, TypeContract,
};

#[test]
fn test_type_contract_session_json() {
    let contract = TypeContract {
        name: "SessionName".into(),
        description: "A valid session identifier".into(),
        constraints: vec![
            Constraint::Length {
                min: Some(1),
                max: Some(64),
            },
            Constraint::Regex {
                pattern: r"^[a-zA-Z][a-zA-Z0-9_-]*$".into(),
                description: "Must start with letter, contain only alphanumeric, dash, underscore"
                    .into(),
            },
        ],
        hints: vec![
            ContextualHint {
                hint_type: HintType::Example,
                message: "Valid names: 'feature-auth', 'bugfix_123', 'main'".into(),
                condition: None,
                related_to: None,
            },
            ContextualHint {
                hint_type: HintType::Warning,
                message: "Avoid special characters that may cause issues in file paths".into(),
                condition: Some("contains_special_chars".into()),
                related_to: None,
            },
        ],
        examples: vec!["main".into(), "feature-auth".into(), "dev-session".into()],
        fields: HashMap::new(),
    };
    let json = serde_json::to_string(&contract).unwrap();
    insta::assert_snapshot!("type_contract_session", json);
}

#[test]
fn test_type_contract_with_fields_json() {
    let mut fields = HashMap::new();
    fields.insert(
        "name".into(),
        FieldContract {
            name: "name".into(),
            field_type: "String".into(),
            required: true,
            description: "The session name".into(),
            constraints: vec![Constraint::Length {
                min: Some(1),
                max: Some(64),
            }],
            default: None,
            depends_on: vec![],
            examples: vec!["main".into(), "feature-x".into()],
        },
    );
    fields.insert(
        "parent".into(),
        FieldContract {
            name: "parent".into(),
            field_type: "Option<String>".into(),
            required: false,
            description: "Optional parent session for stacking".into(),
            constraints: vec![],
            default: Some("None".into()),
            depends_on: vec![],
            examples: vec!["main".into()],
        },
    );

    let contract = TypeContract {
        name: "CreateSessionInput".into(),
        description: "Input for creating a new session".into(),
        constraints: vec![],
        hints: vec![],
        examples: vec![],
        fields,
    };
    let json = serde_json::to_string(&contract).unwrap();
    insta::assert_snapshot!("type_contract_with_fields", json);
}

#[test]
fn test_constraint_regex_json() {
    let constraint = Constraint::Regex {
        pattern: r"^\d{4}-\d{2}-\d{2}$".into(),
        description: "ISO date format".into(),
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_regex", json);
}

#[test]
fn test_constraint_range_json() {
    let constraint = Constraint::Range {
        min: Some(1),
        max: Some(1000),
        inclusive: true,
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_range", json);
}

#[test]
fn test_constraint_length_json() {
    let constraint = Constraint::Length {
        min: Some(1),
        max: Some(255),
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_length", json);
}

#[test]
fn test_constraint_enum_json() {
    let constraint = Constraint::Enum {
        values: vec![
            "open".into(),
            "in_progress".into(),
            "blocked".into(),
            "closed".into(),
        ],
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_enum", json);
}

#[test]
fn test_constraint_path_exists_json() {
    let constraint = Constraint::PathExists {
        must_be_absolute: true,
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_path_exists", json);
}

#[test]
fn test_constraint_path_absolute_json() {
    let constraint = Constraint::PathAbsolute;
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_path_absolute", json);
}

#[test]
fn test_constraint_unique_json() {
    let constraint = Constraint::Unique;
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_unique", json);
}

#[test]
fn test_constraint_custom_json() {
    let constraint = Constraint::Custom {
        rule: "valid_session_name".into(),
        description: "Must pass session name validation".into(),
    };
    let json = serde_json::to_string(&constraint).unwrap();
    insta::assert_snapshot!("constraint_custom", json);
}

#[test]
fn test_contextual_hint_best_practice_json() {
    let hint = ContextualHint {
        hint_type: HintType::BestPractice,
        message: "Use descriptive names that indicate purpose".into(),
        condition: None,
        related_to: None,
    };
    let json = serde_json::to_string(&hint).unwrap();
    insta::assert_snapshot!("hint_best_practice", json);
}

#[test]
fn test_contextual_hint_warning_json() {
    let hint = ContextualHint {
        hint_type: HintType::Warning,
        message: "Avoid using 'temp' in permanent resource names".into(),
        condition: Some("name_contains_temp".into()),
        related_to: Some("name".into()),
    };
    let json = serde_json::to_string(&hint).unwrap();
    insta::assert_snapshot!("hint_warning", json);
}

#[test]
fn test_contextual_hint_performance_json() {
    let hint = ContextualHint {
        hint_type: HintType::Performance,
        message: "Consider pagination for large result sets".into(),
        condition: None,
        related_to: None,
    };
    let json = serde_json::to_string(&hint).unwrap();
    insta::assert_snapshot!("hint_performance", json);
}

#[test]
fn test_hint_type_serialization() {
    let hint_types = vec![
        (HintType::BestPractice, "best_practice"),
        (HintType::Warning, "warning"),
        (HintType::Example, "example"),
        (HintType::Performance, "performance"),
        (HintType::Security, "security"),
        (HintType::Compatibility, "compatibility"),
    ];

    for (hint_type, name) in hint_types {
        let json = serde_json::to_string(&hint_type).unwrap();
        insta::assert_snapshot!(format!("hint_type_{}", name), json);
    }
}
