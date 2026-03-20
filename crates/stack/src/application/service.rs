use chrono::Utc;

use crate::application::traits::{GitHubClientTrait, StackRepository, VcsClientTrait};
use crate::domain::stack::{PrInfo, Stack, StackBranch, StackId};
use crate::domain::state::{BranchState, StackState};
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};
use scp_vcs::domain::types::CommitHash;

pub struct StackService<R, G, V> {
    stack_repo: R,
    github: G,
    vcs: V,
}

impl<R, G, V> StackService<R, G, V> {
    pub fn new(stack_repo: R, github: G, vcs: V) -> Self {
        Self {
            stack_repo,
            github,
            vcs,
        }
    }
}

impl<R: StackRepository, G: GitHubClientTrait, V: VcsClientTrait> StackService<R, G, V> {
    pub fn create_stack(
        &self,
        base_branch: BranchName,
        head_branch: BranchName,
        name: String,
    ) -> Result<Stack, StackError> {
        let branches = self.build_stack_tree(&base_branch, &head_branch)?;

        let stack = Stack {
            id: StackId::new(),
            name,
            base_branch,
            branches,
            state: StackState::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn publish_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        for branch in &stack.branches {
            let pr_info = self
                .github
                .create_pull_request(branch, &stack.base_branch)?;
            branch.pr_info = Some(pr_info);
        }

        stack.state = StackState::Published;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn restack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        self.github.fetch(&stack.base_branch)?;

        for (i, branch) in stack.branches.iter_mut().enumerate() {
            let parent = if i == 0 {
                stack.base_branch.clone()
            } else {
                stack.branches[i - 1].branch_name.clone()
            };

            self.vcs.rebase(&branch.branch_name, &parent)?;
            self.github.force_push(&branch.branch_name)?;
        }

        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn merge_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        stack.state = StackState::Merging;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        for branch in &mut stack.branches {
            if let Some(pr_info) = &branch.pr_info {
                self.github.merge_pull_request(pr_info.pr_number)?;
                branch.state = BranchState::Merged;
            }
        }

        stack.state = StackState::Merged;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn add_branch_to_stack(
        &self,
        stack_id: StackId,
        branch_name: BranchName,
        parent_branch: Option<BranchName>,
    ) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        let position = stack.branches.len() as u32;
        let last_commit = self.github.get_commit_hash(&branch_name)?;

        let new_branch = StackBranch {
            branch_name: branch_name.clone(),
            position,
            pr_info: None,
            state: BranchState::Open,
            last_commit,
            parent_branch: parent_branch
                .or_else(|| stack.branches.last().map(|b| b.branch_name.clone())),
        };

        stack.add_branch(new_branch);
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn remove_branch_from_stack(
        &self,
        stack_id: StackId,
        branch_name: &BranchName,
    ) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        let position_to_remove = stack
            .branches
            .iter()
            .position(|b| &b.branch_name == branch_name)
            .ok_or(StackError::BranchNotFound(branch_name.to_string()))?;

        stack.branches.remove(position_to_remove);

        for (i, branch) in stack.branches.iter_mut().enumerate() {
            branch.position = i as u32;
        }

        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn close_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        for branch in &mut stack.branches {
            if let Some(pr_info) = &branch.pr_info {
                branch.state = BranchState::Closed;
                let _ = self.github.update_pull_request(
                    pr_info.pr_number,
                    None,
                    Some("Stack closed".to_string()),
                );
            }
        }

        stack.state = StackState::Failed;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    fn build_stack_tree(
        &self,
        base: &BranchName,
        head: &BranchName,
    ) -> Result<Vec<StackBranch>, StackError> {
        let mut branches = Vec::new();
        let mut current = head.clone();

        loop {
            let commit = self.vcs.get_current_commit(&current)?;
            let parent = self.vcs.get_parent_commit(&current)?;

            let position = branches.len() as u32;
            let parent_branch = if branches.is_empty() {
                None
            } else {
                Some(
                    branches
                        .last()
                        .map(|b| b.branch_name.clone())
                        .unwrap_or_else(|| base.clone()),
                )
            };

            branches.push(StackBranch {
                branch_name: current.clone(),
                position,
                pr_info: None,
                state: BranchState::Open,
                last_commit: commit,
                parent_branch,
            });

            if current == *base {
                break;
            }

            current = parent.ok_or_else(|| {
                StackError::GitError(format!("Could not find parent of branch {}", current))
            })?;
        }

        branches.reverse();

        for (i, branch) in branches.iter_mut().enumerate() {
            branch.position = i as u32;
        }

        Ok(branches)
    }
}

pub fn assert_branch_order(stack: &Stack) -> Result<(), StackError> {
    for window in stack.branches.windows(2) {
        let lower = &window[0];
        let higher = &window[1];
        if higher.position <= lower.position {
            return Err(StackError::InvalidBranchName(format!(
                "Branch {} has position {} which is not greater than {}",
                higher.branch_name, higher.position, lower.position
            )));
        }
        if higher.parent_branch.as_ref() != Some(&lower.branch_name) {
            return Err(StackError::InvalidBranchName(format!(
                "Branch {} parent {:?} does not match expected {:?}",
                higher.branch_name,
                higher.parent_branch,
                Some(&lower.branch_name)
            )));
        }
    }
    Ok(())
}

pub fn assert_base_not_in_stack(stack: &Stack) -> Result<(), StackError> {
    if stack
        .branches
        .iter()
        .any(|b| b.branch_name == stack.base_branch)
    {
        return Err(StackError::InvalidBranchName(format!(
            "Base branch {} should not be in branches list",
            stack.base_branch
        )));
    }
    Ok(())
}

pub fn assert_unique_branch_names(stack: &Stack) -> Result<(), StackError> {
    use std::collections::HashSet;
    let mut names = HashSet::new();
    for branch in &stack.branches {
        if !names.insert(&branch.branch_name) {
            return Err(StackError::InvalidBranchName(format!(
                "Duplicate branch name: {}",
                branch.branch_name
            )));
        }
    }
    Ok(())
}

pub fn assert_draft_stack_no_prs(stack: &Stack) -> Result<(), StackError> {
    if stack.state == StackState::Draft {
        for branch in &stack.branches {
            if branch.pr_info.is_some() {
                return Err(StackError::InvalidBranchName(
                    "Draft stack should not have PRs".to_string(),
                ));
            }
        }
    }
    Ok(())
}

pub fn assert_published_stack_has_prs(stack: &Stack) -> Result<(), StackError> {
    if stack.state == StackState::Published {
        for branch in &stack.branches {
            if branch.pr_info.is_none() {
                return Err(StackError::InvalidBranchName(
                    "Published stack must have all PRs".to_string(),
                ));
            }
        }
    }
    Ok(())
}

pub fn assert_merged_stack_all_merged(stack: &Stack) -> Result<(), StackError> {
    if stack.state == StackState::Merged {
        for branch in &stack.branches {
            if branch.state != BranchState::Merged {
                return Err(StackError::InvalidBranchName(format!(
                    "Branch {} should be merged but is {:?}",
                    branch.branch_name, branch.state
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::stack::PrInfo as DomainPrInfo;
    use crate::domain::state::PrState;
    use std::sync::Arc;

    struct MockRepo {
        stacks: std::sync::Mutex<Vec<Stack>>,
    }

    impl MockRepo {
        fn new() -> Self {
            Self {
                stacks: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl StackRepository for MockRepo {
        fn save(&self, stack: &Stack) -> Result<(), StackError> {
            let mut stacks = self
                .stacks
                .lock()
                .map_err(|_| StackError::NotFound("lock error".to_string()))?;
            if let Some(pos) = stacks.iter().position(|s| s.id == stack.id) {
                stacks[pos] = stack.clone();
            } else {
                stacks.push(stack.clone());
            }
            Ok(())
        }

        fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>, StackError> {
            let stacks = self
                .stacks
                .lock()
                .map_err(|_| StackError::NotFound("lock error".to_string()))?;
            Ok(stacks.iter().find(|s| s.id == *id).cloned())
        }

        fn find_by_branch(&self, _branch: &BranchName) -> Result<Option<Stack>, StackError> {
            Ok(None)
        }

        fn find_by_pr(&self, _pr_number: u32) -> Result<Option<Stack>, StackError> {
            Ok(None)
        }

        fn list_all(&self) -> Result<Vec<Stack>, StackError> {
            let stacks = self
                .stacks
                .lock()
                .map_err(|_| StackError::NotFound("lock error".to_string()))?;
            Ok(stacks.clone())
        }

        fn list_by_state(&self, _state: StackState) -> Result<Vec<Stack>, StackError> {
            Ok(Vec::new())
        }

        fn delete(&self, _id: &StackId) -> Result<(), StackError> {
            Ok(())
        }
    }

    struct MockGitHub;

    impl GitHubClientTrait for MockGitHub {
        fn create_pull_request(
            &self,
            _branch: &StackBranch,
            _base_branch: &BranchName,
        ) -> Result<DomainPrInfo, StackError> {
            Ok(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Test PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            ))
        }

        fn update_pull_request(
            &self,
            _pr_number: u32,
            _title: Option<String>,
            _body: Option<String>,
        ) -> Result<DomainPrInfo, StackError> {
            Ok(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Updated PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            ))
        }

        fn get_pull_request(&self, _pr_number: u32) -> Result<DomainPrInfo, StackError> {
            Ok(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Test PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            ))
        }

        fn merge_pull_request(&self, _pr_number: u32) -> Result<(), StackError> {
            Ok(())
        }

        fn force_push(&self, _branch: &BranchName) -> Result<(), StackError> {
            Ok(())
        }

        fn fetch(&self, _branch: &BranchName) -> Result<(), StackError> {
            Ok(())
        }

        fn get_commit_hash(&self, _branch: &BranchName) -> Result<CommitHash, StackError> {
            Ok(CommitHash::new("abc123"))
        }
    }

    struct MockVcs;

    impl VcsClientTrait for MockVcs {
        fn rebase(&self, _branch: &BranchName, _onto: &BranchName) -> Result<(), StackError> {
            Ok(())
        }

        fn get_current_commit(&self, _branch: &BranchName) -> Result<CommitHash, StackError> {
            Ok(CommitHash::new("abc123"))
        }

        fn get_parent_commit(
            &self,
            _branch: &BranchName,
        ) -> Result<Option<BranchName>, StackError> {
            Ok(None)
        }
    }

    #[test]
    fn test_create_stack() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let base = BranchName::new("main");
        let head = BranchName::new("feature-a");

        let result = service.create_stack(base, head, "test-stack".to_string());
        assert!(result.is_ok());
    }
}
