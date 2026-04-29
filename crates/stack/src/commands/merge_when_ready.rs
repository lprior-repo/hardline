use std::time::{Duration, Instant};

use crate::{
    application::traits::{GitHubClientTrait, MergeMethod},
    domain::{land_status::LandStatus, stack::Stack, value_objects::BranchName},
};

#[derive(Debug, Clone)]
pub struct LandBranchInfo {
    pub branch: BranchName,
    pub pr_number: u32,
    pub status: LandStatus,
}

#[derive(Debug, Clone)]
pub struct RemainingBranchInfo {
    pub branch: BranchName,
    pub pr_number: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MergeWhenReadyScope {
    pub to_merge: Vec<BranchName>,
    pub remaining: Vec<BranchName>,
    pub trunk: BranchName,
}

#[derive(Debug, Clone)]
pub struct MergeWhenReadyOptions {
    pub all: bool,
    pub method: MergeMethod,
    pub timeout_mins: u64,
    pub poll_interval_secs: u64,
    pub no_delete: bool,
    pub no_sync: bool,
    pub yes: bool,
    pub quiet: bool,
}

impl Default for MergeWhenReadyOptions {
    fn default() -> Self {
        Self {
            all: false,
            method: MergeMethod::Squash,
            timeout_mins: 60,
            poll_interval_secs: 30,
            no_delete: false,
            no_sync: false,
            yes: false,
            quiet: false,
        }
    }
}

pub fn calculate_merge_scope(
    stack: &Stack,
    current: &BranchName,
    all: bool,
) -> MergeWhenReadyScope {
    let mut to_merge: Vec<BranchName> = Vec::new();

    if let Some(current_branch) = stack.branch_named(current) {
        let mut node = current_branch
            .parent_branch
            .as_ref()
            .and_then(|p| stack.branch_named(p));
        while let Some(branch) = node {
            to_merge.push(branch.branch_name.clone());
            node = branch
                .parent_branch
                .as_ref()
                .and_then(|p| stack.branch_named(p));
        }
    }

    to_merge.reverse();
    to_merge.retain(|b| b.as_str() != stack.base_branch.as_str());
    to_merge.push(current.clone());

    let remaining = stack.descendants(current);

    if all && !remaining.is_empty() {
        to_merge.extend(remaining.iter().cloned());
    }

    MergeWhenReadyScope {
        to_merge,
        remaining,
        trunk: stack.base_branch.clone(),
    }
}

pub fn calculate_land_scope(stack: &Stack, current: &BranchName, all: bool) -> MergeWhenReadyScope {
    calculate_merge_scope(stack, current, all)
}

pub struct MergeWhenReadyContext<'a, T: GitHubClientTrait> {
    pub client: &'a T,
    pub timeout: Duration,
    pub poll_interval: Duration,
    pub quiet: bool,
}

pub fn wait_for_pr_ready<T: GitHubClientTrait>(
    ctx: &MergeWhenReadyContext<'_, T>,
    pr_number: u32,
) -> Result<WaitResult, String> {
    let start = Instant::now();

    loop {
        match ctx.client.get_pr_merge_status(pr_number) {
            Ok(status) => {
                if status.is_ready() {
                    return Ok(WaitResult::Ready);
                }

                if status.is_blocked() {
                    return Ok(WaitResult::Failed(status.status_text().to_string()));
                }
            }
            Err(e) => {
                return Ok(WaitResult::Failed(e.to_string()));
            }
        }

        if start.elapsed() > ctx.timeout {
            return Ok(WaitResult::Timeout);
        }

        if !ctx.quiet {
            let elapsed = start.elapsed().as_secs();
            println!("      ⏳ Waiting for PR #{}... ({}s)", pr_number, elapsed);
        }

        std::thread::sleep(ctx.poll_interval);
    }
}

pub enum WaitResult {
    Ready,
    Failed(String),
    Timeout,
}

pub fn print_land_preview(branches: &[LandBranchInfo], trunk: &BranchName, method: &MergeMethod) {
    println!();
    println!("╭──────────────────────────────────────────────────────────────╮");
    println!("│                    Merge When Ready                          │");
    println!("╰──────────────────────────────────────────────────────────────╯");
    println!();

    let pr_word = if branches.len() == 1 { "PR" } else { "PRs" };
    println!(
        "Will merge {} {} bottom-up into {}:",
        branches.len(),
        pr_word,
        trunk
    );
    println!();

    for (idx, branch) in branches.iter().enumerate() {
        println!("  {}. {} (#{})", idx + 1, branch.branch, branch.pr_number);
    }

    println!();
    println!("Merge method: {}", method.as_str());
    println!("Each PR will be polled for CI + approval before merging.");
}

pub fn print_dashboard(branches: &[LandBranchInfo]) {
    for (idx, branch) in branches.iter().enumerate() {
        let status_str = format!(
            "      [{}] {} (#{})\t{:?}",
            idx + 1,
            branch.branch,
            branch.pr_number,
            branch.status
        );
        if branch.status != LandStatus::Pending {
            println!("      {} {}", status_str, branch.status.symbol());
        }
    }
}

impl LandStatus {
    pub fn symbol(&self) -> String {
        match self {
            Self::Pending => "○".to_string(),
            Self::WaitingForCi => "⏳".to_string(),
            Self::Merging => "⏳".to_string(),
            Self::Merged => "✓".to_string(),
            Self::Failed(_) => "✗".to_string(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Pending => "pending".to_string(),
            Self::WaitingForCi => "waiting for CI...".to_string(),
            Self::Merging => "merging...".to_string(),
            Self::Merged => "merged".to_string(),
            Self::Failed(reason) => format!("failed: {}", reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::traits::{MergeMethod, PrMergeStatus},
        domain::stack::{CommitHash, StackBranch},
    };

    fn create_test_stack() -> Stack {
        let main = StackBranch::new(BranchName::new("main"), 0, CommitHash::new("abc123"), None);

        let feature_a = StackBranch::new(
            BranchName::new("feature-a"),
            1,
            CommitHash::new("def456"),
            Some(BranchName::new("main")),
        );

        let feature_b = StackBranch::new(
            BranchName::new("feature-b"),
            2,
            CommitHash::new("ghi789"),
            Some(BranchName::new("feature-a")),
        );

        let feature_c = StackBranch::new(
            BranchName::new("feature-c"),
            3,
            CommitHash::new("jkl012"),
            Some(BranchName::new("feature-b")),
        );

        let branches = vec![main, feature_a, feature_b, feature_c];

        Stack {
            id: crate::domain::stack::StackId::from_u64(1),
            name: crate::domain::stack::StackName::new("test-stack".to_string()),
            base_branch: BranchName::new("main"),
            branches,
            state: crate::domain::state::StackState::Published,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_land_status_symbols() {
        assert_eq!(LandStatus::Pending.symbol(), "○");
        assert_eq!(LandStatus::WaitingForCi.symbol(), "⏳");
        assert_eq!(LandStatus::Merging.symbol(), "⏳");
        assert_eq!(LandStatus::Merged.symbol(), "✓");
        assert_eq!(LandStatus::Failed("test".to_string()).symbol(), "✗");
    }

    #[test]
    fn test_land_status_labels() {
        assert_eq!(LandStatus::Pending.label(), "pending");
        assert_eq!(LandStatus::WaitingForCi.label(), "waiting for CI...");
        assert_eq!(LandStatus::Merging.label(), "merging...");
        assert_eq!(LandStatus::Merged.label(), "merged");
        assert_eq!(
            LandStatus::Failed("error".to_string()).label(),
            "failed: error"
        );
    }

    #[test]
    fn test_land_branch_info_creation() {
        let info = LandBranchInfo {
            branch: BranchName::new("feature-test"),
            pr_number: 42,
            status: LandStatus::Pending,
        };

        assert_eq!(info.branch.as_str(), "feature-test");
        assert_eq!(info.pr_number, 42);
        assert_eq!(info.status, LandStatus::Pending);
    }

    #[test]
    fn test_calculate_merge_scope_from_middle_without_all_keeps_descendants_remaining() {
        let stack = create_test_stack();

        let scope = calculate_merge_scope(&stack, &BranchName::new("feature-b"), false);

        assert_eq!(scope.to_merge.len(), 2);
        assert_eq!(scope.remaining.len(), 1);
        assert_eq!(scope.trunk.as_str(), "main");
    }

    #[test]
    fn test_calculate_merge_scope_from_bottom() {
        let stack = create_test_stack();

        let scope = calculate_merge_scope(&stack, &BranchName::new("feature-a"), false);

        assert_eq!(scope.to_merge.len(), 1);
        assert_eq!(scope.remaining.len(), 2);
    }

    #[test]
    fn test_remaining_branch_info() {
        let info = RemainingBranchInfo {
            branch: BranchName::new("feature-c"),
            pr_number: Some(123),
        };

        assert_eq!(info.branch.as_str(), "feature-c");
        assert_eq!(info.pr_number, Some(123));
    }

    #[test]
    fn test_merge_when_ready_scope_trunk() {
        let stack = create_test_stack();
        let scope = calculate_merge_scope(&stack, &BranchName::new("feature-a"), false);

        assert_eq!(scope.trunk.as_str(), "main");
    }

    #[test]
    fn test_merge_when_ready_options_default() {
        let options = MergeWhenReadyOptions::default();

        assert!(!options.all);
        assert_eq!(options.method, MergeMethod::Squash);
        assert_eq!(options.timeout_mins, 60);
        assert_eq!(options.poll_interval_secs, 30);
        assert!(!options.no_delete);
        assert!(!options.no_sync);
        assert!(!options.yes);
        assert!(!options.quiet);
    }

    #[test]
    fn test_wait_result_variants() {
        let ready = WaitResult::Ready;
        let failed = WaitResult::Failed("test error".to_string());
        let timeout = WaitResult::Timeout;

        assert!(matches!(ready, WaitResult::Ready));
        assert!(matches!(failed, WaitResult::Failed(_)));
        assert!(matches!(timeout, WaitResult::Timeout));
    }

    #[test]
    fn test_pr_merge_status_ready() {
        let status = PrMergeStatus::ready();
        assert!(status.is_ready());
        assert!(!status.is_blocked());
        assert_eq!(status.status_text(), "Ready to merge");
    }

    #[test]
    fn test_pr_merge_status_blocked() {
        let status = PrMergeStatus::blocked("needs review");
        assert!(!status.is_ready());
        assert!(status.is_blocked());
        assert_eq!(status.status_text(), "needs review");
    }

    #[test]
    fn test_pr_merge_status_not_ready() {
        let status = PrMergeStatus::not_ready("CI pending");
        assert!(!status.is_ready());
        assert!(!status.is_blocked());
        assert_eq!(status.status_text(), "CI pending");
    }

    #[test]
    fn test_merge_method_as_str() {
        assert_eq!(MergeMethod::Squash.as_str(), "squash");
        assert_eq!(MergeMethod::Merge.as_str(), "merge");
        assert_eq!(MergeMethod::Rebase.as_str(), "rebase");
    }
}
