#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! # Twins - Declarative Twin Runtime Engine
//!
//! A runtime engine for creating declarative HTTP service twins that can be used
//! for testing, mocking, and development purposes.
//!
//! ## Architecture
//!
//! - **Definition**: Parse twin definitions from YAML
//! - **State**: In-memory state management for request/response tracking
//! - **Server**: HTTP server using axum

pub mod definition;
pub mod server;
pub mod state;

pub use definition::{Endpoint, EndpointResponse, TwinDefinition};
pub use state::{InMemoryTwinState, RequestRecord, TwinState};
