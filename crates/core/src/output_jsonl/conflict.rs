//! Conflict resolution output types
//!
//! Provides conflict detection and resolution reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// CONFLICT RESOLUTION TYPES
// ============================================================================

/// Type of conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Files modified on both branches
    Overlapping,
    /// Conflict already exists in workspace
    Existing,
    /// File deleted on one branch, modified on other
    DeleteModify,
    /// File renamed on one branch, modified on other
    RenameModify,
    /// Binary file conflict
    Binary,
}

/// Strategy for resolving a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    AcceptOurs,
    AcceptTheirs,
    JjResolve,
    ManualMerge,
    Rebase,
    Abort,
    Skip,
}

/// Risk level of a resolution option
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionRisk {
    Safe,
    Moderate,
    Destructive,
}

/// A resolution option for a conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionOption {
    pub strategy: ResolutionStrategy,
    pub description: String,
    pub risk: ResolutionRisk,
    pub automatic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ResolutionOption {
    #[must_use]
    pub fn accept_ours() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptOurs,
            description: "Accept workspace version".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj resolve --with workspace".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn accept_theirs() -> Self {
        Self {
            strategy: ResolutionStrategy::AcceptTheirs,
            description: "Accept main version".to_string(),
            risk: ResolutionRisk::Destructive,
            automatic: true,
            command: Some("jj resolve --with main".to_string()),
            notes: Some("Will discard workspace changes".to_string()),
        }
    }

    #[must_use]
    pub fn manual_merge() -> Self {
        Self {
            strategy: ResolutionStrategy::ManualMerge,
            description: "Manually resolve conflicts".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: false,
            command: None,
            notes: Some("Open file in editor".to_string()),
        }
    }

    #[must_use]
    pub fn jj_resolve(file: &str) -> Self {
        Self {
            strategy: ResolutionStrategy::JjResolve,
            description: "Use jj resolve tool".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: Some(format!("jj resolve {file}")),
            notes: None,
        }
    }

    #[must_use]
    pub fn rebase() -> Self {
        Self {
            strategy: ResolutionStrategy::Rebase,
            description: "Rebase onto fresh main".to_string(),
            risk: ResolutionRisk::Moderate,
            automatic: true,
            command: Some("jj rebase -d main".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn abort() -> Self {
        Self {
            strategy: ResolutionStrategy::Abort,
            description: "Abort the operation".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: Some("jj abort".to_string()),
            notes: None,
        }
    }

    #[must_use]
    pub fn skip() -> Self {
        Self {
            strategy: ResolutionStrategy::Skip,
            description: "Skip this file".to_string(),
            risk: ResolutionRisk::Safe,
            automatic: true,
            command: None,
            notes: Some("File will remain conflicted".to_string()),
        }
    }
}

/// Details about a specific conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictDetail {
    pub file: String,
    pub conflict_type: ConflictType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_additions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_deletions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_additions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_deletions: Option<u32>,
    pub resolutions: Vec<ResolutionOption>,
    pub recommended: ResolutionStrategy,
}

impl ConflictDetail {
    #[must_use]
    pub fn overlapping(file: &str) -> Self {
        Self {
            file: file.to_string(),
            conflict_type: ConflictType::Overlapping,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(file),
                ResolutionOption::manual_merge(),
                ResolutionOption::accept_ours(),
                ResolutionOption::accept_theirs(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }

    #[must_use]
    pub fn existing(file: &str) -> Self {
        Self {
            file: file.to_string(),
            conflict_type: ConflictType::Existing,
            workspace_additions: None,
            workspace_deletions: None,
            main_additions: None,
            main_deletions: None,
            resolutions: vec![
                ResolutionOption::jj_resolve(file),
                ResolutionOption::manual_merge(),
                ResolutionOption::rebase(),
                ResolutionOption::abort(),
            ],
            recommended: ResolutionStrategy::JjResolve,
        }
    }
}

/// Analysis of all conflicts in a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictAnalysis {
    #[serde(rename = "type")]
    pub type_field: String,
    pub session: String,
    pub merge_safe: bool,
    pub total_conflicts: usize,
    pub conflicts: Vec<ConflictDetail>,
    pub existing_conflicts: usize,
    pub overlapping_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_time_ms: Option<u64>,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConflictType ─────────────────────────────────────────────────────────

    #[test]
    fn test_conflict_type_all_variants() {
        let variants = [
            ConflictType::Overlapping,
            ConflictType::Existing,
            ConflictType::DeleteModify,
            ConflictType::RenameModify,
            ConflictType::Binary,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_conflict_type_serde_roundtrip() {
        for ct in [
            ConflictType::Overlapping,
            ConflictType::Existing,
            ConflictType::DeleteModify,
            ConflictType::RenameModify,
            ConflictType::Binary,
        ] {
            let json = serde_json::to_string(&ct).expect("serialize ok");
            let deserialized: ConflictType =
                serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(ct, deserialized);
        }
    }

    #[test]
    fn test_conflict_type_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&ConflictType::Overlapping).expect("ok"),
            "\"overlapping\""
        );
        assert_eq!(
            serde_json::to_string(&ConflictType::Existing).expect("ok"),
            "\"existing\""
        );
    }

    // ── ResolutionStrategy ───────────────────────────────────────────────────

    #[test]
    fn test_resolution_strategy_all_variants() {
        let variants = [
            ResolutionStrategy::AcceptOurs,
            ResolutionStrategy::AcceptTheirs,
            ResolutionStrategy::JjResolve,
            ResolutionStrategy::ManualMerge,
            ResolutionStrategy::Rebase,
            ResolutionStrategy::Abort,
            ResolutionStrategy::Skip,
        ];
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn test_resolution_strategy_serde_roundtrip() {
        for s in [
            ResolutionStrategy::AcceptOurs,
            ResolutionStrategy::AcceptTheirs,
            ResolutionStrategy::JjResolve,
            ResolutionStrategy::ManualMerge,
            ResolutionStrategy::Rebase,
            ResolutionStrategy::Abort,
            ResolutionStrategy::Skip,
        ] {
            let json = serde_json::to_string(&s).expect("serialize ok");
            let deserialized: ResolutionStrategy =
                serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(s, deserialized);
        }
    }

    // ── ResolutionRisk ───────────────────────────────────────────────────────

    #[test]
    fn test_resolution_risk_all_variants() {
        let variants = [ResolutionRisk::Safe, ResolutionRisk::Moderate, ResolutionRisk::Destructive];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn test_resolution_risk_serde_roundtrip() {
        for r in [ResolutionRisk::Safe, ResolutionRisk::Moderate, ResolutionRisk::Destructive] {
            let json = serde_json::to_string(&r).expect("serialize ok");
            let deserialized: ResolutionRisk =
                serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(r, deserialized);
        }
    }

    #[test]
    fn test_resolution_risk_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&ResolutionRisk::Safe).expect("ok"),
            "\"safe\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionRisk::Destructive).expect("ok"),
            "\"destructive\""
        );
    }

    // ── ResolutionOption constructors ────────────────────────────────────────

    #[test]
    fn test_resolution_option_accept_ours() {
        let opt = ResolutionOption::accept_ours();
        assert_eq!(opt.strategy, ResolutionStrategy::AcceptOurs);
        assert!(opt.automatic);
        assert_eq!(opt.risk, ResolutionRisk::Moderate);
        assert!(opt.command.is_some());
    }

    #[test]
    fn test_resolution_option_accept_theirs() {
        let opt = ResolutionOption::accept_theirs();
        assert_eq!(opt.strategy, ResolutionStrategy::AcceptTheirs);
        assert!(opt.automatic);
        assert_eq!(opt.risk, ResolutionRisk::Destructive);
        assert!(opt.notes.is_some());
    }

    #[test]
    fn test_resolution_option_manual_merge() {
        let opt = ResolutionOption::manual_merge();
        assert_eq!(opt.strategy, ResolutionStrategy::ManualMerge);
        assert!(!opt.automatic);
        assert_eq!(opt.risk, ResolutionRisk::Safe);
        assert!(opt.command.is_none());
    }

    #[test]
    fn test_resolution_option_jj_resolve() {
        let opt = ResolutionOption::jj_resolve("main.rs");
        assert_eq!(opt.strategy, ResolutionStrategy::JjResolve);
        assert!(opt.automatic);
        assert!(opt.command.is_some());
        let cmd = opt.command.expect("has command");
        assert!(cmd.contains("main.rs"));
    }

    #[test]
    fn test_resolution_option_rebase() {
        let opt = ResolutionOption::rebase();
        assert_eq!(opt.strategy, ResolutionStrategy::Rebase);
        assert!(opt.automatic);
    }

    #[test]
    fn test_resolution_option_abort() {
        let opt = ResolutionOption::abort();
        assert_eq!(opt.strategy, ResolutionStrategy::Abort);
        assert!(opt.automatic);
    }

    #[test]
    fn test_resolution_option_skip() {
        let opt = ResolutionOption::skip();
        assert_eq!(opt.strategy, ResolutionStrategy::Skip);
        assert!(opt.automatic);
        assert!(opt.command.is_none());
        assert!(opt.notes.is_some());
    }

    // ── ResolutionOption serde ───────────────────────────────────────────────

    #[test]
    fn test_resolution_option_serde_roundtrip() {
        let opt = ResolutionOption::accept_ours();
        let json = serde_json::to_string(&opt).expect("serialize ok");
        let deserialized: ResolutionOption =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(opt.strategy, deserialized.strategy);
        assert_eq!(opt.description, deserialized.description);
    }

    #[test]
    fn test_resolution_option_serde_skips_none() {
        let opt = ResolutionOption::accept_ours();
        let json_val = serde_json::to_value(&opt).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("notes"));
    }

    // ── ConflictDetail constructors ──────────────────────────────────────────

    #[test]
    fn test_conflict_detail_overlapping() {
        let detail = ConflictDetail::overlapping("src/main.rs");
        assert_eq!(detail.file, "src/main.rs");
        assert_eq!(detail.conflict_type, ConflictType::Overlapping);
        assert_eq!(detail.recommended, ResolutionStrategy::JjResolve);
        assert_eq!(detail.resolutions.len(), 4); // jj_resolve, manual, accept_ours, accept_theirs
    }

    #[test]
    fn test_conflict_detail_existing() {
        let detail = ConflictDetail::existing("src/lib.rs");
        assert_eq!(detail.file, "src/lib.rs");
        assert_eq!(detail.conflict_type, ConflictType::Existing);
        assert_eq!(detail.resolutions.len(), 4); // jj_resolve, manual, rebase, abort
    }

    // ── ConflictDetail serde ─────────────────────────────────────────────────

    #[test]
    fn test_conflict_detail_serde_roundtrip() {
        let detail = ConflictDetail::overlapping("test.rs");
        let json = serde_json::to_string(&detail).expect("serialize ok");
        let deserialized: ConflictDetail =
            serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(detail, deserialized);
    }

    #[test]
    fn test_conflict_detail_serde_skips_none_additions_deletions() {
        let detail = ConflictDetail::overlapping("test.rs");
        let json_val = serde_json::to_value(&detail).expect("serialize ok");
        let obj = json_val.as_object().expect("obj");
        assert!(!obj.contains_key("workspace_additions"));
        assert!(!obj.contains_key("workspace_deletions"));
        assert!(!obj.contains_key("main_additions"));
        assert!(!obj.contains_key("main_deletions"));
    }

    // ── ConflictAnalysis ─────────────────────────────────────────────────────

    #[test]
    fn test_conflict_analysis_serde_roundtrip() {
        use crate::output_jsonl::output_line::OutputLine;
        let line = OutputLine::conflict_analysis(
            "test-session",
            true,
            vec![ConflictDetail::overlapping("a.rs"), ConflictDetail::existing("b.rs")],
        );
        if let OutputLine::ConflictAnalysis(analysis) = line {
            assert_eq!(analysis.total_conflicts, 2);
            assert_eq!(analysis.existing_conflicts, 1);
            assert_eq!(analysis.overlapping_files, 1);
            assert!(analysis.merge_safe);

            // Serde roundtrip
            let json = serde_json::to_string(&analysis).expect("serialize ok");
            let deserialized: ConflictAnalysis =
                serde_json::from_str(&json).expect("deserialize ok");
            assert_eq!(analysis.session, deserialized.session);
            assert_eq!(analysis.total_conflicts, deserialized.total_conflicts);
        } else {
            panic!("Expected ConflictAnalysis variant");
        }
    }

    #[test]
    fn test_conflict_analysis_serde_skips_none_merge_base() {
        use crate::output_jsonl::output_line::OutputLine;
        let line = OutputLine::conflict_analysis("s", false, vec![]);
        if let OutputLine::ConflictAnalysis(analysis) = line {
            let json_val = serde_json::to_value(&analysis).expect("serialize ok");
            let obj = json_val.as_object().expect("obj");
            assert!(!obj.contains_key("merge_base"));
            assert!(!obj.contains_key("analysis_time_ms"));
        }
    }
}
