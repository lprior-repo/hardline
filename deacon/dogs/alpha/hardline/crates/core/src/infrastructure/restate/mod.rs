//! Restate SDK Integration
//!
//! Provides durable execution primitives for long-running workflows and services.
//! Based on ADR-003: Restate Feature Parity Analysis.
//!
//! ## Core Concepts
//!
//! - **Journal**: Append-only log of all operations for crash recovery
//! - **State Store**: Key-value state per entity
//! - **Invocation State**: Track running/completed/failed states
//!
//! ## Context Types
//!
//! | Type | Context | State | Promises | Use Case |
//! |------|---------|-------|----------|----------|
//! | Service | `Context<'_>` | None | No | Stateless handlers |
//! | Virtual Object | `ObjectContext<'_>` | Per-key K/V | No | Stateful entities |
//! | Workflow | `WorkflowContext<'_>` | Per-instance | Yes | Long-running with waits |

pub mod clients;
pub mod context;
pub mod errors;
pub mod promises;

pub use clients::{ContextClient, ObjectClient, ServiceClient, WorkflowClient};
pub use context::{
    ContextSideEffects, ContextTimers, DurableContext, DurableFuture, RunClosure, RunFuture,
};
pub use errors::{HandlerError, HandlerResult, TerminalError};
pub use promises::{AwakeableId, ContextAwakeables, ContextPromises, Promise, PromiseResolver};
