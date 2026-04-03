//! CLI command handlers from hardline project
//!
//! This module contains handlers adapted from the hardline project.
//! They require significant adaptation to work with hardline's architecture.
//!
//! Hardline uses derive macros in main.rs for CLI definition, while hardline
//! uses a builder pattern with build_cli().

pub mod ai;
pub mod backup;
pub mod batch;
pub mod done;
pub mod json_format;
pub mod revert;
pub mod session;
pub mod sync;
pub mod task;
