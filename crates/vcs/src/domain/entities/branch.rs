//! VCS Domain Entities — Branch

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let branch = Branch::new(
            "feature".to_string(),
            false,
            Some("origin/feature".to_string()),
        );
        let cloned = branch.clone();
        assert_eq!(branch.name, cloned.name);
        assert_eq!(branch.is_current, cloned.is_current);
    }

    #[test]
    fn branch_serde_roundtrip() {
        let branch = Branch::new(
            "release".to_string(),
            true,
            Some("origin/release".to_string()),
        );
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
}