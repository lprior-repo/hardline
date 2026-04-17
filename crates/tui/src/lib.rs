#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::type_complexity,
    clippy::result_large_err
)]
#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod app;
pub mod error;
pub mod input;
pub mod views;
pub mod widgets;

pub use app::{BranchProvider, TuiApp};
pub use error::{Result, TuiError};
