//! Semantic version for capability versioning.
//!
//! Follows semver.org format: major.minor.patch

use serde::{Deserialize, Serialize};

use super::{AgentRegistryError, WorkspaceId};

/// Semantic version for capability versioning.
///
/// Follows semver.org format: major.minor.patch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemanticVersion {
    /// Create a new semantic version
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse from a string in format "major.minor.patch"
    pub fn parse(s: &str) -> Result<Self, AgentRegistryError> {
        let parts: Vec<u64> = s
            .split('.')
            .map(|p| {
                p.parse()
                    .map_err(|_| AgentRegistryError::InvalidCapability(s.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        match parts.as_slice() {
            [major, minor, patch] => Ok(Self::new(*major, *minor, *patch)),
            _ => Err(AgentRegistryError::InvalidCapability(s.to_string())),
        }
    }

    /// Convert to string format "major.minor.patch"
    #[must_use]
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
