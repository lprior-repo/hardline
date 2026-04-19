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
                    Error::not_found(format!("No contract found for command '{command_name}'"))
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
                Output::info(&format!(
                    "  Side effects: {}",
                    contract.side_effects.join(", ")
                ));
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
                Output::info(&format!("  {} - {}", contract.name, contract.description));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_contract_list_all() {
        let options = ContractOptions { command: None };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_specific() {
        let options = ContractOptions {
            command: Some("spawn".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_unknown_command() {
        let options = ContractOptions {
            command: Some("nonexistent".to_string()),
        };
        assert!(run_contract(&options).is_err());
    }

    #[test]
    fn run_contract_revert() {
        let options = ContractOptions {
            command: Some("revert".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }

    #[test]
    fn run_contract_done() {
        let options = ContractOptions {
            command: Some("done".to_string()),
        };
        assert!(run_contract(&options).is_ok());
    }
}
