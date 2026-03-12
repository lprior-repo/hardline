use crate::domain::entities::PrInfo;
use crate::error::{Result, StackError};

pub struct GitHubClient {
    _owner: String,
    _repo: String,
}

impl GitHubClient {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            _owner: owner.into(),
            _repo: repo.into(),
        }
    }

    pub fn get_pull_request(&self, _pr_number: u32) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn list_pull_requests(&self) -> Result<Vec<PrInfo>> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn create_pull_request(
        &self,
        _title: String,
        _head: String,
        _base: String,
    ) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }

    pub fn update_pull_request(
        &self,
        _pr_number: u32,
        _title: Option<String>,
        _body: Option<String>,
    ) -> Result<PrInfo> {
        Err(StackError::GitHubError("Not yet implemented".to_string()))
    }
}
