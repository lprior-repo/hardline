//! Data types for the bookmark command handler (Tier 1).
//!
//! Inert, serializable types with no business logic.
//! These types represent the inputs and outputs of the bookmark command,
//! which manages Git branch-style bookmarks (create, list, delete, track).

use serde::{Deserialize, Serialize};

// ============================================================================
// Input Types
// ============================================================================

/// Options for the bookmark command (parsed from CLI).
#[derive(Debug, Clone)]
pub struct BookmarkOptions {
    /// Subcommand to execute.
    pub subcommand: BookmarkSubcommand,
}

/// Bookmark subcommands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookmarkSubcommand {
    /// Create a new bookmark at the current revision.
    Create {
        /// Bookmark name.
        name: String,
        /// Push to remote after creation.
        push: bool,
    },
    /// List bookmarks.
    List {
        /// Show all bookmarks including remotes.
        show_all: bool,
    },
    /// Delete a bookmark.
    Delete {
        /// Bookmark name to delete.
        name: String,
    },
    /// Track a remote bookmark (set upstream).
    Track {
        /// Bookmark name to track.
        name: String,
        /// Remote name (defaults to "origin").
        remote: Option<String>,
    },
}

// ============================================================================
// Output Types
// ============================================================================

/// Information about a single bookmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkInfo {
    /// Bookmark name.
    pub name: String,
    /// Current revision/commit hash.
    pub revision: String,
    /// Whether this bookmark exists on a remote.
    pub remote: bool,
}

/// Result of a bookmark list operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkListOutput {
    /// List of bookmarks found.
    pub bookmarks: Vec<BookmarkInfo>,
    /// Total count.
    pub count: usize,
}

/// Result of a bookmark create operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkCreateOutput {
    /// Created bookmark name.
    pub name: String,
    /// Revision the bookmark was created at.
    pub revision: String,
    /// Whether it was pushed to remote.
    pub pushed: bool,
}

/// Result of a bookmark delete operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkDeleteOutput {
    /// Deleted bookmark name.
    pub name: String,
}

/// Result of a bookmark track operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkTrackOutput {
    /// Bookmark name that was tracked.
    pub name: String,
    /// Remote being tracked.
    pub remote: String,
}

/// Unified output from the bookmark command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookmarkOutput {
    /// Output from a list operation.
    List(BookmarkListOutput),
    /// Output from a create operation.
    Create(BookmarkCreateOutput),
    /// Output from a delete operation.
    Delete(BookmarkDeleteOutput),
    /// Output from a track operation.
    Track(BookmarkTrackOutput),
}

// ============================================================================
// Pure Helper Functions
// ============================================================================

/// Validate a bookmark name contains only allowed characters.
///
/// Allowed: ASCII alphanumeric, hyphens, underscores.
/// Disallowed: empty strings, spaces, special characters.
///
/// # Examples (in tests)
///
/// Valid names: `feature-auth`, `bugfix_123`, `main`
/// Invalid names: ``, `has space`, `special!char`
#[must_use]
pub fn validate_bookmark_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Parse Git branch listing output into `BookmarkInfo` structs.
///
/// Handles format: `name: revision_hash` or `name: change_id commit_id description`.
/// Skips lines starting with whitespace (remote branch indicators)
/// and lines containing `(deleted)`.
#[must_use]
pub fn parse_bookmark_list(output: &str) -> Vec<BookmarkInfo> {
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Skip indented remote bookmark lines
            if line.starts_with("  ") {
                return false;
            }
            // Filter out deleted bookmarks
            !trimmed.contains("(deleted)")
        })
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            match parts.as_slice() {
                [name_part, rest] => {
                    let name = name_part.trim().to_string();
                    let tokens: Vec<&str> = rest.split_whitespace().collect();
                    let revision = if tokens.len() >= 2 {
                        // New format: skip change_id, get commit_id
                        tokens
                            .get(1)
                            .map_or_else(|| "unknown".to_string(), ToString::to_string)
                    } else {
                        // Legacy format: use first token
                        tokens
                            .first()
                            .map_or_else(|| "unknown".to_string(), ToString::to_string)
                    };
                    let remote = name_part.contains("@origin");
                    Some(BookmarkInfo {
                        name,
                        revision,
                        remote,
                    })
                }
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- BookmarkSubcommand construction --

    #[test]
    fn bookmark_subcommand_create() {
        let sub = BookmarkSubcommand::Create {
            name: "feature-auth".to_string(),
            push: false,
        };
        assert!(matches!(sub, BookmarkSubcommand::Create { name, push } if name == "feature-auth" && !push));
    }

    #[test]
    fn bookmark_subcommand_create_with_push() {
        let sub = BookmarkSubcommand::Create {
            name: "feature".to_string(),
            push: true,
        };
        assert!(matches!(sub, BookmarkSubcommand::Create { push: true, .. }));
    }

    #[test]
    fn bookmark_subcommand_list() {
        let sub = BookmarkSubcommand::List { show_all: false };
        assert!(matches!(sub, BookmarkSubcommand::List { show_all: false }));
    }

    #[test]
    fn bookmark_subcommand_list_all() {
        let sub = BookmarkSubcommand::List { show_all: true };
        assert!(matches!(sub, BookmarkSubcommand::List { show_all: true }));
    }

    #[test]
    fn bookmark_subcommand_delete() {
        let sub = BookmarkSubcommand::Delete {
            name: "old-feature".to_string(),
        };
        assert!(matches!(sub, BookmarkSubcommand::Delete { name } if name == "old-feature"));
    }

    #[test]
    fn bookmark_subcommand_track() {
        let sub = BookmarkSubcommand::Track {
            name: "main".to_string(),
            remote: Some("origin".to_string()),
        };
        assert!(matches!(sub, BookmarkSubcommand::Track { remote: Some(_), .. }));
    }

    #[test]
    fn bookmark_subcommand_track_default_remote() {
        let sub = BookmarkSubcommand::Track {
            name: "main".to_string(),
            remote: None,
        };
        assert!(matches!(sub, BookmarkSubcommand::Track { remote: None, .. }));
    }

    // -- validate_bookmark_name --

    #[test]
    fn validate_bookmark_name_valid_simple() {
        assert!(validate_bookmark_name("main"));
    }

    #[test]
    fn validate_bookmark_name_valid_hyphenated() {
        assert!(validate_bookmark_name("feature-auth"));
    }

    #[test]
    fn validate_bookmark_name_valid_with_underscore() {
        assert!(validate_bookmark_name("bugfix_123"));
    }

    #[test]
    fn validate_bookmark_name_valid_numeric() {
        assert!(validate_bookmark_name("v2"));
    }

    #[test]
    fn validate_bookmark_name_rejects_empty() {
        assert!(!validate_bookmark_name(""));
    }

    #[test]
    fn validate_bookmark_name_rejects_spaces() {
        assert!(!validate_bookmark_name("has space"));
    }

    #[test]
    fn validate_bookmark_name_rejects_special_chars() {
        assert!(!validate_bookmark_name("special!char"));
    }

    #[test]
    fn validate_bookmark_name_rejects_dot() {
        assert!(!validate_bookmark_name("feature.name"));
    }

    #[test]
    fn validate_bookmark_name_rejects_slash() {
        assert!(!validate_bookmark_name("feature/name"));
    }

    // -- parse_bookmark_list --

    #[test]
    fn parse_bookmark_list_empty() {
        let result = parse_bookmark_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_bookmark_list_single_legacy() {
        let output = "feature-v1: abc123def456\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "feature-v1");
        assert_eq!(result[0].revision, "abc123def456");
        assert!(!result[0].remote);
    }

    #[test]
    fn parse_bookmark_list_single_new_format() {
        let output = "main: ntzomurw e553bf6b feat: some description\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "main");
        assert_eq!(result[0].revision, "e553bf6b");
        assert!(!result[0].remote);
    }

    #[test]
    fn parse_bookmark_list_multiple() {
        let output = "main: abc123\nfeature: def456\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "main");
        assert_eq!(result[1].name, "feature");
    }

    #[test]
    fn parse_bookmark_list_skips_indented_remotes() {
        let output = "main: ntzomurw e553bf6b main bookmark\n  @origin: ntzomurw e553bf6b remote\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "main");
    }

    #[test]
    fn parse_bookmark_list_skips_deleted() {
        let output = "main: abc123def456\nold-feature: xyz789 (deleted)\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "main");
    }

    #[test]
    fn parse_bookmark_list_complex_mixed() {
        let output = "\
main: ntzomurw e553bf6b feat: Broadcast command
  @origin: ntzomurw e553bf6b remote
feature: pqrlsyvw 195a784b test: Another feature
deprecated: vwxyz123 (deleted)
bugfix: kmnopqr6 2d4e5f6c fix: Critical bug\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "main");
        assert_eq!(result[0].revision, "e553bf6b");
        assert_eq!(result[1].name, "feature");
        assert_eq!(result[1].revision, "195a784b");
        assert_eq!(result[2].name, "bugfix");
        assert_eq!(result[2].revision, "2d4e5f6c");
    }

    #[test]
    fn parse_bookmark_list_detects_origin_remote() {
        let output = "main@origin: abc123\n";
        let result = parse_bookmark_list(output);
        assert_eq!(result.len(), 1);
        assert!(result[0].remote);
    }

    // -- BookmarkInfo serialization --

    #[test]
    fn bookmark_info_serialization_roundtrip() {
        let info = BookmarkInfo {
            name: "main".to_string(),
            revision: "abc123".to_string(),
            remote: true,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let deserialized: BookmarkInfo =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, info);
    }

    #[test]
    fn bookmark_info_json_contains_fields() {
        let info = BookmarkInfo {
            name: "feature".to_string(),
            revision: "def456".to_string(),
            remote: false,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        assert!(json.contains("\"name\":\"feature\""));
        assert!(json.contains("\"revision\":\"def456\""));
        assert!(json.contains("\"remote\":false"));
    }

    // -- BookmarkListOutput --

    #[test]
    fn bookmark_list_output_empty() {
        let output = BookmarkListOutput {
            bookmarks: vec![],
            count: 0,
        };
        assert_eq!(output.count, 0);
        assert!(output.bookmarks.is_empty());
    }

    #[test]
    fn bookmark_list_output_serialization_roundtrip() {
        let output = BookmarkListOutput {
            bookmarks: vec![BookmarkInfo {
                name: "main".to_string(),
                revision: "abc".to_string(),
                remote: false,
            }],
            count: 1,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BookmarkListOutput =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.count, 1);
        assert_eq!(deserialized.bookmarks[0].name, "main");
    }

    // -- BookmarkCreateOutput --

    #[test]
    fn bookmark_create_output_serialization() {
        let output = BookmarkCreateOutput {
            name: "feature".to_string(),
            revision: "abc123".to_string(),
            pushed: true,
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BookmarkCreateOutput =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "feature");
        assert!(deserialized.pushed);
    }

    // -- BookmarkDeleteOutput --

    #[test]
    fn bookmark_delete_output_serialization() {
        let output = BookmarkDeleteOutput {
            name: "old-feature".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BookmarkDeleteOutput =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "old-feature");
    }

    // -- BookmarkTrackOutput --

    #[test]
    fn bookmark_track_output_serialization() {
        let output = BookmarkTrackOutput {
            name: "main".to_string(),
            remote: "origin".to_string(),
        };
        let json = serde_json::to_string(&output).expect("serialize");
        let deserialized: BookmarkTrackOutput =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.remote, "origin");
    }

    // -- BookmarkOutput enum --

    #[test]
    fn bookmark_output_list_variant() {
        let output = BookmarkOutput::List(BookmarkListOutput {
            bookmarks: vec![],
            count: 0,
        });
        assert!(matches!(output, BookmarkOutput::List(_)));
    }

    #[test]
    fn bookmark_output_create_variant() {
        let output = BookmarkOutput::Create(BookmarkCreateOutput {
            name: "test".to_string(),
            revision: "abc".to_string(),
            pushed: false,
        });
        assert!(matches!(output, BookmarkOutput::Create(_)));
    }

    #[test]
    fn bookmark_output_delete_variant() {
        let output = BookmarkOutput::Delete(BookmarkDeleteOutput {
            name: "test".to_string(),
        });
        assert!(matches!(output, BookmarkOutput::Delete(_)));
    }

    #[test]
    fn bookmark_output_track_variant() {
        let output = BookmarkOutput::Track(BookmarkTrackOutput {
            name: "test".to_string(),
            remote: "origin".to_string(),
        });
        assert!(matches!(output, BookmarkOutput::Track(_)));
    }

    // -- BookmarkOptions construction --

    #[test]
    fn bookmark_options_with_create() {
        let opts = BookmarkOptions {
            subcommand: BookmarkSubcommand::Create {
                name: "feature".to_string(),
                push: true,
            },
        };
        assert!(matches!(
            opts.subcommand,
            BookmarkSubcommand::Create { name, push: true } if name == "feature"
        ));
    }

    #[test]
    fn bookmark_options_with_list() {
        let opts = BookmarkOptions {
            subcommand: BookmarkSubcommand::List { show_all: true },
        };
        assert!(matches!(opts.subcommand, BookmarkSubcommand::List { show_all: true }));
    }
}
