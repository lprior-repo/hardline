//! Bead type enumeration and Priority level.
//!
//! These define the classification and urgency of beads.

use serde::{Deserialize, Serialize};

use crate::error::SessionError;

/// Bead type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
}

impl BeadType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Task => "task",
            Self::Epic => "epic",
            Self::Chore => "chore",
        }
    }
}

impl Default for BeadType {
    fn default() -> Self {
        Self::Task
    }
}

/// Priority level (0-4)
///
/// - 0: Critical (security, data loss, broken builds)
/// - 1: High (major features, important bugs)
/// - 2: Medium (default, nice-to-have)
/// - 3: Low (polish, optimization)
/// - 4: Backlog (future ideas)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub fn new(priority: u8) -> Result<Self, SessionError> {
        if priority > 4 {
            return Err(SessionError::InvalidPriority(format!(
                "Priority must be 0-4, got {}",
                priority
            )));
        }
        Ok(Self(priority))
    }

    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(2) // Medium priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_type_default_is_task() {
        assert_eq!(BeadType::default(), BeadType::Task);
    }

    #[test]
    fn bead_type_as_str() {
        assert_eq!(BeadType::Bug.as_str(), "bug");
        assert_eq!(BeadType::Feature.as_str(), "feature");
        assert_eq!(BeadType::Task.as_str(), "task");
        assert_eq!(BeadType::Epic.as_str(), "epic");
        assert_eq!(BeadType::Chore.as_str(), "chore");
    }

    #[test]
    fn priority_valid() {
        let p = Priority::new(2).unwrap();
        assert_eq!(p.as_u8(), 2);
    }

    #[test]
    fn priority_default_is_medium() {
        assert_eq!(Priority::default().as_u8(), 2);
    }

    #[test]
    fn priority_out_of_range_fails() {
        assert!(Priority::new(5).is_err());
        assert!(Priority::new(255).is_err());
    }
}
