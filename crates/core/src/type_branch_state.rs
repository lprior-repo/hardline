//! Branch state representation
//!
//! Can be either detached (no branch) or on a named branch.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BranchState {
    Detached,
    OnBranch(String),
}

impl BranchState {
    pub fn detached() -> Self {
        Self::Detached
    }

    pub fn on_branch(branch: impl Into<String>) -> Self {
        Self::OnBranch(branch.into())
    }

    #[must_use]
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            Self::Detached => None,
            Self::OnBranch(name) => Some(name),
        }
    }

    #[must_use]
    pub fn is_detached(&self) -> bool {
        matches!(self, Self::Detached)
    }
}

impl Serialize for BranchState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Detached => serializer.serialize_str("detached"),
            Self::OnBranch(name) => serializer.serialize_str(name),
        }
    }
}

impl<'de> Deserialize<'de> for BranchState {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "detached" {
            Ok(Self::Detached)
        } else {
            Ok(Self::OnBranch(s))
        }
    }
}
