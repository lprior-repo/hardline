//! Config ports - Configuration path resolution and port adapters
//!
//! Provides the seam for configuration loading and path resolution,
//! adapted from isolate's config_ports module.
//!
//! # Architecture (Data -> Calc -> Actions)
//!
//! - **Data**: ConfigPortInfo, ConfigPaths (inert, serializable)
//! - **Calculations**: path resolution (pure functions)
//! - **Actions**: run_config_ports (I/O - output)

use std::path::PathBuf;

use scp_core::{output::Output, Error, Result};
use serde::{Deserialize, Serialize};

/// Configuration port information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPortInfo {
    /// Global config path
    pub global_config_path: String,
    /// Project config path
    pub project_config_path: String,
    /// State database path (resolved)
    pub state_db_path: String,
    /// Whether global config exists
    pub global_exists: bool,
    /// Whether project config exists
    pub project_exists: bool,
    /// Whether state database exists
    pub state_db_exists: bool,
}

/// Resolved configuration paths
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub global_config: PathBuf,
    pub project_config: PathBuf,
    pub state_db: PathBuf,
}

/// Resolve global config directory path
///
/// **Calculations (Tier 2)**: Pure function
pub fn global_config_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("", "", "scp")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
        .ok_or_else(|| Error::io_error("Failed to determine global config directory".to_string()))
}

/// Resolve project config path
///
/// **Calculations (Tier 2)**: Pure function (deterministic from cwd)
pub fn project_config_path() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|dir| dir.join(".scp/config.toml"))
        .map_err(|e| Error::io_error(format!("Failed to get current directory: {e}")))
}

/// Resolve state database path
///
/// Priority: env `SCP_STATE_DB` > default `.scp/state.db`
///
/// **Calculations (Tier 2)**: Pure function
pub fn resolve_state_db_path(repo_root: &std::path::Path) -> Result<PathBuf> {
    if let Ok(env_db) = std::env::var("SCP_STATE_DB") {
        return Ok(PathBuf::from(env_db));
    }

    if let Ok(db_flag) = std::env::var("SCP_DATABASE_PATH") {
        return Ok(PathBuf::from(db_flag));
    }

    Ok(repo_root.join(".scp").join("state.db"))
}

/// Resolve all config paths
///
/// **Calculations (Tier 2)**: Pure function
pub fn resolve_all_paths() -> Result<ConfigPaths> {
    Ok(ConfigPaths {
        global_config: global_config_path()?,
        project_config: project_config_path()?,
        state_db: resolve_state_db_path(
            &std::env::current_dir().map_err(|e| Error::io_error(e.to_string()))?,
        )?,
    })
}

/// Run the config ports command - display resolved configuration paths
///
/// **Actions (Tier 3)**: I/O - outputs config path info
pub fn run_config_ports(json: bool) -> Result<()> {
    let paths = resolve_all_paths()?;

    let info = ConfigPortInfo {
        global_config_path: paths.global_config.display().to_string(),
        project_config_path: paths.project_config.display().to_string(),
        state_db_path: paths.state_db.display().to_string(),
        global_exists: paths.global_config.exists(),
        project_exists: paths.project_config.exists(),
        state_db_exists: paths.state_db.exists(),
    };

    if json {
        let output = serde_json::to_string_pretty(&info)
            .map_err(|e| Error::io_error(format!("Failed to serialize config info: {e}")))?;
        println!("{output}");
    } else {
        Output::info(&format!("Global config: {}", info.global_config_path));
        Output::info(&format!(
            "  Exists: {}",
            if info.global_exists { "yes" } else { "no" }
        ));
        Output::info(&format!("Project config: {}", info.project_config_path));
        Output::info(&format!(
            "  Exists: {}",
            if info.project_exists { "yes" } else { "no" }
        ));
        Output::info(&format!("State database: {}", info.state_db_path));
        Output::info(&format!(
            "  Exists: {}",
            if info.state_db_exists { "yes" } else { "no" }
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_global_config_path_returns_something() {
        let path = global_config_path();
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("scp"));
        assert!(p.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_project_config_path_returns_something() {
        let path = project_config_path();
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains(".scp"));
        assert!(p.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    #[serial]
    fn test_resolve_state_db_default() {
        // Remove env vars to test default behavior
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd);
        assert!(path.is_ok());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains(".scp"));
        assert!(p.to_string_lossy().contains("state.db"));
    }

    #[test]
    #[serial]
    fn test_resolve_state_db_from_env() {
        std::env::set_var("SCP_STATE_DB", "/tmp/custom.db");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd);
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), PathBuf::from("/tmp/custom.db"));

        std::env::remove_var("SCP_STATE_DB");
    }

    #[test]
    #[serial]
    fn test_resolve_all_paths() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths();
        assert!(paths.is_ok());
        let p = paths.unwrap();
        assert!(p.global_config.to_string_lossy().contains("scp"));
        assert!(p.project_config.to_string_lossy().contains(".scp"));
        assert!(p.state_db.to_string_lossy().contains("state.db"));
    }

    #[test]
    fn test_config_port_info_serialization() {
        let info = ConfigPortInfo {
            global_config_path: "/home/user/.config/scp/config.toml".to_string(),
            project_config_path: "/project/.scp/config.toml".to_string(),
            state_db_path: "/project/.scp/state.db".to_string(),
            global_exists: false,
            project_exists: true,
            state_db_exists: true,
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        assert!(json.contains("global_config_path"));
        assert!(json.contains("state_db_path"));
        assert!(json.contains("true"));
        assert!(json.contains("false"));
    }

    #[test]
    #[serial]
    fn test_resolve_state_db_priority_database_path() {
        // SCP_DATABASE_PATH should be used if SCP_STATE_DB is not set
        std::env::remove_var("SCP_STATE_DB");
        std::env::set_var("SCP_DATABASE_PATH", "/tmp/from-flag.db");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd);
        assert_eq!(path.unwrap(), PathBuf::from("/tmp/from-flag.db"));

        std::env::remove_var("SCP_DATABASE_PATH");
    }

    #[test]
    #[serial]
    fn test_resolve_state_db_state_db_overrides_database_path() {
        // SCP_STATE_DB should take priority over SCP_DATABASE_PATH
        std::env::set_var("SCP_STATE_DB", "/tmp/state-db-path.db");
        std::env::set_var("SCP_DATABASE_PATH", "/tmp/database-path.db");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd);
        assert_eq!(path.unwrap(), PathBuf::from("/tmp/state-db-path.db"));

        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");
    }
}
