//! Tests for workspace argument parsing

use crate::cli::workspace_args::*;
use clap::Parser;

/// Wrapper to parse WorkspaceCommands directly via clap
#[derive(Parser)]
struct WorkspaceParser {
    #[command(subcommand)]
    command: WorkspaceCommands,
}

fn parse(args: &[&str]) -> WorkspaceCommands {
    let full: Vec<&str> = std::iter::once("scp").chain(args.iter().copied()).collect();
    WorkspaceParser::parse_from(full).command
}

// -- List / Status / Uncommitted / Branches / BranchCurrent --
#[test]
fn list_no_args() {
    assert!(matches!(parse(&["list"]), WorkspaceCommands::List));
}

#[test]
fn status_no_args() {
    assert!(matches!(parse(&["status"]), WorkspaceCommands::Status));
}

#[test]
fn uncommitted_no_args() {
    assert!(matches!(
        parse(&["uncommitted"]),
        WorkspaceCommands::Uncommitted
    ));
}

#[test]
fn branches_no_args() {
    assert!(matches!(parse(&["branches"]), WorkspaceCommands::Branches));
}

#[test]
fn branch_current_no_args() {
    assert!(matches!(
        parse(&["branch-current"]),
        WorkspaceCommands::BranchCurrent
    ));
}

// -- Spawn (required name, bool flag) --
#[test]
fn spawn_default_sync() {
    match parse(&["spawn", "ws-name"]) {
        WorkspaceCommands::Spawn { name, sync } => {
            assert_eq!(name, "ws-name");
            assert!(!sync);
        }
        other => panic!("Expected Spawn, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn spawn_with_sync() {
    match parse(&["spawn", "ws-name", "--sync"]) {
        WorkspaceCommands::Spawn { sync, .. } => assert!(sync),
        other => panic!("Expected Spawn, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn spawn_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "spawn"]);
    assert!(result.is_err());
}

// -- Switch (required name) --
#[test]
fn switch_parses() {
    match parse(&["switch", "ws-name"]) {
        WorkspaceCommands::Switch { name } => assert_eq!(name, "ws-name"),
        other => panic!("Expected Switch, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn switch_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "switch"]);
    assert!(result.is_err());
}

// -- Sync (optional name, bool flag) --
#[test]
fn sync_defaults() {
    match parse(&["sync"]) {
        WorkspaceCommands::Sync { name, all } => {
            assert_eq!(name, None);
            assert!(!all);
        }
        other => panic!("Expected Sync, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn sync_with_name_and_all() {
    match parse(&["sync", "ws-name", "--all"]) {
        WorkspaceCommands::Sync { name, all } => {
            assert_eq!(name, Some("ws-name".to_string()));
            assert!(all);
        }
        other => panic!("Expected Sync, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Done (all bool flags default false, optional fields default None) --
#[test]
fn done_all_defaults() {
    match parse(&["done"]) {
        WorkspaceCommands::Done {
            name,
            message,
            keep_workspace,
            squash,
            dry_run,
            detect_conflicts,
            no_bead_update,
        } => {
            assert_eq!(name, None);
            assert_eq!(message, None);
            assert!(!keep_workspace);
            assert!(!squash);
            assert!(!dry_run);
            assert!(!detect_conflicts);
            assert!(!no_bead_update);
        }
        other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn done_all_flags_set() {
    match parse(&[
        "done",
        "ws-name",
        "-m",
        "msg",
        "--keep-workspace",
        "--squash",
        "--dry-run",
        "--detect-conflicts",
        "--no-bead-update",
    ]) {
        WorkspaceCommands::Done {
            name,
            message,
            keep_workspace,
            squash,
            dry_run,
            detect_conflicts,
            no_bead_update,
        } => {
            assert_eq!(name, Some("ws-name".to_string()));
            assert_eq!(message, Some("msg".to_string()));
            assert!(keep_workspace);
            assert!(squash);
            assert!(dry_run);
            assert!(detect_conflicts);
            assert!(no_bead_update);
        }
        other => panic!("Expected Done, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Abort (optional name) --
#[test]
fn abort_defaults() {
    match parse(&["abort"]) {
        WorkspaceCommands::Abort { name } => assert_eq!(name, None),
        other => panic!("Expected Abort, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn abort_with_name() {
    match parse(&["abort", "ws-name"]) {
        WorkspaceCommands::Abort { name } => {
            assert_eq!(name, Some("ws-name".to_string()));
        }
        other => panic!("Expected Abort, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Log (optional limit) --
#[test]
fn log_defaults() {
    match parse(&["log"]) {
        WorkspaceCommands::Log { limit } => assert_eq!(limit, None),
        other => panic!("Expected Log, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn log_with_limit() {
    match parse(&["log", "10"]) {
        WorkspaceCommands::Log { limit } => assert_eq!(limit, Some(10)),
        other => panic!("Expected Log, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Diff (optional path) --
#[test]
fn diff_defaults() {
    match parse(&["diff"]) {
        WorkspaceCommands::Diff { path } => assert_eq!(path, None),
        other => panic!("Expected Diff, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn diff_with_path() {
    match parse(&["diff", "src/main.rs"]) {
        WorkspaceCommands::Diff { path } => {
            assert_eq!(path, Some("src/main.rs".to_string()));
        }
        other => panic!("Expected Diff, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Commit (required message) --
#[test]
fn commit_parses() {
    match parse(&["commit", "my changes"]) {
        WorkspaceCommands::Commit { message } => assert_eq!(message, "my changes"),
        other => panic!("Expected Commit, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn commit_requires_message() {
    let result = WorkspaceParser::try_parse_from(["scp", "commit"]);
    assert!(result.is_err());
}

// -- Branch (required name) --
#[test]
fn branch_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "branch"]);
    assert!(result.is_err());
}

// -- BranchDelete (required name) --
#[test]
fn branch_delete_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "branch-delete"]);
    assert!(result.is_err());
}

// -- Fork (required name, optional from) --
#[test]
fn fork_defaults() {
    match parse(&["fork", "new-ws"]) {
        WorkspaceCommands::Fork { name, from } => {
            assert_eq!(name, "new-ws");
            assert_eq!(from, None);
        }
        other => panic!("Expected Fork, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn fork_with_from() {
    match parse(&["fork", "new-ws", "existing-ws"]) {
        WorkspaceCommands::Fork { name, from } => {
            assert_eq!(name, "new-ws");
            assert_eq!(from, Some("existing-ws".to_string()));
        }
        other => panic!("Expected Fork, got {:?}", std::mem::discriminant(&other)),
    }
}

// -- Merge (required name) --
#[test]
fn merge_parses() {
    match parse(&["merge", "ws-name"]) {
        WorkspaceCommands::Merge { name } => assert_eq!(name, "ws-name"),
        other => panic!("Expected Merge, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn merge_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "merge"]);
    assert!(result.is_err());
}

// -- Add (required path) --
#[test]
fn add_parses() {
    match parse(&["add", "/path/to/dir"]) {
        WorkspaceCommands::Add { path } => assert_eq!(path, "/path/to/dir"),
        other => panic!("Expected Add, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn add_requires_path() {
    let result = WorkspaceParser::try_parse_from(["scp", "add"]);
    assert!(result.is_err());
}

// -- Revert (required name, optional dry_run flag) --
#[test]
fn revert_defaults() {
    match parse(&["revert", "feature-x"]) {
        WorkspaceCommands::Revert { name, dry_run } => {
            assert_eq!(name, "feature-x");
            assert!(!dry_run);
        }
        other => panic!("Expected Revert, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn revert_with_dry_run() {
    match parse(&["revert", "feature-x", "--dry-run"]) {
        WorkspaceCommands::Revert { name, dry_run } => {
            assert_eq!(name, "feature-x");
            assert!(dry_run);
        }
        other => panic!("Expected Revert, got {:?}", std::mem::discriminant(&other)),
    }
}

#[test]
fn revert_requires_name() {
    let result = WorkspaceParser::try_parse_from(["scp", "revert"]);
    assert!(result.is_err());
}

// -- BranchRename (required old_name, new_name) --
#[test]
fn branch_rename_defaults() {
    match parse(&["branch-rename", "old-name", "new-name"]) {
        WorkspaceCommands::BranchRename {
            old_name,
            new_name,
            dry_run,
        } => {
            assert_eq!(old_name, "old-name");
            assert_eq!(new_name, "new-name");
            assert!(!dry_run);
        }
        other => panic!(
            "Expected BranchRename, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

#[test]
fn branch_rename_with_dry_run() {
    match parse(&["branch-rename", "old-name", "new-name", "--dry-run"]) {
        WorkspaceCommands::BranchRename {
            old_name,
            new_name,
            dry_run,
        } => {
            assert_eq!(old_name, "old-name");
            assert_eq!(new_name, "new-name");
            assert!(dry_run);
        }
        other => panic!(
            "Expected BranchRename with dry_run, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

#[test]
fn branch_rename_requires_names() {
    let result = WorkspaceParser::try_parse_from(["scp", "branch-rename"]);
    assert!(result.is_err());
}
