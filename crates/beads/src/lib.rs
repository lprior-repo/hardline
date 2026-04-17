//! `scp-beads` — Domain-driven issue tracking with typestate-enforced state machines.
//!
//! This crate provides the core domain model for beads (issues/work items),
//! including value objects, state transitions, event sourcing, and repository
//! abstractions.
//!
//! # Architecture
//!
//! - **Domain layer** (`domain`): Entities, value objects, events, and repository trait
//! - **Application layer** (`application`): `BeadService` orchestrating domain operations
//! - **Infrastructure layer** (`infrastructure`): `InMemoryBeadRepository` implementation
//! - **Error types** (`error`): `BeadError` enum and `Result` alias
//!
//! # Quick Start
//!
//! ```ignore
//! use scp_beads::{BeadService, InMemoryBeadRepository, BeadId, Priority, BeadState};
//!
//! let repo = InMemoryBeadRepository::new();
//! let service = BeadService::new(repo);
//!
//! // Create a bead
//! let (bead, event) = tokio::runtime::Runtime::new().unwrap().block_on(async {
//!     service.create_bead("my-issue", "Fix the bug", Some("Description".into())).await.unwrap()
//! });
//! ```

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
