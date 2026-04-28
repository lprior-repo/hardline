use crate::{
    domain::{
        stack::{CommitHash, PrInfo, Stack, StackBranch, StackId},
        state::StackState,
        value_objects::BranchName,
    },
    error::Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    Squash,
    Merge,
    Rebase,
}

impl MergeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeMethod::Squash => "squash",
            MergeMethod::Merge => "merge",
            MergeMethod::Rebase => "rebase",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrMergeStatus {
    pub can_merge: bool,
    pub is_blocked: bool,
    pub status_text: String,
}

impl PrMergeStatus {
    pub fn ready() -> Self {
        Self {
            can_merge: true,
            is_blocked: false,
            status_text: "Ready to merge".to_string(),
        }
    }

    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            can_merge: false,
            is_blocked: true,
            status_text: reason.into(),
        }
    }

    pub fn not_ready(reason: impl Into<String>) -> Self {
        Self {
            can_merge: false,
            is_blocked: false,
            status_text: reason.into(),
        }
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.can_merge && !self.is_blocked
    }

    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        self.is_blocked
    }

    #[must_use]
    pub fn status_text(&self) -> &str {
        &self.status_text
    }
}

pub trait StackRepository: Send + Sync {
    fn save(&self, stack: &Stack) -> Result<()>;
    fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>>;
    fn find_by_branch(&self, branch: &BranchName) -> Result<Option<Stack>>;
    fn find_by_pr(&self, pr_number: u32) -> Result<Option<Stack>>;
    fn list_all(&self) -> Result<Vec<Stack>>;
    fn list_by_state(&self, state: StackState) -> Result<Vec<Stack>>;
    fn delete(&self, id: &StackId) -> Result<()>;
}

pub trait GitHubClientTrait: Send + Sync {
    fn create_pull_request(&self, branch: &StackBranch, base_branch: &BranchName)
        -> Result<PrInfo>;

    fn update_pull_request(
        &self,
        pr_number: u32,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<PrInfo>;

    fn get_pull_request(&self, pr_number: u32) -> Result<PrInfo>;

    fn find_pr(&self, branch_name: &str) -> Result<Option<PrInfo>>;

    fn is_pr_merged(&self, pr_number: u32) -> Result<bool>;

    fn get_pr_merge_status(&self, pr_number: u32) -> Result<PrMergeStatus>;

    fn merge_pull_request(&self, pr_number: u32) -> Result<()>;

    fn merge_pr(
        &self,
        pr_number: u32,
        merge_method: MergeMethod,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<()>;

    fn update_pr_base(&self, pr_number: u32, base_branch: &BranchName) -> Result<()>;

    fn force_push(&self, branch: &BranchName) -> Result<()>;

    fn fetch(&self, branch: &BranchName) -> Result<()>;

    fn get_commit_hash(&self, branch: &BranchName) -> Result<CommitHash>;
}

pub trait VcsClientTrait: Send + Sync {
    fn rebase(&self, branch: &BranchName, onto: &BranchName) -> Result<()>;
    fn get_current_commit(&self, branch: &BranchName) -> Result<CommitHash>;
    fn get_parent_commit(&self, branch: &BranchName) -> Result<Option<BranchName>>;
}
