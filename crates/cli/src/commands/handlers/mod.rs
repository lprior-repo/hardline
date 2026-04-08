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
pub mod branch;
pub mod bookmark;
pub mod can_i;
pub mod checkpoint;
pub mod clean;
pub mod completions;
pub mod config_ports;
pub mod contract;
pub mod done;
pub mod events;
pub mod examples;
pub mod export_import;
pub mod integrity;
pub mod introspect;
pub mod json_format;
pub mod lock;
pub mod prune;
pub mod query;
pub mod recover;
pub mod rename;
pub mod revert;
pub mod schema;
pub mod session;
pub mod stack_sync;
pub mod sync;
pub mod task;
pub mod undo;
pub mod validate;
pub mod wait;
pub mod whatif;
pub mod whoami;
pub mod work;
