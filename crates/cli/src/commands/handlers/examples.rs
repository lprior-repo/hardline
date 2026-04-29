//! Examples command - Show usage examples for commands
//!
//! Provides copy-pastable examples for AI agents and users.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: ExamplesOptions, Example, ExamplesResponse (inert, serializable)
//! - **Calculations**: build_examples, filter examples (pure functions)
//! - **Actions**: run_examples (I/O - output)

use scp_core::{output::Output, Error, OutputFormat, Result};
use serde::{Deserialize, Serialize};

/// Options for the examples command
#[derive(Debug, Clone)]
pub struct ExamplesOptions {
    /// Specific command to show examples for
    pub command: Option<String>,
    /// Filter by use case
    pub use_case: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Example entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// Command or workflow name
    pub name: String,
    /// Description of what this example does
    pub description: String,
    /// The actual command(s) to run
    pub commands: Vec<String>,
    /// Expected output (truncated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<String>,
    /// Use case category
    pub use_case: String,
    /// Prerequisites
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prerequisites: Vec<String>,
    /// Notes or warnings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Examples response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplesResponse {
    pub examples: Vec<Example>,
    pub use_cases: Vec<String>,
}

/// Run the examples command
///
/// **Actions (Tier 3)**: I/O - outputs filtered examples
pub fn run_examples(options: &ExamplesOptions) -> Result<()> {
    let all_examples = build_examples();

    let filtered: Vec<Example> = all_examples
        .examples
        .into_iter()
        .filter(|ex| {
            if let Some(cmd) = &options.command {
                if !ex.commands.iter().any(|c| c.contains(cmd.as_str())) {
                    return false;
                }
            }
            if let Some(use_case) = &options.use_case {
                if ex.use_case != *use_case {
                    return false;
                }
            }
            true
        })
        .collect();

    let response = ExamplesResponse {
        examples: filtered,
        use_cases: all_examples.use_cases,
    };

    if options.format == OutputFormat::Json {
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| Error::io_error(format!("Failed to serialize examples: {e}")))?;
        println!("{json}");
    } else {
        display_examples_text(&response.examples);
    }

    Ok(())
}

/// Display examples in human-readable text format.
fn display_examples_text(examples: &[Example]) {
    for example in examples {
        Output::info(&format!("# {}", example.name));
        Output::info(&format!("# {}", example.description));
        if !example.prerequisites.is_empty() {
            Output::info(&format!(
                "# Prerequisites: {}",
                example.prerequisites.join(", ")
            ));
        }
        println!();
        for cmd in &example.commands {
            println!("{cmd}");
        }
        if let Some(output) = &example.expected_output {
            println!();
            Output::info("# Expected output:");
            for line in output.lines() {
                println!("# {line}");
            }
        }
        if let Some(note) = &example.notes {
            println!();
            Output::info(&format!("# Note: {note}"));
        }
        println!();
        println!("---");
        println!();
    }
}

/// Build the complete catalog of examples
///
/// **Calculations (Tier 2)**: Pure function, no I/O
fn build_examples() -> ExamplesResponse {
    let mut examples = Vec::new();
    examples.extend(workflow_examples());
    examples.extend(single_command_examples());
    examples.extend(error_handling_examples());
    examples.extend(automation_examples());
    examples.extend(ai_agent_examples());
    examples.extend(safety_examples());
    examples.extend(maintenance_examples());

    let use_cases = vec![
        "workflow".to_string(),
        "single-command".to_string(),
        "error-handling".to_string(),
        "maintenance".to_string(),
        "automation".to_string(),
        "ai-agent".to_string(),
        "safety".to_string(),
    ];

    ExamplesResponse {
        examples,
        use_cases,
    }
}

fn workflow_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Start working on a feature".to_string(),
            description: "Create a workspace and start coding".to_string(),
            commands: vec!["scp workspace work --name feature-auth".to_string()],
            expected_output: Some(
                "Created session 'feature-auth'\nRegistered as agent".to_string(),
            ),
            use_case: "workflow".to_string(),
            prerequisites: vec!["scp init".to_string()],
            notes: None,
        },
        Example {
            name: "Complete work and merge".to_string(),
            description: "Finish work and merge to main".to_string(),
            commands: vec!["scp workspace done".to_string()],
            expected_output: Some("Merged 'feature-auth' to main".to_string()),
            use_case: "workflow".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: Some("Use --dry-run to preview first".to_string()),
        },
    ]
}

fn single_command_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Check current location".to_string(),
            description: "Quick orientation command for AI agents".to_string(),
            commands: vec!["scp whereami".to_string()],
            expected_output: Some("workspace:feature-auth".to_string()),
            use_case: "single-command".to_string(),
            prerequisites: vec![],
            notes: None,
        },
        Example {
            name: "List all sessions".to_string(),
            description: "View all active sessions with status".to_string(),
            commands: vec![
                "scp session list".to_string(),
                "scp session list --format json".to_string(),
            ],
            expected_output: None,
            use_case: "single-command".to_string(),
            prerequisites: vec!["scp init".to_string()],
            notes: None,
        },
        Example {
            name: "Sync workspace with main".to_string(),
            description: "Rebase workspace onto latest main".to_string(),
            commands: vec!["scp workspace sync".to_string()],
            expected_output: Some("Synced 1 session".to_string()),
            use_case: "single-command".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: None,
        },
    ]
}

fn error_handling_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Undo a merge".to_string(),
            description: "Revert the last done operation".to_string(),
            commands: vec![
                "scp workspace undo --dry-run".to_string(),
                "scp workspace undo".to_string(),
            ],
            expected_output: Some("Reverted merge of 'feature-auth'".to_string()),
            use_case: "error-handling".to_string(),
            prerequisites: vec![
                "Must have undo history".to_string(),
                "Not pushed to remote".to_string(),
            ],
            notes: None,
        },
        Example {
            name: "Abort work without merging".to_string(),
            description: "Discard work and cleanup".to_string(),
            commands: vec!["scp workspace abort".to_string()],
            expected_output: Some("Aborted 'feature-auth'".to_string()),
            use_case: "error-handling".to_string(),
            prerequisites: vec!["Must be in a workspace".to_string()],
            notes: None,
        },
    ]
}

fn automation_examples() -> Vec<Example> {
    vec![Example {
        name: "Spawn automated agent".to_string(),
        description: "Run an AI agent on a task".to_string(),
        commands: vec!["scp workspace spawn task-abc12".to_string()],
        expected_output: None,
        use_case: "automation".to_string(),
        prerequisites: vec!["Task must exist".to_string()],
        notes: Some("Agent runs in background".to_string()),
    }]
}

fn ai_agent_examples() -> Vec<Example> {
    vec![Example {
        name: "Get full context (AI agent)".to_string(),
        description: "Get complete environment context for AI".to_string(),
        commands: vec![
            "scp context".to_string(),
            "scp context --format json".to_string(),
        ],
        expected_output: None,
        use_case: "ai-agent".to_string(),
        prerequisites: vec![],
        notes: None,
    }]
}

fn safety_examples() -> Vec<Example> {
    vec![
        Example {
            name: "Create checkpoint".to_string(),
            description: "Save current state for rollback".to_string(),
            commands: vec![
                "scp workspace checkpoint create -d \"Before refactor\"".to_string(),
                "scp workspace checkpoint list".to_string(),
                "scp workspace checkpoint restore <checkpoint_id>".to_string(),
            ],
            expected_output: None,
            use_case: "safety".to_string(),
            prerequisites: vec!["scp init".to_string()],
            notes: None,
        },
        Example {
            name: "Dry-run preview".to_string(),
            description: "Preview operations without executing".to_string(),
            commands: vec![
                "scp workspace done --dry-run".to_string(),
                "scp workspace undo --dry-run".to_string(),
            ],
            expected_output: None,
            use_case: "safety".to_string(),
            prerequisites: vec![],
            notes: Some("No side effects, just shows what would happen".to_string()),
        },
    ]
}

fn maintenance_examples() -> Vec<Example> {
    vec![Example {
        name: "Run health checks".to_string(),
        description: "Diagnose and fix issues".to_string(),
        commands: vec!["scp doctor".to_string(), "scp doctor --full".to_string()],
        expected_output: None,
        use_case: "maintenance".to_string(),
        prerequisites: vec![],
        notes: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_examples_not_empty() {
        let examples = build_examples();
        assert!(!examples.examples.is_empty());
        assert!(!examples.use_cases.is_empty());
    }

    #[test]
    fn test_examples_have_commands() {
        let examples = build_examples();
        for ex in &examples.examples {
            assert!(!ex.commands.is_empty());
            assert!(!ex.name.is_empty());
            assert!(!ex.description.is_empty());
        }
    }

    #[test]
    fn test_all_use_cases_covered() {
        let examples = build_examples();
        for use_case in &examples.use_cases {
            let count = examples
                .examples
                .iter()
                .filter(|e| e.use_case == *use_case)
                .count();
            assert!(count > 0, "Use case {use_case} has no examples");
        }
    }

    #[test]
    fn test_filter_by_command() {
        let _options = ExamplesOptions {
            command: Some("done".to_string()),
            use_case: None,
            format: OutputFormat::Json,
        };
        let all = build_examples();
        let filtered: Vec<Example> = all
            .examples
            .into_iter()
            .filter(|ex| ex.commands.iter().any(|c| c.contains("done")))
            .collect();
        assert!(!filtered.is_empty());
        for ex in &filtered {
            assert!(ex.commands.iter().any(|c| c.contains("done")));
        }
    }

    #[test]
    fn test_filter_by_use_case() {
        let all = build_examples();
        let filtered: Vec<Example> = all
            .examples
            .into_iter()
            .filter(|ex| ex.use_case == "workflow")
            .collect();
        assert!(!filtered.is_empty());
        for ex in &filtered {
            assert_eq!(ex.use_case, "workflow");
        }
    }

    #[test]
    fn test_example_serialization() {
        let example = Example {
            name: "test".to_string(),
            description: "test desc".to_string(),
            commands: vec!["scp test".to_string()],
            expected_output: None,
            use_case: "test".to_string(),
            prerequisites: vec![],
            notes: None,
        };
        let json = serde_json::to_string(&example).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("scp test"));
        // Optional fields should be skipped
        assert!(!json.contains("expected_output"));
        assert!(!json.contains("prerequisites"));
        assert!(!json.contains("notes"));
    }

    #[test]
    fn test_examples_response_serialization() {
        let response = build_examples();
        let json = serde_json::to_string_pretty(&response).unwrap();
        assert!(json.contains("examples"));
        assert!(json.contains("use_cases"));
    }
}
