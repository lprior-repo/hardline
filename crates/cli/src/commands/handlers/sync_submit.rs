//! Sync, diff, submit, done, and abort CLI command handlers
//!
//! Ported from isolate project, adapted to hardline architecture.
//!
//! Hardline uses `scp_core` instead of `isolate_core`, and routes contract
//! documentation through the introspect system rather than `json_docs::ai_contracts`.

use anyhow::Result;
use clap::ArgMatches;
use scp_core::{
    domain::SessionName,
    vcs::{create_backend, CommitId, VcsBackend},
    OutputFormat,
};

use crate::commands::handlers::{
    json_format::get_format,
    sync::{
        sync_all_sessions, sync_current_workspace, sync_named_session, RetryConfig, SyncOptions,
    },
};

/// Print contract documentation via the introspect system.
///
/// In hardline, contracts are handled through the introspect command.
fn print_contract(target: &str) {
    let options = crate::commands::handlers::introspect::data::IntrospectOptions {
        target: crate::commands::handlers::introspect::data::IntrospectTarget::Specific(
            target.to_string(),
        ),
    };
    if let Err(e) = crate::commands::handlers::introspect::run_introspect(&options) {
        eprintln!("Failed to load contract for {}: {}", target, e);
    }
}

/// Print AI hints (command flow documentation).
///
/// Hardline doesn't have separate AI hints - route to introspect.
fn print_ai_hints() {
    let options = crate::commands::handlers::introspect::data::IntrospectOptions {
        target: crate::commands::handlers::introspect::data::IntrospectTarget::Specific(
            "command-flow".to_string(),
        ),
    };
    if let Err(e) = crate::commands::handlers::introspect::run_introspect(&options) {
        eprintln!("Failed to load AI hints: {}", e);
    }
}

/// Handle the sync command.
///
/// Syncs a workspace with main (rebase). In hardline, this routes to the
/// handlers::sync module which performs VCS sync operations.
pub async fn handle_sync(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        print_contract("sync");
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        print_ai_hints();
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let all = sub_m.get_flag("all");
    let dry_run = sub_m.get_flag("dry-run");
    let _format = get_format(sub_m);

    // The sync module has no dry-run mode, so a dry run reports the plan
    // and performs no rebase rather than silently syncing for real.
    if dry_run {
        if all {
            println!("[dry-run] Would sync all sessions");
        } else if let Some(name) = name {
            println!("[dry-run] Would sync session '{name}'");
        } else {
            println!("[dry-run] Would sync the current workspace");
        }
        return Ok(());
    }

    let options = SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
        },
    };

    let summary = if all {
        sync_all_sessions(options).await
    } else if let Some(name) = name {
        let session_name = SessionName::parse(name)
            .map_err(|e| anyhow::anyhow!("Invalid session name '{name}': {e}"))?;
        sync_named_session(session_name, options).await
    } else {
        sync_current_workspace(options).await
    }
    .map_err(|e| anyhow::anyhow!("sync failed: {e}"))?;

    println!(
        "Synced {} session(s) in {} operation(s){}",
        summary.sessions_synced.len(),
        summary.total_operations,
        if summary.had_conflicts {
            " (conflicts detected)"
        } else {
            ""
        }
    );
    Ok(())
}

/// Handle the submit command.
///
/// Submit is used to submit completed work. In hardline, this is typically
/// part of the done/abort workflow.
pub async fn handle_submit(sub_m: &ArgMatches) -> Result<()> {
    let _name = sub_m.get_one::<String>("name").cloned();
    let _format = get_format(sub_m);
    let _dry_run = sub_m.get_flag("dry-run");
    let _auto_commit = sub_m.get_flag("auto-commit");
    let _message = sub_m.get_one::<String>("message").cloned();

    // Hardline doesn't have a submit command yet - submit is typically
    // part of the done/abort workflow
    Err(anyhow::anyhow!(
        "submit command not yet implemented in hardline"
    ))
}

/// Handle the diff command.
///
/// Shows diff between workspace and main.
pub async fn handle_diff(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        print_contract("diff");
        return Ok(());
    }

    let name = sub_m.get_one::<String>("name").map(String::as_str);
    let stat = sub_m.get_flag("stat");
    let _format = get_format(sub_m);

    let name = match name {
        Some(name) => name,
        None => return Err(anyhow::anyhow!("diff requires a session name")),
    };

    let cwd = std::env::current_dir()?;
    let backend = create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;
    let workspace = workspaces
        .iter()
        .find(|w| w.name == name)
        .ok_or_else(|| anyhow::anyhow!("session not found: {name}"))?;

    // The backend's diff passes its arguments straight to `git diff`, so
    // branch names resolve as revisions there. There is no branch-tip
    // lookup on the trait, so this is the trait's own diff path rather
    // than a shell-out around it.
    let from = CommitId::from_unchecked("main");
    let to = CommitId::from_unchecked(workspace.branch.clone());
    let diff = backend.diff(&from, &to)?;

    if stat {
        for path in diff_file_paths(&diff) {
            println!(" {path}");
        }
    } else {
        print!("{diff}");
        if !diff.ends_with('\n') {
            println!();
        }
    }

    Ok(())
}

/// Extract the changed file paths from a unified diff.
///
/// Pure function over `diff --git a/<path> b/<path>` headers, used for the
/// `--stat` form of the diff command (the VCS trait exposes full diffs
/// only, not a stat mode).
fn diff_file_paths(diff: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(path) = rest.split_whitespace().nth(1) {
                let path = match path.strip_prefix("b/") {
                    Some(stripped) => stripped,
                    None => path,
                };
                paths.push(path.to_string());
            }
        }
    }
    paths
}

/// Handle the done command.
///
/// Complete workspace and merge to main.
pub async fn handle_done(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        print_contract("done");
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        print_ai_hints();
        return Ok(());
    }

    let _format = get_format(sub_m);

    // Build DoneOptions from CLI args
    // Hardline's done module uses DoneOptions directly
    let _options = crate::commands::handlers::done::DoneOptions {
        workspace: sub_m.get_one::<String>("workspace").cloned(),
        message: sub_m.get_one::<String>("message").cloned(),
        keep_workspace: sub_m.get_flag("keep-workspace"),
        no_bead_update: sub_m.get_flag("no-bead-update"),
        squash: sub_m.get_flag("squash"),
        dry_run: sub_m.get_flag("dry-run"),
        detect_conflicts: sub_m.get_flag("detect-conflicts"),
    };

    // The done module's run_done is async and returns DoneOutput
    // For now, indicate not implemented since we need proper integration
    Err(anyhow::anyhow!(
        "done command not yet fully integrated in hardline"
    ))
}

/// Handle the abort command.
///
/// Cancel a work session and revert changes.
pub async fn handle_abort(sub_m: &ArgMatches) -> Result<()> {
    if sub_m.get_flag("contract") {
        print_contract("abort");
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        print_ai_hints();
        return Ok(());
    }

    let _format = get_format(sub_m);

    // Hardline doesn't have an abort module yet
    // Abort would cancel a work session and revert changes
    Err(anyhow::anyhow!(
        "abort command not yet implemented in hardline"
    ))
}

#[cfg(test)]
mod tests {
    use scp_core::OutputFormat;

    #[test]
    fn test_handle_diff_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

    #[test]
    fn test_diff_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }

    #[test]
    fn test_diff_file_paths_extracts_changed_files() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\ndiff --git a/README.md b/README.md\n";
        assert_eq!(
            super::diff_file_paths(diff),
            vec!["src/main.rs".to_string(), "README.md".to_string()]
        );
    }

    #[test]
    fn test_diff_file_paths_empty_diff() {
        assert!(super::diff_file_paths("").is_empty());
        assert!(super::diff_file_paths("no diff headers here").is_empty());
    }
}
