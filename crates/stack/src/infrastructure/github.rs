use crate::application::traits::ForgeClientTrait;
use crate::domain::stack::{PrInfo, StackBranch};
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};
use scp_vcs::domain::types::CommitHash;

pub struct GitHubClientImpl {
    owner: String,
    repo: String,
    client: octocrab::Octocrab,
}

impl GitHubClientImpl {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            client: octocrab::Octocrab::default(),
        }
    }

    pub fn with_client(
        owner: impl Into<String>,
        repo: impl Into<String>,
        client: octocrab::Octocrab,
    ) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            client,
        }
    }
}

impl ForgeClientTrait for GitHubClientImpl {
    fn create_pull_request(
        &self,
        branch: &StackBranch,
        base_branch: &BranchName,
    ) -> Result<PrInfo, StackError> {
        self.client
            .pulls(&self.owner, &self.repo)
            .create(
                format!("PR for {}", branch.branch_name),
                branch.branch_name.to_string(),
                base_branch.to_string(),
            )
            .body(format!(
                "Auto-generated PR for stack branch {}",
                branch.branch_name
            ))
            .send()
            .map_err(|e| StackError::GitHubError(e.to_string()))
            .map(|pr| {
                PrInfo::new(
                    pr.number,
                    pr.html_url.to_string(),
                    pr.title.unwrap_or_default(),
                    pr.body.unwrap_or_default(),
                    pr.user.login,
                    pr.draft,
                )
            })
    }

    fn update_pull_request(
        &self,
        pr_number: u32,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<PrInfo, StackError> {
        self.client
            .pulls(&self.owner, &self.repo)
            .update(pr_number)
            .title(title.map(|t| t.as_str()))
            .body(body.as_ref().map(|b| b.as_str()))
            .send()
            .map_err(|e| StackError::GitHubError(e.to_string()))
            .map(|pr| {
                PrInfo::new(
                    pr.number,
                    pr.html_url.to_string(),
                    pr.title.unwrap_or_default(),
                    pr.body.unwrap_or_default(),
                    pr.user.login,
                    pr.draft,
                )
            })
    }

    fn get_pull_request(&self, pr_number: u32) -> Result<PrInfo, StackError> {
        self.client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .map_err(|e| StackError::GitHubError(e.to_string()))
            .map(|pr| {
                PrInfo::new(
                    pr.number,
                    pr.html_url.to_string(),
                    pr.title.unwrap_or_default(),
                    pr.body.unwrap_or_default(),
                    pr.user.login,
                    pr.draft,
                )
            })
    }

    fn merge_pull_request(&self, pr_number: u32) -> Result<(), StackError> {
        self.client
            .pulls(&self.owner, &self.repo)
            .merge(pr_number)
            .map_err(|e| StackError::GitHubError(e.to_string()))?;
        Ok(())
    }

    fn force_push(&self, _branch: &BranchName) -> Result<(), StackError> {
        Err(StackError::GitHubError(
            "Force push not yet implemented via this API".to_string(),
        ))
    }

    fn fetch(&self, _branch: &BranchName) -> Result<(), StackError> {
        Err(StackError::GitHubError(
            "Fetch not yet implemented via this API".to_string(),
        ))
    }

    fn get_commit_hash(&self, _branch: &BranchName) -> Result<CommitHash, StackError> {
        Err(StackError::GitHubError(
            "Get commit hash not yet implemented via this API".to_string(),
        ))
    }
}
