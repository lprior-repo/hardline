use crate::application::traits::{GitHubClientTrait, MergeMethod, PrMergeStatus};
use crate::domain::stack::{CommitHash, PrInfo, StackBranch};
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};

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

    fn block_on<F: std::future::Future>(&self, f: F) -> F::Output {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(f)
    }
}

fn pr_from_octocrab(pr: octocrab::models::pulls::PullRequest) -> PrInfo {
    let author = pr
        .user
        .as_ref()
        .map(|u| u.login.as_str())
        .unwrap_or("unknown");
    let url = pr
        .html_url
        .as_ref()
        .map(|u| u.to_string())
        .unwrap_or_default();
    PrInfo::new(
        pr.number as u32,
        url,
        pr.title.clone().unwrap_or_default(),
        pr.body.clone().unwrap_or_default(),
        author.to_string(),
        pr.draft.unwrap_or(false),
    )
}

impl GitHubClientTrait for GitHubClientImpl {
    fn create_pull_request(
        &self,
        branch: &StackBranch,
        base_branch: &BranchName,
    ) -> Result<PrInfo> {
        self.block_on(
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
                .send(),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))
        .map(pr_from_octocrab)
    }

    fn update_pull_request(
        &self,
        pr_number: u32,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<PrInfo> {
        let pulls = self.client.pulls(&self.owner, &self.repo);
        let mut update_builder = pulls.update(pr_number as u64);

        if let Some(t) = title {
            update_builder = update_builder.title(t.as_str());
        }
        if let Some(b) = body {
            update_builder = update_builder.body(b.as_str());
        }

        self.block_on(update_builder.send())
            .map_err(|e| StackError::GitHubError(e.to_string()))
            .map(pr_from_octocrab)
    }

    fn get_pull_request(&self, pr_number: u32) -> Result<PrInfo> {
        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .get(pr_number as u64),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))
        .map(pr_from_octocrab)
    }

    fn find_pr(&self, branch_name: &str) -> Result<Option<PrInfo>> {
        let pulls = self
            .block_on(
                self.client
                    .pulls(&self.owner, &self.repo)
                    .list()
                    .head(&format!("{}:{}", self.owner, branch_name))
                    .send(),
            )
            .map_err(|e| StackError::GitHubError(e.to_string()))?;

        if pulls.items.is_empty() {
            return Ok(None);
        }

        Ok(Some(pr_from_octocrab(
            pulls.items.into_iter().next().expect("item"),
        )))
    }

    fn is_pr_merged(&self, pr_number: u32) -> Result<bool> {
        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .get(pr_number as u64),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))
        .map(|pr| pr.merged.unwrap_or(false))
    }

    fn get_pr_merge_status(&self, pr_number: u32) -> Result<PrMergeStatus> {
        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .get(pr_number as u64),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))
        .map(|pr| {
            if pr.merged.unwrap_or(false) {
                return PrMergeStatus::ready();
            }

            if pr.state == Some(octocrab::models::IssueState::Closed) {
                return PrMergeStatus::blocked("PR is closed");
            }

            if !pr.mergeable.unwrap_or(true) {
                return PrMergeStatus::blocked("PR is not mergeable");
            }

            if pr.mergeable_state == Some(octocrab::models::pulls::MergeableState::Blocked) {
                return PrMergeStatus::blocked("PR is blocked by required checks");
            }

            PrMergeStatus::ready()
        })
    }

    fn merge_pull_request(&self, pr_number: u32) -> Result<()> {
        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .merge(pr_number as u64)
                .send(),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))?;
        Ok(())
    }

    fn merge_pr(
        &self,
        pr_number: u32,
        merge_method: MergeMethod,
        _title: Option<String>,
        _body: Option<String>,
    ) -> Result<()> {
        let method = match merge_method {
            MergeMethod::Squash => octocrab::params::pulls::MergeMethod::Squash,
            MergeMethod::Merge => octocrab::params::pulls::MergeMethod::Merge,
            MergeMethod::Rebase => octocrab::params::pulls::MergeMethod::Rebase,
        };

        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .merge(pr_number as u64)
                .method(method)
                .send(),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))?;
        Ok(())
    }

    fn update_pr_base(&self, pr_number: u32, base_branch: &BranchName) -> Result<()> {
        self.block_on(
            self.client
                .pulls(&self.owner, &self.repo)
                .update(pr_number as u64)
                .base(base_branch.as_str())
                .send(),
        )
        .map_err(|e| StackError::GitHubError(e.to_string()))?;
        Ok(())
    }

    fn force_push(&self, _branch: &BranchName) -> Result<()> {
        Err(StackError::GitHubError(
            "Force push not yet implemented via this API".to_string(),
        ))
    }

    fn fetch(&self, _branch: &BranchName) -> Result<()> {
        Err(StackError::GitHubError(
            "Fetch not yet implemented via this API".to_string(),
        ))
    }

    fn get_commit_hash(&self, _branch: &BranchName) -> Result<CommitHash> {
        Err(StackError::GitHubError(
            "Get commit hash not yet implemented via this API".to_string(),
        ))
    }
}
