use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Snapshot corrupt: {0}")]
    Corrupt(String),

    #[error("Storage error: {message}")]
    StorageError {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        message: String,
    },

    #[error("Git error: {message}")]
    GitError {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        message: String,
    },

    #[error("Invalid snapshot: {0}")]
    InvalidSnapshot(String),

    #[error("Snapshot creation failed: {0}")]
    CreationFailed(String),

    #[error("Snapshot not ready: {0}")]
    SnapshotNotReady(String),

    #[error("Snapshot restore failed: {0}")]
    RestoreFailed(String),

    #[error("Serialization error: {message}")]
    SerializationError {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        message: String,
    },

    #[error("Deserialization error: {message}")]
    DeserializationError {
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        message: String,
    },
}

impl SnapshotError {
    /// Leaf constructor — no source error.
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::StorageError {
            source: None,
            message: msg.into(),
        }
    }

    /// Leaf constructor — no source error.
    pub fn git(msg: impl Into<String>) -> Self {
        Self::GitError {
            source: None,
            message: msg.into(),
        }
    }

    /// Leaf constructor — no source error.
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::SerializationError {
            source: None,
            message: msg.into(),
        }
    }

    /// Leaf constructor — no source error.
    pub fn deserialization(msg: impl Into<String>) -> Self {
        Self::DeserializationError {
            source: None,
            message: msg.into(),
        }
    }

    /// Wrapping constructor — preserves the source error for `std::error::Error::source()`.
    pub fn storage_with_source(
        source: impl std::error::Error + Send + Sync + 'static,
        msg: impl Into<String>,
    ) -> Self {
        Self::StorageError {
            source: Some(Box::new(source)),
            message: msg.into(),
        }
    }

    /// Wrapping constructor — preserves the source error for `std::error::Error::source()`.
    pub fn git_with_source(
        source: impl std::error::Error + Send + Sync + 'static,
        msg: impl Into<String>,
    ) -> Self {
        Self::GitError {
            source: Some(Box::new(source)),
            message: msg.into(),
        }
    }

    /// Wrapping constructor — preserves the source error for `std::error::Error::source()`.
    pub fn serialization_with_source(
        source: impl std::error::Error + Send + Sync + 'static,
        msg: impl Into<String>,
    ) -> Self {
        Self::SerializationError {
            source: Some(Box::new(source)),
            message: msg.into(),
        }
    }

    /// Wrapping constructor — preserves the source error for `std::error::Error::source()`.
    pub fn deserialization_with_source(
        source: impl std::error::Error + Send + Sync + 'static,
        msg: impl Into<String>,
    ) -> Self {
        Self::DeserializationError {
            source: Some(Box::new(source)),
            message: msg.into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum SnapshotRepoError {
    #[error("Failed to save snapshot: {0}")]
    SaveFailed(String),

    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Failed to delete snapshot: {0}")]
    DeleteFailed(String),
}

pub type Result<T> = std::result::Result<T, SnapshotError>;
pub type RepoResult<T> = std::result::Result<T, SnapshotRepoError>;
pub type StorageResult<T> = std::result::Result<T, SnapshotError>;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assert;
    use proptest::prop_assert_eq;
    use proptest::proptest;

    // --- SnapshotError Display tests ---

    #[test]
    fn not_found_display() {
        let err = SnapshotError::NotFound("snap-123".to_string());
        assert_eq!(err.to_string(), "Snapshot not found: snap-123");
    }

    #[test]
    fn corrupt_display() {
        let err = SnapshotError::Corrupt("bad data".to_string());
        assert_eq!(err.to_string(), "Snapshot corrupt: bad data");
    }

    #[test]
    fn storage_error_display() {
        let err = SnapshotError::storage("disk full");
        assert_eq!(err.to_string(), "Storage error: disk full");
    }

    #[test]
    fn git_error_display() {
        let err = SnapshotError::git("ref not found");
        assert_eq!(err.to_string(), "Git error: ref not found");
    }

    #[test]
    fn invalid_snapshot_display() {
        let err = SnapshotError::InvalidSnapshot("missing id".to_string());
        assert_eq!(err.to_string(), "Invalid snapshot: missing id");
    }

    #[test]
    fn creation_failed_display() {
        let err = SnapshotError::CreationFailed("timeout".to_string());
        assert_eq!(err.to_string(), "Snapshot creation failed: timeout");
    }

    #[test]
    fn snapshot_not_ready_display() {
        let err = SnapshotError::SnapshotNotReady("snap-456".to_string());
        assert_eq!(err.to_string(), "Snapshot not ready: snap-456");
    }

    #[test]
    fn restore_failed_display() {
        let err = SnapshotError::RestoreFailed("conflict".to_string());
        assert_eq!(err.to_string(), "Snapshot restore failed: conflict");
    }

    // --- SnapshotError Debug / thiserror ---

    #[test]
    fn snapshot_error_is_debug() {
        let err = SnapshotError::NotFound("test".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NotFound"));
    }

    #[test]
    fn snapshot_error_debug_for_each_variant() {
        let variants: Vec<SnapshotError> = vec![
            SnapshotError::NotFound("nf".to_string()),
            SnapshotError::Corrupt("cor".to_string()),
            SnapshotError::storage("se"),
            SnapshotError::git("ge"),
            SnapshotError::InvalidSnapshot("is".to_string()),
            SnapshotError::CreationFailed("cf".to_string()),
            SnapshotError::SnapshotNotReady("snr".to_string()),
            SnapshotError::RestoreFailed("rf".to_string()),
        ];
        for (variant, name) in variants.into_iter().zip([
            "NotFound",
            "Corrupt",
            "StorageError",
            "GitError",
            "InvalidSnapshot",
            "CreationFailed",
            "SnapshotNotReady",
            "RestoreFailed",
        ]) {
            let debug_str = format!("{variant:?}");
            assert!(
                debug_str.contains(name),
                "debug output for {name} should contain variant name"
            );
        }
    }

    #[test]
    fn snapshot_error_display_with_empty_string() {
        let err = SnapshotError::NotFound(String::new());
        assert_eq!(err.to_string(), "Snapshot not found: ");
    }

    #[test]
    fn snapshot_error_display_with_long_message() {
        let long_msg = "x".repeat(10_000);
        let err = SnapshotError::storage(long_msg.clone());
        assert_eq!(err.to_string(), format!("Storage error: {long_msg}"));
    }

    #[test]
    fn snapshot_error_display_with_newlines() {
        let err = SnapshotError::Corrupt("line1\nline2".to_string());
        let display = err.to_string();
        assert!(display.contains("line1\nline2"));
    }

    #[test]
    fn snapshot_error_display_with_unicode() {
        let err = SnapshotError::git("エラー発生");
        assert_eq!(err.to_string(), "Git error: エラー発生");
    }

    // --- SnapshotError matching ---

    #[test]
    fn snapshot_error_match_not_found() {
        let err = SnapshotError::NotFound("test".to_string());
        match err {
            SnapshotError::NotFound(msg) => assert_eq!(msg, "test"),
            _ => panic!("should match NotFound"),
        }
    }

    #[test]
    fn snapshot_error_match_corrupt() {
        let err = SnapshotError::Corrupt("data".to_string());
        assert!(matches!(err, SnapshotError::Corrupt(_)));
    }

    #[test]
    fn snapshot_error_match_storage_error() {
        let err = SnapshotError::storage("io");
        assert!(matches!(err, SnapshotError::StorageError { .. }));
    }

    #[test]
    fn snapshot_error_match_git_error() {
        let err = SnapshotError::git("merge");
        assert!(matches!(err, SnapshotError::GitError { .. }));
    }

    #[test]
    fn snapshot_error_match_invalid_snapshot() {
        let err = SnapshotError::InvalidSnapshot("bad".to_string());
        assert!(matches!(err, SnapshotError::InvalidSnapshot(_)));
    }

    #[test]
    fn snapshot_error_match_creation_failed() {
        let err = SnapshotError::CreationFailed("fail".to_string());
        assert!(matches!(err, SnapshotError::CreationFailed(_)));
    }

    #[test]
    fn snapshot_error_match_not_ready() {
        let err = SnapshotError::SnapshotNotReady("pending".to_string());
        assert!(matches!(err, SnapshotError::SnapshotNotReady(_)));
    }

    #[test]
    fn snapshot_error_match_restore_failed() {
        let err = SnapshotError::RestoreFailed("conflict".to_string());
        assert!(matches!(err, SnapshotError::RestoreFailed(_)));
    }

    #[test]
    fn snapshot_error_different_variants_are_not_equal() {
        let e1 = SnapshotError::NotFound("msg".to_string());
        let e2 = SnapshotError::storage("msg");
        assert_ne!(format!("{e1:?}"), format!("{e2:?}"));
    }

    #[test]
    fn snapshot_error_same_variant_same_message_same_debug() {
        let e1 = SnapshotError::NotFound("same".to_string());
        let e2 = SnapshotError::NotFound("same".to_string());
        assert_eq!(format!("{e1:?}"), format!("{e2:?}"));
    }

    // --- SnapshotError implements std::error::Error ---

    #[test]
    fn snapshot_error_implements_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(SnapshotError::NotFound("test".to_string()));
        let _msg = err.to_string();
    }

    #[test]
    fn snapshot_error_source_is_none_for_leaf_variants() {
        let err = SnapshotError::NotFound("test".to_string());
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn snapshot_error_source_is_none_for_leaf_storage_error() {
        let err = SnapshotError::storage("no source");
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    fn snapshot_error_source_preserved_for_wrapping_storage_error() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = SnapshotError::storage_with_source(inner, "context msg");
        let source = std::error::Error::source(&err);
        assert!(source.is_some(), "source should be preserved via #[source]");
        let source_str = source.unwrap().to_string();
        assert!(source_str.contains("file missing"));
    }

    #[test]
    fn snapshot_error_source_chain_depth() {
        let deep = std::io::Error::new(std::io::ErrorKind::Other, "deep error");
        let mid = SnapshotError::storage_with_source(deep, "mid layer");
        // SnapshotError wraps io::Error, which has no further source
        let source = std::error::Error::source(&mid).unwrap();
        assert_eq!(source.to_string(), "deep error");
    }

    // --- SnapshotRepoError Display tests ---

    #[test]
    fn repo_save_failed_display() {
        let err = SnapshotRepoError::SaveFailed("write error".to_string());
        assert_eq!(err.to_string(), "Failed to save snapshot: write error");
    }

    #[test]
    fn repo_not_found_display() {
        let err = SnapshotRepoError::NotFound("snap-789".to_string());
        assert_eq!(err.to_string(), "Snapshot not found: snap-789");
    }

    #[test]
    fn repo_delete_failed_display() {
        let err = SnapshotRepoError::DeleteFailed("locked".to_string());
        assert_eq!(err.to_string(), "Failed to delete snapshot: locked");
    }

    #[test]
    fn repo_error_debug_contains_variant_name() {
        let variants: Vec<SnapshotRepoError> = vec![
            SnapshotRepoError::SaveFailed("sf".to_string()),
            SnapshotRepoError::NotFound("nf".to_string()),
            SnapshotRepoError::DeleteFailed("df".to_string()),
        ];
        for (variant, name) in variants
            .into_iter()
            .zip(["SaveFailed", "NotFound", "DeleteFailed"])
        {
            let debug_str = format!("{variant:?}");
            assert!(debug_str.contains(name));
        }
    }

    #[test]
    fn repo_error_display_with_empty_message() {
        let err = SnapshotRepoError::SaveFailed(String::new());
        assert_eq!(err.to_string(), "Failed to save snapshot: ");
    }

    #[test]
    fn repo_error_display_with_unicode() {
        let err = SnapshotRepoError::NotFound("スナップショット".to_string());
        assert_eq!(err.to_string(), "Snapshot not found: スナップショット");
    }

    #[test]
    fn repo_error_implements_error_trait() {
        let err: Box<dyn std::error::Error> =
            Box::new(SnapshotRepoError::SaveFailed("test".to_string()));
        let _msg = err.to_string();
    }

    #[test]
    fn repo_error_matching() {
        let err = SnapshotRepoError::SaveFailed("io error".to_string());
        assert!(matches!(err, SnapshotRepoError::SaveFailed(msg) if msg == "io error"));
    }

    // --- Type alias tests ---

    #[test]
    fn result_type_ok() {
        let val: Result<i32> = Ok(42);
        assert_eq!(val.expect("should be Ok"), 42);
    }

    #[test]
    fn result_type_err() {
        let val: Result<i32> = Err(SnapshotError::NotFound("x".to_string()));
        assert!(val.is_err());
    }

    #[test]
    fn repo_result_type_ok() {
        let val: RepoResult<String> = Ok("hello".to_string());
        assert_eq!(val.expect("should be Ok"), "hello");
    }

    #[test]
    fn repo_result_type_err() {
        let val: RepoResult<()> = Err(SnapshotRepoError::SaveFailed("fail".to_string()));
        assert!(val.is_err());
    }

    #[test]
    fn storage_result_type_ok() {
        let val: StorageResult<bool> = Ok(true);
        assert!(val.expect("should be Ok"));
    }

    #[test]
    fn storage_result_type_err() {
        let val: StorageResult<()> = Err(SnapshotError::storage("fail"));
        assert!(val.is_err());
    }

    #[test]
    fn result_type_with_unit() {
        let val: Result<()> = Ok(());
        assert!(val.is_ok());
    }

    #[test]
    fn result_type_map_ok() {
        let val: Result<i32> = Ok(42);
        let mapped = val.map(|v| v * 2);
        assert_eq!(mapped.expect("should be Ok"), 84);
    }

    #[test]
    fn result_type_map_err() {
        let val: Result<i32> = Err(SnapshotError::NotFound("x".to_string()));
        let mapped = val.map(|v| v * 2);
        assert!(mapped.is_err());
    }

    #[test]
    fn result_type_and_then() {
        let val: Result<i32> = Ok(42);
        let result = val.and_then(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(SnapshotError::InvalidSnapshot("negative".to_string()))
            }
        });
        assert_eq!(result.expect("should be Ok"), 42);
    }

    #[test]
    fn result_type_unwrap_or() {
        let val: Result<i32> = Err(SnapshotError::NotFound("x".to_string()));
        assert_eq!(val.unwrap_or(0), 0);
    }

    // --- Proptests ---

    proptest! {
        #[test]
        fn snapshot_error_display_always_includes_message(msg in ".{0,500}") {
            let err = SnapshotError::NotFound(msg.clone());
            let display = err.to_string();
            prop_assert!(display.contains(&msg));
        }

        #[test]
        fn snapshot_repo_error_display_always_includes_message(msg in ".{0,500}") {
            let err = SnapshotRepoError::SaveFailed(msg.clone());
            let display = err.to_string();
            prop_assert!(display.contains(&msg));
        }

        #[test]
        fn snapshot_error_not_found_display_format(msg in ".{0,200}") {
            let err = SnapshotError::NotFound(msg.clone());
            let display = err.to_string();
            prop_assert!(display.starts_with("Snapshot not found:"));
        }

        #[test]
        fn snapshot_error_corrupt_display_format(msg in ".{0,200}") {
            let err = SnapshotError::Corrupt(msg.clone());
            let display = err.to_string();
            prop_assert!(display.starts_with("Snapshot corrupt:"));
        }

        #[test]
        fn snapshot_error_storage_error_display_format(msg in ".{0,200}") {
            let err = SnapshotError::storage(msg.clone());
            let display = err.to_string();
            prop_assert!(display.starts_with("Storage error:"));
        }

        #[test]
        fn snapshot_error_git_error_display_format(msg in ".{0,200}") {
            let err = SnapshotError::git(msg.clone());
            let display = err.to_string();
            prop_assert!(display.starts_with("Git error:"));
        }

        #[test]
        fn snapshot_error_invalid_snapshot_display_format(msg in ".{0,200}") {
            let err = SnapshotError::InvalidSnapshot(msg.clone());
            let display = err.to_string();
            prop_assert!(display.starts_with("Invalid snapshot:"));
        }

        #[test]
        fn result_type_ok_roundtrip(val in "[a-zA-Z0-9 ]{0,100}") {
            let r: Result<String> = Ok(val.clone());
            prop_assert!(r.is_ok());
            prop_assert_eq!(r.expect("should be Ok"), val);
        }
    }
}
