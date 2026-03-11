use crate::domain::entities::{PrInfo, PrState};
use crate::error::{Result, StackError};

pub struct GitHubClient {
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }

    pub fn get_pull_request(&self, pr_number: u32) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn list_pull_requests(&self) -> Result<Vec<PrInfo>> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn create_pull_request(&self, title: String, head: String, base: String) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn update_pull_request(
        &self,
        pr_number: u32,
        title: Option<String>,
        body: Option<String>,
    ) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }
}
