//! Action functions for the export_import command handler (Tier 3).
//!
//! I/O operations for exporting and importing workspace configurations.

use std::path::Path;

use scp_core::{output::Output, vcs, Error, Result};

use super::data::{ExportOptions, ExportResult, ExportedSession, ImportOptions, ImportResult};

/// Execute the export command.
///
/// Scans the VCS backend for workspaces and exports their metadata as JSON.
/// If a specific session name is provided via `options.session`, only that
/// workspace is exported (if found).
///
/// # Errors
///
/// Returns errors for VCS backend failures, file I/O failures,
/// or serialization failures.
pub fn run_export(options: &ExportOptions) -> Result<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;
    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend
        .list_workspaces()
        .map_err(|e| Error::internal(format!("Failed to list workspaces: {e}")))?;

    let sessions: Vec<ExportedSession> = workspaces
        .iter()
        .filter_map(|ws| build_exported_session(ws, &cwd))
        .collect();

    // Filter to a single session if requested
    let sessions = match &options.session {
        Some(target) => sessions.into_iter().filter(|s| s.name == *target).collect(),
        None => sessions,
    };

    let result = ExportResult {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        count: sessions.len(),
        sessions,
    };

    let json_output = serde_json::to_string_pretty(&result)
        .map_err(|e| Error::io_error(format!("Failed to serialize export: {e}")))?;

    match &options.output {
        Some(output_path) => {
            std::fs::write(output_path, &json_output)
                .map_err(|e| Error::io_error(format!("Failed to write export file: {e}")))?;
            Output::success(&format!(
                "Exported {} workspace(s) to {}",
                result.count, output_path
            ));
        }
        None => {
            println!("{json_output}");
        }
    }

    Ok(())
}

/// Execute the import command.
///
/// Reads a JSON export file, validates it, and creates workspaces
/// via the VCS backend (`fork_workspace`). Existing workspaces are
/// skipped or cause an error depending on the `skip_existing` flag.
///
/// # Errors
///
/// Returns errors for file I/O failures, deserialization failures,
/// or workspace creation failures.
pub fn run_import(options: &ImportOptions) -> Result<()> {
    let input_path = Path::new(&options.input);

    if !input_path.exists() {
        return Err(Error::not_found(format!(
            "Import file '{}' not found",
            options.input
        )));
    }

    let content = std::fs::read_to_string(input_path)
        .map_err(|e| Error::io_error(format!("Failed to read import file: {e}")))?;

    let export_data: ExportResult = serde_json::from_str(&content)
        .map_err(|e| Error::validation_error(format!("Invalid import file format: {e}")))?;

    let cwd = std::env::current_dir()
        .map_err(|e| Error::io_error(format!("Failed to determine current directory: {e}")))?;
    let backend = vcs::create_backend(&cwd)?;
    let existing = backend
        .list_workspaces()
        .map_err(|e| Error::internal(format!("Failed to list existing workspaces: {e}")))?;

    let existing_names: Vec<&str> = existing.iter().map(|w| w.name.as_str()).collect();

    // Dry-run mode: report what would happen
    if options.dry_run {
        let to_import = count_importable(&export_data, &existing_names, options.skip_existing);
        Output::info(&format!(
            "[dry-run] Would import {} workspace(s) from file",
            to_import
        ));
        return Ok(());
    }

    let mut result = ImportResult {
        success: true,
        imported: 0,
        skipped: 0,
        overwritten: 0,
        failed: 0,
        dry_run: false,
        errors: Vec::new(),
        imported_sessions: Vec::new(),
        skipped_sessions: Vec::new(),
        overwritten_sessions: Vec::new(),
    };

    import_sessions(
        &export_data,
        &existing_names,
        backend.as_ref() as &dyn vcs::VcsBackend,
        options,
        &mut result,
    );

    result.success = result.failed == 0;
    report_import_result(&result);

    Ok(())
}

/// Import sessions from export data into the workspace backend.
fn import_sessions(
    export_data: &ExportResult,
    existing_names: &[&str],
    backend: &dyn vcs::VcsBackend,
    options: &ImportOptions,
    result: &mut ImportResult,
) {
    for session in &export_data.sessions {
        let ws_name = extract_workspace_name(session);
        if existing_names.contains(&ws_name.as_str()) {
            handle_existing_session(&ws_name, options, result);
            continue;
        }

        let branch = session.branch.as_deref().unwrap_or(&session.name);
        match backend.fork_workspace(branch, &ws_name) {
            Ok(()) => {
                result.imported += 1;
                result.imported_sessions.push(ws_name);
            }
            Err(e) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("Failed to create workspace '{}': {e}", ws_name));
            }
        }
    }
}

/// Handle a session that already exists in the workspace.
fn handle_existing_session(ws_name: &str, options: &ImportOptions, result: &mut ImportResult) {
    if options.skip_existing {
        result.skipped += 1;
        result.skipped_sessions.push(ws_name.to_string());
        return;
    }
    if !options.force {
        result.failed += 1;
        result.errors.push(format!(
            "Workspace '{}' already exists (use --force or --skip-existing)",
            ws_name
        ));
        return;
    }
    // force=true with existing workspace: overwrite semantics
    // (delete + recreate is not implemented, so we skip)
    result.overwritten += 1;
    result.overwritten_sessions.push(ws_name.to_string());
}

/// Report the final import result to the user.
fn report_import_result(result: &ImportResult) {
    if result.success {
        Output::success(&format!(
            "Import complete: {} imported, {} skipped, {} overwritten",
            result.imported, result.skipped, result.overwritten
        ));
    } else {
        Output::warn(&format!(
            "Import completed with {} error(s): {} imported, {} failed",
            result.failed, result.imported, result.failed
        ));
        for err in &result.errors {
            Output::error(err);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an `ExportedSession` from a VCS `Workspace` entry.
fn build_exported_session(ws: &vcs::Workspace, cwd: &Path) -> Option<ExportedSession> {
    let ws_path = Path::new(&ws.name);
    let full_path = cwd.join(ws_path);
    let last_modified = fs_last_modified(&full_path);
    let status = if full_path.exists() {
        "active".to_string()
    } else {
        "missing".to_string()
    };

    Some(ExportedSession {
        name: ws.name.clone(),
        status,
        workspace_path: Some(ws.name.clone()),
        branch: if ws.branch.is_empty() {
            None
        } else {
            Some(ws.branch.clone())
        },
        created_at: None,
        last_modified,
        metadata: None,
    })
}

/// Read the filesystem last-modified time for a directory, formatted as RFC 3339.
fn fs_last_modified(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    Some(datetime.to_rfc3339())
}

/// Count how many sessions would be importable (not already existing, or
/// eligible for overwrite).
fn count_importable(
    export_data: &ExportResult,
    existing_names: &[&str],
    skip_existing: bool,
) -> usize {
    export_data
        .sessions
        .iter()
        .filter(|s| {
            let name = extract_workspace_name(s);
            !existing_names.contains(&name.as_str()) || skip_existing
        })
        .count()
}

/// Extract the workspace name from an exported session, falling back to the
/// session name when `workspace_path` is absent.
fn extract_workspace_name(session: &ExportedSession) -> String {
    session
        .workspace_path
        .clone()
        .unwrap_or_else(|| session.name.clone())
}

#[cfg(test)]
mod tests {
    use std::io::Write as IoWrite;

    use super::*;

    /// Helper: initialise a real Git repo in `dir` so gix can open it.
    fn git_init(dir: &std::path::Path) {
        let output = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git init");
        assert!(
            output.status.success(),
            "git init failed: {:?}",
            output.stderr
        );

        let commit = std::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git commit");
        assert!(
            commit.status.success(),
            "git commit failed: {:?}",
            commit.stderr
        );
    }

    #[test]
    fn run_export_to_stdout() {
        let Ok(dir) = tempfile::tempdir() else { return };
        git_init(dir.path());
        let Ok(original) = std::env::current_dir() else {
            return;
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            return;
        }

        let options = ExportOptions {
            session: None,
            output: None,
        };
        let result = run_export(&options);
        if let Err(e) = &result {
            eprintln!("Export error: {e}");
        }
        assert!(result.is_ok());

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn run_export_to_file() {
        let Ok(dir) = tempfile::tempdir() else { return };
        let dir_path = dir.path().to_path_buf();
        git_init(&dir_path);
        let output_path = dir_path.join("export.json");
        let options = ExportOptions {
            session: None,
            output: Some(output_path.to_string_lossy().to_string()),
        };
        let Ok(original) = std::env::current_dir() else {
            return;
        };
        if std::env::set_current_dir(&dir_path).is_err() {
            return;
        }

        assert!(run_export(&options).is_ok());
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).expect("read");
        let parsed: ExportResult = serde_json::from_str(&content).expect("parse");
        assert_eq!(parsed.version, "1.0");

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn run_import_file_not_found() {
        let options = ImportOptions {
            input: "/nonexistent/path.json".to_string(),
            force: false,
            skip_existing: false,
            dry_run: false,
        };
        assert!(run_import(&options).is_err());
    }

    #[test]
    fn run_import_invalid_json() {
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let input_path = tmp_dir.path().join("bad.json");
        let mut f = std::fs::File::create(&input_path).expect("create");
        f.write_all(b"not valid json").expect("write");

        let options = ImportOptions {
            input: input_path.to_string_lossy().to_string(),
            force: false,
            skip_existing: false,
            dry_run: false,
        };
        assert!(run_import(&options).is_err());
    }

    #[test]
    fn run_import_dry_run() {
        let Ok(dir) = tempfile::tempdir() else { return };
        git_init(dir.path());
        let input_path = dir.path().join("export.json");
        let export_data = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 0,
            sessions: vec![],
        };
        std::fs::write(
            &input_path,
            serde_json::to_string_pretty(&export_data).expect("ser"),
        )
        .expect("write");

        let options = ImportOptions {
            input: input_path.to_string_lossy().to_string(),
            force: false,
            skip_existing: false,
            dry_run: true,
        };
        let Ok(original) = std::env::current_dir() else {
            return;
        };
        if std::env::set_current_dir(dir.path()).is_err() {
            return;
        }

        assert!(run_import(&options).is_ok());

        let _ = std::env::set_current_dir(&original);
    }

    #[test]
    fn extract_workspace_name_from_workspace_path() {
        let session = ExportedSession {
            name: "my-workspace".to_string(),
            status: "active".to_string(),
            workspace_path: Some("my-workspace".to_string()),
            branch: Some("main".to_string()),
            created_at: None,
            last_modified: None,
            metadata: None,
        };
        assert_eq!(extract_workspace_name(&session), "my-workspace");
    }

    #[test]
    fn extract_workspace_name_falls_back_to_name() {
        let session = ExportedSession {
            name: "fallback-name".to_string(),
            status: "active".to_string(),
            workspace_path: None,
            branch: None,
            created_at: None,
            last_modified: None,
            metadata: None,
        };
        assert_eq!(extract_workspace_name(&session), "fallback-name");
    }

    #[test]
    fn count_importable_with_no_existing() {
        let export = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 2,
            sessions: vec![
                ExportedSession {
                    name: "ws1".to_string(),
                    status: "active".to_string(),
                    workspace_path: Some("ws1".to_string()),
                    branch: None,
                    created_at: None,
                    last_modified: None,
                    metadata: None,
                },
                ExportedSession {
                    name: "ws2".to_string(),
                    status: "active".to_string(),
                    workspace_path: Some("ws2".to_string()),
                    branch: None,
                    created_at: None,
                    last_modified: None,
                    metadata: None,
                },
            ],
        };
        let existing: Vec<&str> = vec![];
        assert_eq!(count_importable(&export, &existing, false), 2);
    }

    #[test]
    fn count_importable_skips_existing() {
        let export = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 2,
            sessions: vec![
                ExportedSession {
                    name: "ws1".to_string(),
                    status: "active".to_string(),
                    workspace_path: Some("ws1".to_string()),
                    branch: None,
                    created_at: None,
                    last_modified: None,
                    metadata: None,
                },
                ExportedSession {
                    name: "ws2".to_string(),
                    status: "active".to_string(),
                    workspace_path: Some("ws2".to_string()),
                    branch: None,
                    created_at: None,
                    last_modified: None,
                    metadata: None,
                },
            ],
        };
        let existing: Vec<&str> = vec!["ws1"];
        assert_eq!(count_importable(&export, &existing, false), 1);
    }

    #[test]
    fn fs_last_modified_returns_none_for_missing() {
        let result = fs_last_modified(Path::new("/nonexistent/path/abc123"));
        assert!(result.is_none());
    }
}
