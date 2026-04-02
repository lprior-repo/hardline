//! File change tracking types
//!
//! Tracks modifications, additions, deletions, and renames.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileStatus {
    #[serde(rename = "M")]
    Modified,
    #[serde(rename = "A")]
    Added,
    #[serde(rename = "D")]
    Deleted,
    #[serde(rename = "R")]
    Renamed,
    #[serde(rename = "?")]
    Untracked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<PathBuf>,
}

impl FileChange {
    pub fn validate(&self) -> Result<()> {
        if self.status == FileStatus::Renamed && self.old_path.is_none() {
            return Err(Error::invalid_state(
                "Renamed files must have old_path set".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangesSummary {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub renamed: usize,
    pub untracked: usize,
}

impl ChangesSummary {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed
    }

    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.total() > 0
    }

    #[must_use]
    pub const fn has_tracked_changes(&self) -> bool {
        self.modified + self.added + self.deleted + self.renamed > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffStat {
    pub path: PathBuf,
    pub insertions: usize,
    pub deletions: usize,
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub insertions: usize,
    pub deletions: usize,
    pub files_changed: usize,
    pub files: Vec<FileDiffStat>,
}

impl DiffSummary {
    pub fn validate(&self) -> Result<()> {
        if self.files.len() != self.files_changed {
            return Err(Error::invalid_state(format!(
                "files_changed ({}) does not match files array length ({})",
                self.files_changed,
                self.files.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FileStatus enum variants ─────────────────────────────────────────────

    #[test]
    fn test_file_status_all_variants() {
        let variants = [
            FileStatus::Modified,
            FileStatus::Added,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Untracked,
        ];
        assert_eq!(variants.len(), 5);

        let mut set = std::collections::HashSet::new();
        for v in &variants {
            assert!(set.insert(*v), "Duplicate: {v:?}");
        }
    }

    #[test]
    fn test_file_status_copy() {
        let status = FileStatus::Modified;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_file_status_hash() {
        let mut set = std::collections::HashSet::new();
        set.insert(FileStatus::Modified);
        set.insert(FileStatus::Added);
        set.insert(FileStatus::Modified); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_file_status_serde_roundtrip() {
        for status in [
            FileStatus::Modified,
            FileStatus::Added,
            FileStatus::Deleted,
            FileStatus::Renamed,
            FileStatus::Untracked,
        ] {
            let json = serde_json::to_string(&status).expect("serialize ok");
            let deserialized: FileStatus = serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(status, deserialized, "Roundtrip failed for {status:?}");
        }
    }

    #[test]
    fn test_file_status_serde_single_char() {
        assert_eq!(serde_json::to_string(&FileStatus::Modified).expect("ok"), "\"M\"");
        assert_eq!(serde_json::to_string(&FileStatus::Added).expect("ok"), "\"A\"");
        assert_eq!(serde_json::to_string(&FileStatus::Deleted).expect("ok"), "\"D\"");
        assert_eq!(serde_json::to_string(&FileStatus::Renamed).expect("ok"), "\"R\"");
        assert_eq!(serde_json::to_string(&FileStatus::Untracked).expect("ok"), "\"?\"");
    }

    #[test]
    fn test_file_status_debug() {
        let status = FileStatus::Modified;
        let debug_str = format!("{status:?}");
        assert!(debug_str.contains("Modified"));
    }

    // ── FileChange construction ──────────────────────────────────────────────

    #[test]
    fn test_file_change_modified() {
        let change = FileChange {
            path: PathBuf::from("src/main.rs"),
            status: FileStatus::Modified,
            old_path: None,
        };
        assert_eq!(change.path, PathBuf::from("src/main.rs"));
        assert_eq!(change.status, FileStatus::Modified);
        assert!(change.old_path.is_none());
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_added() {
        let change = FileChange {
            path: PathBuf::from("new_file.rs"),
            status: FileStatus::Added,
            old_path: None,
        };
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_deleted() {
        let change = FileChange {
            path: PathBuf::from("old_file.rs"),
            status: FileStatus::Deleted,
            old_path: None,
        };
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_untracked() {
        let change = FileChange {
            path: PathBuf::from("scratch.rs"),
            status: FileStatus::Untracked,
            old_path: None,
        };
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_renamed_valid() {
        let change = FileChange {
            path: PathBuf::from("new_name.rs"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("old_name.rs")),
        };
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_renamed_without_old_path_fails() {
        let change = FileChange {
            path: PathBuf::from("new_name.rs"),
            status: FileStatus::Renamed,
            old_path: None,
        };
        let result = change.validate();
        assert!(result.is_err());
        let err_msg = format!("{result:?}");
        assert!(err_msg.contains("old_path"), "Error should mention old_path: {err_msg}");
    }

    #[test]
    fn test_file_change_non_renamed_with_old_path_still_valid() {
        // Having old_path set for non-Renamed statuses is allowed
        let change = FileChange {
            path: PathBuf::from("file.rs"),
            status: FileStatus::Modified,
            old_path: Some(PathBuf::from("irrelevant.rs")),
        };
        assert!(change.validate().is_ok());
    }

    #[test]
    fn test_file_change_clone() {
        let change = FileChange {
            path: PathBuf::from("file.rs"),
            status: FileStatus::Added,
            old_path: Some(PathBuf::from("old.rs")),
        };
        let cloned = change.clone();
        assert_eq!(cloned.path, change.path);
        assert_eq!(cloned.status, change.status);
        assert_eq!(cloned.old_path, change.old_path);
    }

    #[test]
    fn test_file_change_serde_roundtrip() {
        let change = FileChange {
            path: PathBuf::from("path/to/file.rs"),
            status: FileStatus::Modified,
            old_path: None,
        };
        let json = serde_json::to_string(&change).expect("serialize ok");
        let deserialized: FileChange = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(change.path, deserialized.path);
        assert_eq!(change.status, deserialized.status);
        assert_eq!(change.old_path, deserialized.old_path);
    }

    #[test]
    fn test_file_change_serde_renamed_with_old_path() {
        let change = FileChange {
            path: PathBuf::from("b.rs"),
            status: FileStatus::Renamed,
            old_path: Some(PathBuf::from("a.rs")),
        };
        let json = serde_json::to_string(&change).expect("serialize ok");
        let deserialized: FileChange = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(deserialized.old_path.as_deref(), Some(std::path::Path::new("a.rs")));
    }

    #[test]
    fn test_file_change_serde_skips_none_old_path() {
        let change = FileChange {
            path: PathBuf::from("file.rs"),
            status: FileStatus::Added,
            old_path: None,
        };
        let json_val = serde_json::to_value(&change).expect("serialize ok");
        let obj = json_val.as_object().expect("should be object");
        assert!(!obj.contains_key("old_path"));
    }

    #[test]
    fn test_file_change_debug() {
        let change = FileChange {
            path: PathBuf::from("test.rs"),
            status: FileStatus::Modified,
            old_path: None,
        };
        let debug_str = format!("{change:?}");
        assert!(debug_str.contains("test.rs"));
        assert!(debug_str.contains("Modified"));
    }

    // ── ChangesSummary construction ──────────────────────────────────────────

    #[test]
    fn test_changes_summary_default() {
        let s = ChangesSummary::default();
        assert_eq!(s.modified, 0);
        assert_eq!(s.added, 0);
        assert_eq!(s.deleted, 0);
        assert_eq!(s.renamed, 0);
        assert_eq!(s.untracked, 0);
        assert_eq!(s.total(), 0);
        assert!(!s.has_changes());
        assert!(!s.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_custom() {
        let s = ChangesSummary {
            modified: 2,
            added: 3,
            deleted: 1,
            renamed: 1,
            untracked: 4,
        };
        assert_eq!(s.total(), 7); // 2+3+1+1 (untracked excluded from total)
        assert!(s.has_changes()); // total > 0
        assert!(s.has_tracked_changes()); // tracked total > 0
    }

    #[test]
    fn test_changes_summary_only_untracked() {
        let s = ChangesSummary {
            modified: 0,
            added: 0,
            deleted: 0,
            renamed: 0,
            untracked: 10,
        };
        // total() excludes untracked, so total is 0
        assert_eq!(s.total(), 0);
        // has_changes() checks total() > 0, so false when only untracked
        assert!(!s.has_changes());
        assert!(!s.has_tracked_changes());
    }

    #[test]
    fn test_changes_summary_serde_roundtrip() {
        let s = ChangesSummary {
            modified: 1,
            added: 2,
            deleted: 3,
            renamed: 4,
            untracked: 5,
        };
        let json = serde_json::to_string(&s).expect("serialize ok");
        let deserialized: ChangesSummary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(s.modified, deserialized.modified);
        assert_eq!(s.added, deserialized.added);
        assert_eq!(s.deleted, deserialized.deleted);
        assert_eq!(s.renamed, deserialized.renamed);
        assert_eq!(s.untracked, deserialized.untracked);
    }

    // ── FileDiffStat construction ────────────────────────────────────────────

    #[test]
    fn test_file_diff_stat_construction() {
        let stat = FileDiffStat {
            path: PathBuf::from("src/lib.rs"),
            insertions: 10,
            deletions: 5,
            status: FileStatus::Modified,
        };
        assert_eq!(stat.path, PathBuf::from("src/lib.rs"));
        assert_eq!(stat.insertions, 10);
        assert_eq!(stat.deletions, 5);
        assert_eq!(stat.status, FileStatus::Modified);
    }

    #[test]
    fn test_file_diff_stat_zero_changes() {
        let stat = FileDiffStat {
            path: PathBuf::from("unchanged.rs"),
            insertions: 0,
            deletions: 0,
            status: FileStatus::Modified,
        };
        assert_eq!(stat.insertions, 0);
        assert_eq!(stat.deletions, 0);
    }

    #[test]
    fn test_file_diff_stat_serde_roundtrip() {
        let stat = FileDiffStat {
            path: PathBuf::from("file.rs"),
            insertions: 42,
            deletions: 17,
            status: FileStatus::Added,
        };
        let json = serde_json::to_string(&stat).expect("serialize ok");
        let deserialized: FileDiffStat = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(stat.path, deserialized.path);
        assert_eq!(stat.insertions, deserialized.insertions);
        assert_eq!(stat.deletions, deserialized.deletions);
        assert_eq!(stat.status, deserialized.status);
    }

    // ── DiffSummary construction and validation ──────────────────────────────

    #[test]
    fn test_diff_summary_empty_valid() {
        let diff = DiffSummary {
            insertions: 0,
            deletions: 0,
            files_changed: 0,
            files: vec![],
        };
        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_consistent() {
        let diff = DiffSummary {
            insertions: 100,
            deletions: 50,
            files_changed: 2,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("a.rs"),
                    insertions: 80,
                    deletions: 30,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("b.rs"),
                    insertions: 20,
                    deletions: 20,
                    status: FileStatus::Added,
                },
            ],
        };
        assert!(diff.validate().is_ok());
    }

    #[test]
    fn test_diff_summary_mismatch_fails() {
        let diff = DiffSummary {
            insertions: 10,
            deletions: 0,
            files_changed: 3,
            files: vec![FileDiffStat {
                path: PathBuf::from("single.rs"),
                insertions: 10,
                deletions: 0,
                status: FileStatus::Modified,
            }],
        };
        let result = diff.validate();
        assert!(result.is_err());
        let err_msg = format!("{result:?}");
        assert!(err_msg.contains("files_changed"), "Error should mention files_changed: {err_msg}");
    }

    #[test]
    fn test_diff_summary_more_files_than_count() {
        let diff = DiffSummary {
            insertions: 0,
            deletions: 0,
            files_changed: 1,
            files: vec![
                FileDiffStat {
                    path: PathBuf::from("a.rs"),
                    insertions: 0,
                    deletions: 0,
                    status: FileStatus::Modified,
                },
                FileDiffStat {
                    path: PathBuf::from("b.rs"),
                    insertions: 0,
                    deletions: 0,
                    status: FileStatus::Modified,
                },
            ],
        };
        assert!(diff.validate().is_err());
    }

    #[test]
    fn test_diff_summary_serde_roundtrip() {
        let diff = DiffSummary {
            insertions: 25,
            deletions: 10,
            files_changed: 1,
            files: vec![FileDiffStat {
                path: PathBuf::from("test.rs"),
                insertions: 25,
                deletions: 10,
                status: FileStatus::Deleted,
            }],
        };
        let json = serde_json::to_string(&diff).expect("serialize ok");
        let deserialized: DiffSummary = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(diff.insertions, deserialized.insertions);
        assert_eq!(diff.deletions, deserialized.deletions);
        assert_eq!(diff.files_changed, deserialized.files_changed);
        assert_eq!(diff.files.len(), deserialized.files.len());
    }

    // ── Proptests ────────────────────────────────────────────────────────────

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_changes_summary_total(
            m in 0u32..100u32,
            a in 0u32..100u32,
            d in 0u32..100u32,
            r in 0u32..100u32,
            u in 0u32..100u32
        ) {
            let s = ChangesSummary {
                modified: m as usize,
                added: a as usize,
                deleted: d as usize,
                renamed: r as usize,
                untracked: u as usize,
            };
            // total does NOT include untracked
            assert_eq!(s.total(), (m + a + d + r) as usize);
            // has_changes checks total > 0
            assert_eq!(s.has_changes(), s.total() > 0);
            // has_tracked_changes checks same as total
            assert_eq!(s.has_tracked_changes(), s.total() > 0);
        }

        #[test]
        fn prop_file_change_validate_non_renamed_always_valid(
            path in "[a-zA-Z0-9_/]+\\.[a-z]{1,5}"
        ) {
            for status in [
                FileStatus::Modified,
                FileStatus::Added,
                FileStatus::Deleted,
                FileStatus::Untracked,
            ] {
                let change = FileChange {
                    path: PathBuf::from(&path),
                    status,
                    old_path: None,
                };
                assert!(change.validate().is_ok());
            }
        }

        #[test]
        fn prop_diff_summary_validates_files_changed_equals_files_len(
            insertions in 0u32..100u32,
            deletions in 0u32..100u32,
            count in 0u32..5u32
        ) {
            let files: Vec<FileDiffStat> = (0..count)
                .map(|i| FileDiffStat {
                    path: PathBuf::from(format!("file_{i}.rs")),
                    insertions: insertions as usize,
                    deletions: deletions as usize,
                    status: FileStatus::Modified,
                })
                .collect();
            let diff = DiffSummary {
                insertions: insertions as usize * count as usize,
                deletions: deletions as usize * count as usize,
                files_changed: count as usize,
                files,
            };
            assert!(diff.validate().is_ok());
        }
    }
}
