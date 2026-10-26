//! Gitoxide Tag Operations

use crate::error::{GitError, GitResult};

const TAG_REF_PREFIX: &str = "refs/tags/";

fn validate_tag_name(name: &str) -> GitResult<()> {
    if name.is_empty() {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name cannot be empty".to_string(),
        });
    }
    if name.starts_with("ref:") {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name cannot start with 'ref:'".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return Err(GitError::InvalidRef {
            name: "create".to_string(),
            reason: "Tag name contains invalid characters".to_string(),
        });
    }
    Ok(())
}

pub fn list(repo: &gix::Repository, pattern: Option<&str>) -> GitResult<Vec<String>> {
    let refs = repo.references().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    let tags_iter = refs.tags().map_err(|e| GitError::InvalidRef {
        name: "list".to_string(),
        reason: e.to_string(),
    })?;

    let tags: GitResult<Vec<String>> = tags_iter
        .map(|tag_result| {
            tag_result
                .map_err(|e| GitError::InvalidRef {
                    name: "list".to_string(),
                    reason: e.to_string(),
                })
                .map(|reference| reference.name().shorten().to_string())
        })
        .collect();

    Ok(filter_tags(tags?, pattern))
}

/// Keep only tags matching `pattern` (glob semantics, see [`glob_match`]);
/// `None` keeps every tag. Pure function, extracted for testability.
fn filter_tags(tags: Vec<String>, pattern: Option<&str>) -> Vec<String> {
    match pattern {
        None => tags,
        Some(pat) => tags
            .into_iter()
            .filter(|tag| glob_match(pat, tag))
            .collect(),
    }
}

/// One glob pattern token: `*` and `?` are handled as token kinds so a
/// character class (`[...]`) also consumes exactly one input character.
#[derive(Debug, Clone, PartialEq, Eq)]
enum GlobToken {
    Star,
    AnySingle,
    Literal(char),
    Class { negated: bool, ranges: Vec<(char, char)> },
}

/// Parse a glob pattern into tokens. A `[` without a closing `]` is a
/// literal `[`; a `\` escapes the following character. `*` matches any
/// sequence of characters (including `/`, as in `git tag -l` fnmatch
/// without `FNM_PATHNAME`), `?` matches exactly one character, and
/// `[...]` matches one character in the set, with `a-z` ranges and
/// `!`/`^` negation.
fn tokenize_glob(pattern: &str) -> Vec<GlobToken> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                tokens.push(GlobToken::Star);
                i += 1;
            }
            '?' => {
                tokens.push(GlobToken::AnySingle);
                i += 1;
            }
            '\\' => {
                if i + 1 < chars.len() {
                    tokens.push(GlobToken::Literal(chars[i + 1]));
                    i += 2;
                } else {
                    tokens.push(GlobToken::Literal('\\'));
                    i += 1;
                }
            }
            '[' => {
                let mut j = i + 1;
                let mut negated = false;
                if j < chars.len() && (chars[j] == '!' || chars[j] == '^') {
                    negated = true;
                    j += 1;
                }
                // A ']' immediately after '[' (or '[!') is a literal member.
                let mut ranges: Vec<(char, char)> = Vec::new();
                let mut first = true;
                let mut closed = false;
                while j < chars.len() {
                    if chars[j] == ']' && !first {
                        closed = true;
                        break;
                    }
                    first = false;
                    if j + 2 < chars.len() && chars[j + 1] == '-' && chars[j + 2] != ']' {
                        ranges.push((chars[j], chars[j + 2]));
                        j += 3;
                    } else {
                        ranges.push((chars[j], chars[j]));
                        j += 1;
                    }
                }
                if closed {
                    tokens.push(GlobToken::Class { negated, ranges });
                    i = j + 1;
                } else {
                    tokens.push(GlobToken::Literal('['));
                    i += 1;
                }
            }
            c => {
                tokens.push(GlobToken::Literal(c));
                i += 1;
            }
        }
    }
    tokens
}

fn class_matches(class: &GlobToken, c: char) -> bool {
    match class {
        GlobToken::Class { negated, ranges } => {
            let inside = ranges.iter().any(|(lo, hi)| *lo <= c && c <= *hi);
            inside != *negated
        }
        _ => false,
    }
}

/// Glob-match `text` against `pattern`: `*` any sequence, `?` one
/// character, `[...]` one character from the class. No other glob
/// features are implied; an unmatched `*` at the end matches the rest.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let tokens = tokenize_glob(pattern);
    let chars: Vec<char> = text.chars().collect();
    // Backtracking matcher: on `*`, remember the retry point and advance
    // the text; on mismatch, rewind to the last star and consume one more.
    let mut ti = 0usize;
    let mut ci = 0usize;
    let mut star_token: Option<usize> = None;
    let mut star_char = 0usize;
    while ci < chars.len() {
        if ti < tokens.len() {
            match &tokens[ti] {
                GlobToken::Star => {
                    star_token = Some(ti);
                    star_char = ci;
                    ti += 1;
                    continue;
                }
                GlobToken::AnySingle => {
                    ti += 1;
                    ci += 1;
                    continue;
                }
                GlobToken::Literal(expected) => {
                    if chars[ci] == *expected {
                        ti += 1;
                        ci += 1;
                        continue;
                    }
                }
                class @ GlobToken::Class { .. } => {
                    if class_matches(class, chars[ci]) {
                        ti += 1;
                        ci += 1;
                        continue;
                    }
                }
            }
        }
        // Mismatch: retry from the last star, consuming one more character.
        match star_token {
            Some(star) => {
                ti = star + 1;
                star_char += 1;
                ci = star_char;
            }
            None => return false,
        }
    }
    // Remaining tokens must all be stars to match the exhausted text.
    while ti < tokens.len() {
        if tokens[ti] != GlobToken::Star {
            return false;
        }
        ti += 1;
    }
    true
}

pub fn create(repo: &gix::Repository, name: &str, _message: &str, force: bool) -> GitResult<()> {
    validate_tag_name(name)?;

    let oid = repo.head_id().map_err(|e| GitError::InvalidRef {
        name: "HEAD".to_string(),
        reason: e.to_string(),
    })?;

    let reference_name = format!("{}{}", TAG_REF_PREFIX, name);

    if !force && repo.find_reference(&reference_name).is_ok() {
        return Err(GitError::Conflict {
            message: format!("Tag '{}' already exists", name),
            conflicted_files: vec![],
        });
    }

    let constraint = if force {
        gix::refs::transaction::PreviousValue::Any
    } else {
        gix::refs::transaction::PreviousValue::MustNotExist
    };

    repo.reference(
        reference_name,
        oid,
        constraint,
        format!("create tag {}", name),
    )
    .map_err(|e| GitError::InvalidRef {
        name: name.to_string(),
        reason: e.to_string(),
    })?;

    Ok(())
}

pub fn delete(repo: &gix::Repository, name: &str, force: bool) -> GitResult<()> {
    let reference_name = format!("{}{}", TAG_REF_PREFIX, name);

    let reference = if force {
        repo.find_reference(&reference_name).ok()
    } else {
        Some(
            repo.find_reference(&reference_name)
                .map_err(|_| GitError::NotFound(std::path::PathBuf::from(&reference_name)))?,
        )
    };

    if let Some(ref reference) = reference {
        reference.delete().map_err(|e| GitError::InvalidRef {
            name: name.to_string(),
            reason: e.to_string(),
        })?;
    }

    Ok(())
}

pub fn push(repo: &gix::Repository, remote: &str, tag: &str) -> GitResult<()> {
    let workdir = crate::gix::cli::require_workdir(repo, "tag push")?;

    let output = crate::gix::cli::run_git(workdir, &["push", remote, tag])?;

    if !output.success {
        return Err(crate::gix::cli::cli_error(&output, "tag push"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_star_prefix() {
        assert!(glob_match("v0*", "v0.1"));
        assert!(glob_match("v0*", "v0.2"));
        assert!(!glob_match("v0*", "v1.0"));
    }

    #[test]
    fn glob_star_matches_empty_and_slashes() {
        assert!(glob_match("v*", "v"));
        assert!(glob_match("release/*", "release/x/y"));
    }

    #[test]
    fn glob_question_mark_single_char() {
        assert!(glob_match("v?.1", "v0.1"));
        assert!(!glob_match("v?.1", "v10.1"));
        assert!(!glob_match("v?.1", "v.1"));
    }

    #[test]
    fn glob_star_middle() {
        assert!(glob_match("v*1", "v0.1"));
        assert!(glob_match("*", "anything"));
        assert!(!glob_match("v*1", "v0.2"));
    }

    #[test]
    fn glob_exact_no_wildcards() {
        assert!(glob_match("v1.0", "v1.0"));
        assert!(!glob_match("v1.0", "v1.1"));
    }

    #[test]
    fn glob_character_class_and_range() {
        assert!(glob_match("v[0-9].1", "v0.1"));
        assert!(!glob_match("v[0-9].1", "vx.1"));
        assert!(glob_match("v[!0].1", "v1.1"));
        assert!(!glob_match("v[!0].1", "v0.1"));
    }

    #[test]
    fn glob_unclosed_bracket_is_literal() {
        assert!(glob_match("v[0", "v[0"));
        assert!(!glob_match("v[0", "v0"));
    }

    #[test]
    fn filter_tags_none_keeps_all() {
        let tags = vec!["v0.1".to_string(), "v1.0".to_string()];
        assert_eq!(filter_tags(tags.clone(), None), tags);
    }

    #[test]
    fn filter_tags_pattern_keeps_only_matches() {
        let tags = vec![
            "v0.1".to_string(),
            "v0.2".to_string(),
            "v1.0".to_string(),
        ];
        assert_eq!(
            filter_tags(tags, Some("v0*")),
            vec!["v0.1".to_string(), "v0.2".to_string()]
        );
    }

    #[test]
    fn filter_tags_no_match_is_empty() {
        let tags = vec!["v1.0".to_string()];
        assert!(filter_tags(tags, Some("v9*")).is_empty());
    }
}
