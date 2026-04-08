//! Action functions for the contract command handler (Tier 3).
//!
//! I/O operations that display command contracts.

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{known_contracts, ContractOptions};

/// Execute the contract command with the given options.
///
/// # Errors
///
/// Returns error if the requested command contract is not found.
pub fn run_contract(options: &ContractOptions) -> Result<()> {
    let contracts = known_contracts();

    match &options.command {
        Some(command_name) => {
            let contract = contracts
                .iter()
                .find(|c| c.name == *command_name)
                .ok_or_else(|| {
                    Error::not_found(format!(
                        "No contract found for command '{command_name}'"
                    ))
                })?;

            Output::info(&format!("Contract for '{}':", contract.name));
            Output::info(&format!("  Description: {}", contract.description));
            Output::info(&format!("  Output: {}", contract.output_schema));
            Output::info(&format!(
                "  Reversible: {}",
                if contract.reversible { "yes" } else { "no" }
            ));

            if !contract.required_args.is_empty() {
                Output::info("  Required args:");
                for arg in &contract.required_args {
                    Output::info(&format!("    {} ({})", arg.name, arg.arg_type));
                }
            }

            if !contract.flags.is_empty() {
                Output::info("  Flags:");
                for flag in &contract.flags {
                    Output::info(&format!(
                        "    {} - {} (default: {})",
                        flag.name,
                        flag.description,
                        if flag.default { "true" } else { "false" }
                    ));
                }
            }

            if !contract.side_effects.is_empty() {
                Output::info(&format!("  Side effects: {}", contract.side_effects.join(", ")));
            }

            if !contract.examples.is_empty() {
                Output::info("  Examples:");
                for example in &contract.examples {
                    Output::info(&format!("    {example}"));
                }
            }
        }
        None => {
            Output::info(&format!("Available contracts ({}):", contracts.len()));
            for contract in &contracts {
                Output::info(&format!(
                    "  {} - {}",
                    contract.name, contract.description
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Listing: command=None ─────────────────────────────────────────────

    #[test]
    fn run_contract_list_all_succeeds() {
        let options = ContractOptions { command: None };
        let result = run_contract(&options);
        assert!(result.is_ok());
    }

    #[test]
    fn run_contract_list_returns_unit() {
        let options = ContractOptions { command: None };
        assert!(run_contract(&options).is_ok());
    }

    // ── Specific contract lookup: command=Some(_) ────────────────────────

    #[test]
    fn run_contract_spawn_succeeds() {
        let options = ContractOptions {
            command: Some("spawn".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_done_succeeds() {
        let options = ContractOptions {
            command: Some("done".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_revert_succeeds() {
        let options = ContractOptions {
            command: Some("revert".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    // ── Unknown command: error path ──────────────────────────────────────

    #[test]
    fn run_contract_unknown_command_returns_error() {
        let options = ContractOptions {
            command: Some("nonexistent".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    #[test]
    fn run_contract_unknown_command_error_is_not_found() {
        let options = ContractOptions {
            command: Some("does_not_exist".to_string()),
        };
        let err = run_contract(&options).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("No contract found"),
            "expected not-found message, got: {msg}"
        );
    }

    #[test]
    fn run_contract_unknown_command_error_includes_name() {
        let options = ContractOptions {
            command: Some("foobar".to_string()),
        };
        let err = run_contract(&options).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("foobar"),
            "error should mention the requested command name, got: {msg}"
        );
    }

    #[test]
    fn run_contract_empty_string_name_returns_error() {
        let options = ContractOptions {
            command: Some(String::new()),
        };
        assert!(run_contract(&options).is_err());
    }

    // ── Case sensitivity ─────────────────────────────────────────────────

    #[test]
    fn run_contract_lookup_is_case_sensitive() {
        // "Spawn" (capitalized) should NOT match "spawn"
        let options = ContractOptions {
            command: Some("Spawn".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    #[test]
    fn run_contract_uppercase_name_returns_error() {
        let options = ContractOptions {
            command: Some("SPAWN".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    // ── Whitespace handling ──────────────────────────────────────────────

    #[test]
    fn run_contract_name_with_leading_whitespace_not_found() {
        let options = ContractOptions {
            command: Some(" spawn".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    #[test]
    fn run_contract_name_with_trailing_whitespace_not_found() {
        let options = ContractOptions {
            command: Some("spawn ".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    // ── Contract detail display (all branches in run_contract) ───────────

    #[test]
    fn run_contract_spawn_shows_details() {
        // spawn has required_args, flags, side_effects, examples, reversible, undo
        let options = ContractOptions {
            command: Some("spawn".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_done_shows_details() {
        // done has optional_args, multiple flags, side_effects, examples, reversible, undo
        let options = ContractOptions {
            command: Some("done".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_revert_shows_details() {
        // revert has required_args, 1 flag, side_effects, examples, NOT reversible
        let options = ContractOptions {
            command: Some("revert".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    // ── Contract verification: each known contract resolves ──────────────

    #[test]
    fn run_contract_all_known_commands_resolve() {
        let contracts = known_contracts();
        for contract in &contracts {
            let options = ContractOptions {
                command: Some(contract.name.clone()),
            };
            assert!(
                run_contract(&options).is_ok(),
                "known contract '{}' should resolve but returned error",
                contract.name
            );
        }
    }

    // ── Multiple sequential calls (idempotency) ─────────────────────────

    #[test]
    fn run_contract_idempotent_list() {
        let options = ContractOptions { command: None };
        assert!(run_contract(&options).is_ok());
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_idempotent_specific() {
        let options = ContractOptions {
            command: Some("spawn".to_string()),
        };
        assert!(run_contract(&options).is_ok());
        assert!(run_contract(&options).is_ok());
    }

    // ── ContractOptions construction ─────────────────────────────────────

    #[test]
    fn contract_options_none_command() {
        let opts = ContractOptions { command: None };
        assert!(opts.command.is_none());
    }

    #[test]
    fn contract_options_some_command() {
        let opts = ContractOptions {
            command: Some("test".to_string()),
        };
        assert_eq!(opts.command.as_deref(), Some("test"));
    }
}
