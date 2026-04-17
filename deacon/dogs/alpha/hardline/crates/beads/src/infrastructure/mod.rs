pub mod repository;

// Re-export both the trait (from domain) and implementation
pub use crate::domain::BeadRepository;
pub use repository::InMemoryBeadRepository;
