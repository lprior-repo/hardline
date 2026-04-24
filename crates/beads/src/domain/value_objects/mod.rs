//! Value objects for the bead domain.
//!
//! Immutable, self-validating types that represent domain concepts:
//! [`BeadId`], [`BeadTitle`], [`BeadDescription`], [`BeadState`],
//! [`Priority`], [`BeadType`], and [`Labels`].

mod bead_description;
mod bead_id;
mod bead_state;
mod bead_title;
mod bead_type;
mod labels;
mod priority;

pub use bead_description::BeadDescription;
pub use bead_id::BeadId;
pub use bead_state::BeadState;
pub use bead_title::BeadTitle;
pub use bead_type::BeadType;
pub use labels::Labels;
pub use priority::Priority;
