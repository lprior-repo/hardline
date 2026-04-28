//! Completions command handler - Generate shell completion scripts.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): CompletionsOptions, CompletionsOutput, Shell (inert, serializable)
//! - **Actions** (`actions.rs`): run_completions, generate_completions_output (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp completions bash          # Generate bash completions
//! scp completions zsh           # Generate zsh completions
//! scp completions fish          # Generate fish completions
//! scp completions powershell    # Generate PowerShell completions
//! scp completions elvish        # Generate Elvish completions
//! ```

pub mod actions;
pub mod data;

pub use actions::run_completions;
pub use data::{
    install_instructions, supported_shells, CompletionsOptions, CompletionsOutput, Shell,
};
