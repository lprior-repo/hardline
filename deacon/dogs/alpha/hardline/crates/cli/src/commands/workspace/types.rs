//! Workspace types and enums

/// Sync option for spawn command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOption {
    /// Do not sync with main
    NoSync,
    /// Sync with main after spawning
    WithSync,
}

impl SyncOption {
    /// Convert bool to SyncOption
    #[must_use]
    pub fn from_bool(sync: bool) -> Self {
        if sync {
            SyncOption::WithSync
        } else {
            SyncOption::NoSync
        }
    }

    /// Returns true if sync is enabled
    #[must_use]
    pub const fn is_sync(&self) -> bool {
        matches!(self, SyncOption::WithSync)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_option_from_bool_true() {
        assert_eq!(SyncOption::from_bool(true), SyncOption::WithSync);
    }

    #[test]
    fn test_sync_option_from_bool_false() {
        assert_eq!(SyncOption::from_bool(false), SyncOption::NoSync);
    }

    #[test]
    fn test_sync_option_is_sync_with_sync() {
        assert!(SyncOption::WithSync.is_sync());
    }

    #[test]
    fn test_sync_option_is_sync_without_sync() {
        assert!(!SyncOption::NoSync.is_sync());
    }
}
