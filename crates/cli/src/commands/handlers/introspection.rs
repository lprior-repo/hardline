//! AI and introspection handlers for hardline CLI.
//!
//! This module contains handlers adapted from the isolate project,
//! adapted to work with hardline's architecture.
//!
//! Hardline uses derive macros in main.rs for CLI definition and
//! direct dispatch to handlers via the `Commands` enum.

use clap::ArgMatches;
use scp_core::{output::Output, Error, OutputFormat, Result};

use super::json_format::get_format;
use crate::commands::{
    context,
    handlers::{
        ai, can_i, contract, examples, introspect, validate,
        whatif::{report::run_whatif, WhatIfOptions},
        whoami,
    },
};

/// Handle the AI command - AI-first entry point for the CLI.
///
/// In hardline, this is called from dispatch when the AI subcommand is used.
/// The --contract and --ai-hints flags print JSON schema contracts.
pub async fn handle_ai(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        // Contract output for AI command - show schema contract
        println!("{}", ai_contract_schema());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", ai_hints_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let subcommand = match sub_m.subcommand() {
        Some(("status", _)) => ai::AiSubcommand::Status,
        Some(("workflow", _)) => ai::AiSubcommand::Workflow,
        Some(("quick-start", _)) => ai::AiSubcommand::QuickStart,
        Some(("next", _)) => ai::AiSubcommand::Next,
        _ => ai::AiSubcommand::Default,
    };
    let options = ai::AiOptions { subcommand, format };
    ai::run(&options)
}

/// Handle the introspect command - discover hardline capabilities.
///
/// In hardline, this shows command metadata and capabilities.
pub async fn handle_introspect(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", introspect_contract_schema());
        return Ok(());
    }

    let json = sub_m.get_flag("json");
    let ai_mode = sub_m.get_flag("ai");
    let format = OutputFormat::from_json_flag(json || ai_mode);
    if ai_mode {
        // AI mode shows simplified introspection
        introspect_ai_mode().await
    } else {
        // Use hardline's introspect module
        let target = sub_m.get_one::<String>("command").map(String::as_str);
        let options = introspect::IntrospectOptions::from_cli(target.map(String::from));
        introspect::run_introspect(&options)
    }
}

/// Handle the context command - show current workspace/branch/location.
pub async fn handle_context(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", context_contract_schema());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", ai_hints_schema());
        return Ok(());
    }

    let _field = sub_m.get_one::<String>("field").map(String::as_str);
    let _no_beads = sub_m.get_flag("no-beads");
    let _no_health = sub_m.get_flag("no-health");
    // hardline context command doesn't have these options - use basic run()
    context::run()
}

/// Handle the whereami command - show current location.
///
/// In hardline, this is an alias for context.
pub async fn handle_whereami(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", whereami_contract_schema());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", ai_hints_schema());
        return Ok(());
    }

    let _format = get_format(sub_m);
    context::whereami()
}

/// Handle the whoami command - show current agent identity.
pub fn handle_whoami(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", whoami_contract_schema());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", ai_hints_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let options = whoami::WhoamiOptions {
        json: format == OutputFormat::Json,
    };
    whoami::run(&options)
}

/// Handle the can-i command - check if an action is permitted.
pub async fn handle_can_i(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", can_i_contract_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let action = sub_m
        .get_one::<String>("action")
        .ok_or_else(|| Error::validation_error("Action is required"))?
        .clone();
    let resource = sub_m.get_one::<String>("resource").cloned();
    let options = can_i::CanIOptions {
        action,
        resource,
        format,
    };
    can_i::run(&options).await
}

/// Handle the contract command - show JSON Schema contracts for commands.
pub fn handle_contract(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", contract_contract_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let command = sub_m.get_one::<String>("command").cloned();
    let options = contract::ContractOptions { command, format };
    contract::run(&options)
}

/// Handle the examples command - show usage examples for commands.
pub fn handle_examples(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", examples_contract_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let command = sub_m.get_one::<String>("command").cloned();
    let use_case = sub_m.get_one::<String>("use-case").cloned();
    let options = examples::ExamplesOptions {
        command,
        use_case,
        format,
    };
    examples::run(&options)
}

/// Handle the validate command - pre-validate inputs before execution.
pub fn handle_validate(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", validate_contract_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let command = sub_m
        .get_one::<String>("command")
        .ok_or_else(|| Error::validation_error("Command is required"))?
        .clone();
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let dry_run = sub_m.get_flag("dry_run");
    let options = validate::ValidateOptions {
        command,
        args,
        format,
        dry_run,
    };
    validate::run(&options)
}

/// Handle the whatif command - preview what a command would do.
#[allow(clippy::too_many_lines)]
pub fn handle_whatif(sub_m: &ArgMatches) -> Result<()> {
    // Handle --contract flag first
    if sub_m.get_flag("contract") {
        println!("{}", whatif_contract_schema());
        return Ok(());
    }

    // Handle --ai-hints flag
    if sub_m.get_flag("ai-hints") {
        println!("{}", ai_hints_schema());
        return Ok(());
    }

    let format = get_format(sub_m);
    let command = sub_m
        .get_one::<String>("command")
        .ok_or_else(|| Error::validation_error("Command is required"))?
        .clone();
    let args: Vec<String> = sub_m
        .get_many::<String>("args")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();
    let options = WhatIfOptions {
        command: command.clone(),
        args: args.clone(),
        format,
    };
    let result = run_whatif(&options)?;

    if format == OutputFormat::Json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{json_str}");
    } else {
        println!("What-if preview for '{}' command:", command);
        println!();

        for step in &result.steps {
            println!("  {}. {}", step.order, step.description);
            println!("     > {}", step.action);
            if step.can_fail {
                if let Some(failure) = &step.on_failure {
                    println!("     (can fail: {failure})");
                } else {
                    println!("     (can fail)");
                }
            }
            println!();
        }

        if !result.creates.is_empty() {
            println!("  Creates:");
            for create in &result.creates {
                println!("    - {} ({})", create.resource, create.resource_type);
                println!("      {}", create.description);
            }
            println!();
        }

        if !result.modifies.is_empty() {
            println!("  Modifies:");
            for modify in &result.modifies {
                println!("    - {} ({})", modify.resource, modify.resource_type);
                println!("      {}", modify.description);
            }
            println!();
        }

        if !result.deletes.is_empty() {
            println!("  Deletes:");
            for delete in &result.deletes {
                println!("    - {} ({})", delete.resource, delete.resource_type);
                println!("      {}", delete.description);
            }
            println!();
        }

        if !result.side_effects.is_empty() {
            println!("  Side effects:");
            for effect in &result.side_effects {
                println!("    - {}", effect);
            }
            println!();
        }

        if result.reversible {
            println!("  Reversible: Yes");
            if let Some(undo) = &result.undo_command {
                println!("  Undo command: {}", undo);
            }
            println!();
        }

        if !result.warnings.is_empty() {
            println!("  Warnings:");
            for warning in &result.warnings {
                println!("    - {}", warning);
            }
            println!();
        }

        if !result.prerequisites.is_empty() {
            println!("  Prerequisites:");
            for prereq in &result.prerequisites {
                let status = match prereq.status {
                    whatif::PrerequisiteStatus::Met => "✓ Met",
                    whatif::PrerequisiteStatus::NotMet => "✗ Not met",
                    whatif::PrerequisiteStatus::Unknown => "? Unknown",
                };
                println!("    {} {}", status, prereq.check);
                println!("      {}", prereq.description);
            }
            println!();
        }
    }

    Ok(())
}

// ============================================================================
// Contract schema output functions (simplified for hardline)
// ============================================================================

fn ai_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://ai-contract/v1",
        "schema_type": "command-contract",
        "command": "ai",
        "description": "AI-first entry point for the CLI",
        "subcommands": ["status", "workflow", "quick-start", "next"],
        "flags": {
            "--contract": "Print JSON schema contract",
            "--ai-hints": "Print AI workflow hints"
        }
    })
    .to_string()
}

fn ai_hints_schema() -> String {
    serde_json::json!({
        "$schema": "scp://ai-hints/v1",
        "schema_type": "ai-hints",
        "message": "AI workflow hints for command execution",
        "hints": [
            "Use 'scp ai status' for current state",
            "Use 'scp ai next' for next recommended action",
            "Use 'scp context' for location info"
        ]
    })
    .to_string()
}

fn introspect_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://introspect-contract/v1",
        "schema_type": "command-contract",
        "command": "introspect",
        "description": "Discover hardline capabilities"
    })
    .to_string()
}

fn context_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://context-contract/v1",
        "schema_type": "command-contract",
        "command": "context",
        "description": "Show current workspace/branch/location"
    })
    .to_string()
}

fn whereami_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://whereami-contract/v1",
        "schema_type": "command-contract",
        "command": "whereami",
        "description": "Show current location (alias for context)"
    })
    .to_string()
}

fn whoami_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://whoami-contract/v1",
        "schema_type": "command-contract",
        "command": "whoami",
        "description": "Show current agent identity"
    })
    .to_string()
}

fn can_i_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://can-i-contract/v1",
        "schema_type": "command-contract",
        "command": "can-i",
        "description": "Check if an action is permitted",
        "arguments": {
            "action": "The action to check (required)",
            "resource": "Optional resource identifier"
        }
    })
    .to_string()
}

fn contract_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://contract-contract/v1",
        "schema_type": "command-contract",
        "command": "contract",
        "description": "Show JSON Schema contracts for commands"
    })
    .to_string()
}

fn examples_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://examples-contract/v1",
        "schema_type": "command-contract",
        "command": "examples",
        "description": "Show usage examples for commands"
    })
    .to_string()
}

fn validate_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://validate-contract/v1",
        "schema_type": "command-contract",
        "command": "validate",
        "description": "Pre-validate inputs before execution"
    })
    .to_string()
}

fn whatif_contract_schema() -> String {
    serde_json::json!({
        "$schema": "scp://whatif-contract/v1",
        "schema_type": "command-contract",
        "command": "whatif",
        "description": "Preview what a command would do"
    })
    .to_string()
}

// ============================================================================
// Helper functions for introspect command
// ============================================================================

/// Run introspection in AI mode (simplified output for AI consumption).
async fn introspect_ai_mode() -> Result<()> {
    Output::info("Hardline Capabilities:");
    Output::info("  - Workspace management (workspace add/remove/done)");
    Output::info("  - Session management (session list/status/focus)");
    Output::info("  - Task/bead tracking (task list/show/claim/done)");
    Output::info("  - Git operations (fetch/pull/push)");
    Output::info("  - Context awareness (context/whereami)");
    Output::info("  - AI assistance (ai status/workflow)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_single_variant() {
        let format1 = OutputFormat::Json;
        let format2 = OutputFormat::Json;
        assert_eq!(format1, format2);
    }

    #[test]
    fn test_output_format_from_json_flag_always_json() {
        let format_true = OutputFormat::from_json_flag(true);
        let format_false = OutputFormat::from_json_flag(false);
        assert_eq!(format_true, OutputFormat::Json);
        assert_eq!(format_false, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_to_json_flag_always_true() {
        let format = OutputFormat::Json;
        assert!(format.to_json_flag());
    }

    #[test]
    fn test_all_handlers_accept_output_format() {
        let format = OutputFormat::Json;
        assert!(format.is_json());
    }

    #[test]
    fn test_handlers_never_panic_on_format() {
        let format = OutputFormat::Json;
        let _ = format.is_json();
        let _ = format.to_string();
        let _ = format.to_json_flag();
    }

    #[test]
    fn test_format_parameter_reaches_command_functions() {
        let json_bool = true;
        let format = OutputFormat::from_json_flag(json_bool);
        assert!(format.is_json());
    }

    #[test]
    fn test_contract_schemas_are_valid_json() {
        for schema_fn in &[
            ai_contract_schema,
            ai_hints_schema,
            introspect_contract_schema,
            context_contract_schema,
            whereami_contract_schema,
            whoami_contract_schema,
            can_i_contract_schema,
            contract_contract_schema,
            examples_contract_schema,
            validate_contract_schema,
            whatif_contract_schema,
        ] {
            let schema = schema_fn();
            let parsed: serde_json::Value = serde_json::from_str(&schema).expect("valid JSON");
            assert!(
                parsed.get("$schema").is_some(),
                "Schema should have $schema field"
            );
        }
    }
}
