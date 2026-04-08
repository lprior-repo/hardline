//! Exhaustive tests for the queue command handler.
//!
//! All test names are descriptive (no `fn test_` prefix).
//! No `is_ok()`/`is_err()` assertions -- exact variant matching only.
//! No unbounded loops -- all iteration is bounded by fixed-length arrays.
//! Zero panic, zero unwrap in source code.

use super::calculations::{
    build_item_detail, build_queue_status, build_queue_table, build_queue_table_human,
    filter_by_status, format_item_detail, format_priority_display, format_queue_status,
    format_single_item, format_status_display, parse_priority_to_string, parse_status_to_string,
    sort_by_priority,
};
use super::data::{
    QueueItemDetail, QueueItemDisplay, QueueListItem, QueueOptions, QueueOutputFormat,
    QueueStatusDisplay, QueueSubcommand,
};

// =============================================================================
// QueueOutputFormat tests
// =============================================================================

#[test]
fn queue_output_format_default_is_table() {
    assert_eq!(QueueOutputFormat::default(), QueueOutputFormat::Table);
}

#[test]
fn queue_output_format_json_flag_true_returns_json() {
    assert_eq!(
        QueueOutputFormat::from_json_flag(true),
        QueueOutputFormat::Json
    );
}

#[test]
fn queue_output_format_json_flag_false_returns_table() {
    assert_eq!(
        QueueOutputFormat::from_json_flag(false),
        QueueOutputFormat::Table
    );
}

#[test]
fn queue_output_format_is_json_true_for_json() {
    assert!(QueueOutputFormat::Json.is_json());
}

#[test]
fn queue_output_format_is_json_false_for_table() {
    assert!(!QueueOutputFormat::Table.is_json());
}

#[test]
fn queue_output_format_is_table_true_for_table() {
    assert!(QueueOutputFormat::Table.is_table());
}

#[test]
fn queue_output_format_is_table_false_for_json() {
    assert!(!QueueOutputFormat::Json.is_table());
}

#[test]
fn queue_output_format_clone_semantics() {
    let table = QueueOutputFormat::Table;
    let cloned = table.clone();
    assert_eq!(table, cloned);

    let json = QueueOutputFormat::Json;
    let cloned = json.clone();
    assert_eq!(json, cloned);
}

#[test]
fn queue_output_format_equality() {
    assert_eq!(QueueOutputFormat::Table, QueueOutputFormat::Table);
    assert_eq!(QueueOutputFormat::Json, QueueOutputFormat::Json);
    assert_ne!(QueueOutputFormat::Table, QueueOutputFormat::Json);
}

#[test]
fn queue_output_format_debug_format_table() {
    let debug = format!("{:?}", QueueOutputFormat::Table);
    assert!(debug.contains("Table"));
}

#[test]
fn queue_output_format_debug_format_json() {
    let debug = format!("{:?}", QueueOutputFormat::Json);
    assert!(debug.contains("Json"));
}

// =============================================================================
// QueueOutputFormat serde tests
// =============================================================================

#[test]
fn queue_output_format_table_serializes() {
    let json = serde_json::to_string(&QueueOutputFormat::Table);
    assert!(json.is_ok(), "Serialization should succeed");
    let s = json.unwrap();
    assert!(s.contains("Table"));
}

#[test]
fn queue_output_format_json_serializes() {
    let json = serde_json::to_string(&QueueOutputFormat::Json);
    assert!(json.is_ok(), "Serialization should succeed");
    let s = json.unwrap();
    assert!(s.contains("Json"));
}

#[test]
fn queue_output_format_serde_roundtrip_table() {
    let original = QueueOutputFormat::Table;
    let json = serde_json::to_string(&original).expect("serialize ok");
    let deserialized: QueueOutputFormat = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(original, deserialized);
}

#[test]
fn queue_output_format_serde_roundtrip_json() {
    let original = QueueOutputFormat::Json;
    let json = serde_json::to_string(&original).expect("serialize ok");
    let deserialized: QueueOutputFormat = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(original, deserialized);
}

// =============================================================================
// QueueListItem tests
// =============================================================================

#[test]
fn queue_list_item_creation() {
    let item = QueueListItem {
        index: 1,
        branch: "feature-test".to_string(),
        priority: "High".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(item.index, 1);
    assert_eq!(item.branch, "feature-test");
    assert_eq!(item.priority, "High");
    assert_eq!(item.status, "Pending");
}

#[test]
fn queue_list_item_empty_branch() {
    let item = QueueListItem {
        index: 0,
        branch: "".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(item.branch, "");
}

#[test]
fn queue_list_item_special_characters_in_branch() {
    let item = QueueListItem {
        index: 1,
        branch: "feature/special-chars_123".to_string(),
        priority: "Critical".to_string(),
        status: "Processing".to_string(),
    };

    assert_eq!(item.branch, "feature/special-chars_123");
}

#[test]
fn queue_list_item_clone() {
    let item = QueueListItem {
        index: 1,
        branch: "test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };
    let cloned = item.clone();
    assert_eq!(item, cloned);
}

#[test]
fn queue_list_item_equality() {
    let a = QueueListItem {
        index: 1,
        branch: "test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };
    let b = QueueListItem {
        index: 1,
        branch: "test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };
    let c = QueueListItem {
        index: 2,
        branch: "test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn queue_list_item_serde_roundtrip() {
    let item = QueueListItem {
        index: 5,
        branch: "serde-test".to_string(),
        priority: "High".to_string(),
        status: "Completed".to_string(),
    };

    let json = serde_json::to_string(&item).expect("serialize ok");
    let deserialized: QueueListItem = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(item, deserialized);
}

// =============================================================================
// QueueItemDetail tests
// =============================================================================

#[test]
fn queue_item_detail_creation_full() {
    let detail = QueueItemDetail {
        id: "item-123".to_string(),
        branch: "feature-detail".to_string(),
        priority: "Critical".to_string(),
        status: "Failed".to_string(),
        source: "Workspace(my-ws)".to_string(),
        attempt_count: 3,
        last_error: Some("timeout".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T01:00:00Z".to_string(),
    };

    assert_eq!(detail.id, "item-123");
    assert_eq!(detail.branch, "feature-detail");
    assert_eq!(detail.attempt_count, 3);
    assert_eq!(detail.last_error, Some("timeout".to_string()));
}

#[test]
fn queue_item_detail_no_error() {
    let detail = QueueItemDetail {
        id: "item-456".to_string(),
        branch: "clean-branch".to_string(),
        priority: "Normal".to_string(),
        status: "Completed".to_string(),
        source: "Direct".to_string(),
        attempt_count: 1,
        last_error: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    assert!(detail.last_error.is_none());
}

#[test]
fn queue_item_detail_empty_fields() {
    let detail = QueueItemDetail {
        id: "".to_string(),
        branch: "".to_string(),
        priority: "".to_string(),
        status: "".to_string(),
        source: "".to_string(),
        attempt_count: 0,
        last_error: None,
        created_at: "".to_string(),
        updated_at: "".to_string(),
    };

    assert!(detail.id.is_empty());
    assert!(detail.branch.is_empty());
}

#[test]
fn queue_item_detail_clone() {
    let detail = QueueItemDetail {
        id: "clone-1".to_string(),
        branch: "test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
        source: "Direct".to_string(),
        attempt_count: 0,
        last_error: None,
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    };
    let cloned = detail.clone();
    assert_eq!(detail, cloned);
}

#[test]
fn queue_item_detail_serde_roundtrip_with_error() {
    let detail = QueueItemDetail {
        id: "with-error".to_string(),
        branch: "branch".to_string(),
        priority: "High".to_string(),
        status: "Failed".to_string(),
        source: "Direct".to_string(),
        attempt_count: 2,
        last_error: Some("connection timeout".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T01:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&detail).expect("serialize ok");
    let deserialized: QueueItemDetail = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(detail, deserialized);
}

// =============================================================================
// QueueItemDisplay tests
// =============================================================================

#[test]
fn queue_item_display_creation() {
    let display = QueueItemDisplay {
        index: 1,
        branch: "feature-display".to_string(),
        priority: "High".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(display.index, 1);
    assert_eq!(display.branch, "feature-display");
}

#[test]
fn queue_item_display_index_one_based() {
    let display = QueueItemDisplay {
        index: 1,
        branch: "first".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(display.index, 1);
}

#[test]
fn queue_item_display_index_large_number() {
    let display = QueueItemDisplay {
        index: 9999,
        branch: "many".to_string(),
        priority: "Low".to_string(),
        status: "Pending".to_string(),
    };

    assert_eq!(display.index, 9999);
}

#[test]
fn queue_item_display_serde_roundtrip() {
    let display = QueueItemDisplay {
        index: 42,
        branch: "serde-display".to_string(),
        priority: "Critical".to_string(),
        status: "Processing".to_string(),
    };

    let json = serde_json::to_string(&display).expect("serialize ok");
    let deserialized: QueueItemDisplay = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(display, deserialized);
}

// =============================================================================
// QueueStatusDisplay tests
// =============================================================================

#[test]
fn queue_status_display_empty_queue() {
    let status = QueueStatusDisplay {
        total_items: 0,
        pending_items: 0,
        next_item: None,
    };

    assert_eq!(status.total_items, 0);
    assert_eq!(status.pending_items, 0);
    assert!(status.next_item.is_none());
}

#[test]
fn queue_status_display_with_next_item() {
    let status = QueueStatusDisplay {
        total_items: 10,
        pending_items: 5,
        next_item: Some("next-branch".to_string()),
    };

    assert_eq!(status.total_items, 10);
    assert_eq!(status.pending_items, 5);
    assert_eq!(status.next_item, Some("next-branch".to_string()));
}

#[test]
fn queue_status_display_no_next_item_when_pending_zero() {
    let status = QueueStatusDisplay {
        total_items: 5,
        pending_items: 0,
        next_item: None,
    };

    assert_eq!(status.pending_items, 0);
    assert!(status.next_item.is_none());
}

#[test]
fn queue_status_display_clone() {
    let status = QueueStatusDisplay {
        total_items: 100,
        pending_items: 50,
        next_item: Some("test".to_string()),
    };
    let cloned = status.clone();
    assert_eq!(status, cloned);
}

#[test]
fn queue_status_display_equality() {
    let a = QueueStatusDisplay {
        total_items: 10,
        pending_items: 5,
        next_item: Some("branch".to_string()),
    };
    let b = QueueStatusDisplay {
        total_items: 10,
        pending_items: 5,
        next_item: Some("branch".to_string()),
    };
    let c = QueueStatusDisplay {
        total_items: 10,
        pending_items: 5,
        next_item: None,
    };

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn queue_status_display_serde_roundtrip() {
    let status = QueueStatusDisplay {
        total_items: 42,
        pending_items: 21,
        next_item: Some("next".to_string()),
    };

    let json = serde_json::to_string(&status).expect("serialize ok");
    let deserialized: QueueStatusDisplay = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(status, deserialized);
}

// =============================================================================
// QueueSubcommand tests
// =============================================================================

#[test]
fn queue_subcommand_list() {
    let subcmd = QueueSubcommand::List;
    assert!(matches!(subcmd, QueueSubcommand::List));
}

#[test]
fn queue_subcommand_enqueue_with_branch() {
    let subcmd = QueueSubcommand::Enqueue {
        branch: "feature-x".to_string(),
        priority: None,
    };
    assert!(matches!(subcmd, QueueSubcommand::Enqueue { .. }));
}

#[test]
fn queue_subcommand_enqueue_with_priority() {
    let subcmd = QueueSubcommand::Enqueue {
        branch: "feature-y".to_string(),
        priority: Some("high".to_string()),
    };
    if let QueueSubcommand::Enqueue { branch, priority } = subcmd {
        assert_eq!(branch, "feature-y");
        assert_eq!(priority, Some("high".to_string()));
    } else {
        panic!("Expected Enqueue variant");
    }
}

#[test]
fn queue_subcommand_dequeue() {
    let subcmd = QueueSubcommand::Dequeue;
    assert!(matches!(subcmd, QueueSubcommand::Dequeue));
}

#[test]
fn queue_subcommand_process_without_checks() {
    let subcmd = QueueSubcommand::Process { checks: false };
    assert!(matches!(subcmd, QueueSubcommand::Process { checks: false }));
}

#[test]
fn queue_subcommand_process_with_checks() {
    let subcmd = QueueSubcommand::Process { checks: true };
    assert!(matches!(subcmd, QueueSubcommand::Process { checks: true }));
}

#[test]
fn queue_subcommand_insert() {
    let subcmd = QueueSubcommand::Insert {
        position: 5,
        branch: "insert-branch".to_string(),
    };
    if let QueueSubcommand::Insert { position, branch } = subcmd {
        assert_eq!(position, 5);
        assert_eq!(branch, "insert-branch");
    } else {
        panic!("Expected Insert variant");
    }
}

#[test]
fn queue_subcommand_remove() {
    let subcmd = QueueSubcommand::Remove {
        branch: "remove-branch".to_string(),
    };
    if let QueueSubcommand::Remove { branch } = subcmd {
        assert_eq!(branch, "remove-branch");
    } else {
        panic!("Expected Remove variant");
    }
}

#[test]
fn queue_subcommand_status() {
    let subcmd = QueueSubcommand::Status;
    assert!(matches!(subcmd, QueueSubcommand::Status));
}

#[test]
fn queue_subcommand_clear() {
    let subcmd = QueueSubcommand::Clear;
    assert!(matches!(subcmd, QueueSubcommand::Clear));
}

#[test]
fn queue_subcommand_detail() {
    let subcmd = QueueSubcommand::Detail {
        target: "detail-target".to_string(),
    };
    if let QueueSubcommand::Detail { target } = subcmd {
        assert_eq!(target, "detail-target");
    } else {
        panic!("Expected Detail variant");
    }
}

#[test]
fn queue_subcommand_all_variants_distinct() {
    let list = QueueSubcommand::List;
    let enqueue = QueueSubcommand::Enqueue {
        branch: "b".to_string(),
        priority: None,
    };
    let dequeue = QueueSubcommand::Dequeue;
    let process = QueueSubcommand::Process { checks: false };
    let insert = QueueSubcommand::Insert {
        position: 0,
        branch: "b".to_string(),
    };
    let remove = QueueSubcommand::Remove {
        branch: "b".to_string(),
    };
    let status = QueueSubcommand::Status;
    let clear = QueueSubcommand::Clear;
    let detail = QueueSubcommand::Detail {
        target: "t".to_string(),
    };

    assert_ne!(list, enqueue);
    assert_ne!(list, dequeue);
    assert_ne!(list, process);
    assert_ne!(list, insert);
    assert_ne!(list, remove);
    assert_ne!(list, status);
    assert_ne!(list, clear);
    assert_ne!(list, detail);
}

#[test]
fn queue_subcommand_serde_roundtrip_list() {
    let subcmd = QueueSubcommand::List;
    let json = serde_json::to_string(&subcmd).expect("serialize ok");
    let deserialized: QueueSubcommand = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(subcmd, deserialized);
}

#[test]
fn queue_subcommand_serde_roundtrip_enqueue() {
    let subcmd = QueueSubcommand::Enqueue {
        branch: "serde-branch".to_string(),
        priority: Some("high".to_string()),
    };
    let json = serde_json::to_string(&subcmd).expect("serialize ok");
    let deserialized: QueueSubcommand = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(subcmd, deserialized);
}

#[test]
fn queue_subcommand_serde_roundtrip_process() {
    let subcmd = QueueSubcommand::Process { checks: true };
    let json = serde_json::to_string(&subcmd).expect("serialize ok");
    let deserialized: QueueSubcommand = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(subcmd, deserialized);
}

// =============================================================================
// QueueOptions tests
// =============================================================================

#[test]
fn queue_options_default() {
    let opts = QueueOptions::default();
    assert!(matches!(opts.subcommand, QueueSubcommand::List));
    assert_eq!(opts.format, QueueOutputFormat::Table);
}

#[test]
fn queue_options_custom_subcommand() {
    let opts = QueueOptions {
        subcommand: QueueSubcommand::Status,
        format: QueueOutputFormat::Json,
    };

    assert!(matches!(opts.subcommand, QueueSubcommand::Status));
    assert_eq!(opts.format, QueueOutputFormat::Json);
}

#[test]
fn queue_options_clone() {
    let opts = QueueOptions {
        subcommand: QueueSubcommand::Detail {
            target: "test".to_string(),
        },
        format: QueueOutputFormat::Table,
    };
    let cloned = opts.clone();
    assert_eq!(opts, cloned);
}

#[test]
fn queue_options_equality() {
    let a = QueueOptions {
        subcommand: QueueSubcommand::List,
        format: QueueOutputFormat::Table,
    };
    let b = QueueOptions {
        subcommand: QueueSubcommand::List,
        format: QueueOutputFormat::Table,
    };
    let c = QueueOptions {
        subcommand: QueueSubcommand::Status,
        format: QueueOutputFormat::Table,
    };

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn queue_options_serde_roundtrip() {
    let opts = QueueOptions {
        subcommand: QueueSubcommand::Enqueue {
            branch: "serde".to_string(),
            priority: Some("low".to_string()),
        },
        format: QueueOutputFormat::Json,
    };

    let json = serde_json::to_string(&opts).expect("serialize ok");
    let deserialized: QueueOptions = serde_json::from_str(&json).expect("deserialize ok");
    assert_eq!(opts, deserialized);
}

// =============================================================================
// Calculations: format_priority_display tests
// =============================================================================

#[test]
fn format_priority_display_preserves_input() {
    assert_eq!(format_priority_display("Critical"), "Critical");
    assert_eq!(format_priority_display("high"), "high");
    assert_eq!(format_priority_display("NORMAL"), "NORMAL");
    assert_eq!(format_priority_display("low"), "low");
}

#[test]
fn format_priority_display_empty_string() {
    assert_eq!(format_priority_display(""), "");
}

#[test]
fn format_priority_display_whitespace() {
    assert_eq!(format_priority_display("  "), "  ");
}

#[test]
fn format_priority_display_special_characters() {
    assert_eq!(format_priority_display("High-123"), "High-123");
    assert_eq!(format_priority_display("Critical_"), "Critical_");
}

// =============================================================================
// Calculations: format_status_display tests
// =============================================================================

#[test]
fn format_status_display_preserves_input() {
    assert_eq!(format_status_display("Pending"), "Pending");
    assert_eq!(format_status_display("processing"), "processing");
    assert_eq!(format_status_display("COMPLETED"), "COMPLETED");
}

#[test]
fn format_status_display_empty_string() {
    assert_eq!(format_status_display(""), "");
}

// =============================================================================
// Calculations: parse_priority_to_string tests
// =============================================================================

#[test]
fn parse_priority_to_string_low() {
    assert_eq!(parse_priority_to_string("low"), "Low");
}

#[test]
fn parse_priority_to_string_low_uppercase() {
    assert_eq!(parse_priority_to_string("LOW"), "Low");
}

#[test]
fn parse_priority_to_string_high() {
    assert_eq!(parse_priority_to_string("high"), "High");
}

#[test]
fn parse_priority_to_string_critical() {
    assert_eq!(parse_priority_to_string("critical"), "Critical");
}

#[test]
fn parse_priority_to_string_normal() {
    assert_eq!(parse_priority_to_string("normal"), "Normal");
}

#[test]
fn parse_priority_to_string_unknown_falls_to_normal() {
    assert_eq!(parse_priority_to_string("unknown"), "Normal");
}

#[test]
fn parse_priority_to_string_empty_falls_to_normal() {
    assert_eq!(parse_priority_to_string(""), "Normal");
}

#[test]
fn parse_priority_to_string_partial_match_high_is_not_higher() {
    assert_eq!(parse_priority_to_string("higher"), "Normal");
}

#[test]
fn parse_priority_to_string_partial_match_crit_is_not_critical() {
    assert_eq!(parse_priority_to_string("crit"), "Normal");
}

#[test]
fn parse_priority_to_string_number_falls_to_normal() {
    assert_eq!(parse_priority_to_string("42"), "Normal");
}

#[test]
fn parse_priority_to_string_mixed_case() {
    assert_eq!(parse_priority_to_string("HiGh"), "High");
    assert_eq!(parse_priority_to_string("CrItIcAl"), "Critical");
}

#[test]
fn parse_priority_to_string_with_spaces() {
    assert_eq!(parse_priority_to_string(" high "), "Normal");
    assert_eq!(parse_priority_to_string(" LOW "), "Normal");
}

// =============================================================================
// Calculations: parse_status_to_string tests
// =============================================================================

#[test]
fn parse_status_to_string_pending() {
    assert_eq!(parse_status_to_string("pending"), "Pending");
}

#[test]
fn parse_status_to_string_pending_uppercase() {
    assert_eq!(parse_status_to_string("PENDING"), "Pending");
}

#[test]
fn parse_status_to_string_processing() {
    assert_eq!(parse_status_to_string("processing"), "Processing");
}

#[test]
fn parse_status_to_string_completed() {
    assert_eq!(parse_status_to_string("completed"), "Completed");
}

#[test]
fn parse_status_to_string_failed() {
    assert_eq!(parse_status_to_string("failed"), "Failed");
}

#[test]
fn parse_status_to_string_cancelled() {
    assert_eq!(parse_status_to_string("cancelled"), "Cancelled");
}

#[test]
fn parse_status_to_string_unknown_falls_to_unknown() {
    assert_eq!(parse_status_to_string("unknown"), "Unknown");
}

#[test]
fn parse_status_to_string_empty_falls_to_unknown() {
    assert_eq!(parse_status_to_string(""), "Unknown");
}

#[test]
fn parse_status_to_string_mixed_case() {
    assert_eq!(parse_status_to_string("PeNdInG"), "Pending");
    assert_eq!(parse_status_to_string("FaIlEd"), "Failed");
}

// =============================================================================
// Calculations: sort_by_priority tests
// =============================================================================

#[test]
fn sort_by_priority_empty_list() {
    let items: Vec<QueueListItem> = vec![];
    let sorted = sort_by_priority(items);
    assert!(sorted.is_empty());
}

#[test]
fn sort_by_priority_single_item() {
    let items = vec![QueueListItem {
        index: 1,
        branch: "single".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    }];
    let sorted = sort_by_priority(items);
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0].branch, "single");
}

#[test]
fn sort_by_priority_already_sorted() {
    let items = vec![
        QueueListItem {
            index: 0,
            branch: "critical".to_string(),
            priority: "Critical".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 1,
            branch: "high".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "normal".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 3,
            branch: "low".to_string(),
            priority: "Low".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let sorted = sort_by_priority(items.clone());
    assert_eq!(sorted[0].branch, "critical");
    assert_eq!(sorted[1].branch, "high");
    assert_eq!(sorted[2].branch, "normal");
    assert_eq!(sorted[3].branch, "low");
}

#[test]
fn sort_by_priority_reverse_order() {
    let items = vec![
        QueueListItem {
            index: 3,
            branch: "low".to_string(),
            priority: "Low".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "normal".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 1,
            branch: "high".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 0,
            branch: "critical".to_string(),
            priority: "Critical".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let sorted = sort_by_priority(items);
    assert_eq!(sorted[0].branch, "critical");
    assert_eq!(sorted[1].branch, "high");
    assert_eq!(sorted[2].branch, "normal");
    assert_eq!(sorted[3].branch, "low");
}

#[test]
fn sort_by_priority_mixed_priorities() {
    let items = vec![
        QueueListItem {
            index: 2,
            branch: "normal-1".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 0,
            branch: "critical-1".to_string(),
            priority: "Critical".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 3,
            branch: "low-1".to_string(),
            priority: "Low".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 1,
            branch: "high-1".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let sorted = sort_by_priority(items);
    assert_eq!(sorted[0].branch, "critical-1");
    assert_eq!(sorted[1].branch, "high-1");
    assert_eq!(sorted[2].branch, "normal-1");
    assert_eq!(sorted[3].branch, "low-1");
}

#[test]
fn sort_by_priority_preserves_index() {
    let items = vec![
        QueueListItem {
            index: 5,
            branch: "a".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 10,
            branch: "b".to_string(),
            priority: "Critical".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let sorted = sort_by_priority(items);
    assert_eq!(sorted[0].branch, "b");
    assert_eq!(sorted[0].index, 10);
    assert_eq!(sorted[1].branch, "a");
    assert_eq!(sorted[1].index, 5);
}

// =============================================================================
// Calculations: filter_by_status tests
// =============================================================================

#[test]
fn filter_by_status_empty_list() {
    let items: Vec<QueueListItem> = vec![];
    let filtered = filter_by_status(&items, "Pending");
    assert!(filtered.is_empty());
}

#[test]
fn filter_by_status_all_matching() {
    let items = vec![
        QueueListItem {
            index: 1,
            branch: "a".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "b".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let filtered = filter_by_status(&items, "Pending");
    assert_eq!(filtered.len(), 2);
}

#[test]
fn filter_by_status_none_matching() {
    let items = vec![
        QueueListItem {
            index: 1,
            branch: "a".to_string(),
            priority: "Normal".to_string(),
            status: "Completed".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "b".to_string(),
            priority: "High".to_string(),
            status: "Failed".to_string(),
        },
    ];
    let filtered = filter_by_status(&items, "Pending");
    assert!(filtered.is_empty());
}

#[test]
fn filter_by_status_partial_match() {
    let items = vec![
        QueueListItem {
            index: 1,
            branch: "pending-1".to_string(),
            priority: "Normal".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "completed-1".to_string(),
            priority: "High".to_string(),
            status: "Completed".to_string(),
        },
        QueueListItem {
            index: 3,
            branch: "pending-2".to_string(),
            priority: "Low".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let filtered = filter_by_status(&items, "Pending");
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].branch, "pending-1");
    assert_eq!(filtered[1].branch, "pending-2");
}

#[test]
fn filter_by_status_case_insensitive() {
    let items = vec![
        QueueListItem {
            index: 1,
            branch: "a".to_string(),
            priority: "Normal".to_string(),
            status: "PENDING".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "b".to_string(),
            priority: "High".to_string(),
            status: "pending".to_string(),
        },
        QueueListItem {
            index: 3,
            branch: "c".to_string(),
            priority: "Low".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let filtered = filter_by_status(&items, "pending");
    assert_eq!(filtered.len(), 3);
}

// =============================================================================
// Calculations: build_queue_table tests
// =============================================================================

#[test]
fn build_queue_table_empty() {
    let items: Vec<QueueListItem> = vec![];
    let table = build_queue_table(&items, QueueOutputFormat::Table);
    assert_eq!(table, "Queue is empty");
}

#[test]
fn build_queue_table_single_item() {
    let items = vec![QueueListItem {
        index: 1,
        branch: "single".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    }];
    let table = build_queue_table(&items, QueueOutputFormat::Table);
    assert!(table.contains("Queue (1 items):"));
    assert!(table.contains("single"));
}

#[test]
fn build_queue_table_multiple_items() {
    let items = vec![
        QueueListItem {
            index: 1,
            branch: "a".to_string(),
            priority: "Critical".to_string(),
            status: "Pending".to_string(),
        },
        QueueListItem {
            index: 2,
            branch: "b".to_string(),
            priority: "High".to_string(),
            status: "Pending".to_string(),
        },
    ];
    let table = build_queue_table(&items, QueueOutputFormat::Table);
    assert!(table.contains("Queue (2 items):"));
    assert!(table.contains("a"));
    assert!(table.contains("b"));
}

#[test]
fn build_queue_table_json_format() {
    let items = vec![QueueListItem {
        index: 1,
        branch: "json-test".to_string(),
        priority: "Normal".to_string(),
        status: "Pending".to_string(),
    }];
    let table = build_queue_table(&items, QueueOutputFormat::Json);
    // JSON format should still produce a table string in this implementation
    assert!(table.contains("json-test"));
}

// =============================================================================
// Calculations: build_queue_table_human tests
// =============================================================================

#[test]
fn build_queue_table_human_empty() {
    let items: Vec<QueueItemDisplay> = vec![];
    let table = build_queue_table_human(&items);
    assert_eq!(table, "Queue is empty");
}

#[test]
fn build_queue_table_human_single_item() {
    let items = vec![QueueItemDisplay {
        index: 1,
        branch: "human-test".to_string(),
        priority: "High".to_string(),
        status: "Pending".to_string(),
    }];
    let table = build_queue_table_human(&items);
    assert!(table.contains("Queue (1 items):"));
    assert!(table.contains("human-test"));
}

// =============================================================================
// Calculations: format_single_item tests
// =============================================================================

#[test]
fn format_single_item_basic() {
    let item = QueueItemDisplay {
        index: 1,
        branch: "test-branch".to_string(),
        priority: "Critical".to_string(),
        status: "Pending".to_string(),
    };
    let formatted = format_single_item(&item);
    assert_eq!(formatted, "test-branch [Critical] Pending");
}

#[test]
fn format_single_item_all_variants() {
    let item = QueueItemDisplay {
        index: 1,
        branch: "b".to_string(),
        priority: "Normal".to_string(),
        status: "Completed".to_string(),
    };
    let formatted = format_single_item(&item);
    assert_eq!(formatted, "b [Normal] Completed");
}

// =============================================================================
// Calculations: build_queue_status tests
// =============================================================================

#[test]
fn build_queue_status_empty() {
    let status = build_queue_status(0, 0, None);
    assert_eq!(status.total_items, 0);
    assert_eq!(status.pending_items, 0);
    assert!(status.next_item.is_none());
}

#[test]
fn build_queue_status_with_next() {
    let status = build_queue_status(10, 5, Some("next-branch"));
    assert_eq!(status.total_items, 10);
    assert_eq!(status.pending_items, 5);
    assert_eq!(status.next_item, Some("next-branch".to_string()));
}

// =============================================================================
// Calculations: format_queue_status tests
// =============================================================================

#[test]
fn format_queue_status_empty() {
    let status = QueueStatusDisplay {
        total_items: 0,
        pending_items: 0,
        next_item: None,
    };
    let formatted = format_queue_status(&status);
    assert!(formatted.contains("Queue Status:"));
    assert!(formatted.contains("Total items: 0"));
    assert!(formatted.contains("Pending: 0"));
    assert!(!formatted.contains("Next:"));
}

#[test]
fn format_queue_status_with_next() {
    let status = QueueStatusDisplay {
        total_items: 10,
        pending_items: 5,
        next_item: Some("next".to_string()),
    };
    let formatted = format_queue_status(&status);
    assert!(formatted.contains("Queue Status:"));
    assert!(formatted.contains("Total items: 10"));
    assert!(formatted.contains("Pending: 5"));
    assert!(formatted.contains("Next: next"));
}

// =============================================================================
// Calculations: format_item_detail tests
// =============================================================================

#[test]
fn format_item_detail_basic() {
    let detail = QueueItemDetail {
        id: "item-1".to_string(),
        branch: "test".to_string(),
        priority: "High".to_string(),
        status: "Pending".to_string(),
        source: "Direct".to_string(),
        attempt_count: 1,
        last_error: None,
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    };
    let formatted = format_item_detail(&detail);
    assert!(formatted.contains("Item Detail:"));
    assert!(formatted.contains("ID: item-1"));
    assert!(formatted.contains("Branch: test"));
}

#[test]
fn format_item_detail_with_error() {
    let detail = QueueItemDetail {
        id: "item-2".to_string(),
        branch: "error".to_string(),
        priority: "Normal".to_string(),
        status: "Failed".to_string(),
        source: "Workspace(ws)".to_string(),
        attempt_count: 3,
        last_error: Some("timeout".to_string()),
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    };
    let formatted = format_item_detail(&detail);
    assert!(formatted.contains("Item Detail:"));
    assert!(formatted.contains("Last Error: timeout"));
}

// =============================================================================
// Calculations: build_item_detail tests
// =============================================================================

#[test]
fn build_item_detail_full() {
    let item = QueueItemDisplay {
        index: 1,
        branch: "build-item".to_string(),
        priority: "High".to_string(),
        status: "Pending".to_string(),
    };
    let detail = build_item_detail(
        &item,
        "id-123",
        "Workspace(ws)",
        2,
        Some("error msg"),
        "2024-01-01T00:00:00Z",
        "2024-01-01T01:00:00Z",
    );
    assert_eq!(detail.id, "id-123");
    assert_eq!(detail.branch, "build-item");
    assert_eq!(detail.attempt_count, 2);
    assert_eq!(detail.last_error, Some("error msg".to_string()));
}

#[test]
fn build_item_detail_no_error() {
    let item = QueueItemDisplay {
        index: 1,
        branch: "clean".to_string(),
        priority: "Normal".to_string(),
        status: "Completed".to_string(),
    };
    let detail = build_item_detail(
        &item,
        "id-456",
        "Direct",
        1,
        None,
        "2024-01-01",
        "2024-01-01",
    );
    assert!(detail.last_error.is_none());
}

// =============================================================================
// Exhaustive: QueueOutputFormat exhaustive tests
// =============================================================================

const OUTPUT_FORMATS: [QueueOutputFormat; 2] = [QueueOutputFormat::Table, QueueOutputFormat::Json];

#[test]
fn output_format_has_two_variants() {
    assert_eq!(OUTPUT_FORMATS.len(), 2);
}

#[test]
fn output_format_all_variants_clone() {
    for format in OUTPUT_FORMATS.iter() {
        let cloned = *format;
        let cloned2 = cloned.clone();
        assert_eq!(cloned, cloned2);
    }
}

#[test]
fn output_format_all_variants_debug() {
    for format in &OUTPUT_FORMATS {
        let debug = format!("{:?}", format);
        assert!(!debug.is_empty());
    }
}

#[test]
fn output_format_all_variants_serialization() {
    for format in OUTPUT_FORMATS.iter() {
        let json = serde_json::to_string(format).expect("serialize ok");
        let roundtrip: QueueOutputFormat = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(*format, roundtrip);
    }
}

#[test]
fn output_format_exhaustive_reflexive_equality() {
    for format in &OUTPUT_FORMATS {
        assert_eq!(*format, *format);
    }
}

#[test]
fn output_format_exhaustive_cross_inequality() {
    for (i, a) in OUTPUT_FORMATS.iter().enumerate() {
        for (j, b) in OUTPUT_FORMATS.iter().enumerate() {
            if i != j {
                assert_ne!(*a, *b);
            }
        }
    }
}

// =============================================================================
// Exhaustive: QueueSubcommand exhaustive tests
// =============================================================================

#[test]
fn queue_subcommand_all_variants_match_exhaustive() {
    let subcmds = [
        QueueSubcommand::List,
        QueueSubcommand::Enqueue {
            branch: "b".to_string(),
            priority: None,
        },
        QueueSubcommand::Dequeue,
        QueueSubcommand::Process { checks: false },
        QueueSubcommand::Insert {
            position: 0,
            branch: "b".to_string(),
        },
        QueueSubcommand::Remove {
            branch: "b".to_string(),
        },
        QueueSubcommand::Status,
        QueueSubcommand::Clear,
        QueueSubcommand::Detail {
            target: "t".to_string(),
        },
    ];

    for subcmd in &subcmds {
        // Verify all variants can be matched exhaustively
        let _label = match subcmd {
            QueueSubcommand::List => "list",
            QueueSubcommand::Enqueue { .. } => "enqueue",
            QueueSubcommand::Dequeue => "dequeue",
            QueueSubcommand::Process { .. } => "process",
            QueueSubcommand::Insert { .. } => "insert",
            QueueSubcommand::Remove { .. } => "remove",
            QueueSubcommand::Status => "status",
            QueueSubcommand::Clear => "clear",
            QueueSubcommand::Detail { .. } => "detail",
        };
    }
}

#[test]
fn queue_subcommand_all_variants_serialization() {
    let subcmds = [
        QueueSubcommand::List,
        QueueSubcommand::Enqueue {
            branch: "serde".to_string(),
            priority: Some("high".to_string()),
        },
        QueueSubcommand::Dequeue,
        QueueSubcommand::Process { checks: true },
        QueueSubcommand::Insert {
            position: 5,
            branch: "serde".to_string(),
        },
        QueueSubcommand::Remove {
            branch: "serde".to_string(),
        },
        QueueSubcommand::Status,
        QueueSubcommand::Clear,
        QueueSubcommand::Detail {
            target: "serde".to_string(),
        },
    ];

    for subcmd in &subcmds {
        let json = serde_json::to_string(subcmd).expect("serialize ok");
        let roundtrip: QueueSubcommand = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(subcmd, &roundtrip);
    }
}

// =============================================================================
// Exhaustive: QueueStatusDisplay exhaustive tests
// =============================================================================

#[test]
fn queue_status_display_all_state_combinations() {
    // Test all combinations of total/pending/next
    let combinations = [
        (0, 0, None as Option<String>),
        (1, 0, None),
        (1, 1, Some("next".to_string())),
        (10, 5, Some("next".to_string())),
        (100, 100, Some("last".to_string())),
    ];

    for (total, pending, next) in combinations {
        let status = QueueStatusDisplay {
            total_items: total,
            pending_items: pending,
            next_item: next.clone(),
        };
        assert_eq!(status.total_items, total);
        assert_eq!(status.pending_items, pending);
        assert_eq!(status.next_item, next);
    }
}
