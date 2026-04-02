//! JJ path resolution and command factory
//!
//! Provides jj executable path resolution and command builders.

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(unused)]

use std::process::Command as StdCommand;
use std::sync::OnceLock;

use tokio::process::Command;

static JJ_PATH: OnceLock<String> = OnceLock::new();

/// Search PATH for jj executable
fn search_path_for_jj() -> String {
    let paths = std::env::var_os("PATH").unwrap_or_default();

    std::env::split_paths(&paths)
        .map(|p| p.join("jj"))
        .find(|p| p.exists()).map_or_else(|| "jj".to_string(), |p| p.to_string_lossy().to_string())
}

/// Resolve jj path from environment or search PATH
fn resolve_jj_path() -> String {
    let env_path = std::env::var("Isolate_JJ_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());

    env_path.as_ref().map_or_else(search_path_for_jj, |p| {
        if std::path::Path::new(p).exists() {
            p.clone()
        } else {
            search_path_for_jj()
        }
    })
}

/// Get async jj command builder
pub fn get_jj_command() -> Command {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    Command::new(path.as_str())
}

/// Get sync jj command builder
pub fn get_jj_command_sync() -> StdCommand {
    let path = JJ_PATH.get_or_init(resolve_jj_path);
    StdCommand::new(path)
}
