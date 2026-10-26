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

    // The main worktree's relative name is empty (its path IS the repo path),
    // and it is not a session created by hd — exporting it as a session
    // named "" is the phantom-session bug, so it is skipped here.

    let sessions: Vec<ExportedSession> = workspaces
        .iter()
        .filter(|ws| !ws.name.is_empty())
        .filter_map(|ws| build_exported_session(ws, &cwd))
        .collect();

    // Filter to a single session if requested. Sessions are exported
    // under their short name, so a path-like target is shortened before
    // comparing (round-A name-key semantics: users type short names).
    let sessions = match &options.session {
        Some(target) => {
            let target = short_name(target);
            sessions
                .into_iter()
                .filter(|s| s.name == target)
                .collect()
        }
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
            if Output::json_output() {
                Output::json(&serde_json::json!({
                    "exported": result.count,
                    "output": output_path,
                }));
            } else {
                Output::success(&format!(
                    "Exported {} workspace(s) to {}",
                    result.count, output_path
                ));
            }
        }
        None => {
            // Stdout export is already a single JSON document in both modes;
            // JSON mode routes it through Output::json so --quiet is honored.
            if Output::json_output() {
                Output::json(&result);
            } else {
                println!("{json_output}");
            }
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

    // Existence is judged on short names: `list_workspaces` keys are
    // path-like (`.scp/workspaces/<name>`) while imported sessions are
    // short names, so comparing raw keys would never match and an
    // already-present session would be re-forked instead of skipped or
    // refused. Session-database rows count too: `work` refuses a name
    // when either a row or a worktree/branch exists, so import must
    // judge by the same union or it would register a duplicate row.
    let mut existing_names: Vec<String> = existing.iter().map(|w| short_name(&w.name)).collect();
    for name in load_session_names(&cwd) {
        if !existing_names.contains(&name) {
            existing_names.push(name);
        }
    }

    // Dry-run mode: report what would happen
    if options.dry_run {
        let to_import = count_importable(&export_data, &existing_names, options.skip_existing);
        if Output::json_output() {
            Output::json(&serde_json::json!({
                "dry_run": true,
                "would_import": to_import,
            }));
        } else {
            Output::info(&format!(
                "[dry-run] Would import {} workspace(s) from file",
                to_import
            ));
        }
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
        &existing,
        &existing_names,
        backend.as_ref() as &dyn vcs::VcsBackend,
        options,
        &mut result,
        &cwd,
    );

    result.success = result.failed == 0;
    report_import_result(&result);

    Ok(())
}

/// Import sessions from export data into the workspace backend.
///
/// A forked worktree alone is not an imported session: `query` reads
/// the session database, so an import that only forked would stay
/// invisible to `query sessions` / `session-exists` while `work`
/// still refused the name (its branch exists) — the two views
/// disagreeing about the same name. Each successful fork therefore
/// registers a session-database row the same way `work` does, with the
/// branch the new worktree actually checks out. A registration failure
/// removes the just-forked worktree and counts as a failed import, so
/// a half-imported session is never reported as imported.
fn import_sessions(
    export_data: &ExportResult,
    existing_workspaces: &[vcs::Workspace],
    existing_names: &[String],
    backend: &dyn vcs::VcsBackend,
    options: &ImportOptions,
    result: &mut ImportResult,
    cwd: &Path,
) {
    for session in &export_data.sessions {
        let ws_name = extract_workspace_name(session);
        if existing_names.iter().any(|name| name == &ws_name) {
            handle_existing_session(&ws_name, options, result);
            continue;
        }

        let recorded_branch = match session.branch.as_deref() {
            Some(branch) => branch,
            None => ws_name.as_str(),
        };
        let (fork_source, actual_branch) =
            resolve_import_source(backend, existing_workspaces, recorded_branch, &ws_name);
        match backend.fork_workspace(&fork_source, &ws_name) {
            Ok(()) => match register_imported_session(cwd, &ws_name, &actual_branch) {
                Ok(()) => {
                    result.imported += 1;
                    result.imported_sessions.push(ws_name);
                }
                Err(e) => {
                    // Unwind the fork: a worktree without its session row
                    // is exactly the invisible-session state this fixes.
                    let _ = backend.delete_workspace(&ws_name);
                    result.failed += 1;
                    result.errors.push(format!(
                        "Failed to register session '{ws_name}': {e}"
                    ));
                }
            },
            Err(e) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("Failed to create workspace '{ws_name}': {e}"));
            }
        }
    }
}

/// Decide what an import forks from and which branch the new worktree
/// actually ends up on.
///
/// - Recorded branch absent in this repo (the fresh-repo round-trip):
///   fork from the current branch instead. Forking the recorded name
///   would fail with "invalid reference", and the current branch is
///   what the session branched from in the common case. The new
///   worktree gets a branch named after the session, exactly as
///   `fork_workspace` does for a checked-out source.
/// - Recorded branch present and checked out somewhere: forking it
///   creates a new branch named after the session (same rule).
/// - Recorded branch present and not checked out: forking checks it
///   out in the new worktree, so the session keeps that branch.
fn resolve_import_source(
    backend: &dyn vcs::VcsBackend,
    existing_workspaces: &[vcs::Workspace],
    recorded_branch: &str,
    target: &str,
) -> (String, String) {
    let branch_exists = match backend.list_branches() {
        Ok(branches) => branches.iter().any(|b| b.name == recorded_branch),
        Err(_) => false,
    };
    if !branch_exists {
        let fallback = match backend.current_branch() {
            Ok(branch) => branch,
            Err(_) => recorded_branch.to_string(),
        };
        return (fallback, target.to_string());
    }
    let checked_out = existing_workspaces
        .iter()
        .any(|ws| ws.branch == recorded_branch);
    if checked_out {
        (recorded_branch.to_string(), target.to_string())
    } else {
        (recorded_branch.to_string(), recorded_branch.to_string())
    }
}

/// Register an imported session in the session database, mirroring
/// `work`'s registration: a `Created` session named `name`, whose
/// workspace is the worktree `fork_workspace` just created (its path
/// relative to the repo root is the short target name) and whose
/// branch is the branch that worktree actually checks out.
fn register_imported_session(cwd: &Path, name: &str, branch: &str) -> Result<()> {
    use scp_core::infrastructure::database::{
        DatabaseConfig, DatabaseService, SqliteDatabaseService,
    };

    let db_path =
        crate::commands::handlers::config_ports::resolve_state_db_path(cwd)?;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| Error::internal(format!("Failed to create runtime: {e}")))?;
    runtime.block_on(async move {
        use scp_session::{
            domain::{
                entities::{BranchState, Session},
                value_objects::{SessionName, WorkspaceId},
            },
            infrastructure::{SessionRepository, SqliteSessionRepository},
        };

        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Error::io_error(format!("Failed to create {}: {e}", parent.display()))
                })?;
            }
        }
        let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
        let service = SqliteDatabaseService::new(config).await?;
        let repo = SqliteSessionRepository::new(service);
        repo.init_schema()
            .await
            .map_err(|e| Error::database(e.to_string()))?;

        let session_name = SessionName::parse(name)
            .map_err(|e| Error::validation_error(format!("Invalid session name: {e:?}")))?;
        let mut session = Session::create(session_name)
            .map_err(|e| Error::validation_error(format!("Invalid session: {e}")))?;
        session.workspace = Some(
            WorkspaceId::parse(name)
                .map_err(|e| Error::validation_error(format!("Invalid workspace: {e:?}")))?,
        );
        session.branch = BranchState::OnBranch {
            name: branch.to_string(),
        };
        repo.save(&session)
            .await
            .map_err(|e| Error::database(e.to_string()))?;
        Ok::<(), Error>(())
    })
}

/// Names of sessions already in the session database, for import's
/// existence check. An absent or unreadable database reads as no
/// rows here; registration is the authoritative step and surfaces
/// real database errors as failed imports.
fn load_session_names(cwd: &Path) -> Vec<String> {
    use scp_core::infrastructure::database::{
        DatabaseConfig, DatabaseService, SqliteDatabaseService,
    };

    let db_path = match crate::commands::handlers::config_ports::resolve_state_db_path(cwd) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    if !db_path.exists() {
        return Vec::new();
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => return Vec::new(),
    };
    match runtime.block_on(async {
        use scp_session::infrastructure::SqliteSessionRepository;

        let config = DatabaseConfig::new(db_path.to_string_lossy().to_string())?;
        let service = SqliteDatabaseService::new(config).await?;
        let repo = SqliteSessionRepository::new(service);
        repo.init_schema()
            .await
            .map_err(|e| Error::database(e.to_string()))?;
        let rows = repo
            .list_rows()
            .await
            .map_err(|e| Error::database(e.to_string()))?;
        Ok::<Vec<String>, Error>(rows.iter().map(|row| row.name.clone()).collect())
    }) {
        Ok(names) => names,
        Err(_) => Vec::new(),
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
///
/// In JSON mode stdout carries the single `ImportResult` object; per-error
/// lines stay on stderr exactly as before (they never touch stdout parsing).
fn report_import_result(result: &ImportResult) {
    if Output::json_output() {
        Output::json(result);
        if !result.success {
            for err in &result.errors {
                Output::error(err);
            }
        }
        return;
    }

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
///
/// The exported name is the short name, not the list key: `list_workspaces`
/// keys are paths relative to the repo root (`.scp/workspaces/<name>` for
/// a `work` session), and writing that key into an export file made every
/// exported file un-importable — import's fork target must be a single
/// directory name, and the export session filter compares against the
/// name users type. The full list key is still used for the filesystem
/// status/mtime checks below; only the exported fields are shortened.
fn build_exported_session(ws: &vcs::Workspace, cwd: &Path) -> Option<ExportedSession> {
    let ws_path = Path::new(&ws.name);
    let full_path = cwd.join(ws_path);
    let last_modified = fs_last_modified(&full_path);
    let status = if full_path.exists() {
        "active".to_string()
    } else {
        "missing".to_string()
    };
    let short = short_name(&ws.name);

    Some(ExportedSession {
        name: short.clone(),
        status,
        workspace_path: Some(short),
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

/// The short (user-typed) name for a workspace key: the final path
/// segment of a path-like key, or the key itself when it already is a
/// single segment. Trailing separators are ignored; an empty key stays
/// empty (the main worktree's key, filtered out by the caller).
fn short_name(value: &str) -> String {
    let trimmed = value.trim_end_matches(['/', '\\']);
    match trimmed.rsplit(['/', '\\']).next() {
        Some(segment) => segment.to_string(),
        None => value.to_string(),
    }
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
    existing_names: &[String],
    skip_existing: bool,
) -> usize {
    export_data
        .sessions
        .iter()
        .filter(|s| {
            let name = extract_workspace_name(s);
            !existing_names.iter().any(|existing| existing == &name) || skip_existing
        })
        .count()
}

/// Extract the workspace name from an exported session, falling back to the
/// session name when `workspace_path` is absent or blank.
///
/// A blank `workspace_path` must NOT win over the name: an export file that
/// carries `{"name":"imported1","workspace_path":""}` names the session
/// `imported1`, and treating the blank path as the name used to make import
/// chase the phantom `""` session (the main worktree's empty relative name)
/// instead of the file's session.
///
/// Either field may still hold a path-like value from an export written
/// before exports were shortened (`.scp/workspaces/<name>`): such a
/// value is defensively shortened to its final segment, so old exported
/// files import exactly like new ones. The shortened name is also what
/// `fork_workspace` accepts — a path-like target fails its single
/// directory name validation, which is what broke every round-trip.
fn extract_workspace_name(session: &ExportedSession) -> String {
    match session.workspace_path.as_deref() {
        Some(path) if !path.trim().is_empty() => short_name(path),
        _ => short_name(&session.name),
    }
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
    fn extract_workspace_name_falls_back_on_blank_path() {
        let session = ExportedSession {
            name: "imported1".to_string(),
            status: "active".to_string(),
            workspace_path: Some(String::new()),
            branch: Some("main".to_string()),
            created_at: None,
            last_modified: None,
            metadata: None,
        };
        assert_eq!(extract_workspace_name(&session), "imported1");
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
        let existing: Vec<String> = vec![];
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
        let existing: Vec<String> = vec!["ws1".to_string()];
        assert_eq!(count_importable(&export, &existing, false), 1);
    }

    #[test]
    fn fs_last_modified_returns_none_for_missing() {
        let result = fs_last_modified(Path::new("/nonexistent/path/abc123"));
        assert!(result.is_none());
    }

    #[test]
    fn short_name_keeps_single_segment() {
        assert_eq!(short_name("plain"), "plain");
        assert_eq!(short_name(""), "");
    }

    #[test]
    fn short_name_takes_final_segment() {
        assert_eq!(short_name(".scp/workspaces/alpha"), "alpha");
        assert_eq!(short_name(".scp/workspaces/alpha/"), "alpha");
        assert_eq!(short_name(".scp\\workspaces\\alpha"), "alpha");
    }

    #[test]
    fn extract_workspace_name_shortens_path_like_values() {
        let session = ExportedSession {
            name: ".scp/workspaces/alpha".to_string(),
            status: "active".to_string(),
            workspace_path: Some(".scp/workspaces/alpha".to_string()),
            branch: Some("alpha".to_string()),
            created_at: None,
            last_modified: None,
            metadata: None,
        };
        assert_eq!(extract_workspace_name(&session), "alpha");
    }

    #[test]
    fn extract_workspace_name_shortens_path_like_name_fallback() {
        let session = ExportedSession {
            name: ".scp/workspaces/beta".to_string(),
            status: "active".to_string(),
            workspace_path: None,
            branch: None,
            created_at: None,
            last_modified: None,
            metadata: None,
        };
        assert_eq!(extract_workspace_name(&session), "beta");
    }

    #[test]
    fn build_exported_session_writes_short_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ws = vcs::Workspace {
            name: ".scp/workspaces/alpha".to_string(),
            branch: "alpha".to_string(),
            is_current: false,
        };
        let session = build_exported_session(&ws, dir.path()).expect("session");
        assert_eq!(session.name, "alpha");
        assert_eq!(session.workspace_path.as_deref(), Some("alpha"));
        assert_eq!(session.branch.as_deref(), Some("alpha"));
    }
}
