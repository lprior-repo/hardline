use chrono::Utc;

use crate::application::traits::{GitHubClientTrait, StackRepository, VcsClientTrait};
use crate::domain::stack::{PrInfo, Stack, StackBranch, StackId};
use crate::domain::state::{BranchState, StackState};
use crate::domain::value_objects::BranchName;
use crate::error::{Result, StackError};
use scp_vcs::application::ops::Transaction;
use scp_vcs::domain::entities::ops::OpKind;
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

    /// Publish a stack by creating PRs for each branch.
    ///
    /// Wraps the operation in a VCS `Transaction` so that if the process
    /// crashes mid-operation, the receipt persists for crash recovery.
    /// Backup refs are created for all branches before any remote changes.
    pub fn publish_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or_else(|| StackError::NotFound(stack_id.to_string()))?;

        let git_dir = self.vcs.git_dir()?;
        let workdir = self.vcs.workdir()?;
        let head_branch = self.vcs.current_branch()?;

        let mut tx = Transaction::begin(
            OpKind::Submit,
            git_dir,
            workdir,
            stack.base_branch.as_str().to_string(),
            head_branch.as_str().to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        // Plan all branches with their current OIDs.
        let branch_oids: Vec<(String, Option<String>)> = stack
            .branches
            .iter()
            .map(|b| {
                let oid = self.vcs.resolve_branch_oid(&b.branch_name).ok().flatten();
                (b.branch_name.as_str().to_string(), oid)
            })
            .collect();

        tx.plan_branches(&branch_oids);
        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        // Execute: create PRs for each branch.
        for branch in &stack.branches {
            let pr_info = self
                .github
                .create_pull_request(branch, &stack.base_branch)?;
            branch.pr_info = Some(pr_info);
        }

        // Record after-state for all branches.
        tx.record_all_after(|branch| {
            self.vcs
                .resolve_branch_oid(&BranchName::new(branch))
                .ok()
                .flatten()
        });

        stack.state = StackState::Published;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(stack)
    }

    /// Restack all branches in a stack onto their parents.
    ///
    /// Uses a VCS `Transaction` with `OpKind::Restack` for crash recovery.
    /// Creates backup refs before rebasing so the operation can be undone
    /// if interrupted.
    pub fn restack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or_else(|| StackError::NotFound(stack_id.to_string()))?;

        let git_dir = self.vcs.git_dir()?;
        let workdir = self.vcs.workdir()?;
        let head_branch = self.vcs.current_branch()?;

        let mut tx = Transaction::begin(
            OpKind::Restack,
            git_dir,
            workdir,
            stack.base_branch.as_str().to_string(),
            head_branch.as_str().to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        // Plan all branches with their pre-rebase OIDs.
        for branch in &stack.branches {
            let oid_before = self
                .vcs
                .resolve_branch_oid(&branch.branch_name)
                .ok()
                .flatten();
            tx.plan_branch(branch.branch_name.as_str(), oid_before.as_deref());
        }

        let mut summary = scp_vcs::domain::entities::ops::PlanSummary::default();
        summary.branches_to_rebase = stack.branches.len();
        summary.branches_to_push = stack.branches.len();
        summary.description = vec![format!(
            "Restacking {} branches onto updated parents",
            stack.branches.len()
        )];
        tx.set_plan_summary(summary);

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        self.github.fetch(&stack.base_branch)?;

        for (i, branch) in stack.branches.iter_mut().enumerate() {
            let parent = if i == 0 {
                stack.base_branch.clone()
            } else {
                stack.branches[i - 1].branch_name.clone()
            };

            self.vcs.rebase(&branch.branch_name, &parent).map_err(|e| {
                let _ = tx.finish_err(
                    &e.to_string(),
                    Some("rebase"),
                    Some(branch.branch_name.as_str()),
                );
                e
            })?;

            self.github.force_push(&branch.branch_name).map_err(|e| {
                let _ = tx.finish_err(
                    &e.to_string(),
                    Some("force_push"),
                    Some(branch.branch_name.as_str()),
                );
                e
            })?;
        }

        // Record post-rebase OIDs.
        tx.record_all_after(|branch| {
            self.vcs
                .resolve_branch_oid(&BranchName::new(branch))
                .ok()
                .flatten()
        });

        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        Ok(stack)
    }

    /// Merge all PRs in a stack (bottom to top).
    ///
    /// Uses a VCS `Transaction` with `OpKind::MergeWhenReady` for crash
    /// recovery. Records the state of each branch before and after merge.
    pub fn merge_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self
            .stack_repo
            .find_by_id(&stack_id)?
            .ok_or_else(|| StackError::NotFound(stack_id.to_string()))?;

        let git_dir = self.vcs.git_dir()?;
        let workdir = self.vcs.workdir()?;
        let head_branch = self.vcs.current_branch()?;

        let mut tx = Transaction::begin(
            OpKind::MergeWhenReady,
            git_dir,
            workdir,
            stack.base_branch.as_str().to_string(),
            head_branch.as_str().to_string(),
        )
        .map_err(|e| StackError::GitError(e.to_string()))?;

        for branch in &stack.branches {
            let oid_before = self
                .vcs
                .resolve_branch_oid(&branch.branch_name)
                .ok()
                .flatten();
            tx.plan_branch(branch.branch_name.as_str(), oid_before.as_deref());
        }

        tx.snapshot()
            .map_err(|e| StackError::GitError(e.to_string()))?;

        stack.state = StackState::Merging;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        for branch in &mut stack.branches {
            if let Some(pr_info) = &branch.pr_info {
                self.github
                    .merge_pull_request(pr_info.pr_number)
                    .map_err(|e| {
                        let _ = tx.finish_err(
                            &e.to_string(),
                            Some("merge_pr"),
                            Some(branch.branch_name.as_str()),
                        );
                        e
                    })?;
                branch.state = BranchState::Merged;
            }
        }

        // Record after-state.
        tx.record_all_after(|branch| {
            self.vcs
                .resolve_branch_oid(&BranchName::new(branch))
                .ok()
                .flatten()
        });

        stack.state = StackState::Merged;
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;

        tx.finish_ok()
            .map_err(|e| StackError::GitError(e.to_string()))?;

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

    // ==================== create_stack exhaustive tests ====================

    #[test]
    fn test_create_stack_happy_path_single_branch() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let base = BranchName::new("main");
        let head = BranchName::new("feature-a");

        let result = service.create_stack(base.clone(), head.clone(), "test-stack".to_string());
        assert!(
            result.is_ok(),
            "create_stack should succeed with valid inputs"
        );

        let stack = result.expect("should have stack");
        assert_eq!(stack.name.as_str(), "test-stack");
        assert_eq!(stack.base_branch.as_str(), "main");
        assert_eq!(stack.state, StackState::Draft);
        assert_eq!(stack.branches.len(), 1);
        assert_eq!(stack.branches[0].branch_name.as_str(), "feature-a");
    }

    #[test]
    fn test_create_stack_happy_path_multiple_branches() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let base = BranchName::new("main");
        let head = BranchName::new("feature-b");

        let result = service.create_stack(base, head, "multi-branch-stack".to_string());
        assert!(result.is_ok());

        let stack = result.expect("should have stack");
        assert_eq!(stack.branches.len(), 1);
        assert_eq!(stack.branches[0].branch_name.as_str(), "feature-b");
    }

    #[test]
    fn test_create_stack_initial_state_is_draft() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-x"),
            "draft-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert_eq!(stack.state, StackState::Draft);
        assert!(!stack.is_published());
        assert!(!stack.is_merged());
    }

    #[test]
    fn test_create_stack_empty_main_branch_name() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new(""),
            BranchName::new("feature-a"),
            "stack".to_string(),
        );

        // Empty branch name should be rejected
        assert!(
            result.is_err(),
            "create_stack should reject empty main branch name"
        );
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_empty_head_branch_name() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new(""),
            "stack".to_string(),
        );

        assert!(
            result.is_err(),
            "create_stack should reject empty head branch name"
        );
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_invalid_branch_name_special_chars() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature@#$%"),
            "stack".to_string(),
        );

        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_invalid_branch_name_starts_with_dot() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new(".feature"),
            "stack".to_string(),
        );

        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_invalid_branch_name_ends_with_dot() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature."),
            "stack".to_string(),
        );

        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_invalid_branch_name_contains_spaces() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature branch"),
            "stack".to_string(),
        );

        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_invalid_branch_name_contains_null() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature\x00branch"),
            "stack".to_string(),
        );

        assert!(result.is_err());
        let err = result.err().expect("error");
        assert!(matches!(err, StackError::InvalidBranchName(_)));
    }

    #[test]
    fn test_create_stack_duplicate_stack_creation_same_main_branch() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let base = BranchName::new("main");
        let head = BranchName::new("feature-a");

        // First creation should succeed
        let result1 = service.create_stack(base.clone(), head.clone(), "stack-1".to_string());
        assert!(result1.is_ok());

        // Second creation with same main branch should succeed (different stack)
        let result2 = service.create_stack(base.clone(), head.clone(), "stack-2".to_string());
        assert!(result2.is_ok(), "Should allow multiple stacks on same main");

        let all_stacks = repo.list_all().expect("list");
        assert_eq!(all_stacks.len(), 2);
    }

    #[test]
    fn test_create_stack_id_uniqueness() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let mut ids = Vec::new();

        for i in 0..10u32 {
            let result = service.create_stack(
                BranchName::new("main"),
                BranchName::new(&format!("feature-{}", i)),
                format!("stack-{}", i),
            );
            assert!(result.is_ok());
            let stack = result.expect("stack");
            ids.push(stack.id);
        }

        // All IDs should be unique
        let mut unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(
            unique_ids.len(),
            ids.len(),
            "All stack IDs should be unique"
        );
    }

    #[test]
    fn test_create_stack_no_prs_in_draft_state() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "no-pr-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert_eq!(stack.state, StackState::Draft);
        for branch in &stack.branches {
            assert!(branch.pr_info.is_none(), "Draft stack should have no PRs");
        }
    }

    #[test]
    fn test_create_stack_main_branch_recorded_correctly() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let test_cases = vec!["main", "master", "develop", "trunk", "release/v1.0"];

        for main_branch in test_cases {
            let result = service.create_stack(
                BranchName::new(main_branch),
                BranchName::new("feature-a"),
                format!("stack-{}", main_branch),
            );

            let stack = result.expect(&format!("stack for {}", main_branch));
            assert_eq!(
                stack.base_branch.as_str(),
                main_branch,
                "Main branch should be recorded correctly"
            );
        }
    }

    #[test]
    fn test_create_stack_head_branch_in_branches_list() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let head = BranchName::new("my-feature");
        let result = service.create_stack(
            BranchName::new("main"),
            head.clone(),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert!(
            stack.branches.iter().any(|b| b.branch_name == head),
            "Head branch should be in branches list"
        );
    }

    #[test]
    fn test_create_stack_branch_has_correct_parent() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert_eq!(stack.branches.len(), 1);
        let branch = &stack.branches[0];
        assert_eq!(
            branch.parent_branch, None,
            "First branch should have no parent"
        );
    }

    #[test]
    fn test_create_stack_branch_position_is_zero() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert_eq!(stack.branches[0].position, 0);
    }

    #[test]
    fn test_create_stack_branch_has_commit_hash() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert!(!stack.branches[0].last_commit.as_str().is_empty());
    }

    #[test]
    fn test_create_stack_with_unicode_branch_name() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature/日本語"),
            "unicode-stack".to_string(),
        );

        assert!(result.is_ok());
        let stack = result.expect("stack");
        assert_eq!(stack.branches[0].branch_name.as_str(), "feature/日本語");
    }

    #[test]
    fn test_create_stack_with_very_long_branch_name() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let long_name = format!("feature/{}", "a".repeat(100));
        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new(&long_name),
            "long-stack".to_string(),
        );

        assert!(result.is_ok());
        let stack = result.expect("stack");
        assert_eq!(stack.branches[0].branch_name.as_str(), long_name);
    }

    #[test]
    fn test_create_stack_branch_name_with_slashes() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature/team/project/component"),
            "nested-stack".to_string(),
        );

        assert!(result.is_ok());
        let stack = result.expect("stack");
        assert_eq!(
            stack.branches[0].branch_name.as_str(),
            "feature/team/project/component"
        );
    }

    #[test]
    fn test_create_stack_branch_name_with_hyphens() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature/my-awesome-new-feature"),
            "hyphen-stack".to_string(),
        );

        assert!(result.is_ok());
        let stack = result.expect("stack");
        assert_eq!(
            stack.branches[0].branch_name.as_str(),
            "feature/my-awesome-new-feature"
        );
    }

    #[test]
    fn test_create_stack_branch_name_with_underscores() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature/my_awesome_feature"),
            "underscore-stack".to_string(),
        );

        assert!(result.is_ok());
        let stack = result.expect("stack");
        assert_eq!(
            stack.branches[0].branch_name.as_str(),
            "feature/my_awesome_feature"
        );
    }

    #[test]
    fn test_create_stack_branch_state_is_open() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        assert_eq!(stack.branches[0].state, BranchState::Open);
    }

    #[test]
    fn test_create_stack_created_at_is_set() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        // created_at should be a valid future timestamp (not 1970)
        let year = stack.created_at.year();
        assert!(year >= 2020, "created_at should be recent");
    }

    #[test]
    fn test_create_stack_updated_at_is_set() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let stack = result.expect("stack");
        let year = stack.updated_at.year();
        assert!(year >= 2020, "updated_at should be recent");
    }

    #[test]
    fn test_create_stack_stored_in_repository() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        let result = service.create_stack(
            BranchName::new("main"),
            BranchName::new("feature-a"),
            "test-stack".to_string(),
        );

        let created_id = result.expect("stack").id;
        let stored = repo
            .find_by_id(&created_id)
            .expect("find")
            .expect("should exist");

        assert_eq!(stored.id, created_id);
        assert_eq!(stored.name.as_str(), "test-stack");
        assert_eq!(stored.base_branch.as_str(), "main");
    }

    #[test]
    fn test_create_stack_multiple_consecutive_creations() {
        let repo = Arc::new(MockRepo::new());
        let github = Arc::new(MockGitHub);
        let vcs = Arc::new(MockVcs);
        let service = StackService::new(repo, github, vcs);

        for i in 0..100u32 {
            let result = service.create_stack(
                BranchName::new("main"),
                BranchName::new(&format!("feature-{}", i)),
                format!("stack-{}", i),
            );
            assert!(result.is_ok(), "creation {} should succeed", i);
        }

        let all = repo.list_all().expect("list");
        assert_eq!(all.len(), 100);
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
            let mut branch = make_branch(
                &format!("b{i}"),
                i,
                if i == 0 {
                    None
                } else {
                    Some(&format!("b{}", i - 1))
                },
            );
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
            let mut branch = make_branch(
                &format!("b{i}"),
                i,
                if i == 0 {
                    None
                } else {
                    Some(&format!("b{}", i - 1))
                },
            );
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
