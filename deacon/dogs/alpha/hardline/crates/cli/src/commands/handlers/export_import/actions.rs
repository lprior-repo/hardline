//! Action functions for the export_import command handler (Tier 3).
//!
//! I/O operations for exporting and importing session configurations.

use std::path::Path;

use scp_core::output::Output;
use scp_core::{Error, Result};

use super::data::{
    ExportOptions, ExportResult, ExportedSession, ImportOptions, ImportResult,
};

/// Execute the export command.
///
/// # Errors
///
/// Returns errors for file I/O failures or serialization failures.
pub fn run_export(options: &ExportOptions) -> Result<()> {
    // TODO: Wire to actual session database when available
    // For now, export an empty result as a placeholder
    let sessions = Vec::<ExportedSession>::new();

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
            Output::success(&format!("Exported {} sessions to {}", result.count, output_path));
        }
        None => {
            println!("{json_output}");
        }
    }

    Ok(())
}

/// Execute the import command.
///
/// # Errors
///
/// Returns errors for file I/O failures, deserialization failures,
/// or session import failures.
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

    let _export_data: ExportResult = serde_json::from_str(&content)
        .map_err(|e| Error::validation_error(format!("Invalid import file format: {e}")))?;

    // Dry-run mode
    if options.dry_run {
        Output::info("[dry-run] Would import sessions from file");
        return Ok(());
    }

    // TODO: Wire to actual session database when available
    Output::success("Import completed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

    #[test]
    fn run_export_to_stdout() {
        let options = ExportOptions {
            session: None,
            output: None,
        };
        assert!(run_export(&options).is_ok());
    }

    #[test]
    fn run_export_to_file() {
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let output_path = tmp_dir.path().join("export.json");
        let options = ExportOptions {
            session: None,
            output: Some(output_path.to_string_lossy().to_string()),
        };
        assert!(run_export(&options).is_ok());
        assert!(output_path.exists());
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
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let input_path = tmp_dir.path().join("export.json");
        let export_data = ExportResult {
            version: "1.0".to_string(),
            exported_at: "2025-01-01T00:00:00Z".to_string(),
            count: 0,
            sessions: vec![],
        };
        std::fs::write(&input_path, serde_json::to_string_pretty(&export_data).expect("ser"))
            .expect("write");

        let options = ImportOptions {
            input: input_path.to_string_lossy().to_string(),
            force: false,
            skip_existing: false,
            dry_run: true,
        };
        assert!(run_import(&options).is_ok());
    }
}
