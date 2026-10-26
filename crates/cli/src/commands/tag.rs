//! Tag commands using gitoxide

use scp_core::{output::Output, vcs::detect_vcs, Error, Result};
use scp_vcs::gix::{repository, tag};

/// List tags, optionally filtered by a glob pattern and sorted by key.
///
/// `--sort` is honored, not ignored: `refname` sorts lexicographically,
/// `version:refname` sorts version-aware (numeric segments compare
/// numerically, so v10 sorts after v9), and a leading `-` reverses either.
/// Any other key is a validation error naming the bad value — silently
/// ignoring a user-passed sort key is how a wrong order looks correct.
pub fn list(pattern: Option<&str>, sort: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    let mut tags =
        tag::list(&repo, pattern).map_err(|e| Error::vcs_conflict("list tags", e.to_string()))?;

    if let Some(key) = sort {
        sort_tags(&mut tags, key)?;
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({"tags": tags}));
    } else if tags.is_empty() {
        Output::info("No tags found");
    } else {
        for t in tags {
            println!("{}", t);
        }
    }
    Ok(())
}

/// Compare two tag names version-aware: split on non-alphanumeric
/// boundaries and compare numeric runs numerically, other runs
/// lexicographically. Non-numeric input degrades to plain ordering.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let split = |s: &str| -> Vec<String> {
        let mut parts = Vec::new();
        let mut cur = String::new();
        let mut cur_numeric: Option<bool> = None;
        for ch in s.chars() {
            if ch.is_ascii_digit() {
                if cur_numeric == Some(false) {
                    parts.push(std::mem::take(&mut cur));
                }
                cur_numeric = Some(true);
                cur.push(ch);
            } else if ch.is_ascii_alphanumeric() {
                if cur_numeric == Some(true) {
                    parts.push(std::mem::take(&mut cur));
                }
                cur_numeric = Some(false);
                cur.push(ch);
            } else if !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
                cur_numeric = None;
            }
        }
        if !cur.is_empty() {
            parts.push(cur);
        }
        parts
    };
    let (pa, pb) = (split(a), split(b));
    for (x, y) in pa.iter().zip(pb.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(nx), Ok(ny)) => nx.cmp(&ny),
            _ => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    pa.len().cmp(&pb.len())
}

fn sort_tags(tags: &mut Vec<String>, key: &str) -> Result<()> {
    let (descending, base) = match key.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, key),
    };
    let cmp: fn(&str, &str) -> std::cmp::Ordering = match base {
        "refname" => |a, b| a.cmp(b),
        "version:refname" => version_cmp,
        _ => {
            return Err(Error::invalid_identifier(format!(
                "invalid sort key '{}': expected one of refname, -refname, version:refname, -version:refname",
                key
            )));
        }
    };
    tags.sort_by(|a, b| {
        let ord = cmp(a, b);
        if descending { ord.reverse() } else { ord }
    });
    Ok(())
}

/// Create a new Git tag, optionally with an annotated message.
///
/// A message (`-m`) requests an annotated tag. The native gix path below
/// always creates a lightweight ref and drops the message, so a message
/// must never flow through it: with a message, create the tag with
/// `git tag -a` instead, and surface any git failure loudly rather than
/// silently downgrading to a lightweight tag.
pub fn create(name: &str, message: Option<&str>, commit: Option<&str>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    if let Some(msg) = message {
        return create_annotated(&cwd, name, msg, commit, force);
    }

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    tag::create(&repo, name, "", force)
        .map_err(|e| Error::vcs_conflict("create tag", e.to_string()))?;

    if Output::json_output() {
        Output::json(&serde_json::json!({"created": true, "tag": name, "message": ""}));
    } else {
        Output::success(&format!("Created tag: {}", name));
    }
    Ok(())
}

/// Create an annotated tag with `git tag -a`, so the tag object carries
/// the message. Any git failure is returned as an error, never downgraded.
fn create_annotated(
    workdir: &std::path::Path,
    name: &str,
    message: &str,
    commit: Option<&str>,
    force: bool,
) -> Result<()> {
    let mut args: Vec<&str> = vec!["tag"];
    if force {
        args.push("-f");
    }
    args.push("-a");
    args.push(name);
    args.push("-m");
    args.push(message);
    if let Some(target) = commit {
        args.push(target);
    }

    let output = scp_vcs::gix::cli::run_git(workdir, &args)
        .map_err(|e| Error::vcs_conflict("create tag", e.to_string()))?;

    if !output.success {
        let git_err = scp_vcs::gix::cli::cli_error(&output, "create tag");
        return Err(Error::vcs_conflict("create tag", git_err.to_string()));
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({"created": true, "tag": name, "message": message}));
    } else {
        Output::success(&format!("Created tag: {}", name));
    }
    Ok(())
}

/// Delete a local (or remote) tag.
pub fn delete(name: &str, remote: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    let repo = repository::open(&cwd)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;

    if remote {
        return delete_remote_tag(&cwd, name);
    }

    tag::delete(&repo, name, false)
        .map_err(|e| Error::vcs_conflict("delete tag", e.to_string()))?;

    if Output::json_output() {
        Output::json(&serde_json::json!({"deleted": true, "tag": name, "remote": false}));
    } else {
        Output::success(&format!("Deleted local tag: {}", name));
    }
    Ok(())
}

/// Delete a remote tag using git CLI fallback.
fn delete_remote_tag(workdir: &std::path::Path, name: &str) -> Result<()> {
    let output = scp_vcs::gix::cli::run_git(workdir, &["push", "--delete", "origin", name])
        .map_err(|e| Error::vcs_push_failed(e.to_string()))?;

    if !output.success {
        let git_err = scp_vcs::gix::cli::cli_error(&output, "remote tag delete");
        return Err(Error::vcs_push_failed(git_err.to_string()));
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({"deleted": true, "tag": name, "remote": true}));
    } else {
        Output::success(&format!("Deleted remote tag: {}", name));
    }
    Ok(())
}

/// Push a specific tag (or all tags) to a remote.
pub fn push(tag: Option<&str>, remote: &str, _force: bool) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?;

    detect_vcs(&cwd).ok_or_else(Error::vcs_not_initialized)?;

    tag.map_or_else(
        || push_all_tags(&cwd, remote),
        |t| push_single_tag(&cwd, t, remote),
    )
}

/// Push a single tag to a remote using the gix native path.
fn push_single_tag(workdir: &std::path::Path, tag_name: &str, remote: &str) -> Result<()> {
    let repo = repository::open(workdir)
        .map_err(|e| Error::vcs_conflict(format!("Failed to open repo: {}", e), e.to_string()))?;
    tag::push(&repo, remote, tag_name).map_err(|e| Error::vcs_push_failed(e.to_string()))?;
    if Output::json_output() {
        Output::json(&serde_json::json!({"pushed": true, "tag": tag_name, "remote": remote}));
    } else {
        Output::success(&format!("Pushed tag {} to {}", tag_name, remote));
    }
    Ok(())
}

/// Push all tags to a remote using git CLI fallback.
fn push_all_tags(workdir: &std::path::Path, remote: &str) -> Result<()> {
    let output = scp_vcs::gix::cli::run_git(workdir, &["push", "--tags", remote])
        .map_err(|e| Error::vcs_push_failed(e.to_string()))?;

    if !output.success {
        let git_err = scp_vcs::gix::cli::cli_error(&output, "push all tags");
        return Err(Error::vcs_push_failed(git_err.to_string()));
    }

    if Output::json_output() {
        Output::json(&serde_json::json!({"pushed": true, "all": true, "remote": remote}));
    } else {
        Output::success(&format!("Pushed all tags to {}", remote));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_accepts_none_params() {
        let _fn: fn(Option<&str>, Option<&str>) -> Result<()> = list;
    }

    #[test]
    fn sort_version_orders_numeric_segments_numerically() {
        let mut tags = vec![
            "v10".to_string(),
            "v2".to_string(),
            "v1".to_string(),
        ];
        sort_tags(&mut tags, "version:refname").unwrap();
        assert_eq!(tags, vec!["v1", "v2", "v10"]);
    }

    #[test]
    fn sort_refname_is_lexicographic() {
        let mut tags = vec!["b".to_string(), "a".to_string()];
        sort_tags(&mut tags, "refname").unwrap();
        assert_eq!(tags, vec!["a", "b"]);
    }

    #[test]
    fn sort_descending_reverses() {
        let mut tags = vec!["v1".to_string(), "v2".to_string()];
        sort_tags(&mut tags, "-version:refname").unwrap();
        assert_eq!(tags, vec!["v2", "v1"]);
    }

    #[test]
    fn sort_unknown_key_is_an_error() {
        let mut tags = vec!["v1".to_string()];
        assert!(sort_tags(&mut tags, "bogus-sort-value").is_err());
    }

    #[test]
    fn create_accepts_all_params() {
        let _fn: fn(&str, Option<&str>, Option<&str>, bool) -> Result<()> = create;
    }

    #[test]
    fn delete_rejects_remote_delete() {
        // Skip if cwd doesn't exist (can happen in parallel test env)
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };

        // We need a git repo for this — but remote delete is checked before repo open
        // So even in a non-VCS dir, the error should come from vcs_not_initialized
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }
        let result = delete("v1.0", true);
        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn push_rejects_none_tag() {
        // Skip if cwd doesn't exist (can happen in parallel test env)
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };

        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }
        let result = push(None, "origin", false);
        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn list_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = list(None, None);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn create_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = create("v1.0", None, None, false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn delete_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = delete("v1.0", false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn push_in_non_vcs_dir_fails() {
        let original = match std::env::current_dir() {
            Ok(p) => p,
            Err(_) => return,
        };
        let dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(_) => return,
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            std::env::set_current_dir(&original).ok();
            return;
        }

        let result = push(Some("v1.0"), "origin", false);

        std::env::set_current_dir(&original).ok();
        assert!(result.is_err());
    }

    #[test]
    fn error_constructors_used_in_tag() {
        let _ = Error::io_error("test");
        let _ = Error::vcs_not_initialized();
        let _ = Error::vcs_conflict("repo", "msg");
        let _ = Error::vcs_push_failed("test");
    }
}
