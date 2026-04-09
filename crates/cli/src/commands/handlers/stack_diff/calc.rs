//! Calculation layer for stack diff - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use scp_stack::{BranchName, Stack};

use super::data::{BranchDiff, DiffError, DiffRange, FileStat, StackDiffResult};

// ============================================================================
// Branch Selection
// ============================================================================

/// Select branches to diff based on range options.
///
/// - No range: all branches in topological order
/// - Range with start only: start and all its descendants
/// - Range with end only: end and all its ancestors
/// - Range with both: path from start to end
///
/// # Errors
///
/// Returns `DiffError::NoStackBranches` if the stack has no branches.
/// Returns `DiffError::BranchNotFound` if range references a missing branch.
/// Returns `DiffError::InvalidRange` if start is not an ancestor of end.
pub fn select_branches(
    stack: &Stack,
    range: &Option<DiffRange>,
) -> Result<Vec<(BranchName, BranchName)>, DiffError> {
    if stack.branches.is_empty() {
        return Err(DiffError::NoStackBranches);
    }

    match range {
        None => all_branch_pairs(stack),
        Some(r) => ranged_branch_pairs(stack, r),
    }
}

/// Get all branch→parent pairs in topological order.
fn all_branch_pairs(stack: &Stack) -> Result<Vec<(BranchName, BranchName)>, DiffError> {
    let topo = stack.topological_order();
    let pairs: Vec<(BranchName, BranchName)> = topo
        .iter()
        .map(|b| {
            let parent = b
                .parent
                .clone()
                .unwrap_or_else(|| stack.main_branch.clone());
            (b.name.clone(), parent)
        })
        .collect();
    Ok(pairs)
}

/// Get branch→parent pairs for a range within the stack.
fn ranged_branch_pairs(
    stack: &Stack,
    range: &DiffRange,
) -> Result<Vec<(BranchName, BranchName)>, DiffError> {
    // Validate start branch exists
    if let Some(ref start) = range.start {
        if !stack.branches.iter().any(|b| &b.name == start) {
            return Err(DiffError::BranchNotFound(start.to_string()));
        }
    }

    // Validate end branch exists
    if let Some(ref end) = range.end {
        if !stack.branches.iter().any(|b| &b.name == end) {
            return Err(DiffError::BranchNotFound(end.to_string()));
        }
    }

    let all_pairs = all_branch_pairs(stack)?;
    match (&range.start, &range.end) {
        // Start only: start + all descendants (walk parent pointers)
        (Some(start), None) => {
            let mut result = Vec::new();
            let mut included: std::collections::HashSet<BranchName> =
                std::collections::HashSet::from_iter(std::iter::once(start.clone()));

            // Walk all branches; include any whose ancestor chain includes start
            for (branch, parent) in &all_pairs {
                if included.contains(branch) {
                    result.push((branch.clone(), parent.clone()));
                    continue;
                }
                let ancestors = stack.ancestors(branch);
                if ancestors.contains(start) {
                    included.insert(branch.clone());
                    result.push((branch.clone(), parent.clone()));
                }
            }
            Ok(result)
        }
        // End only: end + all ancestors
        (None, Some(end)) => {
            let mut result = Vec::new();
            let ancestors = stack.ancestors(end);
            let mut included: std::collections::HashSet<BranchName> = ancestors.into_iter().collect();
            included.insert(end.clone());

            for (branch, parent) in &all_pairs {
                if included.contains(branch) {
                    result.push((branch.clone(), parent.clone()));
                }
            }
            Ok(result)
        }
        // Both: validate start is ancestor of end, then return path
        (Some(start), Some(end)) => {
            let ancestors = stack.ancestors(end);
            if start != end && !ancestors.contains(start) {
                return Err(DiffError::InvalidRange {
                    start: start.to_string(),
                    end: end.to_string(),
                });
            }

            let mut included: std::collections::HashSet<BranchName> = std::iter::once(start.clone()).collect();

            // Walk from end back to start through ancestors
            included.insert(end.clone());
            for anc in &ancestors {
                included.insert(anc.clone());
                if anc == start {
                    break;
                }
            }

            let mut result = Vec::new();
            for (branch, parent) in &all_pairs {
                if included.contains(branch) {
                    result.push((branch.clone(), parent.clone()));
                }
            }
            Ok(result)
        }
        // Neither: shouldn't reach here, but return all
        (None, None) => all_branch_pairs(stack),
    }
}

// ============================================================================
// Result Aggregation
// ============================================================================

/// Aggregate per-branch diffs into a final result with totals.
///
/// # Invariants
///
/// - `total_additions` equals the sum of all branch additions
/// - `total_deletions` equals the sum of all branch deletions
/// - `total_files_changed` equals the count of unique file paths
#[must_use]
pub fn aggregate_result(branch_diffs: Vec<BranchDiff>) -> StackDiffResult {
    let total_additions: usize = branch_diffs.iter().map(|b| b.additions).sum();
    let total_deletions: usize = branch_diffs.iter().map(|b| b.deletions).sum();

    let mut seen_paths: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for bd in &branch_diffs {
        for fs in &bd.file_stats {
            seen_paths.insert(&fs.path);
        }
    }
    let total_files_changed = seen_paths.len();

    StackDiffResult {
        branch_diffs,
        total_additions,
        total_deletions,
        total_files_changed,
    }
}

/// Parse numstat output lines into FileStat entries.
///
/// Input format: `additions\tdeletions\tfilepath`
/// Binary files show as `-\t-\tfilepath`.
#[must_use]
pub fn parse_numstat(lines: &[String]) -> Vec<FileStat> {
    let mut stats = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let additions: usize = parts[0].parse().unwrap_or(0);
            let deletions: usize = parts[1].parse().unwrap_or(0);
            let path = parts[2].to_string();
            stats.push(FileStat {
                path,
                additions,
                deletions,
            });
        }
    }
    stats
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use scp_stack::StackBranch;

    fn bn(name: &str) -> BranchName {
        BranchName::new(name.to_string())
    }

    fn make_stack(branches: Vec<(&str, Option<&str>)>) -> Stack {
        let main = bn("main");
        let mut stack = Stack::new(main.clone());
        for (name, parent) in branches {
            let parent_bn = parent.map(|p| bn(p));
            let children = Vec::new();
            stack
                .add_branch(StackBranch {
                    name: bn(name),
                    parent: parent_bn,
                    children,
                    needs_restack: false,
                    pr_info: None,
                })
                .ok();
        }
        stack
    }

    // ---- select_branches ----

    #[test]
    fn select_all_branches_no_range() {
        let stack = make_stack(vec![("a", Some("main")), ("b", Some("a"))]);
        let pairs = select_branches(&stack, &None).expect("ok");
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn select_empty_stack_errors() {
        let stack = make_stack(vec![]);
        let result = select_branches(&stack, &None);
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(DiffError::NoStackBranches)));
    }

    #[test]
    fn select_range_start_not_found() {
        let stack = make_stack(vec![("a", Some("main"))]);
        let range = DiffRange {
            start: Some(bn("ghost")),
            end: None,
        };
        let result = select_branches(&stack, &Some(range));
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(DiffError::BranchNotFound(_))));
    }

    #[test]
    fn select_range_end_not_found() {
        let stack = make_stack(vec![("a", Some("main"))]);
        let range = DiffRange {
            start: None,
            end: Some(bn("ghost")),
        };
        let result = select_branches(&stack, &Some(range));
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(DiffError::BranchNotFound(_))));
    }

    #[test]
    fn select_range_start_only_includes_descendants() {
        let stack = make_stack(vec![
            ("a", Some("main")),
            ("b", Some("a")),
            ("c", Some("b")),
            ("d", Some("main")),
        ]);
        let range = DiffRange {
            start: Some(bn("a")),
            end: None,
        };
        let pairs = select_branches(&stack, &Some(range)).expect("ok");
        let names: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
        assert!(!names.contains(&"d"));
    }

    #[test]
    fn select_range_end_only_includes_ancestors() {
        let stack = make_stack(vec![
            ("a", Some("main")),
            ("b", Some("a")),
            ("c", Some("b")),
        ]);
        let range = DiffRange {
            start: None,
            end: Some(bn("c")),
        };
        let pairs = select_branches(&stack, &Some(range)).expect("ok");
        let names: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn select_range_both_valid_path() {
        let stack = make_stack(vec![
            ("a", Some("main")),
            ("b", Some("a")),
            ("c", Some("b")),
        ]);
        let range = DiffRange {
            start: Some(bn("a")),
            end: Some(bn("b")),
        };
        let pairs = select_branches(&stack, &Some(range)).expect("ok");
        let names: Vec<&str> = pairs.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(!names.contains(&"c"));
    }

    #[test]
    fn select_range_both_invalid_path() {
        let stack = make_stack(vec![
            ("a", Some("main")),
            ("b", Some("a")),
            ("c", Some("main")),
        ]);
        // c is not a descendant of a
        let range = DiffRange {
            start: Some(bn("a")),
            end: Some(bn("c")),
        };
        let result = select_branches(&stack, &Some(range));
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(DiffError::InvalidRange { .. })));
    }

    #[test]
    fn select_range_same_start_end() {
        let stack = make_stack(vec![("a", Some("main")), ("b", Some("a"))]);
        let range = DiffRange {
            start: Some(bn("a")),
            end: Some(bn("a")),
        };
        let pairs = select_branches(&stack, &Some(range)).expect("ok");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, bn("a"));
    }

    // ---- aggregate_result ----

    #[test]
    fn aggregate_empty() {
        let result = aggregate_result(Vec::new());
        assert_eq!(result.total_additions, 0);
        assert_eq!(result.total_deletions, 0);
        assert_eq!(result.total_files_changed, 0);
    }

    #[test]
    fn aggregate_sums_totals() {
        let diffs = vec![
            BranchDiff::new(
                bn("a"),
                bn("main"),
                vec![
                    FileStat {
                        path: "a.rs".to_string(),
                        additions: 10,
                        deletions: 2,
                    },
                ],
            ),
            BranchDiff::new(
                bn("b"),
                bn("a"),
                vec![
                    FileStat {
                        path: "b.rs".to_string(),
                        additions: 5,
                        deletions: 3,
                    },
                ],
            ),
        ];
        let result = aggregate_result(diffs);
        assert_eq!(result.total_additions, 15);
        assert_eq!(result.total_deletions, 5);
        assert_eq!(result.total_files_changed, 2);
    }

    #[test]
    fn aggregate_deduplicates_files() {
        let diffs = vec![
            BranchDiff::new(
                bn("a"),
                bn("main"),
                vec![FileStat {
                    path: "shared.rs".to_string(),
                    additions: 10,
                    deletions: 2,
                }],
            ),
            BranchDiff::new(
                bn("b"),
                bn("a"),
                vec![FileStat {
                    path: "shared.rs".to_string(),
                    additions: 5,
                    deletions: 1,
                }],
            ),
        ];
        let result = aggregate_result(diffs);
        assert_eq!(result.total_files_changed, 1);
        assert_eq!(result.total_additions, 15);
        assert_eq!(result.total_deletions, 3);
    }

    // ---- parse_numstat ----

    #[test]
    fn parse_numstat_valid_lines() {
        let lines = vec![
            "10\t2\tsrc/main.rs".to_string(),
            "5\t3\tlib/mod.rs".to_string(),
        ];
        let stats = parse_numstat(&lines);
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0].path, "src/main.rs");
        assert_eq!(stats[0].additions, 10);
        assert_eq!(stats[0].deletions, 2);
        assert_eq!(stats[1].path, "lib/mod.rs");
        assert_eq!(stats[1].additions, 5);
        assert_eq!(stats[1].deletions, 3);
    }

    #[test]
    fn parse_numstat_binary_files() {
        let lines = vec!["-\t-\timage.png".to_string()];
        let stats = parse_numstat(&lines);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "image.png");
        assert_eq!(stats[0].additions, 0);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn parse_numstat_empty() {
        let stats = parse_numstat(&[]);
        assert!(stats.is_empty());
    }

    #[test]
    fn parse_numstat_malformed() {
        let lines = vec!["garbage".to_string(), "1\t2".to_string()];
        let stats = parse_numstat(&lines);
        assert!(stats.is_empty());
    }

    #[test]
    fn parse_numstat_partial() {
        let lines = vec![
            "garbage".to_string(),
            "3\t1\tvalid.rs".to_string(),
        ];
        let stats = parse_numstat(&lines);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "valid.rs");
    }
}
