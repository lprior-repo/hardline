#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![forbid(unsafe_code)]

use crate::application::{
    commands::{
        CreateWorktreeCommand, InitializeWorktreeCommand, ListWorktreesQuery,
        RemoveWorktreeCommand, ResumeWorktreeCommand, SuspendWorktreeCommand,
    },
    hooks::{HookContext, NoOpWorktreeHooks, WorktreeHookEvent, WorktreeHooks},
    repositories::WorktreeRepository,
};
use crate::domain::Worktree;
use crate::domain::{WorktreeDomainError, WorktreeId, WorktreeState};

pub struct WorktreeService<R: WorktreeRepository, H: WorktreeHooks = NoOpWorktreeHooks> {
    repository: R,
    hooks: H,
}

impl<R: WorktreeRepository> WorktreeService<R, NoOpWorktreeHooks> {
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            hooks: NoOpWorktreeHooks,
        }
    }
}

impl<R: WorktreeRepository, H: WorktreeHooks> WorktreeService<R, H> {
    pub fn with_hooks(repository: R, hooks: H) -> Self {
        Self { repository, hooks }
    }

    pub async fn create_worktree(
        &mut self,
        cmd: CreateWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        if self.repository.name_exists(cmd.name.as_str()).await? {
            return Err(WorktreeDomainError::NameAlreadyExists(
                cmd.name.as_str().to_string(),
            ));
        }

        // Pre-create hook
        let pre_ctx = HookContext {
            event: WorktreeHookEvent::PreCreate,
            worktree_id: None,
            worktree_name: Some(cmd.name.as_str().to_string()),
            worktree_path: Some(cmd.path.as_ref().to_path_buf()),
            parent_path: Some(cmd.parent_path.as_ref().to_path_buf()),
            worktree_type: Some(cmd.worktree_type.to_string()),
            branch: cmd.branch.as_ref().map(|b| b.as_str().to_string()),
        };
        let pre_results = self.hooks.run(WorktreeHookEvent::PreCreate, &pre_ctx)?;
        if let Some(failed) = pre_results.iter().find(|r| !r.success) {
            return Err(WorktreeDomainError::HookFailed {
                event: failed.event.to_string(),
                hook_name: "pre-create".to_string(),
                detail: failed.error.clone().unwrap_or_default(),
            });
        }

        let worktree = Worktree::new(
            cmd.name,
            cmd.path,
            cmd.parent_path,
            cmd.worktree_type,
            cmd.branch,
        );

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;

        // Post-create hook (non-fatal)
        let post_ctx = HookContext {
            event: WorktreeHookEvent::PostCreate,
            worktree_id: Some(worktree_clone.id().clone()),
            worktree_name: Some(worktree_clone.name().as_str().to_string()),
            worktree_path: Some(worktree_clone.path().as_ref().to_path_buf()),
            parent_path: Some(worktree_clone.parent_path().as_ref().to_path_buf()),
            worktree_type: Some(worktree_clone.worktree_type().to_string()),
            branch: worktree_clone.branch().map(|b| b.as_str().to_string()),
        };
        // Post-hooks are informational — ignore failures.
        let _ = self.hooks.run(WorktreeHookEvent::PostCreate, &post_ctx);

        Ok(worktree_clone)
    }

    pub async fn initialize_worktree(
        &mut self,
        cmd: InitializeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Validate the worktree is in Creating state before initializing
        if worktree.state() != WorktreeState::Creating {
            return Err(WorktreeDomainError::InvalidStateTransition(
                worktree.state(),
                WorktreeState::Active,
            ));
        }

        let worktree = worktree.activate();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn suspend_worktree(
        &mut self,
        cmd: SuspendWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Validate the worktree is actually active before suspending
        if worktree.state() != WorktreeState::Active {
            return Err(WorktreeDomainError::InvalidStateTransition(
                worktree.state(),
                WorktreeState::Suspended,
            ));
        }

        // Use into_state to get the right phantom type, then suspend
        let active: crate::domain::worktree::Worktree<crate::domain::worktree::Active> =
            worktree.into_state();
        let worktree = active.suspend();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn resume_worktree(
        &mut self,
        cmd: ResumeWorktreeCommand,
    ) -> Result<Worktree, WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Validate the worktree is actually suspended before resuming
        if worktree.state() != WorktreeState::Suspended {
            return Err(WorktreeDomainError::InvalidStateTransition(
                worktree.state(),
                WorktreeState::Active,
            ));
        }

        // Use into_state to get the right phantom type, then resume
        let suspended: crate::domain::worktree::Worktree<crate::domain::worktree::Suspended> =
            worktree.into_state();
        let worktree = suspended.resume();

        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone.into())
    }

    pub async fn remove_worktree(
        &mut self,
        cmd: RemoveWorktreeCommand,
    ) -> Result<(), WorktreeDomainError> {
        let worktree = self
            .repository
            .find_by_id(&cmd.worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(cmd.worktree_id.clone()))?;

        // Pre-remove hook
        let pre_ctx = HookContext {
            event: WorktreeHookEvent::PreRemove,
            worktree_id: Some(cmd.worktree_id.clone()),
            worktree_name: Some(worktree.name().as_str().to_string()),
            worktree_path: Some(worktree.path().as_ref().to_path_buf()),
            parent_path: Some(worktree.parent_path().as_ref().to_path_buf()),
            worktree_type: Some(worktree.worktree_type().to_string()),
            branch: worktree.branch().map(|b| b.as_str().to_string()),
        };
        let pre_results = self.hooks.run(WorktreeHookEvent::PreRemove, &pre_ctx)?;
        if let Some(failed) = pre_results.iter().find(|r| !r.success) {
            return Err(WorktreeDomainError::HookFailed {
                event: failed.event.to_string(),
                hook_name: "pre-remove".to_string(),
                detail: failed.error.clone().unwrap_or_default(),
            });
        }

        let worktree_name = worktree.name().as_str().to_string();
        let worktree_path = worktree.path().as_ref().to_path_buf();
        let parent_path = worktree.parent_path().as_ref().to_path_buf();
        let worktree_type = worktree.worktree_type().to_string();
        let branch = worktree.branch().map(|b| b.as_str().to_string());

        // Transition to active first, then mark for removal
        let worktree = worktree.activate();
        let worktree = worktree.mark_for_removal();
        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;

        let worktree = worktree_clone;
        let worktree = worktree.complete_removal();
        self.repository.save(worktree).await?;

        self.repository.delete(&cmd.worktree_id).await?;

        // Post-remove hook (non-fatal)
        let post_ctx = HookContext {
            event: WorktreeHookEvent::PostRemove,
            worktree_id: Some(cmd.worktree_id),
            worktree_name: Some(worktree_name),
            worktree_path: Some(worktree_path),
            parent_path: Some(parent_path),
            worktree_type: Some(worktree_type),
            branch,
        };
        let _ = self.hooks.run(WorktreeHookEvent::PostRemove, &post_ctx);

        Ok(())
    }

    pub async fn find_by_id(
        &self,
        id: &WorktreeId,
    ) -> Result<Option<Worktree>, WorktreeDomainError> {
        self.repository.find_by_id(id).await
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
        self.repository.find_by_name(name).await
    }

    pub async fn list_worktrees(
        &self,
        query: ListWorktreesQuery,
    ) -> Result<Vec<Worktree>, WorktreeDomainError> {
        let worktrees = self.repository.list_all().await?;

        let result: Vec<Worktree> = worktrees
            .into_iter()
            .filter(|w| query.include_removed || !w.is_removed())
            .filter(|w| query.state_filter.is_none_or(|state| w.state() == state))
            .filter(|w| {
                query
                    .worktree_type_filter
                    .is_none_or(|wt| w.worktree_type() == wt)
            })
            .filter(|w| {
                query
                    .name_prefix
                    .as_ref()
                    .is_none_or(|prefix| w.name().as_str().starts_with(prefix))
            })
            .collect();

        Ok(result)
    }

    pub async fn add_metadata(
        &mut self,
        worktree_id: &WorktreeId,
        key: &str,
        value: &str,
    ) -> Result<Worktree, WorktreeDomainError> {
        let mut worktree = self
            .repository
            .find_by_id(worktree_id)
            .await?
            .ok_or_else(|| WorktreeDomainError::NotFound(worktree_id.clone()))?;

        worktree.add_metadata(key, value);
        let worktree_clone = worktree.clone();
        self.repository.save(worktree).await?;
        Ok(worktree_clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::hooks::HookOutcome;
    use crate::application::repositories::WorktreeRepository;
    use crate::domain::{AbsolutePath, BranchName, WorktreeName, WorktreeState, WorktreeTypeEnum};

    #[derive(Default)]
    struct InMemoryRepository {
        worktrees: Vec<Worktree>,
    }

    #[async_trait::async_trait]
    impl WorktreeRepository for InMemoryRepository {
        async fn save<S: Send>(
            &mut self,
            worktree: Worktree<S>,
        ) -> Result<(), WorktreeDomainError> {
            if let Some(existing) = self.worktrees.iter_mut().find(|w| w.id() == worktree.id()) {
                *existing = worktree.into_state();
            } else {
                self.worktrees.push(worktree.into_state());
            }
            Ok(())
        }

        async fn find_by_id(
            &self,
            id: &WorktreeId,
        ) -> Result<Option<Worktree>, WorktreeDomainError> {
            Ok(self.worktrees.iter().find(|w| w.id() == id).cloned())
        }

        async fn find_by_name(&self, name: &str) -> Result<Option<Worktree>, WorktreeDomainError> {
            Ok(self
                .worktrees
                .iter()
                .find(|w| w.name().as_str() == name)
                .cloned())
        }

        async fn list_all(&self) -> Result<Vec<Worktree>, WorktreeDomainError> {
            Ok(self.worktrees.clone())
        }

        async fn delete(&mut self, id: &WorktreeId) -> Result<(), WorktreeDomainError> {
            self.worktrees.retain(|w| w.id() != id);
            Ok(())
        }

        async fn name_exists(&self, name: &str) -> Result<bool, WorktreeDomainError> {
            Ok(self.worktrees.iter().any(|w| w.name().as_str() == name))
        }
    }

    // -- Recording hook for testing --

    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Default)]
    struct HookCall {
        event: WorktreeHookEvent,
        worktree_name: Option<String>,
    }

    #[derive(Debug, Default)]
    struct RecordingHook {
        calls: Arc<Mutex<Vec<HookCall>>>,
        fail_on_pre_create: bool,
    }

    impl RecordingHook {
        fn new() -> Self {
            Self::default()
        }

        fn failing_on_pre_create() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                fail_on_pre_create: true,
            }
        }

        fn calls(&self) -> Vec<HookCall> {
            self.calls.lock().map(|c| c.clone()).unwrap_or_default()
        }
    }

    impl WorktreeHooks for RecordingHook {
        fn run(
            &self,
            event: WorktreeHookEvent,
            ctx: &HookContext,
        ) -> Result<Vec<HookOutcome>, WorktreeDomainError> {
            self.calls.lock().map(|mut calls| {
                calls.push(HookCall {
                    event,
                    worktree_name: ctx.worktree_name.clone(),
                });
            }).map_err(|_| WorktreeDomainError::GitError("lock poisoned".to_string()))?;

            if self.fail_on_pre_create && matches!(event, WorktreeHookEvent::PreCreate) {
                return Ok(vec![HookOutcome::failure(
                    event,
                    "pre-create rejected".to_string(),
                    1,
                )]);
            }

            Ok(vec![HookOutcome::success(event, "ok".to_string(), 0)])
        }
    }

    // -- Existing tests (unchanged) --

    #[tokio::test]
    async fn worktree_service_create_saves_worktree_to_repository() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("main").unwrap()),
        );

        let result = service.create_worktree(cmd).await;
        assert!(result.is_ok());
        let worktree = result.unwrap();
        assert_eq!(worktree.name().as_str(), "test-worktree");
    }

    #[tokio::test]
    async fn worktree_service_initialize_transitions_worktree_to_active() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("test-worktree").unwrap(),
            AbsolutePath::new("/tmp/test").unwrap(),
            AbsolutePath::new("/home/user/project").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let worktree = service.create_worktree(cmd).await.unwrap();
        let id = worktree.id().clone();

        let init_cmd = InitializeWorktreeCommand::new(id);
        let result = service.initialize_worktree(init_cmd).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().state(), WorktreeState::Active);
    }

    // -- Hook integration tests --

    #[tokio::test]
    async fn create_worktree_runs_pre_and_post_hooks() {
        let hook = RecordingHook::new();
        let calls = hook.calls.clone();
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::with_hooks(repo, hook);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("hooked-wt").unwrap(),
            AbsolutePath::new("/tmp/hooked").unwrap(),
            AbsolutePath::new("/home/user/repo").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );

        let result = service.create_worktree(cmd).await;
        assert!(result.is_ok());

        let recorded = calls.lock().map(|c| c.clone()).unwrap_or_default();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].event, WorktreeHookEvent::PreCreate);
        assert_eq!(recorded[0].worktree_name, Some("hooked-wt".to_string()));
        assert_eq!(recorded[1].event, WorktreeHookEvent::PostCreate);
    }

    #[tokio::test]
    async fn create_worktree_pre_hook_failure_aborts_creation() {
        let hook = RecordingHook::failing_on_pre_create();
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::with_hooks(repo, hook);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("should-not-create").unwrap(),
            AbsolutePath::new("/tmp/fail").unwrap(),
            AbsolutePath::new("/home/user/repo").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );

        let result = service.create_worktree(cmd).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, WorktreeDomainError::HookFailed { .. }),
            "expected HookFailed, got {err:?}"
        );
    }

    #[tokio::test]
    async fn remove_worktree_runs_pre_and_post_hooks() {
        let hook = RecordingHook::new();
        let calls = hook.calls.clone();
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::with_hooks(repo, hook);

        // Create a worktree first
        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("to-remove").unwrap(),
            AbsolutePath::new("/tmp/remove").unwrap(),
            AbsolutePath::new("/home/user/repo").unwrap(),
            WorktreeTypeEnum::Development,
            None,
        );
        let wt = service.create_worktree(cmd).await.unwrap();

        // Clear hook calls from create
        calls.lock().map(|mut c| c.clear()).ok();

        // Remove it
        let remove_cmd = RemoveWorktreeCommand::new(wt.id().clone());
        let result = service.remove_worktree(remove_cmd).await;
        assert!(result.is_ok());

        let recorded = calls.lock().map(|c| c.clone()).unwrap_or_default();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].event, WorktreeHookEvent::PreRemove);
        assert_eq!(recorded[0].worktree_name, Some("to-remove".to_string()));
        assert_eq!(recorded[1].event, WorktreeHookEvent::PostRemove);
    }

    #[tokio::test]
    async fn no_op_hooks_do_not_interfere_with_operations() {
        let repo = InMemoryRepository::default();
        let mut service = WorktreeService::new(repo);

        let cmd = CreateWorktreeCommand::new(
            WorktreeName::new("noop-wt").unwrap(),
            AbsolutePath::new("/tmp/noop").unwrap(),
            AbsolutePath::new("/home/user/repo").unwrap(),
            WorktreeTypeEnum::Development,
            Some(BranchName::new("feature").unwrap()),
        );

        let result = service.create_worktree(cmd).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name().as_str(), "noop-wt");
    }

    #[tokio::test]
    async fn hook_failed_error_displays_event_and_detail() {
        let err = WorktreeDomainError::HookFailed {
            event: "pre-create".to_string(),
            hook_name: "validate-setup".to_string(),
            detail: "workspace config missing".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("pre-create"));
        assert!(msg.contains("validate-setup"));
        assert!(msg.contains("workspace config missing"));
    }
}
