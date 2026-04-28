//! Tag command definitions
//!
//! Subcommand enum for Git tag operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum TagCommands {
    /// Create a tag
    Create {
        /// Tag name
        name: String,

        /// Annotated tag message
        #[arg(short, long)]
        message: Option<String>,

        /// Tag specific commit
        #[arg(short, long)]
        commit: Option<String>,

        /// Replace existing tag
        #[arg(long)]
        force: bool,
    },

    /// List tags
    List {
        /// Pattern to match
        #[arg(short, long)]
        pattern: Option<String>,

        /// Sort by key
        #[arg(long)]
        sort: Option<String>,
    },

    /// Delete a tag
    Delete {
        /// Tag to delete
        tag: String,

        /// Delete remote tag
        #[arg(short, long)]
        remote: bool,
    },

    /// Push tags to remote
    Push {
        /// Specific tag to push
        tag: Option<String>,

        /// Remote to push to
        #[arg(short, long, default_value = "origin")]
        remote: String,

        /// Force push
        #[arg(long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct TagParser {
        #[command(subcommand)]
        command: TagCommands,
    }

    fn parse(args: &[&str]) -> TagCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        TagParser::parse_from(full).command
    }

    // -- Create --
    #[test]
    fn create_defaults() {
        match parse(&["create", "v1.0.0"]) {
            TagCommands::Create {
                name,
                message,
                commit,
                force,
            } => {
                assert_eq!(name, "v1.0.0");
                assert_eq!(message, None);
                assert_eq!(commit, None);
                assert!(!force);
            }
            other => panic!("Expected Create, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn create_with_all_options() {
        match parse(&[
            "create", "v1.0.0", "-m", "release", "-c", "abc123", "--force",
        ]) {
            TagCommands::Create {
                name,
                message,
                commit,
                force,
            } => {
                assert_eq!(name, "v1.0.0");
                assert_eq!(message, Some("release".to_string()));
                assert_eq!(commit, Some("abc123".to_string()));
                assert!(force);
            }
            other => panic!("Expected Create, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn create_requires_name() {
        let result = TagParser::try_parse_from(["scp", "create"]);
        assert!(result.is_err());
    }

    // -- List --
    #[test]
    fn list_defaults() {
        match parse(&["list"]) {
            TagCommands::List { pattern, sort } => {
                assert_eq!(pattern, None);
                assert_eq!(sort, None);
            }
            other => panic!("Expected List, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn list_with_filters() {
        match parse(&["list", "-p", "v*", "--sort", "version:refname"]) {
            TagCommands::List { pattern, sort } => {
                assert_eq!(pattern, Some("v*".to_string()));
                assert_eq!(sort, Some("version:refname".to_string()));
            }
            other => panic!("Expected List, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Delete --
    #[test]
    fn delete_defaults() {
        match parse(&["delete", "v0.9.0"]) {
            TagCommands::Delete { tag, remote } => {
                assert_eq!(tag, "v0.9.0");
                assert!(!remote);
            }
            other => panic!("Expected Delete, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn delete_with_remote_flag() {
        match parse(&["delete", "v0.9.0", "-r"]) {
            TagCommands::Delete { remote, .. } => assert!(remote),
            other => panic!("Expected Delete, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn delete_requires_tag() {
        let result = TagParser::try_parse_from(["scp", "delete"]);
        assert!(result.is_err());
    }

    // -- Push --
    #[test]
    fn push_defaults() {
        match parse(&["push"]) {
            TagCommands::Push { tag, remote, force } => {
                assert_eq!(tag, None);
                assert_eq!(remote, "origin");
                assert!(!force);
            }
            other => panic!("Expected Push, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn push_with_specific_tag_and_custom_remote() {
        match parse(&["push", "v1.0.0", "-r", "upstream", "--force"]) {
            TagCommands::Push { tag, remote, force } => {
                assert_eq!(tag, Some("v1.0.0".to_string()));
                assert_eq!(remote, "upstream");
                assert!(force);
            }
            other => panic!("Expected Push, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
