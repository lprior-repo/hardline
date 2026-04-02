#![allow(clippy::module_inception)]
pub mod storage {
    use crate::domain::snapshot::{Snapshot, SnapshotId};
    use crate::error::{Result, SnapshotError};

    pub struct SnapshotStore;

    impl Default for SnapshotStore {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SnapshotStore {
        pub fn new() -> Self {
            Self
        }

        pub fn save(&self, _snapshot: Snapshot) -> Result<()> {
            Err(SnapshotError::NotFound(
                "Storage not yet implemented".to_string(),
            ))
        }

        pub fn load(&self, _id: &SnapshotId) -> Result<Snapshot> {
            Err(SnapshotError::NotFound(
                "Storage not yet implemented".to_string(),
            ))
        }

        pub fn list(&self) -> Result<Vec<Snapshot>> {
            Err(SnapshotError::NotFound(
                "Storage not yet implemented".to_string(),
            ))
        }

        pub fn delete(&self, _id: &SnapshotId) -> Result<()> {
            Err(SnapshotError::NotFound(
                "Storage not yet implemented".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::storage::SnapshotStore;
    use crate::domain::snapshot::{Snapshot, SnapshotId};
    use crate::error::SnapshotError;
    use proptest::proptest;
    use proptest::prop_assert;

    fn make_store() -> SnapshotStore {
        SnapshotStore::new()
    }

    #[test]
    fn store_new_creates_instance() {
        let _store = make_store();
    }

    #[test]
    fn store_default_creates_instance() {
        let _store = SnapshotStore::default();
    }

    #[test]
    fn store_save_returns_err_not_implemented() {
        let store = make_store();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let result = store.save(snapshot);
        assert!(result.is_err());
        let msg = result.expect_err("should be Err").to_string();
        assert!(msg.contains("not yet implemented"), "unexpected message: {msg}");
    }

    #[test]
    fn store_load_returns_err_not_implemented() {
        let store = make_store();
        let id = SnapshotId::generate();
        let result = store.load(&id);
        assert!(result.is_err());
        let msg = result.expect_err("should be Err").to_string();
        assert!(msg.contains("not yet implemented"), "unexpected message: {msg}");
    }

    #[test]
    fn store_list_returns_err_not_implemented() {
        let store = make_store();
        let result = store.list();
        assert!(result.is_err());
        let msg = result.expect_err("should be Err").to_string();
        assert!(msg.contains("not yet implemented"), "unexpected message: {msg}");
    }

    #[test]
    fn store_delete_returns_err_not_implemented() {
        let store = make_store();
        let id = SnapshotId::generate();
        let result = store.delete(&id);
        assert!(result.is_err());
        let msg = result.expect_err("should be Err").to_string();
        assert!(msg.contains("not yet implemented"), "unexpected message: {msg}");
    }

    #[test]
    fn store_all_errors_are_snapshot_error_not_found() {
        let store = make_store();
        let id = SnapshotId::generate();
        let snapshot = Snapshot::create("a".to_string(), "b".to_string(), None);

        let save_err = store.save(snapshot).expect_err("should be Err");
        let load_err = store.load(&id).expect_err("should be Err");
        let list_err = store.list().expect_err("should be Err");
        let delete_err = store.delete(&id).expect_err("should be Err");

        assert!(matches!(save_err, SnapshotError::NotFound(_)));
        assert!(matches!(load_err, SnapshotError::NotFound(_)));
        assert!(matches!(list_err, SnapshotError::NotFound(_)));
        assert!(matches!(delete_err, SnapshotError::NotFound(_)));
    }

    #[test]
    fn store_save_err_message_contains_prefix() {
        let store = make_store();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let err = store.save(snapshot).expect_err("should be Err");
        assert!(err.to_string().starts_with("Snapshot not found:"));
    }

    #[test]
    fn store_load_err_message_contains_prefix() {
        let store = make_store();
        let id = SnapshotId::generate();
        let err = store.load(&id).expect_err("should be Err");
        assert!(err.to_string().starts_with("Snapshot not found:"));
    }

    #[test]
    fn store_list_err_message_contains_prefix() {
        let store = make_store();
        let err = store.list().expect_err("should be Err");
        assert!(err.to_string().starts_with("Snapshot not found:"));
    }

    #[test]
    fn store_delete_err_message_contains_prefix() {
        let store = make_store();
        let id = SnapshotId::generate();
        let err = store.delete(&id).expect_err("should be Err");
        assert!(err.to_string().starts_with("Snapshot not found:"));
    }

    #[test]
    fn store_save_err_is_debug() {
        let store = make_store();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let err = store.save(snapshot).expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn store_load_err_is_debug() {
        let store = make_store();
        let id = SnapshotId::generate();
        let err = store.load(&id).expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn store_list_err_is_debug() {
        let store = make_store();
        let err = store.list().expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn store_delete_err_is_debug() {
        let store = make_store();
        let id = SnapshotId::generate();
        let err = store.delete(&id).expect_err("should be Err");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn store_save_different_snapshots_all_fail() {
        let store = make_store();
        let s1 = Snapshot::create("a".to_string(), "h1".to_string(), None);
        let s2 = Snapshot::create("b".to_string(), "h2".to_string(), Some("desc".to_string()));
        let s3 = Snapshot::create("c".to_string(), "h3".to_string(), Some(String::new()));
        assert!(store.save(s1).is_err());
        assert!(store.save(s2).is_err());
        assert!(store.save(s3).is_err());
    }

    #[test]
    fn store_load_different_ids_all_fail() {
        let store = make_store();
        let ids = [
            SnapshotId::generate(),
            SnapshotId::generate(),
            SnapshotId::generate(),
            SnapshotId::generate(),
        ];
        for id in &ids {
            assert!(store.load(id).is_err());
        }
    }

    #[test]
    fn store_delete_different_ids_all_fail() {
        let store = make_store();
        let ids = [
            SnapshotId::generate(),
            SnapshotId::generate(),
            SnapshotId::generate(),
        ];
        for id in &ids {
            assert!(store.delete(id).is_err());
        }
    }

    #[test]
    fn store_multiple_instances_all_fail() {
        let store1 = SnapshotStore::new();
        let store2 = SnapshotStore::default();
        let store3 = SnapshotStore::new();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        assert!(store1.save(snapshot.clone()).is_err());
        assert!(store2.save(snapshot.clone()).is_err());
        assert!(store3.save(snapshot).is_err());
    }

    #[test]
    fn store_operations_return_unit_on_ok_type() {
        // The save and delete return Result<()>, so on Err they are Err(SnapshotError)
        let store = make_store();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let id = SnapshotId::generate();

        // Verify the Err variant carries SnapshotError (not any other error type)
        let save_result: crate::error::Result<()> = store.save(snapshot);
        let delete_result: crate::error::Result<()> = store.delete(&id);
        assert!(save_result.is_err());
        assert!(delete_result.is_err());
    }

    #[test]
    fn store_load_returns_snapshot_type_on_ok() {
        // The return type is Result<Snapshot>, verify the type
        let store = make_store();
        let id = SnapshotId::generate();
        let result: crate::error::Result<Snapshot> = store.load(&id);
        assert!(result.is_err());
    }

    #[test]
    fn store_list_returns_vec_snapshot_type_on_ok() {
        // The return type is Result<Vec<Snapshot>>, verify the type
        let store = make_store();
        let result: crate::error::Result<Vec<Snapshot>> = store.list();
        assert!(result.is_err());
    }

    #[test]
    fn store_save_with_snapshot_containing_special_chars() {
        let store = make_store();
        let snapshot = Snapshot::create(
            "feature/日本語 🎉".to_string(),
            "abc".to_string(),
            Some("description with\nnewlines and\ttabs".to_string()),
        );
        assert!(store.save(snapshot).is_err());
    }

    #[test]
    fn store_err_implements_error_trait() {
        let store = make_store();
        let snapshot = Snapshot::create("main".to_string(), "abc".to_string(), None);
        let err = store.save(snapshot).expect_err("should be Err");
        let _: Box<dyn std::error::Error> = Box::new(err);
    }

    // --- Proptests ---

    proptest! {
        #[test]
        fn store_save_always_fails(branch in "[a-zA-Z0-9_-]{1,50}", commit in "[a-f0-9]{1,40}") {
            let store = make_store();
            let snapshot = Snapshot::create(branch, commit, None);
            prop_assert!(store.save(snapshot).is_err());
        }

        #[test]
        fn store_load_always_fails_for_generated_ids(_v in 0..100u32) {
            let store = make_store();
            let id = SnapshotId::generate();
            prop_assert!(store.load(&id).is_err());
        }

        #[test]
        fn store_delete_always_fails_for_generated_ids(_v in 0..100u32) {
            let store = make_store();
            let id = SnapshotId::generate();
            prop_assert!(store.delete(&id).is_err());
        }

        #[test]
        fn store_list_always_fails(_v in 0..10u32) {
            let store = make_store();
            prop_assert!(store.list().is_err());
        }

        #[test]
        fn store_save_error_always_not_found(branch in "[a-zA-Z0-9_-]{1,50}", commit in "[a-f0-9]{1,40}") {
            let store = make_store();
            let snapshot = Snapshot::create(branch, commit, None);
            let err = store.save(snapshot).expect_err("should be Err");
            prop_assert!(matches!(err, SnapshotError::NotFound(_)));
        }
    }
}
