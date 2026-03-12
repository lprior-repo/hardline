#![deny(warnings)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use application::BeadService;
pub use domain::{Bead, BeadEvent, BeadId, BeadState, BeadTitle, BeadType, Labels, Priority};
pub use error::{BeadError, Result};
pub use infrastructure::{BeadRepository, InMemoryBeadRepository};
