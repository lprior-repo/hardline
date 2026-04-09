//! Calculation layer for stack log - pure functions only.
//!
//! No I/O. All functions are deterministic and testable.

use scp_stack::{BranchName, Stack};

use super::data::{StackLogBranchEntry, StackLogOutput, StackLogOptions};

/// Compute depth for each branch based on its position in the stack tree.
///
/// Returns a map from branch name to depth (0 for branches whose parent is trunk
/// or not in the stack, incrementing from there).
pub fn compute_depths(
    stack: &Stack,
    trunk: &BranchName,
) -> std::collections::HashMap<BranchName, usize> {
    let mut depths = std::collections::HashMap::new();
    let ordered = stack.topological_order();

    for branch in &ordered {
        let depth = match &branch.parent {
            Some(parent) if parent == trunk => 0,
            Some(parent) => depths
                .get(parent)
                .map(|d| d + 1)
                .unwrap_or(0),
            None => 0,
        };
        depths.insert(branch.name.clone(), depth);
    }

    depths
}

/// Filter stack branches to only those in the lineage of a target branch.
///
/// Returns branches ordered topologically, including the target branch,
/// its ancestors, and its descendants.
pub fn filter_to_lineage<'a>(
    stack: &'a Stack,
    target: &BranchName,
) -> Vec<&'a scp_stack::StackBranch> {
    let ancestors = stack.ancestors(target);
    let descendants = stack.descendants(target);

    let lineage: std::collections::HashSet<&BranchName> = ancestors
        .iter()
        .chain(std::iter::once(target))
        .chain(descendants.iter())
        .collect();

    stack
        .topological_order()
        .into_iter()
        .filter(|b| lineage.contains(&b.name))
        .collect()
}

/// Count total commits across all branch entries.
pub fn count_total_commits(branches: &[StackLogBranchEntry]) -> usize {
    branches.iter().map(|b| b.commits.len()).sum()
}

/// Collect branches that need restacking.
pub fn collect_needs_restack(branches: &[StackLogBranchEntry]) -> Vec<BranchName> {
    branches
        .iter()
        .filter(|b| b.needs_restack)
        .map(|b| b.branch.clone())
        .collect()
}

/// Format the stack log as a tree display string.
pub fn format_tree(output: &StackLogOutput) -> String {
    let mut lines = Vec::new();

    lines.push(format!("{} (trunk)", output.trunk));

    for entry in &output.branches {
        let indent = "  ".repeat(entry.depth);
        let restack_marker = if entry.needs_restack { " *" } else { "" };
        let pr_info = match (entry.pr_number, entry.pr_state.as_deref()) {
            (Some(n), Some(s)) => format!(" [PR #{} ({})]", n, s),
            (Some(n), None) => format!(" [PR #{}]", n),
            _ => String::new(),
        };
        let ahead_behind = if entry.ahead > 0 || entry.behind > 0 {
            format!(" ({} ahead, {} behind)", entry.ahead, entry.behind)
        } else {
            String::new()
        };

        lines.push(format!(
            "{}{} {}{}{}{} commits={}",
            indent,
            if entry.depth > 0 { "|-- " } else { "" },
            entry.branch,
            pr_info,
            ahead_behind,
            restack_marker,
            entry.commits.len(),
        ));

        for commit in &entry.commits {
            lines.push(format!(
                "{}  {} {}",
                indent,
                commit.short_hash,
                commit.message,
            ));
        }
    }

    lines.join("\n")
}

/// Format the stack log as a linear display string.
pub fn format_linear(output: &StackLogOutput) -> String {
    let mut lines = Vec::new();

    for entry in &output.branches {
        lines.push(format!(
            "=== {} (parent: {}) ===",
            entry.branch,
            entry
                .parent
                .as_ref()
                .map(BranchName::as_str)
                .unwrap_or("none"),
        ));

        for commit in &entry.commits {
            lines.push(format!(
                "{} {} ({}, {})",
                commit.short_hash, commit.message, commit.author, commit.datetime,
            ));
        }

        if entry.commits.is_empty() {
            lines.push("  (no unique commits)".to_string());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::{PrInfo, PrState, StackBranch, StackBranch as Sb};

    fn make_branch(name: &str, parent: Option<&str>) -> StackBranch {
        StackBranch {
            name: BranchName::new(name.to_string()),
            parent: parent.map(|p| BranchName::new(p.to_string())),
            children: vec![],
            needs_restack: false,
            pr_info: None,
        }
    }

    fn make_stack(branches: Vec<StackBranch>) -> Stack {
        let main = BranchName::new("main".to_string());
        let mut stack = Stack::new(main);
        for b in branches {
            stack.branches.push(b);
        }
        stack
    }

    #[test]
    fn compute_depths_single_branch() {
        let stack = make_stack(vec![make_branch("feat", Some("main"))]);
        let trunk = BranchName::new("main".to_string());
        let depths = compute_depths(&stack, &trunk);
        assert_eq!(depths.get(&BranchName::new("feat".to_string())), Some(&0));
    }

    #[test]
    fn compute_depths_linear_chain() {
        let stack = make_stack(vec![
            make_branch("a", Some("main")),
            make_branch("b", Some("a")),
            make_branch("c", Some("b")),
        ]);
        let trunk = BranchName::new("main".to_string());
        let depths = compute_depths(&stack, &trunk);
        assert_eq!(depths[&BranchName::new("a".to_string())], 0);
        assert_eq!(depths[&BranchName::new("b".to_string())], 1);
        assert_eq!(depths[&BranchName::new("c".to_string())], 2);
    }

    #[test]
    fn compute_depths_empty_stack() {
        let stack = make_stack(vec![]);
        let trunk = BranchName::new("main".to_string());
        let depths = compute_depths(&stack, &trunk);
        assert!(depths.is_empty());
    }

    #[test]
    fn compute_depths_fan_out() {
        let stack = make_stack(vec![
            make_branch("a", Some("main")),
            make_branch("b", Some("main")),
            make_branch("c", Some("main")),
        ]);
        let trunk = BranchName::new("main".to_string());
        let depths = compute_depths(&stack, &trunk);
        assert_eq!(depths[&BranchName::new("a".to_string())], 0);
        assert_eq!(depths[&BranchName::new("b".to_string())], 0);
        assert_eq!(depths[&BranchName::new("c".to_string())], 0);
    }

    #[test]
    fn count_total_commits_empty() {
        let branches: Vec<StackLogBranchEntry> = vec![];
        assert_eq!(count_total_commits(&branches), 0);
    }

    #[test]
    fn count_total_commits_with_entries() {
        let branches = vec![
            StackLogBranchEntry {
                branch: BranchName::new("a".to_string()),
                parent: None,
                depth: 0,
                commits: vec![
                    super::super::data::StackLogCommit {
                        short_hash: "abc".to_string(),
                        hash: "abcdef".to_string(),
                        message: "m1".to_string(),
                        author: "a".to_string(),
                        datetime: "2026-01-01".to_string(),
                    },
                ],
                ahead: 1,
                behind: 0,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
            },
            StackLogBranchEntry {
                branch: BranchName::new("b".to_string()),
                parent: None,
                depth: 0,
                commits: vec![
                    super::super::data::StackLogCommit {
                        short_hash: "def".to_string(),
                        hash: "defghi".to_string(),
                        message: "m2".to_string(),
                        author: "b".to_string(),
                        datetime: "2026-01-02".to_string(),
                    },
                    super::super::data::StackLogCommit {
                        short_hash: "ghi".to_string(),
                        hash: "ghijkl".to_string(),
                        message: "m3".to_string(),
                        author: "c".to_string(),
                        datetime: "2026-01-03".to_string(),
                    },
                ],
                ahead: 2,
                behind: 0,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
            },
        ];
        assert_eq!(count_total_commits(&branches), 3);
    }

    #[test]
    fn collect_needs_restack_filters() {
        let branches = vec![
            StackLogBranchEntry {
                branch: BranchName::new("clean".to_string()),
                parent: None,
                depth: 0,
                commits: vec![],
                ahead: 0,
                behind: 0,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
            },
            StackLogBranchEntry {
                branch: BranchName::new("dirty".to_string()),
                parent: None,
                depth: 0,
                commits: vec![],
                ahead: 0,
                behind: 0,
                needs_restack: true,
                pr_number: None,
                pr_state: None,
            },
        ];
        let result = collect_needs_restack(&branches);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], BranchName::new("dirty".to_string()));
    }

    #[test]
    fn format_tree_basic() {
        let output = StackLogOutput {
            branches: vec![StackLogBranchEntry {
                branch: BranchName::new("feature-a".to_string()),
                parent: Some(BranchName::new("main".to_string())),
                depth: 0,
                commits: vec![super::super::data::StackLogCommit {
                    short_hash: "abc1234567".to_string(),
                    hash: "abc1234567890".to_string(),
                    message: "Add feature".to_string(),
                    author: "dev".to_string(),
                    datetime: "2026-04-09".to_string(),
                }],
                ahead: 1,
                behind: 0,
                needs_restack: false,
                pr_number: Some(42),
                pr_state: Some("open".to_string()),
            }],
            trunk: BranchName::new("main".to_string()),
            total_branches: 1,
            total_commits: 1,
            needs_restack: vec![],
        };
        let result = format_tree(&output);
        assert!(result.contains("main (trunk)"));
        assert!(result.contains("feature-a"));
        assert!(result.contains("PR #42 (open)"));
        assert!(result.contains("abc1234567"));
        assert!(result.contains("Add feature"));
    }

    #[test]
    fn format_tree_with_needs_restack() {
        let output = StackLogOutput {
            branches: vec![StackLogBranchEntry {
                branch: BranchName::new("stale".to_string()),
                parent: None,
                depth: 0,
                commits: vec![],
                ahead: 0,
                behind: 3,
                needs_restack: true,
                pr_number: None,
                pr_state: None,
            }],
            trunk: BranchName::new("main".to_string()),
            total_branches: 1,
            total_commits: 0,
            needs_restack: vec![BranchName::new("stale".to_string())],
        };
        let result = format_tree(&output);
        assert!(result.contains("*"));
        assert!(result.contains("3 behind"));
    }

    #[test]
    fn format_linear_basic() {
        let output = StackLogOutput {
            branches: vec![StackLogBranchEntry {
                branch: BranchName::new("feat".to_string()),
                parent: Some(BranchName::new("main".to_string())),
                depth: 0,
                commits: vec![super::super::data::StackLogCommit {
                    short_hash: "abc".to_string(),
                    hash: "abcdef".to_string(),
                    message: "msg".to_string(),
                    author: "dev".to_string(),
                    datetime: "2026-04-09".to_string(),
                }],
                ahead: 1,
                behind: 0,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
            }],
            trunk: BranchName::new("main".to_string()),
            total_branches: 1,
            total_commits: 1,
            needs_restack: vec![],
        };
        let result = format_linear(&output);
        assert!(result.contains("=== feat (parent: main) ==="));
        assert!(result.contains("abc msg (dev, 2026-04-09)"));
    }

    #[test]
    fn format_linear_empty_commits() {
        let output = StackLogOutput {
            branches: vec![StackLogBranchEntry {
                branch: BranchName::new("empty".to_string()),
                parent: None,
                depth: 0,
                commits: vec![],
                ahead: 0,
                behind: 0,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
            }],
            trunk: BranchName::new("main".to_string()),
            total_branches: 1,
            total_commits: 0,
            needs_restack: vec![],
        };
        let result = format_linear(&output);
        assert!(result.contains("(no unique commits)"));
    }
}
