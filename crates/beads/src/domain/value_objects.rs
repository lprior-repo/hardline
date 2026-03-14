// Re-export all types for backward compatibility
// The actual implementations are in dedicated modules:
// - identifiers.rs: BeadId, AgentId
// - text_values.rs: BeadTitle, BeadDescription
// - bead_state.rs: BeadState, Priority, BeadType
// - labels.rs: Labels

pub use crate::domain::identifiers::{AgentId, BeadId};
pub use crate::domain::text_values::{BeadDescription, BeadTitle};
pub use crate::domain::bead_state::{BeadState, BeadType, Priority};
pub use crate::domain::labels::Labels;
