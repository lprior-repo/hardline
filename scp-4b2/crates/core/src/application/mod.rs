//! Core Application Layer - Use cases and orchestration
//!
//! This module contains:
//! - Use cases that orchestrate domain operations
//! - CLI command handlers
//! - Coordination logic

pub mod coordination;
pub mod queue_service;

pub use coordination::{create_coordination_service, CoordinationService, CoordinationServiceImpl};
pub use queue_service::{create_queue_service, QueueService, QueueServiceImpl};
