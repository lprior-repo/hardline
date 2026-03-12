#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod views;
pub mod input;
pub mod app;
pub mod error;

pub use app::TuiApp;
pub use error::{TuiError, Result};
