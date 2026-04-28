//! Contract command handler - Show JSON Schema contracts for commands.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): ContractOptions, CommandContract, ArgContract, FlagContract (inert,
//!   serializable)
//! - **Actions** (`actions.rs`): run_contract, list_contracts (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp contract                   # Show all command contracts
//! scp contract spawn             # Show contract for spawn command
//! scp contract --json            # Output as JSON
//! ```

pub mod actions;
pub mod data;

pub use actions::run_contract;
pub use data::{CommandContract, ContractOptions};
