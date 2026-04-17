//! Stash command definitions
//!
//! Subcommand enum for Git stash operations.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum StashCommands {
    /// Save changes to stash
    Save {
        /// Stash message
        #[arg(short, long)]
        message: Option<String>,

        /// Include untracked files
        #[arg(short, long)]
        include_untracked: bool,

        /// Interactively select hunks to stash
        #[arg(short, long)]
        patch: bool,
    },

    /// Apply and remove stash
    Pop {
        /// Stash to pop
        stash: Option<String>,

        /// Also restore staged changes
        #[arg(short, long)]
        index: bool,
    },

    /// List stashed changes
    List,

    /// Drop a stash
    Drop {
        /// Stash reference
        stash: String,

        /// Force drop without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Show stash contents
    Show {
        /// Stash reference
        stash: Option<String>,

        /// Show diffstat only
        #[arg(short, long)]
        stat: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct StashParser {
        #[command(subcommand)]
        command: StashCommands,
    }

    fn parse(args: &[&str]) -> StashCommands {
        let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
        StashParser::parse_from(full).command
    }

    // -- List --
    #[test]
    fn list_no_args() {
        assert!(matches!(parse(&["list"]), StashCommands::List));
    }

    // -- Save (all defaults false/None) --
    #[test]
    fn save_defaults() {
        match parse(&["save"]) {
            StashCommands::Save {
                message,
                include_untracked,
                patch,
            } => {
                assert_eq!(message, None);
                assert!(!include_untracked);
                assert!(!patch);
            }
            other => panic!("Expected Save, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn save_with_all_flags() {
        match parse(&["save", "-m", "my stash", "--include-untracked", "--patch"]) {
            StashCommands::Save {
                message,
                include_untracked,
                patch,
            } => {
                assert_eq!(message, Some("my stash".to_string()));
                assert!(include_untracked);
                assert!(patch);
            }
            other => panic!("Expected Save, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Pop --
    #[test]
    fn pop_defaults() {
        match parse(&["pop"]) {
            StashCommands::Pop { stash, index } => {
                assert_eq!(stash, None);
                assert!(!index);
            }
            other => panic!("Expected Pop, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn pop_with_stash_and_index() {
        match parse(&["pop", "stash@{1}", "-i"]) {
            StashCommands::Pop { stash, index } => {
                assert_eq!(stash, Some("stash@{1}".to_string()));
                assert!(index);
            }
            other => panic!("Expected Pop, got {:?}", std::mem::discriminant(&other)),
        }
    }

    // -- Drop (required stash, bool force) --
    #[test]
    fn drop_defaults() {
        match parse(&["drop", "stash@{0}"]) {
            StashCommands::Drop { stash, force } => {
                assert_eq!(stash, "stash@{0}");
                assert!(!force);
            }
            other => panic!("Expected Drop, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn drop_with_force() {
        match parse(&["drop", "stash@{0}", "--force"]) {
            StashCommands::Drop { force, .. } => assert!(force),
            other => panic!("Expected Drop, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn drop_requires_stash() {
        let result = StashParser::try_parse_from(["scp", "drop"]);
        assert!(result.is_err());
    }

    // -- Show --
    #[test]
    fn show_defaults() {
        match parse(&["show"]) {
            StashCommands::Show { stash, stat } => {
                assert_eq!(stash, None);
                assert!(!stat);
            }
            other => panic!("Expected Show, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn show_with_args() {
        match parse(&["show", "stash@{2}", "--stat"]) {
            StashCommands::Show { stash, stat } => {
                assert_eq!(stash, Some("stash@{2}".to_string()));
                assert!(stat);
            }
            other => panic!("Expected Show, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
