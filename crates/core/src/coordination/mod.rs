//! # Coordination Layer
//!
//! This module implements the **coordination layer** for multi-agent session management.
//! It provides distributed coordination primitives for managing concurrent operations across
//! multiple agents and workspaces.
//!
//! ## Overview
//!
//! The coordination layer handles:
//! - **Conflict resolution** - Detecting and resolving merge conflicts
//! - **Locking** - Distributed locking for critical sections
//!
//! ## Module Structure
//!
//! ### Conflict Resolution
//!
//! **Conflict detection:**
//! - [`conflict_resolutions`] - Store and retrieve conflict resolutions
//! - [`conflict_resolutions_entities`] - Conflict resolution domain types
//! - [`ConflictResolution`] - Represents a resolved conflict
//!
//! **Implementation modules:**
//! - [`conflict_resolutions_schema`] - Schema initialization
//! - [`conflict_resolutions_insert`] - Insert operations
//! - [`conflict_resolutions_query`] - Query operations
//! - [`conflict_resolutions_error`] - Error conversion
//!
//! Conflict resolution enables:
//! - Automatic conflict detection during merges
//! - Storing resolution decisions for reuse
//! - Tracking who resolved what and when
//!
//! ### Locking
//!
//! **Distributed locks:**
//! - [`locks`] - Distributed lock manager with TTL and heartbeat
//! - [`LockManager`] - Acquire and release locks with TTL
//! - [`LockResponse`] - Lock acquisition response
//! - [`LockInfo`] - Lock metadata (owner, expiration)
//! - [`LockState`] - Current lock state for a session
//! - [`LockAuditEntry`] - Audit log entries
//! - [`LockOperation`] - Types of lock operations
//! - [`Ttl`] - TTL value object
//!
//! Locking ensures:
//! - Mutual exclusion for critical sections
//! - Automatic expiration on failure
//! - Safe cleanup on release
//! - Heartbeat mechanism to extend TTL
//!
//! ## Domain Types
//!
//! This module re-exports domain types from [`domain_types`]:
//! - [`AgentId`] - Agent identifier
//! - [`BeadId`] - Bead identifier
//! - [`WorkspaceName`] - Workspace name
//! - [`DomainError`] - Domain error type
//!
//! ## Related Modules
//!
//! - **`crate::domain`** - Core domain types and aggregates
//! - **`crate::output`** - Output types for coordination operations
//! - **`crate::beads`** - Beads issue tracker integration

pub mod conflict_resolutions;
pub mod conflict_resolutions_entities;
pub mod conflict_resolutions_schema;
pub mod conflict_resolutions_insert;
pub mod conflict_resolutions_queries;
pub mod conflict_resolutions_error_convert;
pub mod domain_types;
pub mod locks;

// Private implementation modules for conflict_resolutions
mod conflict_resolutions_query;

pub use conflict_resolutions::{
    get_conflict_resolutions, get_resolutions_by_decider, get_resolutions_by_time_range,
    init_conflict_resolutions_schema, insert_conflict_resolution,
};
pub use conflict_resolutions_entities::{ConflictResolution, ConflictResolutionError};
pub use domain_types::{AgentId, BeadId, DomainError, WorkspaceName};

// Lock types
pub use locks::{LockAuditEntry, LockInfo, LockManager, LockOperation, LockResponse, LockState, Ttl};
