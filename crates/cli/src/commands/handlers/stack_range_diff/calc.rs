//! Calculation layer for stack range-diff - pure functions, no I/O.
//!
//! All functions are deterministic and testable without a VCS backend.

use super::data::{
    CommitPairing, CommitSummary, PairingStatus, RangeDiffFormat, RangeDiffOptions,
    RangeDiffResult, RangeDiffError, RangeSpec,
};

// ============================================================================
// Range Specification
// ============================================================================

/// Build the git range-diff range arguments from options.
///
/// Returns a tuple of (range_a, range_b) in git notation: `base_a..tip_a base_b..tip_b`.
///
/// # Errors
///
/// Returns `RangeDiffError` if any ref is empty.
pub fn build_range_args(options: &RangeDiffOptions) -> Result<(String, String), RangeDiffError> {
    validate_refs(options)?;

    let range_a = format!("{}..{}", options.base_a, options.tip_a);
    let range_b = format!("{}..{}", options.base_b, options.tip_b);

    Ok((range_a, range_b))
}

/// Validate that all required refs are non-empty.
///
/// # Errors
///
/// Returns `RangeDiffError::InvalidRef` if any required ref is empty.
pub fn validate_refs(options: &RangeDiffOptions) -> Result<(), RangeDiffError> {
    let empty_refs: Vec<&str> = [
        ("base_a", options.base_a.as_str()),
        ("tip_a", options.tip_a.as_str()),
        ("base_b", options.base_b.as_str()),
        ("tip_b", options.tip_b.as_str()),
    ]
    .iter()
    .filter(|(_, v)| v.is_empty())
    .map(|(name, _)| *name)
    .collect();

    if empty_refs.is_empty() {
        return Ok(());
    }

    Err(RangeDiffError::InvalidRef {
        ref_name: empty_refs.join(", "),
        reason: "required ref is empty".to_string(),
    })
}

/// Build the format flag for git range-diff from the output format enum.
#[must_use]
pub fn format_flag(format: RangeDiffFormat) -> Option<&'static str> {
    match format {
        RangeDiffFormat::Default => None,
        RangeDiffFormat::Stat => Some("--stat"),
        RangeDiffFormat::Patch => Some("--patch"),
    }
}

/// Build the complete argument list for `git range-diff`.
///
/// Returns the full argument vector (excluding "git" itself).
#[must_use]
pub fn build_git_args(options: &RangeDiffOptions) -> Vec<String> {
    let mut args = vec!["range-diff".to_string()];

    if let Some(factor) = options.creation_factor {
        args.push("--creation-factor".to_string());
        args.push(factor.to_string());
    }

    if options.dual {
        args.push("--dual".to_string());
    }

    if let Some(flag) = format_flag(options.format) {
        args.push(flag.to_string());
    }

    args.push(format!("{}..{}", options.base_a, options.tip_a));
    args.push(format!("{}..{}", options.base_b, options.tip_b));

    args
}

// ============================================================================
// Output Parsing
// ============================================================================

/// Parse the raw git range-diff output into structured pairings.
///
/// Git range-diff output format:
/// ```text
///     1:  aaaa1111 = bbbb2222: Commit subject
///     2:  cccc3333 + dddd4444: Added commit
///     3:  eeee5555 < ffff6666: Removed commit (or just gone)
///     4:  gggg7777 ! hhhh8888: Modified commit
/// ```
#[must_use]
pub fn parse_range_diff_output(raw: &str) -> Vec<CommitPairing> {
    raw.lines().filter_map(parse_pairing_line).collect()
}

/// Parse a single pairing line from range-diff output.
fn parse_pairing_line(line: &str) -> Option<CommitPairing> {
    let trimmed = line.trim();

    // Lines must start with a number followed by colon
    let colon_pos = trimmed.find(':')?;
    let _number_part = &trimmed[..colon_pos];

    let rest = trimmed.get(colon_pos + 1..)?;
    let rest = rest.trim_start();

    // Detect status marker and split hashes
    let (hash_a, status, right_side) = if let Some(idx) = rest.find('=') {
        let (left, right) = rest.split_at(idx);
        (left.trim(), PairingStatus::Unchanged, right.get(1..)?.trim())
    } else if let Some(idx) = rest.find('+') {
        let (left, right) = rest.split_at(idx);
        (left.trim(), PairingStatus::Added, right.get(1..)?.trim())
    } else if let Some(idx) = rest.find('-') {
        let (left, right) = rest.split_at(idx);
        (left.trim(), PairingStatus::Removed, right.get(1..)?.trim())
    } else if let Some(idx) = rest.find('!') {
        let (left, right) = rest.split_at(idx);
        (left.trim(), PairingStatus::Modified, right.get(1..)?.trim())
    } else {
        return None;
    };

    // Parse right side: "hash: subject"
    let (hash_b, subject) = parse_hash_and_subject(right_side)?;

    Some(CommitPairing {
        commit_a: Some(CommitSummary {
            short_hash: hash_a.to_string(),
            subject: subject.clone(),
        }),
        commit_b: Some(CommitSummary {
            short_hash: hash_b.to_string(),
            subject,
        }),
        status,
    })
}

/// Parse "hash: subject" portion after the status marker.
fn parse_hash_and_subject(s: &str) -> Option<(String, String)> {
    let colon_pos = s.find(':')?;
    let hash = s.get(..colon_pos)?.trim();
    let subject = s.get(colon_pos + 1..)?.trim().to_string();
    Some((hash.to_string(), subject))
}

/// Determine if the range-diff output indicates changes.
#[must_use]
pub fn has_changes(raw: &str) -> bool {
    // Non-empty output with any non-whitespace content means changes
    !raw.trim().is_empty()
}

/// Build a range-diff result from raw git output.
#[must_use]
pub fn build_result(raw_output: &str) -> RangeDiffResult {
    let pairings = parse_range_diff_output(raw_output);
    let changes = has_changes(raw_output);

    RangeDiffResult {
        output: raw_output.to_string(),
        pairings,
        has_changes: changes,
    }
}

/// Convenience: build options for comparing a branch before/after rebase.
///
/// Compares `base_a..tip_a` (old) vs `base_b..tip_b` (new).
#[must_use]
pub fn compare_branch_ranges(
    old_base: impl Into<String>,
    old_tip: impl Into<String>,
    new_base: impl Into<String>,
    new_tip: impl Into<String>,
) -> RangeDiffOptions {
    RangeDiffOptions {
        base_a: old_base.into(),
        tip_a: old_tip.into(),
        base_b: new_base.into(),
        tip_b: new_tip.into(),
        ..Default::default()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_refs_all_present() {
        let opts = RangeDiffOptions {
            base_a: "main".to_string(),
            tip_a: "feat".to_string(),
            base_b: "main".to_string(),
            tip_b: "feat".to_string(),
            ..Default::default()
        };
        assert!(validate_refs(&opts).is_ok());
    }

    #[test]
    fn validate_refs_missing_base_a() {
        let opts = RangeDiffOptions {
            base_a: String::new(),
            tip_a: "feat".to_string(),
            base_b: "main".to_string(),
            tip_b: "feat".to_string(),
            ..Default::default()
        };
        let err = validate_refs(&opts).unwrap_err();
        assert!(err.to_string().contains("base_a"));
    }

    #[test]
    fn validate_refs_missing_multiple() {
        let opts = RangeDiffOptions {
            base_a: String::new(),
            tip_a: String::new(),
            base_b: "main".to_string(),
            tip_b: "feat".to_string(),
            ..Default::default()
        };
        let err = validate_refs(&opts).unwrap_err();
        assert!(err.to_string().contains("base_a"));
        assert!(err.to_string().contains("tip_a"));
    }

    #[test]
    fn build_range_args_valid() {
        let opts = RangeDiffOptions {
            base_a: "abc123".to_string(),
            tip_a: "def456".to_string(),
            base_b: "abc123".to_string(),
            tip_b: "ghi789".to_string(),
            ..Default::default()
        };
        let (a, b) = build_range_args(&opts).unwrap();
        assert_eq!(a, "abc123..def456");
        assert_eq!(b, "abc123..ghi789");
    }

    #[test]
    fn format_flag_default_is_none() {
        assert_eq!(format_flag(RangeDiffFormat::Default), None);
    }

    #[test]
    fn format_flag_stat() {
        assert_eq!(format_flag(RangeDiffFormat::Stat), Some("--stat"));
    }

    #[test]
    fn format_flag_patch() {
        assert_eq!(format_flag(RangeDiffFormat::Patch), Some("--patch"));
    }

    #[test]
    fn build_git_args_basic() {
        let opts = RangeDiffOptions {
            base_a: "main".to_string(),
            tip_a: "v1".to_string(),
            base_b: "main".to_string(),
            tip_b: "v2".to_string(),
            ..Default::default()
        };
        let args = build_git_args(&opts);
        assert_eq!(args[0], "range-diff");
        assert!(args.contains(&"main..v1".to_string()));
        assert!(args.contains(&"main..v2".to_string()));
    }

    #[test]
    fn build_git_args_with_creation_factor() {
        let opts = RangeDiffOptions {
            base_a: "main".to_string(),
            tip_a: "v1".to_string(),
            base_b: "main".to_string(),
            tip_b: "v2".to_string(),
            creation_factor: Some(2),
            ..Default::default()
        };
        let args = build_git_args(&opts);
        assert!(args.contains(&"--creation-factor".to_string()));
        assert!(args.contains(&"2".to_string()));
    }

    #[test]
    fn build_git_args_with_dual() {
        let opts = RangeDiffOptions {
            base_a: "main".to_string(),
            tip_a: "v1".to_string(),
            base_b: "main".to_string(),
            tip_b: "v2".to_string(),
            dual: true,
            ..Default::default()
        };
        let args = build_git_args(&opts);
        assert!(args.contains(&"--dual".to_string()));
    }

    #[test]
    fn build_git_args_with_stat_format() {
        let opts = RangeDiffOptions {
            base_a: "main".to_string(),
            tip_a: "v1".to_string(),
            base_b: "main".to_string(),
            tip_b: "v2".to_string(),
            format: RangeDiffFormat::Stat,
            ..Default::default()
        };
        let args = build_git_args(&opts);
        assert!(args.contains(&"--stat".to_string()));
    }

    #[test]
    fn has_changes_empty_string() {
        assert!(!has_changes(""));
        assert!(!has_changes("  \n  \t  "));
    }

    #[test]
    fn has_changes_with_content() {
        assert!(has_changes("1:  abc = def: subject"));
    }

    #[test]
    fn parse_empty_output() {
        let pairings = parse_range_diff_output("");
        assert!(pairings.is_empty());
    }

    #[test]
    fn build_result_empty() {
        let result = build_result("");
        assert!(result.output.is_empty());
        assert!(!result.has_changes);
        assert!(result.pairings.is_empty());
    }

    #[test]
    fn build_result_with_content() {
        let result = build_result("1:  abc = def: subject");
        assert!(!result.output.is_empty());
        assert!(result.has_changes);
    }

    #[test]
    fn compare_branch_ranges_convenience() {
        let opts = compare_branch_ranges("main", "v1", "main", "v2");
        assert_eq!(opts.base_a, "main");
        assert_eq!(opts.tip_a, "v1");
        assert_eq!(opts.base_b, "main");
        assert_eq!(opts.tip_b, "v2");
    }
}
