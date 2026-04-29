//! Security and access control module (ADR-015).
//!
//! Provides authentication, authorization, audit logging, and rate limiting
//! for the Source Control Plane.

pub mod audit;
pub mod auth;
pub mod rate_limiter;

// Re-exports
pub use audit::{AuditEntry, AuditFilter, AuditLogger, AuditOutcome, AuditOutcomeFilter};
pub use auth::{AgentToken, AuthContext, Scope};
pub use rate_limiter::RateLimiter;
