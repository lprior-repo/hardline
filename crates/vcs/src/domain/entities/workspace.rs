//! VCS Domain Entities — Workspace

use serde::{Deserialize, Serialize};

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
