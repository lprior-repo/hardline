//! VCS Domain Entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

impl Commit {
    pub fn new(
        id: String,
        message: String,
        author: String,
        timestamp: DateTime<Utc>,
        parents: Vec<String>,
    ) -> Self {
        Self {
            id,
            message,
            author,
            timestamp,
            parents,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}

impl Branch {
    pub fn new(name: String, is_current: bool, tracking: Option<String>) -> Self {
        Self {
            name,
            is_current,
            tracking,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub branch: String,
    pub is_current: bool,
}

impl Workspace {
    pub fn new(name: String, branch: String, is_current: bool) -> Self {
        Self {
            name,
            branch,
            is_current,
        }
    }
}
