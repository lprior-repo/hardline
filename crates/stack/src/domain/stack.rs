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
}
