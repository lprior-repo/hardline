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
    fn test_get_format_returns_json_by_default() {
        let matches = make_matches(false);
        let format = get_format(&matches);
        assert!(format.is_json(), "OutputFormat is always JSON");
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

    #[test]
    fn test_extract_json_flag_is_alias_for_get_format() {
        let matches_json = make_matches(true);
        let matches_human = make_matches(false);

        assert_eq!(
            extract_json_flag(&matches_json),
            get_format(&matches_json),
            "extract_json_flag should match get_format when json is set"
        );
        assert_eq!(
            extract_json_flag(&matches_human),
            get_format(&matches_human),
            "extract_json_flag should match get_format when json is not set"
        );
    }

    #[test]
    fn test_get_format_idempotent_multiple_calls() {
        let matches = make_matches(false);
        let f1 = get_format(&matches);
        let f2 = get_format(&matches);
        let f3 = get_format(&matches);
        assert_eq!(f1, f2);
        assert_eq!(f2, f3);
    }

    #[test]
    fn test_get_format_without_json_arg_defined_panics() {
        // When a command doesn't define --json at all, get_flag will panic
        // because the arg id doesn't exist. This is expected behavior --
        // get_format should only be called with commands that define --json.
        let matches = Command::new("no-json-flag")
            .try_get_matches_from(vec!["no-json-flag"])
            .expect("valid");
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| get_format(&matches)));
        assert!(
            result.is_err(),
            "get_format should panic when --json is not defined"
        );
    }

    #[test]
    fn test_get_format_with_other_flags_ignored() {
        // --json flag should work regardless of other flags present
        let matches = Command::new("test")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .action(clap::ArgAction::SetTrue),
            )
            .try_get_matches_from(vec!["test", "--verbose", "--json"])
            .expect("valid matches");
        let format = get_format(&matches);
        assert!(format.is_json());
    }

    #[test]
    fn test_get_format_with_json_false_flag() {
        // When --json is explicitly passed as --no-json (if supported) or omitted
        let matches = Command::new("test")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(clap::ArgAction::SetTrue),
            )
            .try_get_matches_from(vec!["test"])
            .expect("valid matches");
        let format = get_format(&matches);
        // OutputFormat is always JSON regardless of flag state
        assert!(format.is_json());
    }

    #[test]
    fn test_extract_json_flag_consistency_across_calls() {
        let matches_json = make_matches(true);
        let matches_no = make_matches(false);
        // Both extract_json_flag and get_format should be consistent
        assert_eq!(
            extract_json_flag(&matches_json),
            extract_json_flag(&matches_no),
            "both return Json regardless of flag since OutputFormat is always JSON"
        );
    }

    #[test]
    fn test_get_format_with_multiple_true_flags_conflicts() {
        // clap SetTrue conflicts when the same flag is passed multiple times.
        // The CLI layer should prevent this, but get_format itself assumes
        // valid matches.
        let result = Command::new("test")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(clap::ArgAction::SetTrue),
            )
            .try_get_matches_from(vec!["test", "--json", "--json"]);
        assert!(result.is_err(), "duplicate SetTrue flags should conflict");
    }

    #[test]
    fn test_output_format_equality() {
        let f1 = OutputFormat::from_json_flag(true);
        let f2 = OutputFormat::from_json_flag(false);
        assert_eq!(f1, f2, "OutputFormat is always JSON");
    }

    #[test]
    fn test_output_format_clone() {
        let format = OutputFormat::from_json_flag(true);
        let cloned = format.clone();
        assert_eq!(format, cloned);
    }

    #[test]
    fn test_output_format_debug() {
        let format = OutputFormat::from_json_flag(true);
        let debug_str = format!("{:?}", format);
        assert!(
            !debug_str.is_empty(),
            "Debug representation should not be empty"
        );
    }

    #[test]
    fn test_get_format_reflexive() {
        let matches = make_matches(true);
        let f = get_format(&matches);
        // Reflexive: format should equal itself
        assert_eq!(f, f);
    }

    #[test]
    fn test_get_format_symmetric() {
        let matches_true = make_matches(true);
        let matches_false = make_matches(false);
        let f_true = get_format(&matches_true);
        let f_false = get_format(&matches_false);
        // Both are always JSON, so symmetric
        assert_eq!(f_true, f_false);
        assert_eq!(f_false, f_true);
    }

    #[test]
    fn test_get_format_transitive() {
        let matches = make_matches(false);
        let f1 = get_format(&matches);
        let f2 = get_format(&matches);
        let f3 = get_format(&matches);
        // Transitive: if a==b and b==c then a==c
        assert_eq!(f1, f2);
        assert_eq!(f2, f3);
        assert_eq!(f1, f3);
    }

    use proptest::prelude::*;
    use proptest::prop_assert;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_get_format_always_returns_json(json_flag in proptest::bool::ANY) {
            let matches = Command::new("test")
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
                .expect("valid matches");
            let format = get_format(&matches);
            prop_assert!(format.is_json());
        }
    }
}
