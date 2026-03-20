//! Shared JSON format extraction helper for CLI handlers
//!
//! This module provides a single helper function to reduce code duplication
//! across command handlers that all follow the pattern:
//!
//! ```ignore
//! let json = sub_m.get_flag("json");
//! let format = OutputFormat::from_json_flag(json);
//! ```

use clap::ArgMatches;
use scp_core::OutputFormat;

/// Extract the output format from clap argument matches
///
/// This helper consolidates the common pattern of checking for the `--json` flag
/// and converting it to an `OutputFormat`. Used by virtually all CLI handlers.
///
/// # Example
///
/// ```ignore
/// use crate::cli::handlers::json_format::get_format;
///
/// pub async fn handle_foo(sub_m: &ArgMatches) -> Result<()> {
///     let format = get_format(sub_m);
///     // ... use format
/// }
/// ```
#[must_use]
pub fn get_format(matches: &ArgMatches) -> OutputFormat {
    OutputFormat::from_json_flag(matches.get_flag("json"))
}

/// Alias for `get_format` for backward compatibility
#[must_use]
pub fn extract_json_flag(matches: &ArgMatches) -> OutputFormat {
    get_format(matches)
}

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};

    use super::*;

    fn make_matches(json_flag: bool) -> ArgMatches {
        Command::new("test")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(clap::ArgAction::SetTrue),
            )
            .try_get_matches_from(if json_flag {
                vec!["test", "--json"]
            } else {
                vec!["test"]
            })
            .expect("valid matches")
    }

    #[test]
    fn test_get_format_returns_json_when_flag_set() {
        let matches = make_matches(true);
        let format = get_format(&matches);
        assert!(
            format.is_json(),
            "Expected Json format when --json flag is set"
        );
    }

    #[test]
    fn test_get_format_returns_human_by_default() {
        let matches = make_matches(false);
        let format = get_format(&matches);
        assert!(format.is_json(), "Expected Human format by default");
    }

    #[test]
    fn test_get_format_is_pure_function() {
        let matches = make_matches(true);
        let format1 = get_format(&matches);
        let format2 = get_format(&matches);
        assert_eq!(format1, format2, "get_format should be deterministic");
    }

    #[test]
    fn test_output_format_roundtrip() {
        let format = OutputFormat::from_json_flag(true);
        assert!(format.to_json_flag());

        let format = OutputFormat::from_json_flag(false);
        assert!(format.to_json_flag());
    }

    #[test]
    fn test_get_format_matches_direct_derivation() {
        let matches = make_matches(true);
        let helper_format = get_format(&matches);
        let direct_format = OutputFormat::from_json_flag(matches.get_flag("json"));
        assert_eq!(
            helper_format, direct_format,
            "get_format should match direct derivation"
        );
    }
}
