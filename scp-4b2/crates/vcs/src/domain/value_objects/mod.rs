//! VCS Value Objects

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VcsStatus {
    Clean,
    Dirty,
    Conflicted,
    Detached,
}

impl std::fmt::Display for VcsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "clean"),
            Self::Dirty => write!(f, "dirty"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Detached => write!(f, "detached"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Jujutsu,
    Git,
}

impl VcsType {
    pub fn detect(path: &std::path::Path) -> Option<Self> {
        if path.join(".jj").exists() {
            Some(Self::Jujutsu)
        } else if path.join(".git").exists() {
            Some(Self::Git)
        } else {
            None
        }
    }
}
