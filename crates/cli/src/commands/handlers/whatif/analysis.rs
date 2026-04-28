//! Analysis utilities for WhatIf preview
//!
//! Helper functions for parsing and validating command arguments.

use scp_core::Result;

use crate::commands::handlers::whatif::WhatIfResult;

/// Extract the name argument from a command's argument list
///
/// Returns a tuple of (name, is_placeholder) where is_placeholder
/// is true if no real name was provided (defaulting to "<name>")
pub fn get_name_arg(args: &[String]) -> (String, bool) {
    let name = args.first().map(String::as_str).unwrap_or("<name>");
    let is_placeholder = name == "<name>";
    (name.to_string(), is_placeholder)
}

/// Validate a session name if it's not a placeholder
///
/// Placeholders (like "<name>") are allowed in preview generation
/// but real names must pass validation.
pub fn ensure_valid_name(name: &str, is_placeholder: bool) -> Result<()> {
    if !is_placeholder {
        scp_core::validation::domain::validate_session_name(name)
            .map_err(|e| scp_core::Error::validation_error(e.to_string()))?;
    }
    Ok(())
}
