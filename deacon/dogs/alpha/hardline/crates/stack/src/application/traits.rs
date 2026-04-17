use crate::domain::stack::{Stack, StackBranch, StackId};
use crate::domain::state::StackState;
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};

pub trait StackRepository: Send + Sync {
    fn save(&self, stack: &Stack) -> Result<(), StackError>;
    fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>, StackError>;
    fn find_by_branch(&self, branch: &BranchName) -> Result<Option<Stack>, StackError>;
    fn find_by_pr(&self, pr_number: u32) -> Result<Option<Stack>, StackError>;
    fn list_all(&self) -> Result<Vec<Stack>, StackError>;
    fn list_by_state(&self, state: StackState) -> Result<Vec<Stack>, StackError>;
    fn delete(&self, id: &StackId) -> Result<(), StackError>;
}

pub trait GitHubClientTrait: Send + Sync {
    fn create_pull_request(
        &self,
        branch: &StackBranch,
        base_branch: &BranchName,
    ) -> Result<crate::domain::stack::PrInfo, StackError>;

    fn update_pull_request(
        &self,
        pr_number: u32,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<crate::domain::stack::PrInfo, StackError>;

    fn get_pull_request(&self, pr_number: u32) -> Result<crate::domain::stack::PrInfo, StackError>;

    fn merge_pull_request(&self, pr_number: u32) -> Result<(), StackError>;

    fn force_push(&self, branch: &BranchName) -> Result<(), StackError>;

    fn fetch(&self, branch: &BranchName) -> Result<(), StackError>;

    fn get_commit_hash(
        &self,
        branch: &BranchName,
    ) -> Result<scp_vcs::domain::types::CommitHash, StackError>;
}

pub trait VcsClientTrait: Send + Sync {
    fn rebase(&self, branch: &BranchName, onto: &BranchName) -> Result<(), StackError>;
    fn get_current_commit(
        &self,
        branch: &BranchName,
    ) -> Result<scp_vcs::domain::types::CommitHash, StackError>;
    fn get_parent_commit(&self, branch: &BranchName) -> Result<Option<BranchName>, StackError>;
}
