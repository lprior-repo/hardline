use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandStatus {
    Pending,
    WaitingForCi,
    Merging,
    Merged,
    Failed(String),
}

impl LandStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, LandStatus::Merged | LandStatus::Failed(_))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, LandStatus::Pending)
    }

    pub fn is_waiting(&self) -> bool {
        matches!(self, LandStatus::WaitingForCi)
    }

    pub fn is_merging(&self) -> bool {
        matches!(self, LandStatus::Merging)
    }

    pub fn is_merged(&self) -> bool {
        matches!(self, LandStatus::Merged)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, LandStatus::Failed(_))
    }

    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            LandStatus::Failed(reason) => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_land_status_pending() {
        let status = LandStatus::Pending;
        assert!(status.is_pending());
        assert!(!status.is_terminal());
        assert!(!status.is_failed());
    }

    #[test]
    fn test_land_status_waiting_for_ci() {
        let status = LandStatus::WaitingForCi;
        assert!(status.is_waiting());
        assert!(!status.is_terminal());
    }

    #[test]
    fn test_land_status_merging() {
        let status = LandStatus::Merging;
        assert!(status.is_merging());
        assert!(!status.is_terminal());
    }

    #[test]
    fn test_land_status_merged() {
        let status = LandStatus::Merged;
        assert!(status.is_merged());
        assert!(status.is_terminal());
        assert!(status.failure_reason().is_none());
    }

    #[test]
    fn test_land_status_failed() {
        let status = LandStatus::Failed("timeout".to_string());
        assert!(status.is_failed());
        assert!(status.is_terminal());
        assert_eq!(status.failure_reason(), Some("timeout"));
    }

    #[test]
    fn test_land_status_equality() {
        assert_eq!(LandStatus::Pending, LandStatus::Pending);
        assert_eq!(LandStatus::Merged, LandStatus::Merged);
        assert_eq!(
            LandStatus::Failed("same".to_string()),
            LandStatus::Failed("same".to_string())
        );
        assert_ne!(
            LandStatus::Failed("a".to_string()),
            LandStatus::Failed("b".to_string())
        );
        assert_ne!(LandStatus::Pending, LandStatus::Merged);
    }

    #[test]
    fn test_land_status_clone() {
        let original = LandStatus::Failed("test".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_land_status_serde_roundtrip() {
        let statuses = [
            LandStatus::Pending,
            LandStatus::WaitingForCi,
            LandStatus::Merging,
            LandStatus::Merged,
            LandStatus::Failed("test error".to_string()),
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).expect("serialize");
            let deserialized: LandStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*status, deserialized);
        }
    }
}
