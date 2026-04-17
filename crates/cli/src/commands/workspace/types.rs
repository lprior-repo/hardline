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
    use proptest::prelude::*;

    // ---- Construction ----

    #[test]
    fn from_bool_true_yields_with_sync() {
        assert_eq!(SyncOption::from_bool(true), SyncOption::WithSync);
    }

    #[test]
    fn from_bool_false_yields_no_sync() {
        assert_eq!(SyncOption::from_bool(false), SyncOption::NoSync);
    }

    #[test]
    fn variants_construct_directly() {
        let _no = SyncOption::NoSync;
        let _with = SyncOption::WithSync;
    }

    // ---- is_sync accessor ----

    #[test]
    fn is_sync_true_for_with_sync() {
        assert!(SyncOption::WithSync.is_sync());
    }

    #[test]
    fn is_sync_false_for_no_sync() {
        assert!(!SyncOption::NoSync.is_sync());
    }

    // ---- Equality & inequality ----

    #[test]
    fn equality_reflexive() {
        assert_eq!(SyncOption::NoSync, SyncOption::NoSync);
        assert_eq!(SyncOption::WithSync, SyncOption::WithSync);
    }

    #[test]
    fn inequality_between_variants() {
        assert_ne!(SyncOption::NoSync, SyncOption::WithSync);
        assert_ne!(SyncOption::WithSync, SyncOption::NoSync);
    }

    // ---- Clone & Copy ----

    #[test]
    fn clone_produces_equal_value() {
        let a = SyncOption::WithSync;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn copy_semantics() {
        let a = SyncOption::NoSync;
        let b = a; // Copy, not move
        assert_eq!(a, b); // a still usable
    }

    // ---- Debug formatting ----

    #[test]
    fn debug_format_no_sync() {
        let dbg = format!("{:?}", SyncOption::NoSync);
        assert_eq!(dbg, "NoSync");
    }

    #[test]
    fn debug_format_with_sync() {
        let dbg = format!("{:?}", SyncOption::WithSync);
        assert_eq!(dbg, "WithSync");
    }

    // ---- Roundtrip: from_bool → is_sync ----

    #[test]
    fn from_bool_roundtrip_true() {
        assert!(SyncOption::from_bool(true).is_sync());
    }

    #[test]
    fn from_bool_roundtrip_false() {
        assert!(!SyncOption::from_bool(false).is_sync());
    }

    // ---- Proptests ----

    proptest! {
        #[test]
        fn proptest_from_bool_inverts_is_sync(b: bool) {
            assert_eq!(SyncOption::from_bool(b).is_sync(), b);
        }

        #[test]
        fn proptest_from_bool_roundtrip(b: bool) {
            let opt = SyncOption::from_bool(b);
            prop_assert_eq!(opt, if b { SyncOption::WithSync } else { SyncOption::NoSync });
        }

        #[test]
        fn proptest_clone_eq(opt in proptest::sample::select(vec![SyncOption::NoSync, SyncOption::WithSync])) {
            prop_assert_eq!(opt, opt.clone());
        }

        #[test]
        fn proptest_copy_eq(opt in proptest::sample::select(vec![SyncOption::NoSync, SyncOption::WithSync])) {
            let other = opt;
            prop_assert_eq!(opt, other);
        }

        #[test]
        fn proptest_debug_does_not_crash(opt in proptest::sample::select(vec![SyncOption::NoSync, SyncOption::WithSync])) {
            let _ = format!("{:?}", opt);
        }

        #[test]
        fn proptest_is_sync_consistent(opt in proptest::sample::select(vec![SyncOption::NoSync, SyncOption::WithSync])) {
            prop_assert_eq!(opt.is_sync(), opt == SyncOption::WithSync);
        }
    }
}
