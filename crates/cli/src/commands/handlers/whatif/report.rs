//! Report/Action layer for WhatIf command
//!
//! I/O actions that output the preview results.

use scp_core::{output::Output, Result};

use crate::commands::handlers::whatif::{simulation::preview, WhatIfOptions, WhatIfResult};

/// Run the whatif command
///
/// **Actions (Tier 3)**: I/O - outputs preview
pub fn run_whatif(options: &WhatIfOptions) -> Result<()> {
    let result = preview(options)?;

    if options.format == scp_core::OutputFormat::Json {
        let json = serde_json::to_string_pretty(&result).map_err(|e| {
            scp_core::Error::io_error(format!("Failed to serialize whatif result: {e}"))
        })?;
        println!("{json}");
    } else {
        Output::info(&format!(
            "Preview for: {} {}",
            options.command,
            options.args.join(" ")
        ));
        Output::info(&format!(
            "Reversible: {}",
            if result.reversible { "yes" } else { "no" }
        ));

        if !result.steps.is_empty() {
            Output::info("Steps:");
            for step in &result.steps {
                Output::info(&format!("  {}. {}", step.order, step.description));
                Output::info(&format!("     Action: {}", step.action));
            }
        }

        if !result.creates.is_empty() {
            Output::info("Creates:");
            for c in &result.creates {
                Output::info(&format!("  {} ({})", c.resource, c.resource_type));
            }
        }

        if !result.modifies.is_empty() {
            Output::info("Modifies:");
            for m in &result.modifies {
                Output::info(&format!("  {} ({})", m.resource, m.resource_type));
            }
        }

        if !result.deletes.is_empty() {
            Output::info("Deletes:");
            for d in &result.deletes {
                Output::info(&format!("  {} ({})", d.resource, d.resource_type));
            }
        }

        if !result.warnings.is_empty() {
            Output::info("Warnings:");
            for w in &result.warnings {
                Output::warn(w);
            }
        }

        if let Some(undo) = &result.undo_command {
            Output::info(&format!("Undo: {undo}"));
        }
    }

    Ok(())
}
