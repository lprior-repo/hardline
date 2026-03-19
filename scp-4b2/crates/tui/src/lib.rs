#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod app;
pub mod error;
pub mod input;
pub mod views;

pub use app::TuiApp;
pub use error::{Result, TuiError};
