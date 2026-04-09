//! PR template discovery ported from stax.
//!
//! Pure filesystem operations to discover GitHub PR templates
//! in standard locations within a repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, StackError};

/// A discovered PR template.
#[derive(Debug, Clone)]
pub struct PrTemplate {
    /// Display name (e.g., "feature", "bugfix", "Default").
    pub name: String,
    /// Full file path.
    pub path: PathBuf,
    /// Template content.
    pub content: String,
}

/// Discover all PR templates in standard GitHub locations.
///
/// Priority order:
/// 1. `.github/PULL_REQUEST_TEMPLATE/` directory — scan for all `.md` files
/// 2. `.github/PULL_REQUEST_TEMPLATE.md` — single template (named "Default")
/// 3. `.github/pull_request_template.md` — lowercase variant
/// 4. `PULL_REQUEST_TEMPLATE.md` at repository root
/// 5. `pull_request_template.md` at repository root
/// 6. `docs/PULL_REQUEST_TEMPLATE.md`
/// 7. `docs/pull_request_template.md`
pub fn discover_pr_templates(workdir: &Path) -> Result<Vec<PrTemplate>> {
    let mut templates = Vec::new();

    // Check directory first (multiple templates)
    let template_dir = workdir.join(".github/PULL_REQUEST_TEMPLATE");
    if template_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&template_dir)
            .map_err(|e| {
                StackError::GitHubError(format!("Failed to read PR template directory: {e}"))
            })?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("template")
                .to_string();

            let content = fs::read_to_string(&path).map_err(|e| {
                StackError::GitHubError(format!(
                    "Failed to read PR template {}: {e}",
                    path.display()
                ))
            })?;

            templates.push(PrTemplate {
                name,
                path,
                content,
            });
        }

        if !templates.is_empty() {
            return Ok(templates);
        }
    }

    // Check single template locations
    let single_template_candidates = [
        ".github/PULL_REQUEST_TEMPLATE.md",
        ".github/pull_request_template.md",
        "PULL_REQUEST_TEMPLATE.md",
        "pull_request_template.md",
        "docs/PULL_REQUEST_TEMPLATE.md",
        "docs/pull_request_template.md",
    ];

    for candidate in &single_template_candidates {
        let path = workdir.join(candidate);
        if path.is_file() {
            let content = fs::read_to_string(&path).map_err(|e| {
                StackError::GitHubError(format!(
                    "Failed to read PR template {}: {e}",
                    path.display()
                ))
            })?;
            templates.push(PrTemplate {
                name: "Default".to_string(),
                path,
                content,
            });
            return Ok(templates);
        }
    }

    Ok(templates)
}

/// Build selection options list: `["No template", ...template names sorted]`.
pub fn build_template_options(templates: &[PrTemplate]) -> Vec<String> {
    let mut options = vec!["No template".to_string()];
    let mut names: Vec<_> = templates.iter().map(|t| t.name.clone()).collect();
    names.sort();
    options.extend(names);
    options
}

/// For single templates, return automatically without prompting.
pub fn select_template_auto(templates: &[PrTemplate]) -> Option<PrTemplate> {
    if templates.len() == 1 {
        Some(templates[0].clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(dir: &Path, relative: &str, content: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dir");
        }
        fs::write(path, content).expect("write file");
    }

    #[test]
    fn test_discover_root_level_template() {
        let dir = TempDir::new().expect("tempdir");
        write_file(dir.path(), "PULL_REQUEST_TEMPLATE.md", "# Root template");

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Default");
        assert!(templates[0].content.contains("Root template"));
    }

    #[test]
    fn test_discover_single_template() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE.md",
            "# Single template",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Default");
        assert!(templates[0].content.contains("Single template"));
    }

    #[test]
    fn test_discover_multiple_templates() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/feature.md",
            "# Feature",
        );
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/bugfix.md",
            "# Bugfix",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 2);

        let names: Vec<_> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"bugfix"));
        assert!(names.contains(&"feature"));
    }

    #[test]
    fn test_discover_no_templates() {
        let dir = TempDir::new().expect("tempdir");
        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert!(templates.is_empty());
    }

    #[test]
    fn test_template_selection_options() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/feature.md",
            "# Feature PR",
        );
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/bugfix.md",
            "# Bugfix PR",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        let options = build_template_options(&templates);
        assert_eq!(options.len(), 3);
        assert_eq!(options[0], "No template");
        assert_eq!(options[1], "bugfix");
        assert_eq!(options[2], "feature");
    }

    #[test]
    fn test_template_selection_single_returns_directly() {
        let dir = TempDir::new().expect("tempdir");
        write_file(dir.path(), ".github/PULL_REQUEST_TEMPLATE.md", "# Single");

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);

        let selected = select_template_auto(&templates);
        assert!(selected.is_some());
        assert_eq!(selected.expect("template").name, "Default");
    }

    #[test]
    fn test_discover_lowercase_variant() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/pull_request_template.md",
            "# Lowercase",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);
        assert!(templates[0].content.contains("Lowercase"));
    }

    #[test]
    fn test_discover_docs_variant() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            "docs/PULL_REQUEST_TEMPLATE.md",
            "# Docs template",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);
        assert!(templates[0].content.contains("Docs template"));
    }

    #[test]
    fn test_directory_takes_priority_over_single() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/a.md",
            "# Template A",
        );
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE.md",
            "# Should not be used",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        // Directory templates take priority
        assert_eq!(templates.len(), 1);
        assert!(templates[0].content.contains("Template A"));
    }

    #[test]
    fn test_only_md_files_discovered() {
        let dir = TempDir::new().expect("tempdir");
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/feature.md",
            "# Feature",
        );
        write_file(
            dir.path(),
            ".github/PULL_REQUEST_TEMPLATE/readme.txt",
            "Not a template",
        );

        let templates = discover_pr_templates(dir.path()).expect("discover");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "feature");
    }
}
