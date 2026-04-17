use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A validated commit hash identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitHash(String);

impl CommitHash {
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StackName(String);

impl StackName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StackName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for StackName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

use super::state::{BranchState, PrState, StackState};
use super::value_objects::BranchName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub pr_number: u32,
    pub url: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub state: PrState,
    pub draft: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PrInfo {
    pub fn new(
        pr_number: u32,
        url: String,
        title: String,
        description: String,
        author: String,
        draft: bool,
    ) -> Self {
        let now = Utc::now();
        Self {
            pr_number,
            url,
            title,
            description,
            author,
            state: PrState::Open,
            draft,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_state(mut self, state: PrState) -> Self {
        self.state = state;
        self.updated_at = Utc::now();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrStatus {
    pub pr_number: u32,
    pub state: PrState,
    pub checks_passed: bool,
    pub reviews_approved: Vec<String>,
    pub mergeable: bool,
    pub conflict_resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub requires_rebase: bool,
    pub conflicting_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackBranch {
    pub branch_name: BranchName,
    pub position: u32,
    pub pr_info: Option<PrInfo>,
    pub state: BranchState,
    pub last_commit: CommitHash,
    pub parent_branch: Option<BranchName>,
}

impl StackBranch {
    pub fn new(
        branch_name: BranchName,
        position: u32,
        last_commit: CommitHash,
        parent_branch: Option<BranchName>,
    ) -> Self {
        Self {
            branch_name,
            position,
            pr_info: None,
            state: BranchState::Open,
            last_commit,
            parent_branch,
        }
    }

    pub fn with_pr_info(mut self, pr_info: PrInfo) -> Self {
        self.pr_info = Some(pr_info);
        self
    }

    pub fn transition_to(&mut self, state: BranchState) {
        self.state = state;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: StackId,
    pub name: StackName,
    pub base_branch: BranchName,
    pub branches: Vec<StackBranch>,
    pub state: StackState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Stack {
    pub fn new(id: StackId, name: StackName, base_branch: BranchName) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            base_branch,
            branches: Vec::new(),
            state: StackState::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_branches(mut self, branches: Vec<StackBranch>) -> Self {
        self.branches = branches;
        self
    }

    pub fn add_branch(&mut self, branch: StackBranch) {
        self.branches.push(branch);
        self.updated_at = Utc::now();
    }

    pub fn transition_to(&mut self, state: StackState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    pub fn is_draft(&self) -> bool {
        self.state == StackState::Draft
    }

    pub fn is_published(&self) -> bool {
        self.state == StackState::Published
    }

    pub fn is_merged(&self) -> bool {
        self.state == StackState::Merged
    }

    pub fn branch_at_position(&self, position: u32) -> Option<&StackBranch> {
        self.branches.iter().find(|b| b.position == position)
    }

    pub fn branch_named(&self, name: &BranchName) -> Option<&StackBranch> {
        self.branches.iter().find(|b| &b.branch_name == name)
    }

    pub fn bottom_branch(&self) -> Option<&StackBranch> {
        self.branches.first()
    }

    pub fn top_branch(&self) -> Option<&StackBranch> {
        self.branches.last()
    }

    pub fn branches_ordered(&self) -> &[StackBranch] {
        &self.branches
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StackId(u64);

impl StackId {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(duration.as_nanos() as u64)
    }

    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn to_u64(self) -> u64 {
        self.0
    }
}

impl Default for StackId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stack-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_stack() -> Stack {
        let stack_id = StackId::from_u64(1);
        let base = BranchName::new("main");
        let mut stack = Stack::new(stack_id, StackName::new("test-stack"), base);

        stack.add_branch(StackBranch::new(
            BranchName::new("feature-a"),
            0,
            CommitHash::new("abc123"),
            Some(BranchName::new("main")),
        ));

        stack.add_branch(StackBranch::new(
            BranchName::new("feature-b"),
            1,
            CommitHash::new("def456"),
            Some(BranchName::new("feature-a")),
        ));

        stack
    }

    #[test]
    fn test_stack_creation() {
        let stack = create_test_stack();
        assert_eq!(stack.branches.len(), 2);
        assert!(stack.is_draft());
        assert_eq!(stack.bottom_branch().map(|b| b.position), Some(0));
        assert_eq!(stack.top_branch().map(|b| b.position), Some(1));
    }

    #[test]
    fn test_branch_lookup() {
        let stack = create_test_stack();
        assert!(stack.branch_named(&BranchName::new("feature-a")).is_some());
        assert!(stack
            .branch_named(&BranchName::new("nonexistent"))
            .is_none());
    }

    #[test]
    fn test_state_transition() {
        let mut stack = create_test_stack();
        stack.transition_to(StackState::Published);
        assert!(stack.is_published());
        assert!(!stack.is_draft());
    }

    #[test]
    fn test_stack_id_display() {
        let id = StackId::from_u64(42);
        assert_eq!(format!("{}", id), "stack-42");
    }

    #[test]
    fn test_stack_id_from_u64_to_u64_roundtrip() {
        let values = [0u64, 1, 42, u32::MAX as u64, u64::MAX];
        for val in &values {
            let id = StackId::from_u64(*val);
            assert_eq!(id.to_u64(), *val);
        }
    }

    #[test]
    fn test_stack_id_ordering() {
        let a = StackId::from_u64(1);
        let b = StackId::from_u64(10);
        let c = StackId::from_u64(1);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, c);
    }

    #[test]
    fn test_stack_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(StackId::from_u64(1));
        set.insert(StackId::from_u64(2));
        assert!(set.contains(&StackId::from_u64(1)));
        assert!(set.contains(&StackId::from_u64(2)));
        assert_eq!(set.len(), 2);
        set.insert(StackId::from_u64(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_stack_id_default() {
        let id = StackId::default();
        assert!(id.to_u64() > 0);
    }

    #[test]
    fn test_stack_name_new_and_as_str() {
        let name = StackName::new("my-stack");
        assert_eq!(name.as_str(), "my-stack");
    }

    #[test]
    fn test_stack_name_display() {
        let name = StackName::new("feature-stack");
        assert_eq!(format!("{name}"), "feature-stack");
    }

    #[test]
    fn test_stack_name_as_ref() {
        let name = StackName::new("main-stack");
        let s: &str = name.as_ref();
        assert_eq!(s, "main-stack");
    }

    #[test]
    fn test_stack_name_equality() {
        let a = StackName::new("same");
        let b = StackName::new("same");
        let c = StackName::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_stack_name_serde_roundtrip() {
        let name = StackName::new("test-stack");
        let json = serde_json::to_string(&name).expect("serialize");
        let deserialized: StackName = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(name, deserialized);
    }

    #[test]
    fn test_commit_hash_new_and_as_str() {
        let hash = CommitHash::new("abc123def456");
        assert_eq!(hash.as_str(), "abc123def456");
    }

    #[test]
    fn test_commit_hash_display() {
        let hash = CommitHash::new("deadbeef");
        assert_eq!(format!("{hash}"), "deadbeef");
    }

    #[test]
    fn test_commit_hash_equality() {
        let a = CommitHash::new("abc");
        let b = CommitHash::new("abc");
        let c = CommitHash::new("def");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_commit_hash_serde_roundtrip() {
        let hash = CommitHash::new("a1b2c3d4");
        let json = serde_json::to_string(&hash).expect("serialize");
        let deserialized: CommitHash = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(hash, deserialized);
    }

    #[test]
    fn test_pr_info_new_defaults() {
        let pr = PrInfo::new(
            42,
            "https://github.com/org/repo/pull/42".to_string(),
            "Fix bug".to_string(),
            "Fixes a bug".to_string(),
            "alice".to_string(),
            false,
        );
        assert_eq!(pr.pr_number, 42);
        assert_eq!(pr.url, "https://github.com/org/repo/pull/42");
        assert_eq!(pr.title, "Fix bug");
        assert_eq!(pr.description, "Fixes a bug");
        assert_eq!(pr.author, "alice");
        assert!(!pr.draft);
        assert_eq!(pr.state, PrState::Open);
    }

    #[test]
    fn test_pr_info_with_state() {
        let pr = PrInfo::new(
            1,
            "url".to_string(),
            "title".to_string(),
            "desc".to_string(),
            "bob".to_string(),
            false,
        )
        .with_state(PrState::Merged);
        assert_eq!(pr.state, PrState::Merged);
    }

    #[test]
    fn test_pr_info_serde_roundtrip() {
        let pr = PrInfo::new(
            100,
            "https://example.com/pr/100".to_string(),
            "My PR".to_string(),
            "Description here".to_string(),
            "charlie".to_string(),
            true,
        );
        let json = serde_json::to_string(&pr).expect("serialize");
        let deserialized: PrInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(pr.pr_number, deserialized.pr_number);
        assert_eq!(pr.url, deserialized.url);
        assert_eq!(pr.title, deserialized.title);
        assert_eq!(pr.description, deserialized.description);
        assert_eq!(pr.author, deserialized.author);
        assert_eq!(pr.draft, deserialized.draft);
        assert_eq!(pr.state, deserialized.state);
    }

    #[test]
    fn test_pr_status_creation_and_fields() {
        let status = PrStatus {
            pr_number: 1,
            state: PrState::Open,
            checks_passed: true,
            reviews_approved: vec!["alice".to_string()],
            mergeable: true,
            conflict_resolution: None,
        };
        assert!(status.checks_passed);
        assert!(status.mergeable);
        assert!(status.conflict_resolution.is_none());
    }

    #[test]
    fn test_pr_status_serde_roundtrip_with_conflict() {
        let status = PrStatus {
            pr_number: 42,
            state: PrState::Closed,
            checks_passed: false,
            reviews_approved: vec!["alice".to_string(), "bob".to_string()],
            mergeable: false,
            conflict_resolution: Some(ConflictResolution {
                requires_rebase: true,
                conflicting_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            }),
        };
        let json = serde_json::to_string(&status).expect("serialize");
        let deserialized: PrStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status.pr_number, deserialized.pr_number);
        assert_eq!(status.reviews_approved, deserialized.reviews_approved);
        assert!(deserialized.conflict_resolution.is_some());
        let cr = deserialized.conflict_resolution.expect("should exist");
        assert!(cr.requires_rebase);
        assert_eq!(cr.conflicting_files.len(), 2);
    }

    #[test]
    fn test_stack_branch_new_defaults() {
        let branch = StackBranch::new(
            BranchName::new("feature-x"),
            0,
            CommitHash::new("abc123"),
            Some(BranchName::new("main")),
        );
        assert_eq!(branch.branch_name.as_str(), "feature-x");
        assert_eq!(branch.position, 0);
        assert!(branch.pr_info.is_none());
        assert_eq!(branch.state, BranchState::Open);
        assert_eq!(branch.last_commit.as_str(), "abc123");
        assert!(branch.parent_branch.is_some());
    }

    #[test]
    fn test_stack_branch_new_no_parent() {
        let branch = StackBranch::new(BranchName::new("root"), 0, CommitHash::new("abc"), None);
        assert!(branch.parent_branch.is_none());
    }

    #[test]
    fn test_stack_branch_with_pr_info() {
        let pr = PrInfo::new(
            1,
            "url".to_string(),
            "title".to_string(),
            "desc".to_string(),
            "author".to_string(),
            false,
        );
        let branch = StackBranch::new(BranchName::new("feat"), 0, CommitHash::new("abc"), None)
            .with_pr_info(pr);
        assert!(branch.pr_info.is_some());
        assert_eq!(branch.pr_info.as_ref().expect("pr").pr_number, 1);
    }

    #[test]
    fn test_stack_branch_transition_to() {
        let mut branch = StackBranch::new(BranchName::new("feat"), 0, CommitHash::new("abc"), None);
        assert_eq!(branch.state, BranchState::Open);
        branch.transition_to(BranchState::Draft);
        assert_eq!(branch.state, BranchState::Draft);
        branch.transition_to(BranchState::Merged);
        assert_eq!(branch.state, BranchState::Merged);
    }

    #[test]
    fn test_stack_branch_serde_roundtrip() {
        let pr = PrInfo::new(
            5,
            "url".to_string(),
            "title".to_string(),
            "desc".to_string(),
            "author".to_string(),
            true,
        );
        let branch = StackBranch::new(
            BranchName::new("feat-branch"),
            2,
            CommitHash::new("def456"),
            Some(BranchName::new("main")),
        )
        .with_pr_info(pr);
        let json = serde_json::to_string(&branch).expect("serialize");
        let deserialized: StackBranch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(branch.branch_name, deserialized.branch_name);
        assert_eq!(branch.position, deserialized.position);
        assert_eq!(branch.state, deserialized.state);
        assert!(deserialized.pr_info.is_some());
    }

    #[test]
    fn test_stack_new_empty() {
        let stack = Stack::new(
            StackId::from_u64(1),
            StackName::new("empty"),
            BranchName::new("main"),
        );
        assert!(stack.branches.is_empty());
        assert!(stack.is_draft());
        assert!(!stack.is_published());
        assert!(!stack.is_merged());
        assert!(stack.bottom_branch().is_none());
        assert!(stack.top_branch().is_none());
    }

    #[test]
    fn test_stack_with_branches() {
        let branches = vec![StackBranch::new(
            BranchName::new("only-branch"),
            0,
            CommitHash::new("abc"),
            None,
        )];
        let stack = Stack::new(
            StackId::from_u64(1),
            StackName::new("test"),
            BranchName::new("main"),
        )
        .with_branches(branches);
        assert_eq!(stack.branches.len(), 1);
    }

    #[test]
    fn test_stack_is_merged() {
        let mut stack = create_test_stack();
        assert!(!stack.is_merged());
        stack.transition_to(StackState::Merged);
        assert!(stack.is_merged());
    }

    #[test]
    fn test_stack_branch_at_position_found() {
        let stack = create_test_stack();
        let branch = stack.branch_at_position(0);
        assert!(branch.is_some());
        assert_eq!(branch.expect("branch").branch_name.as_str(), "feature-a");
    }

    #[test]
    fn test_stack_branch_at_position_not_found() {
        let stack = create_test_stack();
        assert!(stack.branch_at_position(99).is_none());
    }

    #[test]
    fn test_stack_branches_ordered() {
        let stack = create_test_stack();
        let ordered = stack.branches_ordered();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].position, 0);
        assert_eq!(ordered[1].position, 1);
    }

    #[test]
    fn test_stack_single_item() {
        let stack_id = StackId::from_u64(1);
        let base = BranchName::new("main");
        let mut stack = Stack::new(stack_id, StackName::new("single"), base);
        stack.add_branch(StackBranch::new(
            BranchName::new("only-feat"),
            0,
            CommitHash::new("abc"),
            Some(BranchName::new("main")),
        ));
        assert_eq!(stack.branches.len(), 1);
        let bottom = stack.bottom_branch().expect("bottom");
        let top = stack.top_branch().expect("top");
        assert_eq!(bottom.branch_name, top.branch_name);
    }

    #[test]
    fn test_stack_serde_roundtrip() {
        let mut stack = create_test_stack();
        stack.transition_to(StackState::Published);
        let json = serde_json::to_string(&stack).expect("serialize");
        let deserialized: Stack = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(stack.id, deserialized.id);
        assert_eq!(stack.name, deserialized.name);
        assert_eq!(stack.base_branch, deserialized.base_branch);
        assert_eq!(stack.branches.len(), deserialized.branches.len());
        assert_eq!(stack.state, deserialized.state);
    }

    #[test]
    fn test_stack_multiple_transitions() {
        let mut stack = create_test_stack();
        stack.transition_to(StackState::Published);
        assert!(stack.is_published());
        stack.transition_to(StackState::Merging);
        assert!(!stack.is_published());
        assert!(!stack.is_draft());
        stack.transition_to(StackState::Conflict);
        stack.transition_to(StackState::Failed);
    }

    #[test]
    fn test_stack_add_branch_updates_count() {
        let stack_id = StackId::from_u64(1);
        let base = BranchName::new("main");
        let mut stack = Stack::new(stack_id, StackName::new("test"), base);
        assert_eq!(stack.branches.len(), 0);

        stack.add_branch(StackBranch::new(
            BranchName::new("b1"),
            0,
            CommitHash::new("c1"),
            None,
        ));
        assert_eq!(stack.branches.len(), 1);

        stack.add_branch(StackBranch::new(
            BranchName::new("b2"),
            1,
            CommitHash::new("c2"),
            Some(BranchName::new("b1")),
        ));
        assert_eq!(stack.branches.len(), 2);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_stack_id_roundtrip(id in 0u64..u32::MAX as u64) {
            let stack_id = StackId::from_u64(id);
            assert_eq!(stack_id.to_u64(), id);
        }

        #[test]
        fn prop_stack_id_ordering(a in 0u64..10_000u64, b in 0u64..10_000u64) {
            let id_a = StackId::from_u64(a);
            let id_b = StackId::from_u64(b);
            assert_eq!(id_a < id_b, a < b);
            assert_eq!(id_a > id_b, a > b);
            assert_eq!(id_a == id_b, a == b);
        }

        #[test]
        fn prop_commit_hash_serde_roundtrip(s in ".{0,256}") {
            let hash = CommitHash::new(s);
            let json = serde_json::to_string(&hash).expect("serialize");
            let deserialized: CommitHash = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(hash, deserialized);
        }

        #[test]
        fn prop_stack_name_serde_roundtrip(s in ".{0,100}") {
            let name = StackName::new(s);
            let json = serde_json::to_string(&name).expect("serialize");
            let deserialized: StackName = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(name, deserialized);
        }

        #[test]
        fn prop_pr_number_valid(pr_num in 1u32..1_000_000u32) {
            let pr = PrInfo::new(
                pr_num,
                "url".to_string(),
                "title".to_string(),
                "desc".to_string(),
                "author".to_string(),
                false,
            );
            assert_eq!(pr.pr_number, pr_num);
        }
    }
}
