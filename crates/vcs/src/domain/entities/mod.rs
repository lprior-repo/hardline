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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // -- Commit tests --

    #[test]
    fn commit_new_with_all_fields() {
        let now = Utc::now();
        let commit = Commit::new(
            "abc123".to_string(),
            "Initial commit".to_string(),
            "Alice <alice@example.com>".to_string(),
            now,
            vec!["parent1".to_string()],
        );
        assert_eq!(commit.id, "abc123");
        assert_eq!(commit.message, "Initial commit");
        assert_eq!(commit.author, "Alice <alice@example.com>");
        assert_eq!(commit.timestamp, now);
        assert_eq!(commit.parents, vec!["parent1"]);
    }

    #[test]
    fn commit_new_with_empty_parents() {
        let commit = Commit::new(
            "root".to_string(),
            "root commit".to_string(),
            "Bob".to_string(),
            Utc::now(),
            vec![],
        );
        assert!(commit.parents.is_empty());
    }

    #[test]
    fn commit_new_with_multiple_parents() {
        let commit = Commit::new(
            "merge".to_string(),
            "merge commit".to_string(),
            "Bob".to_string(),
            Utc::now(),
            vec!["p1".to_string(), "p2".to_string()],
        );
        assert_eq!(commit.parents.len(), 2);
    }

    #[test]
    fn commit_clone() {
        let commit = Commit::new(
            "id".to_string(), "msg".to_string(), "a".to_string(),
            Utc::now(), vec![],
        );
        let cloned = commit.clone();
        assert_eq!(commit.id, cloned.id);
        assert_eq!(commit.message, cloned.message);
    }

    #[test]
    fn commit_serde_roundtrip() {
        let commit = Commit::new(
            "sha123".to_string(),
            "test commit".to_string(),
            "Test <test@test.com>".to_string(),
            Utc::now(),
            vec!["parent".to_string()],
        );
        let json = serde_json::to_string(&commit).expect("serialize");
        let deserialized: Commit = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(commit.id, deserialized.id);
        assert_eq!(commit.message, deserialized.message);
        assert_eq!(commit.author, deserialized.author);
        assert_eq!(commit.parents, deserialized.parents);
    }

    // -- Branch tests --

    #[test]
    fn branch_new_with_tracking() {
        let branch = Branch::new("main".to_string(), true, Some("origin/main".to_string()));
        assert_eq!(branch.name, "main");
        assert!(branch.is_current);
        assert_eq!(branch.tracking, Some("origin/main".to_string()));
    }

    #[test]
    fn branch_new_without_tracking() {
        let branch = Branch::new("develop".to_string(), false, None);
        assert_eq!(branch.name, "develop");
        assert!(!branch.is_current);
        assert!(branch.tracking.is_none());
    }

    #[test]
    fn branch_clone() {
        let branch = Branch::new("feature".to_string(), false, Some("origin/feature".to_string()));
        let cloned = branch.clone();
        assert_eq!(branch.name, cloned.name);
        assert_eq!(branch.is_current, cloned.is_current);
    }

    #[test]
    fn branch_serde_roundtrip() {
        let branch = Branch::new("release".to_string(), true, Some("origin/release".to_string()));
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(branch.name, deserialized.name);
        assert_eq!(branch.is_current, deserialized.is_current);
        assert_eq!(branch.tracking, deserialized.tracking);
    }

    #[test]
    fn branch_serde_roundtrip_no_tracking() {
        let branch = Branch::new("hotfix".to_string(), false, None);
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: Branch = serde_json::from_str(&json).expect("deserialize");
        assert!(deserialized.tracking.is_none());
    }

    // -- Workspace tests --

    #[test]
    fn workspace_new_current() {
        let ws = Workspace::new("default".to_string(), "main".to_string(), true);
        assert_eq!(ws.name, "default");
        assert_eq!(ws.branch, "main");
        assert!(ws.is_current);
    }

    #[test]
    fn workspace_new_not_current() {
        let ws = Workspace::new("feature-ws".to_string(), "feature/x".to_string(), false);
        assert_eq!(ws.name, "feature-ws");
        assert!(!ws.is_current);
    }

    #[test]
    fn workspace_clone() {
        let ws = Workspace::new("ws1".to_string(), "main".to_string(), true);
        let cloned = ws.clone();
        assert_eq!(ws.name, cloned.name);
        assert_eq!(ws.branch, cloned.branch);
        assert_eq!(ws.is_current, cloned.is_current);
    }

    #[test]
    fn workspace_serde_roundtrip() {
        let ws = Workspace::new("ws2".to_string(), "develop".to_string(), false);
        let json = serde_json::to_string(&ws).expect("serialize");
        let deserialized: Workspace = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ws.name, deserialized.name);
        assert_eq!(ws.branch, deserialized.branch);
        assert_eq!(ws.is_current, deserialized.is_current);
    }
}
