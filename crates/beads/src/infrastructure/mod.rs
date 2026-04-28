//! Infrastructure layer — repository implementations.

pub mod repository;

// Re-export both the trait (from domain) and implementation
pub use repository::InMemoryBeadRepository;

pub use crate::domain::BeadRepository;
