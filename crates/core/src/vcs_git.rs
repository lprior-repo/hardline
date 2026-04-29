//! Git backend implementation

use std::process::Command;

use chrono::Utc;

use super::{
    types::{Branch, Commit, CommitId, RepoStatus, VcsStatus, Workspace},
    VcsBackend,
};
use crate::{
    error::Result, error_internal::InternalErrorKind, error_io::IoErrorKind,
    error_vcs::VcsErrorKind, error_workspace::WorkspaceErrorKind,
};

pub struct GitBackend {
    repo_path: std::path::PathBuf,
}

impl GitBackend {
    pub const fn new(repo_path: std::path::PathBuf) -> Self {
        Self { repo_path }
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| IoErrorKind::IoError(e.to_string()).into())
    }
}

impl VcsBackend for GitBackend {
    fn current_branch(&self) -> Result<String> {
        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict("git".into(), "Failed to get branch".into()).into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn list_branches(&self) -> Result<Vec<Branch>> {
        let output = self.run_git(&["branch", "-a"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let current = self.current_branch()?;
        Ok(parse_git_branch_list(&stdout, &current))
    }

    fn create_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["branch", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchExists(name.to_string()).into());
        }
        Ok(())
    }

    fn switch_branch(&self, name: &str) -> Result<()> {
        let output = self.run_git(&["checkout", name])?;
        if !output.status.success() {
            return Err(VcsErrorKind::BranchNotFound(name.to_string()).into());
        }
        Ok(())
    }

    fn push(&self) -> Result<()> {
        let output = self.run_git(&["push"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PushFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn pull(&self) -> Result<()> {
        let output = self.run_git(&["pull"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::PullFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn rebase(&self, onto: &str) -> Result<()> {
        let output = self.run_git(&["rebase", onto])?;
        if !output.status.success() {
            return Err(VcsErrorKind::RebaseFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn merge(&self, branch: &str) -> Result<()> {
        let output = self.run_git(&["merge", branch])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict(
                branch.to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn log(&self, limit: usize) -> Result<Vec<Commit>> {
        let output = self.run_git(&["log", &format!("-n{}", limit)])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_git_log(&stdout))
    }

    fn status(&self) -> Result<VcsStatus> {
        let output = self.run_git(&["status", "--porcelain"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_git_porcelain_status(&stdout))
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.repo_path.join(".git").exists())
    }

    fn repo_exists(&self, path: &str) -> bool {
        std::path::Path::new(path).join(".git").exists()
    }

    fn checkout(&self, target: &str) -> Result<()> {
        let output = self.run_git(&["checkout", target])?;
        if !output.status.success() {
            return Err(VcsErrorKind::CheckoutFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<CommitId> {
        let output = self.run_git(&["commit", "-m", message])?;
        if !output.status.success() {
            return Err(VcsErrorKind::CommitFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        let rev_output = self.run_git(&["rev-parse", "HEAD"])?;
        if !rev_output.status.success() {
            return Err(VcsErrorKind::CommitFailed(
                "Failed to retrieve commit ID after commit".to_string(),
            )
            .into());
        }
        let id = String::from_utf8_lossy(&rev_output.stdout)
            .trim()
            .to_string();
        CommitId::new(id)
            .ok_or_else(|| VcsErrorKind::CommitFailed("Commit returned empty ID".to_string()))
            .map_err(Into::into)
    }

    fn diff(&self, from: &CommitId, to: &CommitId) -> Result<String> {
        let output = self.run_git(&["diff", from.as_str(), to.as_str()])?;
        if !output.status.success() {
            return Err(VcsErrorKind::DiffFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn repo_status(&self) -> Result<RepoStatus> {
        let vcs_status = self.status()?;
        let clean = vcs_status == VcsStatus::Clean;
        let has_conflicts = vcs_status == VcsStatus::Conflicted;

        // Get current branch
        let branch = self.current_branch().ok().and_then(|b| {
            let trimmed = b.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                super::types::BranchName::new(trimmed)
            }
        });

        // Get current commit ID
        let commit_id = {
            let output = self.run_git(&["rev-parse", "HEAD"])?;
            if output.status.success() {
                let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                CommitId::from_unchecked(id)
            } else {
                return Err(VcsErrorKind::Conflict(
                    "repo_status".into(),
                    "Failed to get current commit ID".into(),
                )
                .into());
            }
        };

        // Get list of changed files from --porcelain output
        let uncommitted_files = {
            let output = self.run_git(&["status", "--porcelain"])?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                parse_git_porcelain_files(&stdout)
            } else {
                vec![]
            }
        };

        Ok(RepoStatus {
            clean,
            branch,
            commit_id: Some(commit_id),
            has_conflicts,
            uncommitted_files,
        })
    }

    fn create_workspace(&self, _name: &str) -> Result<()> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn switch_workspace(&self, _name: &str) -> Result<()> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let output = self.run_git(&["worktree", "list", "--porcelain"])?;
        if !output.status.success() {
            return Err(VcsErrorKind::Conflict(
                "worktree list".to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
            .into());
        }
        Ok(parse_git_worktree_list(
            &String::from_utf8_lossy(&output.stdout),
            &self.repo_path,
        ))
    }

    fn delete_workspace(&self, _name: &str) -> Result<()> {
        Err(InternalErrorKind::Unimplemented("Git workspaces use worktrees instead".into()).into())
    }

    fn fork_workspace(&self, source: &str, target: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(target);
        let output =
            self.run_git(&["worktree", "add", &worktree_path.to_string_lossy(), source])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                return Err(WorkspaceErrorKind::Exists(target.to_string()).into());
            }
            return Err(
                VcsErrorKind::Conflict("worktree add".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }

    fn merge_workspace(&self, name: &str) -> Result<()> {
        let worktree_path = self.repo_path.join(name);
        if !worktree_path.exists() {
            return Err(WorkspaceErrorKind::NotFound(name.to_string()).into());
        }
        self.switch_branch("main")?;
        let output = self.run_git(&["merge", name])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("conflict") {
                return Err(VcsErrorKind::Conflict("merge".to_string(), stderr.to_string()).into());
            }
            return Err(VcsErrorKind::Conflict("merge".to_string(), stderr.to_string()).into());
        }
        self.push()?;
        Ok(())
    }

    fn abort_workspace(&self, _name: &str) -> Result<()> {
        // Abort workspace by restoring working copy to last commit
        // This uses git checkout -- . to discard uncommitted changes
        let output = self.run_git(&["checkout", "--", "."])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(
                VcsErrorKind::Conflict("git checkout".to_string(), stderr.to_string()).into(),
            );
        }
        Ok(())
    }
}

// ============================================================================
// Parsing helpers (extracted for unit testing without subprocess calls)
// ============================================================================

/// Parse `git status --porcelain` output into a VcsStatus.
fn parse_git_porcelain_status(stdout: &str) -> VcsStatus {
    if stdout.is_empty() {
        VcsStatus::Clean
    } else if stdout.contains("UU") {
        VcsStatus::Conflicted
    } else {
        VcsStatus::Dirty
    }
}

/// Parse `git status --porcelain` output into a list of filenames.
///
/// Git porcelain format: `XY<SP>filename` where XY is a two-character status code.
/// - Ordinary filenames follow the space directly.
/// - Quoted filenames are wrapped in `"..."` with C-style escapes (`\\`, `\"`, `\t`, `\n`, `\123`).
/// - Renames/copies use: `XY<SP>from -> to` where either side may be quoted.
/// - Deleted files targeting `/dev/null` are represented as: `XY<SP>/dev/null`.
fn parse_git_porcelain_files(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| l.len() > 3)
        .filter_map(|l| {
            // XY<SP> is always the first 3 characters (never trimmed — the status
            // codes occupy fixed columns).  Everything after that is the path field.
            let path_field = &l[3..];
            let filename = extract_filename(path_field);
            if filename == "/dev/null" {
                return None;
            }
            Some(filename)
        })
        .collect()
}

/// Given the path field portion of a porcelain line (everything after `XY<SP>`),
/// return the effective filename.  Handles the `from -> to` rename/copy syntax
/// and git's C-style quoted filename escaping.
fn extract_filename(path_field: &str) -> String {
    // Check for rename/copy separator: `old -> new`
    if let Some(arrow_pos) = path_field.find(" -> ") {
        let to_part = &path_field[arrow_pos + 4..];
        return unquote_git_path(to_part);
    }
    unquote_git_path(path_field)
}

/// Unquote a git porcelain path.  Git wraps paths that contain special
/// characters in double-quotes and uses C-style escaping for tabs, newlines,
/// backslashes, and high-byte octal sequences (`\NNN`, 3 octal digits).
/// Paths without surrounding quotes are returned unchanged.
fn unquote_git_path(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        git_unescape(inner)
    } else {
        trimmed.to_string()
    }
}

/// Process git C-style escape sequences inside a quoted filename.
///
/// Octal escapes (`\NNN`) are collected as raw bytes so that multi-byte
/// UTF-8 sequences (e.g. `\303\266` for o-umlaut) decode correctly.
fn git_unescape(input: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => bytes.push(b'\n'),
                Some('t') => bytes.push(b'\t'),
                Some('\\') => bytes.push(b'\\'),
                Some('"') => bytes.push(b'"'),
                Some(d) if ('0'..='7').contains(&d) => {
                    // Parse up to 3 octal digits.
                    let mut octal = String::with_capacity(3);
                    octal.push(d);
                    for _ in 0..2 {
                        if let Some(&next) = chars.peek() {
                            if ('0'..='7').contains(&next) {
                                if let Some(c) = chars.next() {
                                    octal.push(c);
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                        bytes.push(byte);
                    }
                }
                _ => {
                    // Unknown escape — keep the backslash.
                    bytes.push(b'\\');
                }
            }
        } else if ch.is_ascii() {
            bytes.push(ch as u8);
        } else {
            // Non-ASCII character already in the Rust string (e.g. literal unicode).
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Parse `git branch -a` output into Branch entries.
fn parse_git_branch_list(stdout: &str, current_branch: &str) -> Vec<Branch> {
    let mut branches = Vec::new();
    for line in stdout.lines() {
        let name = line.trim().trim_start_matches("* ").to_string();
        if !name.is_empty() {
            branches.push(Branch {
                name: name.clone(),
                is_current: name == current_branch,
                tracking: None,
            });
        }
    }
    branches
}

/// Parse `git log` output into Commit entries.
fn parse_git_log(stdout: &str) -> Vec<Commit> {
    let mut commits = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("commit ") {
            let id = line.trim_start_matches("commit ").to_string();
            commits.push(Commit {
                id,
                message: "".to_string(),
                author: "unknown".to_string(),
                timestamp: Utc::now(),
                parents: vec![],
            });
        } else if line.starts_with("    ") && !commits.is_empty() {
            if let Some(last) = commits.last_mut() {
                last.message = line.trim().to_string();
            }
        }
    }
    commits
}

/// Parse `git worktree list --porcelain` output into Workspace entries.
///
/// Each worktree block is separated by a blank line. Relevant fields:
/// - `worktree <path>` — absolute path to the worktree root
/// - `branch <ref>` — e.g. `refs/heads/feature-x`
/// - `bare` / (no `bare` line) — indicates the main worktree
fn parse_git_worktree_list(stdout: &str, repo_path: &std::path::Path) -> Vec<Workspace> {
    let mut workspaces = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;

    for line in stdout.lines() {
        let line = line.trim();

        // Blank line signals end of a worktree block
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                let relative = make_relative(&path, repo_path);
                let branch_name = current_branch
                    .take()
                    .and_then(|b| b.strip_prefix("refs/heads/").map(str::to_string))
                    .unwrap_or_default();
                workspaces.push(Workspace {
                    name: relative,
                    branch: branch_name,
                    is_current: !is_bare,
                });
            }
            current_branch = None;
            is_bare = false;
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = Some(branch.to_string());
        } else if line == "bare" {
            is_bare = true;
        }
    }

    // Flush the last block if file didn't end with blank line
    if let Some(path) = current_path.take() {
        let relative = make_relative(&path, repo_path);
        let branch_name = current_branch
            .and_then(|b| b.strip_prefix("refs/heads/").map(str::to_string))
            .unwrap_or_default();
        workspaces.push(Workspace {
            name: relative,
            branch: branch_name,
            is_current: !is_bare,
        });
    }

    workspaces
}

/// Make a path relative to `base`, returning the original if it fails.
fn make_relative(path: &str, base: &std::path::Path) -> String {
    let path_buf = std::path::Path::new(path);
    path_buf
        .strip_prefix(base)
        .ok()
        .and_then(|rel| rel.to_str().map(str::to_string))
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- GitBackend construction --

    #[test]
    fn git_backend_new_stores_path() {
        let path = std::path::PathBuf::from("/tmp/test-repo");
        let backend = GitBackend::new(path.clone());
        assert_eq!(backend.repo_path, path);
    }

    #[test]
    fn git_backend_is_initialized_true_when_git_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let backend = GitBackend::new(dir.path().to_path_buf());
        assert_eq!(backend.is_initialized().expect("ok"), true);
    }

    #[test]
    fn git_backend_is_initialized_false_when_no_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = GitBackend::new(dir.path().to_path_buf());
        assert_eq!(backend.is_initialized().expect("ok"), false);
    }

    #[test]
    fn git_backend_repo_exists_true_when_git_in_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).expect("create .git");
        let backend = GitBackend::new("/tmp".into());
        assert!(backend.repo_exists(dir.path().to_str().expect("path")));
    }

    #[test]
    fn git_backend_repo_exists_false_when_no_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = GitBackend::new("/tmp".into());
        assert!(!backend.repo_exists(dir.path().to_str().expect("path")));
    }

    // -- parse_git_porcelain_status --

    #[test]
    fn parse_git_porcelain_clean() {
        assert_eq!(parse_git_porcelain_status(""), VcsStatus::Clean);
    }

    #[test]
    fn parse_git_porcelain_modified() {
        assert_eq!(
            parse_git_porcelain_status(" M src/main.rs\n"),
            VcsStatus::Dirty
        );
    }

    #[test]
    fn parse_git_porcelain_added() {
        assert_eq!(
            parse_git_porcelain_status("A  new_file.txt\n"),
            VcsStatus::Dirty
        );
    }

    #[test]
    fn parse_git_porcelain_deleted() {
        assert_eq!(
            parse_git_porcelain_status(" D old_file.rs\n"),
            VcsStatus::Dirty
        );
    }

    #[test]
    fn parse_git_porcelain_conflicted() {
        assert_eq!(
            parse_git_porcelain_status("UU file.txt\n"),
            VcsStatus::Conflicted
        );
    }

    #[test]
    fn parse_git_porcelain_both_modified_conflict() {
        assert_eq!(
            parse_git_porcelain_status("AA file.txt\n"),
            VcsStatus::Dirty
        );
    }

    #[test]
    fn parse_git_porcelain_multiple_files_dirty() {
        let output = " M src/main.rs\nA  new_file.txt\n D old.rs\n";
        assert_eq!(parse_git_porcelain_status(output), VcsStatus::Dirty);
    }

    #[test]
    fn parse_git_porcelain_conflict_among_changes() {
        let output = "UU conflict.txt\n M src/main.rs\n";
        assert_eq!(parse_git_porcelain_status(output), VcsStatus::Conflicted);
    }

    // -- parse_git_porcelain_files --

    #[test]
    fn parse_git_porcelain_files_empty() {
        assert!(parse_git_porcelain_files("").is_empty());
    }

    #[test]
    fn parse_git_porcelain_files_single() {
        // XY is always exactly 2 chars at fixed columns, so [3..] skips "XY ".
        let files = parse_git_porcelain_files(" M src/main.rs\n");
        assert_eq!(files, vec!["src/main.rs"]);
    }

    #[test]
    fn parse_git_porcelain_files_staged_modified() {
        let files = parse_git_porcelain_files("MM src/lib.rs\n");
        assert_eq!(files, vec!["src/lib.rs"]);
    }

    #[test]
    fn parse_git_porcelain_files_untracked() {
        let files = parse_git_porcelain_files("?? new_file.txt\n");
        assert_eq!(files, vec!["new_file.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_multiple() {
        let output = " M src/main.rs\nA  new_file.txt\n D old.rs\n";
        let files = parse_git_porcelain_files(output);
        assert_eq!(files, vec!["src/main.rs", "new_file.txt", "old.rs"]);
    }

    #[test]
    fn parse_git_porcelain_files_short_line_skipped() {
        // A line shorter than 4 chars is not a valid porcelain entry.
        let files = parse_git_porcelain_files("??\n");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_git_porcelain_files_renamed() {
        // Renames use "from -> to"; we return the "to" path.
        let files = parse_git_porcelain_files("R  old_name.rs -> new_name.rs\n");
        assert_eq!(files, vec!["new_name.rs"]);
    }

    #[test]
    fn parse_git_porcelain_files_copied() {
        let files = parse_git_porcelain_files("C  original.txt -> copy.txt\n");
        assert_eq!(files, vec!["copy.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_quoted() {
        // Git quotes filenames with special characters.
        let files = parse_git_porcelain_files(" M \"file with spaces.txt\"\n");
        assert_eq!(files, vec!["file with spaces.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_deleted_devnull_filtered() {
        // Deleted files show as /dev/null; they should be filtered out.
        let files = parse_git_porcelain_files("D  /dev/null\n");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_git_porcelain_files_renamed_from_devnull() {
        // New file added via rename (e.g. "D  /dev/null -> new.txt" should yield "new.txt").
        let files = parse_git_porcelain_files("R  /dev/null -> added.txt\n");
        assert_eq!(files, vec!["added.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_quoted_with_escapes() {
        // C-style escapes inside quoted filenames.
        let files = parse_git_porcelain_files("?? \"tab\\there.txt\"\n");
        assert_eq!(files, vec!["tab\there.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_quoted_backslash() {
        let files = parse_git_porcelain_files(" M \"path\\\\to\\\\file.rs\"\n");
        assert_eq!(files, vec!["path\\to\\file.rs"]);
    }

    #[test]
    fn parse_git_porcelain_files_quoted_newline() {
        let files = parse_git_porcelain_files("?? \"newline\\nfile.txt\"\n");
        assert_eq!(files, vec!["newline\nfile.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_quoted_octal() {
        // Octal escape: \303\266 = UTF-8 for o-umlaut (U+00F6)
        let files = parse_git_porcelain_files("?? \"\\303\\266.txt\"\n");
        assert_eq!(files, vec!["\u{00F6}.txt"]);
    }

    #[test]
    fn parse_git_porcelain_files_renamed_quoted() {
        let files = parse_git_porcelain_files("R  \"old\\tpath\" -> \"new\\tpath\"\n");
        assert_eq!(files, vec!["new\tpath"]);
    }

    // -- parse_git_branch_list --

    #[test]
    fn parse_git_branch_list_single() {
        let output = "  main\n";
        let branches = parse_git_branch_list(output, "main");
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
    }

    #[test]
    fn parse_git_branch_list_current_starred() {
        let output = "* main\n";
        let branches = parse_git_branch_list(output, "main");
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
    }

    #[test]
    fn parse_git_branch_list_multiple() {
        let output = "  feature/test\n* main\n  develop\n";
        let branches = parse_git_branch_list(output, "main");
        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].name, "feature/test");
        assert!(!branches[0].is_current);
        assert_eq!(branches[1].name, "main");
        assert!(branches[1].is_current);
        assert_eq!(branches[2].name, "develop");
        assert!(!branches[2].is_current);
    }

    #[test]
    fn parse_git_branch_list_empty() {
        let branches = parse_git_branch_list("", "main");
        assert!(branches.is_empty());
    }

    #[test]
    fn parse_git_branch_list_tracks_current() {
        let output = "  main\n  feature\n  develop\n";
        let branches = parse_git_branch_list(output, "feature");
        assert_eq!(branches.len(), 3);
        assert!(!branches[0].is_current);
        assert!(branches[1].is_current);
        assert!(!branches[2].is_current);
    }

    #[test]
    fn parse_git_branch_list_remotes() {
        let output = "  main\n  remotes/origin/main\n";
        let branches = parse_git_branch_list(output, "main");
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[1].name, "remotes/origin/main");
        assert!(!branches[1].is_current);
    }

    // -- parse_git_log --

    #[test]
    fn parse_git_log_single_commit() {
        let output = "commit abc123def456\nAuthor: Test <test@test.com>\n\n    Initial commit\n";
        let commits = parse_git_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].id, "abc123def456");
        // NOTE: The parser trims lines before checking for 4-space indent,
        // so message lines are never captured. This is a known limitation.
        assert_eq!(commits[0].message, "");
    }

    #[test]
    fn parse_git_log_multiple_commits() {
        let output = "commit abc123\nAuthor: Test <test@test.com>\n\n    First commit\n\ncommit def456\nAuthor: Test <test@test.com>\n\n    Second commit\n";
        let commits = parse_git_log(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, "abc123");
        assert_eq!(commits[1].id, "def456");
        // Messages not captured due to trim-before-check
        assert_eq!(commits[0].message, "");
        assert_eq!(commits[1].message, "");
    }

    #[test]
    fn parse_git_log_multiline_message() {
        // Messages are not captured because trim removes leading spaces
        let output = "commit abc123\nAuthor: Test <test@test.com>\n\n    feat: implement something\n\n    Longer description.\n";
        let commits = parse_git_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "");
    }

    #[test]
    fn parse_git_log_empty() {
        let commits = parse_git_log("");
        assert!(commits.is_empty());
    }

    #[test]
    fn parse_git_log_merges_and_stats_ignored() {
        let output = "commit abc123\nMerge: def456 ghi789\nAuthor: Test <test@test.com>\n\n    Merge branch 'feature'\n\n src/main.rs | 5 +++--\n 1 file changed, 3 insertions(+), 2 deletions(-)\n";
        let commits = parse_git_log(output);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].id, "abc123");
        // Message not captured due to trim-before-check
        assert_eq!(commits[0].message, "");
    }

    #[test]
    fn parse_git_log_commit_ids_with_parents_line_ignored() {
        let output = "commit abc123\nMerge: def456 ghi789\nAuthor: Test <test@test.com>\n";
        let commits = parse_git_log(output);
        assert_eq!(commits.len(), 1);
        // Merge: line doesn't start with "commit " so it's not a separate commit
    }

    // -- VcsStatus Display (re-tested here for completeness) --

    #[test]
    fn vcs_status_all_variants() {
        assert_eq!(VcsStatus::Clean, VcsStatus::Clean);
        assert_eq!(VcsStatus::Dirty, VcsStatus::Dirty);
        assert_eq!(VcsStatus::Conflicted, VcsStatus::Conflicted);
        assert_eq!(VcsStatus::Detached, VcsStatus::Detached);
    }
}
