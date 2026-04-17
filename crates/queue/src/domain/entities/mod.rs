pub mod queue_entry;

// Re-export QueueEntryId from identifiers so existing import paths still work.
// The canonical location is identifiers; this re-export avoids breaking entities consumers.
pub use crate::domain::identifiers::QueueEntryId;
pub use queue_entry::{QueueEntry, QueueStatus};
