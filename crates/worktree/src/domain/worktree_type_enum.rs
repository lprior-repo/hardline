use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Value object representing the type of worktree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WorktreeTypeEnum {
    /// Regular worktree for development
    Development = 0,

    /// Worktree for testing specific scenarios
    Testing = 1,

    /// Worktree for reviewing code
    Review = 2,

    /// Worktree for debugging production issues
    Debugging = 3,

    /// Worktree for documentation or research
    Research = 4,
}

impl WorktreeTypeEnum {
    /// Create a worktree type from a numeric value
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Development),
            1 => Some(Self::Testing),
            2 => Some(Self::Review),
            3 => Some(Self::Debugging),
            4 => Some(Self::Research),
            _ => None,
        }
    }

    /// Convert to numeric value
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Get a human-readable name for the type
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Development => "Development",
            Self::Testing => "Testing",
            Self::Review => "Review",
            Self::Debugging => "Debugging",
            Self::Research => "Research",
        }
    }

    /// Get a short code for the type
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Development => "dev",
            Self::Testing => "test",
            Self::Review => "review",
            Self::Debugging => "debug",
            Self::Research => "research",
        }
    }

    /// Check if this type is commonly used for development
    #[must_use]
    pub const fn is_development_focused(self) -> bool {
        matches!(self, Self::Development)
    }

    /// Check if this type is for quality assurance
    #[must_use]
    pub const fn is_qa_focused(self) -> bool {
        matches!(self, Self::Testing | Self::Review)
    }

    /// Check if this type is for troubleshooting
    #[must_use]
    pub const fn is_troubleshooting_focused(self) -> bool {
        matches!(self, Self::Debugging | Self::Research)
    }
}

impl Display for WorktreeTypeEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.name(), f)
    }
}

impl From<WorktreeTypeEnum> for u8 {
    fn from(type_enum: WorktreeTypeEnum) -> Self {
        type_enum.as_u8()
    }
}

impl TryFrom<u8> for WorktreeTypeEnum {
    type Error = super::WorktreeDomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or_else(|| {
            super::WorktreeDomainError::InvalidPath(format!("Invalid worktree type: {value}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_type_enum_from_u8_returns_correct_type() {
        assert_eq!(
            WorktreeTypeEnum::from_u8(0),
            Some(WorktreeTypeEnum::Development)
        );
        assert_eq!(WorktreeTypeEnum::from_u8(2), Some(WorktreeTypeEnum::Review));
        assert_eq!(WorktreeTypeEnum::from_u8(99), None);
    }

    #[test]
    fn worktree_type_enum_as_u8_returns_correct_value() {
        assert_eq!(WorktreeTypeEnum::Development.as_u8(), 0);
        assert_eq!(WorktreeTypeEnum::Research.as_u8(), 4);
    }

    #[test]
    fn worktree_type_enum_name_returns_human_readable_name() {
        assert_eq!(WorktreeTypeEnum::Development.name(), "Development");
        assert_eq!(WorktreeTypeEnum::Research.name(), "Research");
    }

    #[test]
    fn worktree_type_enum_code_returns_short_code() {
        assert_eq!(WorktreeTypeEnum::Development.code(), "dev");
        assert_eq!(WorktreeTypeEnum::Review.code(), "review");
    }

    #[test]
    fn worktree_type_enum_is_development_focused_returns_true_for_development() {
        assert!(WorktreeTypeEnum::Development.is_development_focused());
        assert!(!WorktreeTypeEnum::Testing.is_development_focused());
    }

    #[test]
    fn worktree_type_enum_is_qa_focused_returns_true_for_testing_and_review() {
        assert!(WorktreeTypeEnum::Testing.is_qa_focused());
        assert!(WorktreeTypeEnum::Review.is_qa_focused());
        assert!(!WorktreeTypeEnum::Development.is_qa_focused());
    }

    #[test]
    fn worktree_type_enum_is_troubleshooting_focused_returns_true_for_debugging() {
        assert!(WorktreeTypeEnum::Debugging.is_troubleshooting_focused());
        assert!(WorktreeTypeEnum::Research.is_troubleshooting_focused());
        assert!(!WorktreeTypeEnum::Development.is_troubleshooting_focused());
    }

    #[test]
    fn worktree_type_enum_try_from_u8_returns_correct_type() {
        let type_enum: WorktreeTypeEnum = WorktreeTypeEnum::Development.as_u8().try_into().unwrap();
        assert_eq!(type_enum, WorktreeTypeEnum::Development);
    }
}
