use worktree::{Worktree, WorktreeState};

#[derive(Debug, Clone, PartialEq)]
pub struct WorktreeItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub state: WorktreeState,
    pub is_active: bool,
}

impl WorktreeItem {
    pub fn from_worktree(wt: &Worktree, is_active: bool) -> Self {
        Self {
            id: wt.id().to_string(),
            name: wt.name().to_string(),
            path: wt.path().to_string(),
            branch: wt.branch().map(|b| b.to_string()),
            state: wt.state(),
            is_active,
        }
    }

    pub fn state_label(&self) -> &'static str {
        self.state.name()
    }

    pub fn branch_label(&self) -> &str {
        self.branch.as_deref().unwrap_or("(no branch)")
    }

    #[cfg(test)]
    fn test_create(name: &str, branch: Option<&str>, state: WorktreeState) -> Self {
        Self {
            id: format!("test-id-{}", name),
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            branch: branch.map(String::from),
            state,
            is_active: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_item_test_create_makes_valid_item() {
        let item = WorktreeItem::test_create("test-wt", Some("main"), WorktreeState::Active);

        assert_eq!(item.name, "test-wt");
        assert_eq!(item.path, "/tmp/test-wt");
        assert_eq!(item.branch.as_deref(), Some("main"));
        assert_eq!(item.state, WorktreeState::Active);
        assert!(!item.is_active);
    }

    #[test]
    fn worktree_item_state_label_returns_correct_name() {
        let item = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        assert_eq!(item.state_label(), "Active");

        let suspended = WorktreeItem::test_create("wt", None, WorktreeState::Suspended);
        assert_eq!(suspended.state_label(), "Suspended");
    }

    #[test]
    fn worktree_item_branch_label_returns_branch_or_default() {
        let with_branch = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        assert_eq!(with_branch.branch_label(), "main");

        let without_branch = WorktreeItem::test_create("wt", None, WorktreeState::Active);
        assert_eq!(without_branch.branch_label(), "(no branch)");
    }

    #[test]
    fn worktree_item_debug_format() {
        let item = WorktreeItem::test_create("test-worktree", Some("main"), WorktreeState::Active);
        let debug = format!("{:?}", item);
        assert!(debug.contains("test-worktree"));
        assert!(debug.contains("Active"));
    }

    #[test]
    fn worktree_item_partial_eq() {
        let item1 = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        let item2 = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        assert_eq!(item1, item2);

        let item3 = WorktreeItem::test_create("wt", Some("develop"), WorktreeState::Active);
        assert_ne!(item1, item3);
    }

    #[test]
    fn worktree_item_clone_is_independent() {
        let item1 = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        let item2 = item1.clone();
        assert_eq!(item1, item2);
    }
<<<<<<< HEAD
=======

    // ── Adversarial: Send + Sync ──

    #[test]
    fn worktree_item_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WorktreeItem>();
    }

    #[test]
    fn worktree_item_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<WorktreeItem>();
    }

    // ── Adversarial: empty string branch ──

    #[test]
    fn branch_label_with_empty_string_branch() {
        let mut item = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        item.branch = Some(String::new());
        assert_eq!(item.branch_label(), "");
    }

    #[test]
    fn branch_label_with_none_branch() {
        let mut item = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        item.branch = None;
        assert_eq!(item.branch_label(), "(no branch)");
    }

    // ── Adversarial: is_active field ──

    #[test]
    fn worktree_item_with_active_flag() {
        let mut item = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Active);
        item.is_active = true;
        assert!(item.is_active);
    }

    #[test]
    fn worktree_item_with_suspended_state() {
        let item = WorktreeItem::test_create("wt", Some("main"), WorktreeState::Suspended);
        assert_eq!(item.state_label(), "Suspended");
    }
>>>>>>> polecat/theta
}
