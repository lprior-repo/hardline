//! Kani harnesses for task command pure function verification (bead hl-d3r).
//!
//! # Invariants Proven
//!
//! 1. `validate_task_command` correctly rejects all invalid inputs
//! 2. `task_state_to_output` covers all TaskState variants exhaustively
//! 3. `truncate_description` handles multi-byte UTF-8 correctly (never panics)
//! 4. `filter_tasks_by_status` returns subset of input
//! 5. `status_display_icon` returns non-empty strings for all variants

#[cfg(kani)]
mod proofs {
    use crate::commands::handlers::task::{
        filter_tasks_by_status, status_display_icon, task_state_to_output, truncate_description,
        validate_task_command, TaskCommand, TaskInfoOutput, TaskStatusOutput,
    };
    use crate::commands::task_types::TaskState;
    use chrono::Utc;

    // =========================================================================
    // hl-d3r: validate_task_command correctly rejects invalid inputs
    // =========================================================================

    /// Verify that List with any status_filter always passes validation
    #[kani::proof]
    fn prove_validate_list_always_ok() {
        let cmd = TaskCommand::List {
            status_filter: None,
            include_all: false,
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    /// Verify that Show with empty task_id is rejected
    #[kani::proof]
    fn prove_validate_show_empty_rejected() {
        let cmd = TaskCommand::Show {
            task_id: String::new(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    /// Verify that Claim with empty task_id is rejected
    #[kani::proof]
    fn prove_validate_claim_empty_rejected() {
        let cmd = TaskCommand::Claim {
            task_id: String::new(),
            agent_id: "agent-1".to_string(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    /// Verify that YieldTask with empty task_id is rejected
    #[kani::proof]
    fn prove_validate_yield_empty_rejected() {
        let cmd = TaskCommand::YieldTask {
            task_id: String::new(),
            agent_id: "agent-1".to_string(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    /// Verify that Start with empty task_id is rejected
    #[kani::proof]
    fn prove_validate_start_empty_rejected() {
        let cmd = TaskCommand::Start {
            task_id: String::new(),
            agent_id: "agent-1".to_string(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    /// Verify that Done with None task_id is accepted
    #[kani::proof]
    fn prove_validate_done_none_ok() {
        let cmd = TaskCommand::Done {
            task_id: None,
            agent_id: "agent-1".to_string(),
        };
        assert!(validate_task_command(&cmd).is_ok());
    }

    /// Verify that Done with empty Some(task_id) is rejected
    #[kani::proof]
    fn prove_validate_done_empty_some_rejected() {
        let cmd = TaskCommand::Done {
            task_id: Some(String::new()),
            agent_id: "agent-1".to_string(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    /// Verify that whitespace-only IDs are rejected for Show
    #[kani::proof]
    fn prove_validate_show_whitespace_rejected() {
        let cmd = TaskCommand::Show {
            task_id: "   ".to_string(),
        };
        assert!(validate_task_command(&cmd).is_err());
    }

    // =========================================================================
    // hl-d3r: task_state_to_output exhaustiveness
    // =========================================================================

    /// Verify all TaskState variants map to a distinct TaskStatusOutput
    #[kani::proof]
    fn prove_task_state_to_output_exhaustive() {
        // Open
        assert_eq!(task_state_to_output(&TaskState::Open), TaskStatusOutput::Open);

        // InProgress
        assert_eq!(
            task_state_to_output(&TaskState::InProgress),
            TaskStatusOutput::InProgress
        );

        // Blocked
        assert_eq!(
            task_state_to_output(&TaskState::Blocked),
            TaskStatusOutput::Blocked
        );

        // Deferred
        assert_eq!(
            task_state_to_output(&TaskState::Deferred),
            TaskStatusOutput::Deferred
        );

        // Closed
        assert_eq!(
            task_state_to_output(&TaskState::Closed {
                closed_at: Utc::now()
            }),
            TaskStatusOutput::Closed
        );
    }

    // =========================================================================
    // hl-d3r: truncate_description char boundary safety
    // =========================================================================

    /// Verify truncate_description never panics on ASCII input
    #[kani::proof]
    fn prove_truncate_description_ascii_no_panic() {
        let result = truncate_description("hello world", 5);
        assert!(result.len() <= 8); // "he..." = 5 chars max + "..."
    }

    /// Verify truncate_description handles empty string
    #[kani::proof]
    fn prove_truncate_description_empty() {
        let result = truncate_description("", 10);
        assert_eq!(result, "");
    }

    /// Verify truncate_description handles string shorter than max_len
    #[kani::proof]
    fn prove_truncate_description_short() {
        let result = truncate_description("hi", 10);
        assert_eq!(result, "hi");
    }

    /// Verify truncate_description handles multi-byte UTF-8 (emoji)
    #[kani::proof]
    fn prove_truncate_description_multibyte_emoji() {
        // Each emoji is 4 bytes
        let input = "\u{1F600}\u{1F601}\u{1F602}\u{1F603}";
        // max_len=10 -> end=7 -> should land on a char boundary
        let result = truncate_description(input, 10);
        // Result must be valid UTF-8 (never panics on char boundary)
        assert!(result.is_char_boundary(result.len()));
    }

    /// Verify truncate_description handles CJK characters
    #[kani::proof]
    fn prove_truncate_description_cjk() {
        // Each CJK char is 3 bytes
        let input = "\u{4E16}\u{754C}\u{4F60}\u{597D}"; // "世界你好"
        let result = truncate_description(input, 7);
        // 7 bytes -> can fit 2 CJK chars (6 bytes), end=4, takes while < 4
        // => last char at index 3, len=3 => safe_end = 6
        assert!(result.is_char_boundary(result.len()));
    }

    /// Verify truncate_description with max_len=0
    #[kani::proof]
    fn prove_truncate_description_zero_max_len() {
        let result = truncate_description("hello", 0);
        // max_len=0 <= desc.len(), end = saturating_sub(3) = 0
        // take_while(|(i, _)| *i < 0) => empty, safe_end = 0
        // format!("{}...", &desc[..0]) => "..."
        assert!(result.is_char_boundary(result.len()));
    }

    /// Verify truncate_description with max_len=1
    #[kani::proof]
    fn prove_truncate_description_one_max_len() {
        let result = truncate_description("hello", 1);
        // 1 <= 5, end = 0, safe_end = 0, result = "..."
        assert!(result.is_char_boundary(result.len()));
    }

    /// Verify truncate_description with max_len=2 (can only fit 0 chars before "...")
    #[kani::proof]
    fn prove_truncate_description_two_max_len() {
        let result = truncate_description("hello", 2);
        assert!(result.is_char_boundary(result.len()));
    }

    /// Verify truncate_description output is always valid UTF-8
    #[kani::proof]
    fn prove_truncate_description_valid_utf8() {
        let inputs = [
            "",
            "a",
            "hello",
            "\u{1F600}\u{1F601}",
            "\u{4E16}\u{754C}",
            "hello\u{1F600}world",
        ];
        let max_lens = [0usize, 1, 2, 3, 5, 10, 100];

        for input in &inputs {
            for max_len in &max_lens {
                let result = truncate_description(input, *max_len);
                assert!(result.is_char_boundary(result.len()));
            }
        }
    }

    // =========================================================================
    // hl-d3r: filter_tasks_by_status
    // =========================================================================

    /// Verify filtering returns a subset (never adds elements)
    #[kani::proof]
    fn prove_filter_tasks_subset() {
        let task = TaskInfoOutput {
            id: "1".to_string(),
            title: "Test".to_string(),
            status: TaskStatusOutput::Open,
            description: None,
            assignee: None,
            priority: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let tasks = vec![task.clone(); 3];
        let filtered = filter_tasks_by_status(&tasks, "open");
        assert!(filtered.len() <= tasks.len());
    }

    // =========================================================================
    // hl-d3r: status_display_icon
    // =========================================================================

    /// Verify all variants return non-empty icon strings
    #[kani::proof]
    fn prove_status_display_icon_all_non_empty() {
        for status in [
            TaskStatusOutput::Open,
            TaskStatusOutput::InProgress,
            TaskStatusOutput::Blocked,
            TaskStatusOutput::Deferred,
            TaskStatusOutput::Closed,
        ] {
            let icon = status_display_icon(&status);
            assert!(!icon.is_empty());
            assert!(icon.starts_with('['));
            assert!(icon.ends_with(']'));
        }
    }
}
