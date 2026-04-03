//! Validate command handler - Pre-validate inputs before execution.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data** (`data.rs`): ValidateOptions, ValidateOutput, ArgValidation
//!   (inert, serializable types + pure computation)
//! - **Actions** (`actions.rs`): run_validate (I/O operations)
//!
//! # CLI Usage
//!
//! ```text
//! scp validate spawn feature-auth         # Validate spawn args
//! scp validate done feature-auth          # Validate done args
//! scp validate add --name feature-auth    # Validate add args
//! ```

pub mod actions;
pub mod data;

pub use actions::run_validate;
pub use data::{ValidateOptions, ValidateOutput};
