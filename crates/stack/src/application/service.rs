use chrono::Utc;

use crate::application::traits::{GitHubClientTrait, StackRepository, VcsClientTrait};
use crate::domain::stack::{Stack, StackBranch, StackId};
use crate::domain::state::{BranchState, StackState};
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};

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
    ) -> Result<Stack> {
        let branches = self.build_stack_tree(&base_branch, &head_branch)?;

        let stack = Stack {
            id: StackId::new(),
            name: crate::domain::stack::StackName::new(name),
            base_branch,
            branches,
            state: StackState::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn publish_stack(&self, stack_id: StackId) -> Result<Stack> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        for branch in &mut stack.branches {
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

    pub fn restack(&self, stack_id: StackId) -> Result<Stack> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or(StackError::NotFound(stack_id.to_string()))?;

        self.github.fetch(&stack.base_branch)?;

        let branch_names: Vec<_> = stack
            .branches
            .iter()
            .map(|b| b.branch_name.clone())
            .collect();

        for (i, branch) in stack.branches.iter_mut().enumerate() {
            let parent = if i == 0 {
                stack.base_branch.clone()
            } else {
                branch_names[i - 1].clone()
            };

            self.vcs.rebase(&branch.branch_name, &parent)?;
            self.github.force_push(&branch.branch_name)?;
        }

        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        Ok(stack)
    }

    pub fn merge_stack(&self, stack_id: StackId) -> Result<Stack> {
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
    ) -> Result<Stack> {
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
    ) -> Result<Stack> {
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

    pub fn close_stack(&self, stack_id: StackId) -> Result<Stack> {
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

    fn build_stack_tree(&self, base: &BranchName, head: &BranchName) -> Result<Vec<StackBranch>> {
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
                        .map(|b: &StackBranch| b.branch_name.clone())
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

pub fn assert_branch_order(stack: &Stack) -> Result<()> {
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

pub fn assert_base_not_in_stack(stack: &Stack) -> Result<()> {
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

pub fn assert_unique_branch_names(stack: &Stack) -> Result<()> {
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

pub fn assert_draft_stack_no_prs(stack: &Stack) -> Result<()> {
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

pub fn assert_published_stack_has_prs(stack: &Stack) -> Result<()> {
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

pub fn assert_merged_stack_all_merged(stack: &Stack) -> Result<()> {
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
    use crate::domain::stack::{CommitHash, PrInfo as DomainPrInfo};
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

    impl StackRepository for Arc<MockRepo> {
        fn save(&self, stack: &Stack) -> Result<()> {
            self.as_ref().save(stack)
        }

        fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>> {
            self.as_ref().find_by_id(id)
        }

        fn find_by_branch(&self, branch: &BranchName) -> Result<Option<Stack>> {
            self.as_ref().find_by_branch(branch)
        }

        fn find_by_pr(&self, pr_number: u32) -> Result<Option<Stack>> {
            self.as_ref().find_by_pr(pr_number)
        }

        fn list_all(&self) -> Result<Vec<Stack>> {
            self.as_ref().list_all()
        }

        fn list_by_state(&self, state: StackState) -> Result<Vec<Stack>> {
            self.as_ref().list_by_state(state)
        }

        fn delete(&self, id: &StackId) -> Result<()> {
            self.as_ref().delete(id)
        }
    }

    impl StackRepository for MockRepo {
        fn save(&self, stack: &Stack) -> Result<()> {
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

        fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>> {
            let stacks = self
                .stacks
                .lock()
                .map_err(|_| StackError::NotFound("lock error".to_string()))?;
            Ok(stacks.iter().find(|s| s.id == *id).cloned())
        }

        fn find_by_branch(&self, _branch: &BranchName) -> Result<Option<Stack>> {
            Ok(None)
        }

        fn find_by_pr(&self, _pr_number: u32) -> Result<Option<Stack>> {
            Ok(None)
        }

        fn list_all(&self) -> Result<Vec<Stack>> {
            let stacks = self
                .stacks
                .lock()
                .map_err(|_| StackError::NotFound("lock error".to_string()))?;
            Ok(stacks.clone())
        }

        fn list_by_state(&self, _state: StackState) -> Result<Vec<Stack>> {
            Ok(Vec::new())
        }

        fn delete(&self, _id: &StackId) -> Result<()> {
            Ok(())
        }
    }

    struct MockGitHub;

    impl GitHubClientTrait for MockGitHub {
        fn create_pull_request(
            &self,
            _branch: &StackBranch,
            _base_branch: &BranchName,
        ) -> Result<DomainPrInfo> {
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
        ) -> Result<DomainPrInfo> {
            Ok(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Updated PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            ))
        }

        fn get_pull_request(&self, _pr_number: u32) -> Result<DomainPrInfo> {
            Ok(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Test PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            ))
        }

        fn find_pr(&self, _branch_name: &str) -> Result<Option<DomainPrInfo>> {
            Ok(Some(DomainPrInfo::new(
                1,
                "https://github.com/test/1".to_string(),
                "Test PR".to_string(),
                "Description".to_string(),
                "author".to_string(),
                false,
            )))
        }

        fn is_pr_merged(&self, _pr_number: u32) -> Result<bool> {
            Ok(false)
        }

        fn get_pr_merge_status(
            &self,
            _pr_number: u32,
        ) -> Result<crate::application::traits::PrMergeStatus> {
            Ok(crate::application::traits::PrMergeStatus::ready())
        }

        fn merge_pull_request(&self, _pr_number: u32) -> Result<()> {
            Ok(())
        }

        fn merge_pr(
            &self,
            _pr_number: u32,
            _merge_method: crate::application::traits::MergeMethod,
            _title: Option<String>,
            _body: Option<String>,
        ) -> Result<()> {
            Ok(())
        }

        fn update_pr_base(&self, _pr_number: u32, _base_branch: &BranchName) -> Result<()> {
            Ok(())
        }

        fn force_push(&self, _branch: &BranchName) -> Result<()> {
            Ok(())
        }

        fn fetch(&self, _branch: &BranchName) -> Result<()> {
            Ok(())
        }

        fn get_commit_hash(&self, _branch: &BranchName) -> Result<CommitHash> {
            Ok(CommitHash::new("abc123"))
        }
    }

    impl GitHubClientTrait for Arc<MockGitHub> {
        fn create_pull_request(
            &self,
            branch: &StackBranch,
            base_branch: &BranchName,
        ) -> Result<DomainPrInfo> {
            self.as_ref().create_pull_request(branch, base_branch)
        }

        fn update_pull_request(
            &self,
            pr_number: u32,
            title: Option<String>,
            body: Option<String>,
        ) -> Result<DomainPrInfo> {
            self.as_ref().update_pull_request(pr_number, title, body)
        }

        fn get_pull_request(&self, pr_number: u32) -> Result<DomainPrInfo> {
            self.as_ref().get_pull_request(pr_number)
        }

        fn find_pr(&self, branch_name: &str) -> Result<Option<DomainPrInfo>> {
            self.as_ref().find_pr(branch_name)
        }

        fn is_pr_merged(&self, pr_number: u32) -> Result<bool> {
            self.as_ref().is_pr_merged(pr_number)
        }

        fn get_pr_merge_status(
            &self,
            pr_number: u32,
        ) -> Result<crate::application::traits::PrMergeStatus> {
            self.as_ref().get_pr_merge_status(pr_number)
        }

        fn merge_pull_request(&self, pr_number: u32) -> Result<()> {
            self.as_ref().merge_pull_request(pr_number)
        }

        fn merge_pr(
            &self,
            pr_number: u32,
            merge_method: crate::application::traits::MergeMethod,
            title: Option<String>,
            body: Option<String>,
        ) -> Result<()> {
            self.as_ref().merge_pr(pr_number, merge_method, title, body)
        }

        fn update_pr_base(&self, pr_number: u32, base_branch: &BranchName) -> Result<()> {
            self.as_ref().update_pr_base(pr_number, base_branch)
        }

        fn force_push(&self, branch: &BranchName) -> Result<()> {
            self.as_ref().force_push(branch)
        }

        fn fetch(&self, branch: &BranchName) -> Result<()> {
            self.as_ref().fetch(branch)
        }

        fn get_commit_hash(&self, branch: &BranchName) -> Result<CommitHash> {
            self.as_ref().get_commit_hash(branch)
        }
    }

    struct MockVcs;

    impl VcsClientTrait for MockVcs {
        fn rebase(&self, _branch: &BranchName, _onto: &BranchName) -> Result<()> {
            Ok(())
        }

        fn get_current_commit(&self, _branch: &BranchName) -> Result<CommitHash> {
            Ok(CommitHash::new("abc123"))
        }

        fn get_parent_commit(&self, branch: &BranchName) -> Result<Option<BranchName>> {
            if branch.as_str() == "main" {
                Ok(None)
            } else {
                Ok(Some(BranchName::new("main")))
            }
        }
    }

    impl VcsClientTrait for Arc<MockVcs> {
        fn rebase(&self, branch: &BranchName, onto: &BranchName) -> Result<()> {
            self.as_ref().rebase(branch, onto)
        }

        fn get_current_commit(&self, branch: &BranchName) -> Result<CommitHash> {
            self.as_ref().get_current_commit(branch)
        }

        fn get_parent_commit(&self, branch: &BranchName) -> Result<Option<BranchName>> {
            self.as_ref().get_parent_commit(branch)
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
        if let Err(e) = &result {
            eprintln!("test_create_stack error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_stack_same_head_as_base() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let base = BranchName::new("main");
        let head = BranchName::new("main");

        let result = service.create_stack(base, head, "single-branch".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_service_new() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let _service = StackService::new(repo, github, vcs);
    }

    #[test]
    fn test_publish_stack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.publish_stack(StackId::from_u64(999));
        assert!(result.is_err());
        let err = result.err().expect("should be error");
        assert!(matches!(err, StackError::NotFound(_)));
    }

    #[test]
    fn test_restack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.restack(StackId::from_u64(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_stack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.merge_stack(StackId::from_u64(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_add_branch_to_stack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.add_branch_to_stack(
            StackId::from_u64(999),
            BranchName::new("new-branch"),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_branch_from_stack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service
            .remove_branch_from_stack(StackId::from_u64(999), &BranchName::new("some-branch"));
        assert!(result.is_err());
    }

    #[test]
    fn test_close_stack_not_found() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.close_stack(StackId::from_u64(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_repo_save_and_find() {
        let repo = MockRepo::new();
        let base = BranchName::new("main");
        let stack = crate::domain::stack::Stack::new(
            StackId::from_u64(42),
            crate::domain::stack::StackName::new("test"),
            base,
        );
        repo.save(&stack).expect("save");
        let found = repo.find_by_id(&StackId::from_u64(42)).expect("find");
        assert!(found.is_some());
        assert_eq!(found.expect("stack").name.as_str(), "test");
    }

    #[test]
    fn test_mock_repo_list_all() {
        let repo = MockRepo::new();
        let all = repo.list_all().expect("list");
        assert!(all.is_empty());

        let base = BranchName::new("main");
        let stack = crate::domain::stack::Stack::new(
            StackId::from_u64(1),
            crate::domain::stack::StackName::new("s1"),
            base,
        );
        repo.save(&stack).expect("save");

        let all = repo.list_all().expect("list");
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_mock_repo_update_existing() {
        let repo = MockRepo::new();
        let base = BranchName::new("main");
        let mut stack = crate::domain::stack::Stack::new(
            StackId::from_u64(1),
            crate::domain::stack::StackName::new("original"),
            base,
        );
        repo.save(&stack).expect("save");

        // Update
        stack.name = crate::domain::stack::StackName::new("updated");
        repo.save(&stack).expect("save");

        let found = repo.find_by_id(&StackId::from_u64(1)).expect("find");
        assert_eq!(found.expect("stack").name.as_str(), "updated");
    }

    #[test]
    fn test_mock_repo_delete() {
        let repo = MockRepo::new();
        assert!(repo.delete(&StackId::from_u64(1)).is_ok());
    }

    #[test]
    fn test_mock_repo_find_by_branch_returns_none() {
        let repo = MockRepo::new();
        let result = repo.find_by_branch(&BranchName::new("nonexistent"));
        assert!(result.expect("ok").is_none());
    }

    #[test]
    fn test_mock_repo_find_by_pr_returns_none() {
        let repo = MockRepo::new();
        let result = repo.find_by_pr(999);
        assert!(result.expect("ok").is_none());
    }

    #[test]
    fn test_mock_repo_list_by_state_returns_empty() {
        let repo = MockRepo::new();
        let result = repo.list_by_state(StackState::Draft);
        assert!(result.expect("ok").is_empty());
    }
}

#[cfg(test)]
mod assertion_tests {
    use super::*;
    use crate::domain::stack::*;

    fn make_branch(name: &str, position: u32, parent: Option<&str>) -> StackBranch {
        StackBranch::new(
            BranchName::new(name),
            position,
            CommitHash::new("abc"),
            parent.map(BranchName::new),
        )
    }

    fn make_stack(branches: Vec<StackBranch>, state: StackState) -> Stack {
        Stack {
            id: StackId::from_u64(1),
            name: StackName::new("test"),
            base_branch: BranchName::new("main"),
            branches,
            state,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_assert_branch_order_valid() {
        let stack = make_stack(
            vec![
                make_branch("a", 0, None),
                make_branch("b", 1, Some("a")),
                make_branch("c", 2, Some("b")),
            ],
            StackState::Draft,
        );
        assert!(assert_branch_order(&stack).is_ok());
    }

    #[test]
    fn test_assert_branch_order_invalid_position() {
        let stack = make_stack(
            vec![
                make_branch("a", 0, Some("b")),
                make_branch("b", 0, Some("a")),
            ],
            StackState::Draft,
        );
        assert!(assert_branch_order(&stack).is_err());
    }

    #[test]
    fn test_assert_branch_order_single() {
        let stack = make_stack(vec![make_branch("solo", 0, None)], StackState::Draft);
        assert!(assert_branch_order(&stack).is_ok());
    }

    #[test]
    fn test_assert_branch_order_empty() {
        let stack = make_stack(vec![], StackState::Draft);
        assert!(assert_branch_order(&stack).is_ok());
    }

    #[test]
    fn test_assert_branch_order_parent_mismatch() {
        let stack = make_stack(
            vec![
                make_branch("a", 0, None),
                make_branch("b", 1, Some("wrong-parent")),
            ],
            StackState::Draft,
        );
        assert!(assert_branch_order(&stack).is_err());
    }

    #[test]
    fn test_assert_base_not_in_stack_valid() {
        let stack = make_stack(
            vec![
                make_branch("feat-a", 0, None),
                make_branch("feat-b", 1, Some("feat-a")),
            ],
            StackState::Draft,
        );
        assert!(assert_base_not_in_stack(&stack).is_ok());
    }

    #[test]
    fn test_assert_base_not_in_stack_invalid() {
        let stack = make_stack(vec![make_branch("main", 0, None)], StackState::Draft);
        assert!(assert_base_not_in_stack(&stack).is_err());
    }

    #[test]
    fn test_assert_base_not_in_stack_empty() {
        let stack = make_stack(vec![], StackState::Draft);
        assert!(assert_base_not_in_stack(&stack).is_ok());
    }

    #[test]
    fn test_assert_unique_branch_names_valid() {
        let stack = make_stack(
            vec![make_branch("a", 0, None), make_branch("b", 1, Some("a"))],
            StackState::Draft,
        );
        assert!(assert_unique_branch_names(&stack).is_ok());
    }

    #[test]
    fn test_assert_unique_branch_names_duplicate() {
        let stack = make_stack(
            vec![make_branch("a", 0, None), make_branch("a", 1, Some("a"))],
            StackState::Draft,
        );
        assert!(assert_unique_branch_names(&stack).is_err());
    }

    #[test]
    fn test_assert_unique_branch_names_empty() {
        let stack = make_stack(vec![], StackState::Draft);
        assert!(assert_unique_branch_names(&stack).is_ok());
    }

    #[test]
    fn test_assert_draft_stack_no_prs_valid() {
        let stack = make_stack(vec![make_branch("a", 0, None)], StackState::Draft);
        assert!(assert_draft_stack_no_prs(&stack).is_ok());
    }

    #[test]
    fn test_assert_draft_stack_no_prs_invalid() {
        let mut branch = make_branch("a", 0, None);
        branch.pr_info = Some(PrInfo::new(
            1,
            "url".to_string(),
            "t".to_string(),
            "d".to_string(),
            "a".to_string(),
            false,
        ));
        let stack = make_stack(vec![branch], StackState::Draft);
        assert!(assert_draft_stack_no_prs(&stack).is_err());
    }

    #[test]
    fn test_assert_draft_stack_no_prs_non_draft_ok() {
        let mut branch = make_branch("a", 0, None);
        branch.pr_info = Some(PrInfo::new(
            1,
            "url".to_string(),
            "t".to_string(),
            "d".to_string(),
            "a".to_string(),
            false,
        ));
        let stack = make_stack(vec![branch], StackState::Published);
        // Should be ok because it's not a draft
        assert!(assert_draft_stack_no_prs(&stack).is_ok());
    }

    #[test]
    fn test_assert_published_stack_has_prs_valid() {
        let mut branches = Vec::new();
        for i in 0..3u32 {
            let parent: Option<String> = if i == 0 {
                None
            } else {
                Some(format!("b{}", i - 1))
            };
            let mut branch = make_branch(&format!("b{i}"), i, parent.as_deref());
            branch.pr_info = Some(PrInfo::new(
                i + 1,
                "url".to_string(),
                "t".to_string(),
                "d".to_string(),
                "a".to_string(),
                false,
            ));
            branches.push(branch);
        }
        let stack = make_stack(branches, StackState::Published);
        assert!(assert_published_stack_has_prs(&stack).is_ok());
    }

    #[test]
    fn test_assert_published_stack_has_prs_invalid_missing() {
        let stack = make_stack(vec![make_branch("a", 0, None)], StackState::Published);
        assert!(assert_published_stack_has_prs(&stack).is_err());
    }

    #[test]
    fn test_assert_published_stack_has_prs_non_published_ok() {
        // Not published, so assertion should pass regardless
        let stack = make_stack(vec![make_branch("a", 0, None)], StackState::Draft);
        assert!(assert_published_stack_has_prs(&stack).is_ok());
    }

    #[test]
    fn test_assert_merged_stack_all_merged_valid() {
        let mut branches = Vec::new();
        for i in 0..3u32 {
            let parent: Option<String> = if i == 0 {
                None
            } else {
                Some(format!("b{}", i - 1))
            };
            let mut branch = make_branch(&format!("b{i}"), i, parent.as_deref());
            branch.state = crate::domain::state::BranchState::Merged;
            branches.push(branch);
        }
        let stack = make_stack(branches, StackState::Merged);
        assert!(assert_merged_stack_all_merged(&stack).is_ok());
    }

    #[test]
    fn test_assert_merged_stack_all_merged_invalid() {
        let stack = make_stack(vec![make_branch("a", 0, None)], StackState::Merged);
        assert!(assert_merged_stack_all_merged(&stack).is_err());
    }

    #[test]
    fn test_assert_merged_stack_all_merged_non_merged_ok() {
        let stack = make_stack(vec![make_branch("a", 0, None)], StackState::Draft);
        // Not merged, so assertion should pass
        assert!(assert_merged_stack_all_merged(&stack).is_ok());
    }
}
