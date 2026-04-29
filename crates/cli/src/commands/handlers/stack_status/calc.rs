//! Calculation layer for stack status - pure functions for display logic.
//!
//! No I/O. Deterministic transformations only.

use std::collections::{HashMap, HashSet};

use super::data::{BranchStatusJson, DisplayBranch, StackStatusOptions};

pub fn compute_display_branches(
    trunk_children: Vec<String>,
    allowed_branches: Option<&HashSet<String>>,
) -> (Vec<DisplayBranch>, usize) {
    let mut display_branches: Vec<DisplayBranch> = Vec::new();
    let mut max_column = 0;
    let mut sorted_trunk_children = trunk_children;
    sorted_trunk_children.sort();

    for (i, root) in sorted_trunk_children.iter().enumerate() {
        collect_display_branches_with_nesting(
            root,
            i,
            &mut display_branches,
            &mut max_column,
            allowed_branches,
        );
    }

    (display_branches, max_column)
}

fn collect_display_branches_with_nesting(
    branch: &str,
    base_column: usize,
    result: &mut Vec<DisplayBranch>,
    max_column: &mut usize,
    allowed: Option<&HashSet<String>>,
) {
    #[derive(Clone)]
    struct Frame {
        branch: String,
        column: usize,
        expanded: bool,
    }

    let mut stack_frames = vec![Frame {
        branch: branch.to_string(),
        column: base_column,
        expanded: false,
    }];
    let mut visiting = HashSet::new();
    let mut emitted = HashSet::new();

    while let Some(frame) = stack_frames.pop() {
        if let Some(set) = allowed {
            if !set.contains(&frame.branch) {
                continue;
            }
        }

        if frame.expanded {
            visiting.remove(&frame.branch);
            if emitted.insert(frame.branch.clone()) {
                result.push(DisplayBranch {
                    name: frame.branch,
                    column: frame.column,
                });
            }
            continue;
        }

        if emitted.contains(&frame.branch) || !visiting.insert(frame.branch.clone()) {
            continue;
        }

        *max_column = (*max_column).max(frame.column);
        stack_frames.push(Frame {
            branch: frame.branch.clone(),
            column: frame.column,
            expanded: true,
        });
    }
}

pub fn build_branch_statuses(
    display_branches: &[DisplayBranch],
    trunk: &str,
    current: &str,
    branch_info_map: &HashMap<String, BranchInfo>,
    linked_worktrees: &HashMap<String, String>,
    remote_branches: &HashSet<String>,
    ci_states: &HashMap<String, String>,
) -> (Vec<BranchStatusJson>, HashMap<String, BranchStatusJson>) {
    let mut branch_statuses: Vec<BranchStatusJson> = Vec::new();
    let mut branch_status_map: HashMap<String, BranchStatusJson> = HashMap::new();

    for db in display_branches {
        let info = branch_info_map.get(&db.name);
        let parent = info.and_then(|b| b.parent.clone());
        let is_trunk = db.name == trunk;

        let pr_state = info
            .and_then(|b| b.pr_state.clone())
            .filter(|s| !s.trim().is_empty());

        let pr_number = info.and_then(|b| b.pr_number);
        let ci_state = ci_states.get(&db.name).cloned();

        let entry = BranchStatusJson {
            name: db.name.clone(),
            parent: parent.clone(),
            is_current: db.name == current,
            is_trunk,
            linked_worktree: linked_worktrees.get(&db.name).cloned(),
            needs_restack: info.map(|b| b.needs_restack).unwrap_or(false),
            pr_number,
            pr_state,
            pr_is_draft: info.and_then(|b| b.pr_is_draft),
            pr_url: None,
            ci_state,
            ahead: info.map(|b| b.ahead).unwrap_or(0),
            behind: info.map(|b| b.behind).unwrap_or(0),
            lines_added: None,
            lines_deleted: None,
            has_remote: remote_branches.contains(&db.name),
        };

        branch_status_map.insert(db.name.clone(), entry.clone());
        branch_statuses.push(entry);
    }

    (branch_statuses, branch_status_map)
}

pub fn format_tree_element(
    db: &DisplayBranch,
    is_current: bool,
    has_remote: bool,
    has_linked_worktree: bool,
    ahead: usize,
    behind: usize,
    needs_restack: bool,
    pr_info: Option<(&u64, Option<&str>, bool)>,
    ci_state: Option<&str>,
    verbose: bool,
    tree_target_width: usize,
    _max_column: usize,
) -> String {
    let mut output = String::new();

    let col_color_idx = db.column % 8;
    let col_color = get_column_color(col_color_idx);

    // Draw tree connector
    for col in 0..=db.column {
        if col == db.column {
            let circle = if is_current { "\u{25c9}" } else { "\u{25cb}" };
            output.push_str(&format!("{}{}", col_color, circle));
            output.push_str(&format!("{}\u{2500}\u{252b}", col_color));
        } else {
            output.push_str(&format!("{}\u{2502} ", col_color));
        }
    }

    // Pad to target width
    while output.chars().count() < tree_target_width {
        output.push(' ');
    }
    output.push(' ');

    // Indicators
    append_remote_indicator(&mut output, has_remote);
    append_worktree_indicator(&mut output, has_linked_worktree, col_color);
    append_branch_name(&mut output, &db.name, is_current, col_color);
    append_ahead_behind(&mut output, ahead, behind);
    append_restack_indicator(&mut output, needs_restack);

    if verbose {
        append_verbose_info(&mut output, pr_info, ci_state);
    }

    output
}

fn append_remote_indicator(output: &mut String, has_remote: bool) {
    if has_remote {
        output.push_str("\u{2601}\u{fe0f} ");
    } else {
        output.push_str("   ");
    }
}

fn append_worktree_indicator(output: &mut String, has_linked_worktree: bool, col_color: &str) {
    if has_linked_worktree {
        output.push_str(&format!("{}\u{21b3} ", col_color));
    }
}

fn append_branch_name(output: &mut String, name: &str, is_current: bool, col_color: &str) {
    if is_current {
        output.push_str(&format!("\u{1b}[1m{}{}\u{1b}[0m", col_color, name));
    } else {
        output.push_str(&format!("{}{}", col_color, name));
    }
}

fn append_ahead_behind(output: &mut String, ahead: usize, behind: usize) {
    if behind > 0 {
        output.push_str(&format!(" \u{1b}[31m{} behind\u{1b}[0m", behind));
    }
    if ahead > 0 {
        output.push_str(&format!(" \u{1b}[32m{} ahead\u{1b}[0m", ahead));
    }
}

fn append_restack_indicator(output: &mut String, needs_restack: bool) {
    if needs_restack {
        output.push_str(" \u{1b}[93m(needs restack)\u{1b}[0m");
    }
}

fn append_verbose_info(
    output: &mut String,
    pr_info: Option<(&u64, Option<&str>, bool)>,
    ci_state: Option<&str>,
) {
    if let Some((pr_num, pr_state, is_draft)) = pr_info {
        let mut pr_text = format!(" PR #{}", pr_num);
        if let Some(state) = pr_state {
            pr_text.push_str(&format!(" {}", state.to_lowercase()));
        }
        if is_draft {
            pr_text.push_str(" draft");
        }
        output.push_str(&format!("\u{1b}[95m{}\u{1b}[0m", pr_text));
    }

    if let Some(ci) = ci_state {
        output.push_str(&format!("\u{1b}[96m CI:{}\u{1b}[0m", ci));
    }
}

pub fn format_trunk_display(
    trunk: &str,
    is_trunk_current: bool,
    has_remote: bool,
    has_linked_worktree: bool,
    ahead: usize,
    behind: usize,
    tree_target_width: usize,
    max_column: usize,
) -> String {
    let mut output = String::new();
    let col_color = get_column_color(0);

    let circle = if is_trunk_current {
        "\u{25c9}"
    } else {
        "\u{25cb}"
    };
    output.push_str(&format!("{}{}", col_color, circle));

    // Draw horizontal connectors for remaining columns
    for col in 1..=max_column {
        let cc = get_column_color(col % 8);
        if col < max_column {
            output.push_str(&format!("{}\u{2500}\u{253b}", cc));
        } else {
            output.push_str(&format!("{}\u{2500}\u{252b}", cc));
        }
    }

    // Pad to target width
    while output.chars().count() < tree_target_width {
        output.push(' ');
    }
    output.push(' ');

    append_remote_indicator(&mut output, has_remote);
    append_worktree_indicator(&mut output, has_linked_worktree, col_color);
    append_branch_name(&mut output, trunk, is_trunk_current, col_color);
    append_ahead_behind(&mut output, ahead, behind);

    output
}

pub fn format_compact_line(entry: &BranchStatusJson) -> String {
    let parent = entry.parent.clone().unwrap_or_default();
    let pr_state = entry.pr_state.clone().unwrap_or_default();
    let pr_number = entry.pr_number.map(|n| n.to_string()).unwrap_or_default();
    let ci_state = entry.ci_state.clone().unwrap_or_default();

    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        entry.name,
        parent,
        entry.ahead,
        entry.behind,
        pr_number,
        pr_state,
        ci_state,
        if entry.needs_restack { "restack" } else { "" }
    )
}

fn get_column_color(idx: usize) -> &'static str {
    match idx % 8 {
        0 => "\u{1b}[36m",
        1 => "\u{1b}[32m",
        2 => "\u{1b}[35m",
        3 => "\u{1b}[34m",
        4 => "\u{1b}[96m",
        5 => "\u{1b}[92m",
        6 => "\u{1b}[95m",
        7 => "\u{1b}[94m",
        _ => "\u{1b}[36m",
    }
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub parent: Option<String>,
    pub needs_restack: bool,
    pub pr_number: Option<u64>,
    pub pr_state: Option<String>,
    pub pr_is_draft: Option<bool>,
    pub ahead: usize,
    pub behind: usize,
}

pub fn compute_stats(branch_statuses: &[BranchStatusJson]) -> (usize, usize, usize) {
    let total_branches = branch_statuses.len();
    let open_prs: usize = branch_statuses
        .iter()
        .filter(|b| {
            b.pr_number.is_some()
                && b.pr_state
                    .as_ref()
                    .map(|s| s.to_lowercase() == "open")
                    .unwrap_or(false)
        })
        .count();
    let branches_with_remote: usize = branch_statuses
        .iter()
        .filter(|b| b.has_remote && !b.is_trunk)
        .count();
    (total_branches, open_prs, branches_with_remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_display_branches_single_trunk_child() {
        let trunk_children = vec!["feature-a".to_string()];
        let (branches, max_col) = compute_display_branches(trunk_children, None);
        assert_eq!(branches.len(), 1);
        assert_eq!(max_col, 0);
    }

    #[test]
    fn compute_display_branches_empty() {
        let trunk_children: Vec<String> = vec![];
        let (branches, max_col) = compute_display_branches(trunk_children, None);
        assert!(branches.is_empty());
        assert_eq!(max_col, 0);
    }

    #[test]
    fn compute_display_branches_with_filter() {
        let trunk_children = vec!["feature-a".to_string(), "feature-b".to_string()];
        let filter: HashSet<String> = ["feature-a".to_string()].into_iter().collect();
        let (branches, _) = compute_display_branches(trunk_children, Some(&filter));
        assert!(branches.iter().all(|b| b.name == "feature-a"));
    }

    #[test]
    fn format_compact_line_basic() {
        let entry = BranchStatusJson {
            name: "feature-a".to_string(),
            parent: Some("main".to_string()),
            is_current: false,
            is_trunk: false,
            linked_worktree: None,
            needs_restack: false,
            pr_number: Some(42),
            pr_state: Some("open".to_string()),
            pr_is_draft: Some(false),
            pr_url: None,
            ci_state: Some("success".to_string()),
            ahead: 3,
            behind: 1,
            lines_added: None,
            lines_deleted: None,
            has_remote: true,
        };
        let line = format_compact_line(&entry);
        assert!(line.contains("feature-a"));
        assert!(line.contains("main"));
        assert!(line.contains("42"));
        assert!(line.contains("open"));
    }

    #[test]
    fn compute_stats_empty() {
        let statuses: Vec<BranchStatusJson> = vec![];
        let (total, open_prs, with_remote) = compute_stats(&statuses);
        assert_eq!(total, 0);
        assert_eq!(open_prs, 0);
        assert_eq!(with_remote, 0);
    }

    #[test]
    fn compute_stats_with_branches() {
        let statuses = vec![
            BranchStatusJson {
                name: "main".to_string(),
                parent: None,
                is_current: false,
                is_trunk: true,
                linked_worktree: None,
                needs_restack: false,
                pr_number: None,
                pr_state: None,
                pr_is_draft: None,
                pr_url: None,
                ci_state: None,
                ahead: 0,
                behind: 0,
                lines_added: None,
                lines_deleted: None,
                has_remote: true,
            },
            BranchStatusJson {
                name: "feature-a".to_string(),
                parent: Some("main".to_string()),
                is_current: true,
                is_trunk: false,
                linked_worktree: None,
                needs_restack: false,
                pr_number: Some(1),
                pr_state: Some("open".to_string()),
                pr_is_draft: Some(false),
                pr_url: None,
                ci_state: Some("success".to_string()),
                ahead: 5,
                behind: 0,
                lines_added: None,
                lines_deleted: None,
                has_remote: true,
            },
        ];
        let (total, open_prs, with_remote) = compute_stats(&statuses);
        assert_eq!(total, 2);
        assert_eq!(open_prs, 1);
        assert_eq!(with_remote, 1);
    }
}
