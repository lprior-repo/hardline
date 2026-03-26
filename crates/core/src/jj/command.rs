//! JJ command execution - Core layer
//!
//! Handles JJ binary path resolution and command construction.

use std::process::Command as StdCommand;
use std::sync::OnceLock;

use tokio::process::Command;

use crate::Error;

static JJ_PATH: OnceLock<String> = OnceLock::new();

/// Resolve the JJ binary path
fn resolve_jj_path() -> String {
    let env_path = std::env::var("SCP_JJ_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let path = env_path.as_ref().map_or_else(search_path_for_jj, |p| {
        if std::path::Path::new(p).exists() {
            p.clone()
        } else {
            search_path_for_jj()
        }
    });
    path
}

/// Search PATH for jj binary
fn search_path_for_jj() -> String {
    let paths = std::env::var_os("PATH").unwrap_or_default();

    let found = std::env::split_paths(&paths)
        .map(|p| p.join("jj"))
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string());

    found.unwrap_or_else(|| "jj".to_string())
}

/// Get async JJ command
pub fn get_jj_command() -> Command {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    Command::new(path.as_str())
}

/// Get sync JJ command
pub fn get_jj_command_sync() -> StdCommand {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    StdCommand::new(path)
}

/// Create a JJ command error
#[allow(clippy::print_stderr)]
pub fn jj_command_error(operation: &str, error: &std::io::Error) -> Error {
    let is_not_found = error.kind() == std::io::ErrorKind::NotFound;
    eprintln!(
        "DEBUG: JJ COMMAND ERROR: operation={operation}, error={error}, kind={error_kind:?}, path={path:?}",
        error_kind = error.kind(),
        path = JJ_PATH.get()
    );
    Error::jj_command_error(operation, error.to_string(), is_not_found)
}
