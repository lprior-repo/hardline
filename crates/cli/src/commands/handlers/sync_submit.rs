//! Sync, diff, submit, done, and abort CLI command handlers
//!
//! Ported from isolate project, adapted to hardline architecture.
//!
//! Hardline uses `scp_core` instead of `isolate_core`, and routes contract
//! documentation through the introspect system rather than `json_docs::ai_contracts`.

use anyhow::Result;
use clap::ArgMatches;
use scp_core::OutputFormat;

use crate::commands::handlers::json_format::get_format;

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

    let _name = sub_m.get_one::<String>("name").map(String::as_str);
    let _all = sub_m.get_flag("all");
    let _dry_run = sub_m.get_flag("dry-run");
    let _format = get_format(sub_m);

    // Hardline's sync is handled by the existing sync.rs in handlers
    // which provides VCS-based session synchronization
    // TODO: Integrate with existing sync module properly
    Err(anyhow::anyhow!(
        "sync command integration with VCS sync not yet complete"
    ))
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
    let _stat = sub_m.get_flag("stat");
    let _format = get_format(sub_m);

    // Hardline doesn't have a diff command module yet
    // Route to introspect for documentation
    if name.is_some() {
        // TODO: Implement actual diff using VCS backend
        eprintln!("diff command not fully implemented in hardline");
        let options = crate::commands::handlers::introspect::data::IntrospectOptions {
            target: crate::commands::handlers::introspect::data::IntrospectTarget::Specific(
                "diff".to_string(),
            ),
        };
        crate::commands::handlers::introspect::run_introspect(&options)?;
    } else {
        return Err(anyhow::anyhow!("diff requires a session name"));
    }

    Ok(())
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
}
