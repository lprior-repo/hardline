//! CLI handlers for backup, export, and import commands.
//!
//! This module provides the CLI entry points that parse `ArgMatches` and dispatch
//! to the appropriate handler functions in the backup and export_import modules.
//!
//! # Architecture
//!
//! - **Handlers**: `handle_backup`, `handle_export`, `handle_import` - parse CLI args
//! - **Backup module**: Uses `BackupCommand` enum with `execute_backup_command`
//! - **Export/Import module**: Uses `ExportOptions`/`ImportOptions` with `run_export`/`run_import`

use std::path::Path;

use anyhow::Result;
use clap::ArgMatches;

use super::json_format::get_format;
use crate::commands::handlers::{
    backup::{
        actions::execute_backup_command,
        data::{BackupCommand, BackupConfig},
    },
    export_import::{
        actions::{run_export, run_import},
        data::{ExportOptions, ImportOptions},
    },
};

/// Handle the backup command - create, list, restore, status, or retention.
///
/// Parses CLI arguments and dispatches to the appropriate backup action.
pub async fn handle_backup(sub_m: &ArgMatches, root: &Path) -> Result<()> {
    let create = sub_m.get_flag("create");
    let list = sub_m.get_flag("list");
    let restore = sub_m.get_one::<String>("restore");
    let status = sub_m.get_flag("status");
    let retention = sub_m.get_flag("retention");

    let config = BackupConfig::default();

    let cmd = match (create, list, restore, status, retention) {
        (true, false, None, false, false) => BackupCommand::Create,
        (false, true, None, false, false) => BackupCommand::List,
        (false, false, Some(database), false, false) => BackupCommand::Restore {
            database: database.clone(),
            timestamp: sub_m.get_one::<String>("timestamp").cloned(),
        },
        (false, false, None, true, false) => BackupCommand::Status,
        (false, false, None, false, true) => BackupCommand::Retention,
        _ => {
            anyhow::bail!(
                "Unknown backup action. Use --create, --list, --restore <DATABASE>, --status, or --retention"
            );
        }
    };

    execute_backup_command(&cmd, root, &config)
        .await
        .map_err(Into::into)
}

/// Handle the export command - export session configurations.
///
/// Parses CLI arguments and delegates to `export_import::run_export`.
pub fn handle_export(sub_m: &ArgMatches) -> Result<()> {
    let session = sub_m.get_one::<String>("session").cloned();
    let output = sub_m.get_one::<String>("output").cloned();

    if let Some(ref session_name) = session {
        if looks_like_file_path(session_name) && output.is_none() {
            anyhow::bail!(
                "Ambiguous argument: '{session_name}' looks like a file path.\n\
                 \n\
                 If you meant to export TO a file, use the -o flag:\n\
                   scp export -o {session_name}\n\
                 \n\
                 If '{session_name}' is actually a session name, please rename it\n\
                 or use the full path to disambiguate."
            );
        }
    }

    let options = ExportOptions { session, output };
    run_export(&options).map_err(Into::into)
}

/// Handle the import command - import session configurations.
///
/// Parses CLI arguments and delegates to `export_import::run_import`.
pub fn handle_import(sub_m: &ArgMatches) -> Result<()> {
    let input = sub_m
        .get_one::<String>("file")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Input file is required"))?;
    let force = sub_m.get_flag("force");
    let skip_existing = sub_m.get_flag("skip-existing");
    let dry_run = sub_m.get_flag("dry-run");

    let options = ImportOptions {
        input,
        force,
        skip_existing,
        dry_run,
    };
    run_import(&options).map_err(Into::into)
}

/// Check if a string looks like a file path based on extension or path separators.
///
/// This helps disambiguate session names from file paths when the user
/// might have a session named like a file path.
fn looks_like_file_path(s: &str) -> bool {
    let has_extension = s.contains('.')
        && s.split('.').next_back().is_some_and(|ext| {
            let ext_lower = ext.to_lowercase();
            matches!(
                ext_lower.as_str(),
                "json" | "yaml" | "yml" | "toml" | "txt" | "csv" | "xml"
            )
        });

    let has_path_sep = s.contains('/') || s.contains('\\');

    has_extension || has_path_sep
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_looks_like_file_path_json_extension() {
        assert!(super::looks_like_file_path("export.json"));
        assert!(super::looks_like_file_path("data.JSON"));
        assert!(super::looks_like_file_path("backup.json"));
    }

    #[test]
    fn test_looks_like_file_path_other_extensions() {
        assert!(super::looks_like_file_path("config.yaml"));
        assert!(super::looks_like_file_path("data.yml"));
        assert!(super::looks_like_file_path("settings.toml"));
        assert!(super::looks_like_file_path("notes.txt"));
        assert!(super::looks_like_file_path("data.csv"));
        assert!(super::looks_like_file_path("config.xml"));
    }

    #[test]
    fn test_looks_like_file_path_with_path_separator() {
        assert!(super::looks_like_file_path("/tmp/export"));
        assert!(super::looks_like_file_path("./output"));
        assert!(super::looks_like_file_path("data/export"));
        assert!(super::looks_like_file_path("C:\\Users\\data"));
    }

    #[test]
    fn test_looks_like_file_path_valid_session_names() {
        assert!(!super::looks_like_file_path("feature-x"));
        assert!(!super::looks_like_file_path("main"));
        assert!(!super::looks_like_file_path("bugfix-123"));
        assert!(!super::looks_like_file_path("my-workspace"));
        assert!(!super::looks_like_file_path("dev"));
    }

    #[test]
    fn test_looks_like_file_path_edge_cases() {
        assert!(!super::looks_like_file_path("v1.2.3"));
        assert!(!super::looks_like_file_path("feature.test"));
        assert!(!super::looks_like_file_path("file.unknownext"));
    }
}
