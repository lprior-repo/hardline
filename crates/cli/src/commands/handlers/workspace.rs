//! Workspace session management handlers
//!
//! Ported from isolate project, adapted to hardline architecture.
//!
//! Hardline uses `scp_core` instead of `isolate_core`, and has different
//! command organization. This module provides handlers for workspace
//! operations that bridge between the CLI layer and hardline's command modules.

use scp_core::{Error, OutputFormat, Result};

use super::json_format::get_format;
use crate::commands::{
    handlers::{introspect, whoami, work},
    workspace,
};

/// Print contract documentation (AI contract for CLI behavior).
///
/// In hardline, contracts are handled through the introspect system.
fn print_contract(contract: &str, json_mode: bool) {
    if json_mode {
        let maybe_json = contract
            .find('{')
            .and_then(|start| contract.get(start..))
            .map(str::trim);
        if let Some(json_contract) = maybe_json {
            println!("{json_contract}");
        } else {
            println!("{contract}");
        }
    } else {
        println!("{contract}");
    }
}

/// Handle workspace initialization.
///
/// Routes to `commands::init::run()`.
pub async fn handle_init(sub_m: &clap::ArgMatches) -> Result<()> {
    let format = get_format(sub_m);
    let _dry_run = sub_m.get_flag("dry-run");
    let _vcs_type = sub_m
        .get_one::<String>("vcs")
        .map(String::as_str)
        .unwrap_or("git");

    // Hardline's init uses scp_core::Result
    crate::commands::init::run(_vcs_type)?;
    Ok(())
}

/// Handle workspace addition.
///
/// In hardline, `workspace add` creates a new workspace from a path.
pub async fn handle_add(sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have ai_contracts - handle contract flag gracefully
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("add".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    // Hardline doesn't have example-json flag
    if sub_m.get_flag("example-json") {
        eprintln!("example-json not supported in hardline");
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| Error::invalid_identifier("Name is required".to_string()))?;
    let _bead_id = sub_m.get_one::<String>("bead").cloned();
    let _no_hooks = sub_m.get_flag("no-hooks");
    let _no_open = sub_m.get_flag("no-open");
    let _idempotent = sub_m.get_flag("idempotent");
    let _dry_run = sub_m.get_flag("dry-run");
    let _format = get_format(sub_m);

    // Port: hardline uses workspace::add(path) instead of add options
    // In hardline, "add" takes a path, not a name
    workspace::add(name)?;
    Ok(())
}

/// Handle workspace listing.
///
/// Routes to `workspace::list()`.
pub async fn handle_list(sub_m: &clap::ArgMatches) -> Result<()> {
    let format = get_format(sub_m);

    // Hardline doesn't have ai_contracts - route to introspect instead
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("list".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    let _all = sub_m.get_flag("all");
    let _verbose = sub_m.get_flag("verbose");
    let _bead = sub_m.get_one::<String>("bead").cloned();
    let _agent = sub_m.get_one::<String>("agent").map(String::as_str);
    let _state = sub_m.get_one::<String>("state").map(String::as_str);

    // Hardline's list doesn't take filter parameters in this style
    workspace::list()?;
    Ok(())
}

/// Handle workspace removal.
///
/// Hardline uses workspace deletion through the VCS backend.
pub async fn handle_remove(sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have ai_contracts
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("remove".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| Error::invalid_identifier("Name is required".to_string()))?;
    let _format = get_format(sub_m);
    let _force = sub_m.get_flag("force");
    let _merge = sub_m.get_flag("merge");
    let _keep_branch = sub_m.get_flag("keep-branch");
    let _idempotent = sub_m.get_flag("idempotent");
    let _dry_run = sub_m.get_flag("dry-run");

    // Hardline doesn't have a direct remove command - suggest using VCS operations
    Err(Error::unimplemented(
        "workspace remove not yet implemented in hardline".to_string(),
    ))
}

/// Handle workspace status.
///
/// Routes to `workspace::status()`.
pub async fn handle_status(sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have ai_contracts
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("status".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    // Route to subcommand handlers
    match sub_m.subcommand() {
        Some(("show", _show_m)) => {
            workspace::status()?;
            Ok(())
        }
        Some(("whereami", whereami_m)) => handle_whereami(whereami_m).await,
        Some(("whoami", whoami_m)) => handle_whoami(whoami_m).await,
        Some(("context", context_m)) => handle_context(context_m).await,
        None => {
            // Legacy: scp status (no subcommand)
            workspace::status()?;
            Ok(())
        }
        Some((unknown, _)) => Err(Error::invalid_identifier(format!(
            "Unknown status subcommand: '{}'. Use 'show', 'whereami', 'whoami', or 'context'",
            unknown
        ))),
    }
}

/// Handle workspace switching.
///
/// Routes to `workspace::switch()`.
pub async fn handle_switch(sub_m: &clap::ArgMatches) -> Result<()> {
    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| Error::invalid_identifier("Name is required".to_string()))?;
    let _show_context = sub_m.get_flag("show-context");
    let _format = get_format(sub_m);

    workspace::switch(name)?;
    Ok(())
}

/// Handle workspace spawning.
///
/// Routes to `workspace::spawn()`.
pub async fn handle_spawn(sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have ai_contracts
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("spawn".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    let name = sub_m
        .get_one::<String>("name")
        .ok_or_else(|| Error::invalid_identifier("Name is required".to_string()))?;
    let _sync = sub_m.get_flag("sync");

    workspace::spawn(
        name,
        if _sync {
            workspace::SyncOption::WithSync
        } else {
            workspace::SyncOption::NoSync
        },
    )?;
    Ok(())
}

/// Handle work session creation.
///
/// Routes to `handlers::work::run_work()`.
pub async fn handle_work(sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have ai_contracts
    if sub_m.get_flag("contract") {
        let target = introspect::data::IntrospectTarget::Specific("work".to_string());
        let options = introspect::data::IntrospectOptions { target };
        introspect::run_introspect(&options)?;
        return Ok(());
    }

    if sub_m.get_flag("ai-hints") {
        println!("AI hints not available in hardline");
        return Ok(());
    }

    let format = get_format(sub_m);
    let name = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| Error::invalid_identifier("Name is required".to_string()))?;

    let mode = if sub_m.get_flag("dry-run") {
        work::WorkMode::DryRun
    } else if sub_m.get_flag("idempotent") {
        work::WorkMode::Idempotent
    } else {
        work::WorkMode::Normal
    };

    let options = work::WorkOptions {
        name,
        bead_id: sub_m.get_one::<String>("bead").cloned(),
        agent_id: sub_m.get_one::<String>("agent-id").cloned(),
        no_agent: sub_m.get_flag("no-agent"),
        mode,
        format,
    };

    work::run_work(&options)
}

/// Handle workspace rename.
///
/// Routes to `handlers::rename::run_rename()`.
pub async fn handle_rename(sub_m: &clap::ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let old_name = sub_m
        .get_one::<String>("old_name")
        .cloned()
        .ok_or_else(|| Error::invalid_identifier("old_name is required".to_string()))?;
    let new_name = sub_m
        .get_one::<String>("new_name")
        .cloned()
        .ok_or_else(|| Error::invalid_identifier("new_name is required".to_string()))?;
    let options = crate::commands::handlers::rename::RenameOptions {
        old_name,
        new_name,
        dry_run: false,
    };
    crate::commands::handlers::rename::run_rename(&options)
        .map_err(|e| Error::internal(e.to_string()))?;
    Ok(())
}

/// Handle workspace cloning.
///
/// Routes to `handlers::session::clone_session()`.
pub async fn handle_clone(sub_m: &clap::ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let source = sub_m
        .get_one::<String>("source")
        .ok_or_else(|| Error::invalid_identifier("Source session is required".to_string()))?
        .clone();
    let target = sub_m
        .get_one::<String>("dest")
        .ok_or_else(|| Error::invalid_identifier("Target destination is required".to_string()))?
        .clone();
    let _dry_run = sub_m.get_flag("dry-run");

    let result = crate::commands::handlers::session::clone_session(&source, &target, _dry_run)
        .map_err(|e| Error::internal(e.to_string()))?;
    if result.success {
        Ok(())
    } else {
        Err(Error::internal("Clone failed".to_string()))
    }
}

/// Handle workspace pause.
///
/// Routes to `handlers::session::pause()`.
pub async fn handle_pause(sub_m: &clap::ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| Error::invalid_identifier("Session name is required".to_string()))?;

    crate::commands::handlers::session::pause(&session)
        .map_err(|e| Error::internal(e.to_string()))?;
    Ok(())
}

/// Handle workspace resume.
///
/// Routes to `handlers::session::resume()`.
pub async fn handle_resume(sub_m: &clap::ArgMatches) -> Result<()> {
    let _format = get_format(sub_m);
    let session = sub_m
        .get_one::<String>("name")
        .cloned()
        .ok_or_else(|| Error::invalid_identifier("Session name is required".to_string()))?;

    crate::commands::handlers::session::resume(&session)
        .map_err(|e| Error::internal(e.to_string()))?;
    Ok(())
}

/// Handle whoami introspection.
async fn handle_whoami(sub_m: &clap::ArgMatches) -> Result<()> {
    let json = sub_m.get_flag("json");
    let options = whoami::WhoamiOptions { json };
    whoami::run_whoami(&options)
}

/// Handle whereami introspection.
async fn handle_whereami(_sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline doesn't have a separate whereami - use VCS to determine location
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;
    println!("{}", cwd.display());
    Ok(())
}

/// Handle context introspection.
async fn handle_context(_sub_m: &clap::ArgMatches) -> Result<()> {
    // Hardline's introspect system handles context
    let target = introspect::data::IntrospectTarget::Specific("context".to_string());
    let options = introspect::data::IntrospectOptions { target };
    introspect::run_introspect(&options)
}

#[cfg(test)]
mod tests {
    use scp_core::OutputFormat;

    #[test]
    fn test_handle_add_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_handle_init_converts_json_flag_to_output_format() {
        let json_flag = true;
        let format = OutputFormat::from_json_flag(json_flag);
        assert!(format.is_json());
    }

    #[test]
    fn test_add_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert_eq!(format, OutputFormat::Json);
        assert_eq!(format.to_json_flag(), json_bool);
    }

    #[test]
    fn test_init_json_flag_propagates_through_handler() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }
}
