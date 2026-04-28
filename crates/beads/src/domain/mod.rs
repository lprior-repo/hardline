//! Domain layer: entities, value objects, events, and repository contracts.

pub mod entities;
pub mod events;
pub mod repository;
pub mod value_objects;

pub use entities::bead::Bead;
pub use events::BeadEvent;
pub use repository::BeadRepository;
pub use value_objects::{
    BeadDescription, BeadId, BeadState, BeadTitle, BeadType, Labels, Priority,
};
