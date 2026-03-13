// Re-export QueueRepository from domain layer (the canonical location)
// InMemoryQueueRepository is also available from domain::ports
pub use crate::domain::QueueRepository;
