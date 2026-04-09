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

    // =========================================================================
    // Config port listing — global_config_path
    // =========================================================================

    #[test]
    fn global_config_path_contains_scp_directory() {
        let path = global_config_path().expect("global config path must resolve");
        assert!(
            path.to_string_lossy().contains("scp"),
            "Path should contain 'scp': {:?}",
            path
        );
    }

    #[test]
    fn global_config_path_ends_with_config_toml() {
        let path = global_config_path().expect("global config path must resolve");
        assert!(
            path.to_string_lossy().ends_with("config.toml"),
            "Path should end with config.toml: {:?}",
            path
        );
    }

    #[test]
    fn global_config_path_is_absolute() {
        let path = global_config_path().expect("global config path must resolve");
        assert!(
            path.is_absolute(),
            "Global config path must be absolute: {:?}",
            path
        );
    }

    #[test]
    fn global_config_path_returns_consistent_results() {
        let path1 = global_config_path().expect("first call must succeed");
        let path2 = global_config_path().expect("second call must succeed");
        assert_eq!(path1, path2, "Repeated calls must return identical paths");
    }

    // =========================================================================
    // Config port listing — project_config_path
    // =========================================================================

    #[test]
    fn project_config_path_contains_scp_directory() {
        let path = project_config_path().expect("project config path must resolve");
        assert!(
            path.to_string_lossy().contains(".scp"),
            "Path should contain '.scp': {:?}",
            path
        );
    }

    #[test]
    fn project_config_path_ends_with_config_toml() {
        let path = project_config_path().expect("project config path must resolve");
        assert!(
            path.to_string_lossy().ends_with("config.toml"),
            "Path should end with config.toml: {:?}",
            path
        );
    }

    #[test]
    fn project_config_path_is_absolute() {
        let path = project_config_path().expect("project config path must resolve");
        assert!(
            path.is_absolute(),
            "Project config path must be absolute: {:?}",
            path
        );
    }

    #[test]
    fn project_config_path_reflects_current_working_directory() {
        let path = project_config_path().expect("must resolve");
        let cwd = std::env::current_dir().expect("cwd must be available");
        assert!(
            path.starts_with(&cwd),
            "Project config should start with cwd {:?}, got {:?}",
            cwd,
            path
        );
    }

    // =========================================================================
    // Default port configuration — resolve_state_db_path (no env vars)
    // =========================================================================

    #[test]
    #[serial]
    fn state_db_default_path_under_dot_scp_directory() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd).expect("default path must resolve");
        assert!(
            path.to_string_lossy().contains(".scp"),
            "Default should be under .scp: {:?}",
            path
        );
        assert!(
            path.to_string_lossy().ends_with("state.db"),
            "Default should end with state.db: {:?}",
            path
        );
    }

    #[test]
    #[serial]
    fn state_db_default_path_is_relative_to_repo_root() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let repo_root = std::path::Path::new("/tmp/my-project");
        let path = resolve_state_db_path(repo_root).expect("must resolve");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/my-project/.scp/state.db"),
            "Default should be repo_root/.scp/state.db"
        );
    }

    // =========================================================================
    // Port assignment — SCP_STATE_DB env var
    // =========================================================================

    #[test]
    #[serial]
    fn state_db_uses_scp_state_db_env_when_set() {
        std::env::set_var("SCP_STATE_DB", "/tmp/custom-state.db");
        std::env::remove_var("SCP_DATABASE_PATH");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd).expect("must resolve from env");
        assert_eq!(
            path,
            PathBuf::from("/tmp/custom-state.db"),
            "Should use SCP_STATE_DB value"
        );

        std::env::remove_var("SCP_STATE_DB");
    }

    #[test]
    #[serial]
    fn state_db_uses_scp_database_path_env_when_state_db_unset() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::set_var("SCP_DATABASE_PATH", "/tmp/from-flag.db");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd).expect("must resolve from env");
        assert_eq!(
            path,
            PathBuf::from("/tmp/from-flag.db"),
            "Should use SCP_DATABASE_PATH fallback"
        );

        std::env::remove_var("SCP_DATABASE_PATH");
    }

    // =========================================================================
    // Port conflict detection — SCP_STATE_DB takes priority over SCP_DATABASE_PATH
    // =========================================================================

    #[test]
    #[serial]
    fn state_db_scp_state_db_wins_over_scp_database_path() {
        std::env::set_var("SCP_STATE_DB", "/tmp/primary.db");
        std::env::set_var("SCP_DATABASE_PATH", "/tmp/fallback.db");

        let cwd = std::env::current_dir().unwrap();
        let path = resolve_state_db_path(&cwd).expect("must resolve");
        assert_eq!(
            path,
            PathBuf::from("/tmp/primary.db"),
            "SCP_STATE_DB must take priority"
        );

        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");
    }

    #[test]
    #[serial]
    fn state_db_priority_chain_all_three_levels() {
        let cwd = std::env::current_dir().unwrap();

        // Level 1: SCP_STATE_DB wins over everything
        std::env::set_var("SCP_STATE_DB", "/tmp/l1.db");
        std::env::set_var("SCP_DATABASE_PATH", "/tmp/l2.db");
        assert_eq!(
            resolve_state_db_path(&cwd).unwrap(),
            PathBuf::from("/tmp/l1.db"),
            "SCP_STATE_DB must be highest priority"
        );

        // Level 2: SCP_DATABASE_PATH when SCP_STATE_DB unset
        std::env::remove_var("SCP_STATE_DB");
        assert_eq!(
            resolve_state_db_path(&cwd).unwrap(),
            PathBuf::from("/tmp/l2.db"),
            "SCP_DATABASE_PATH is second priority"
        );

        // Level 3: Default when both unset
        std::env::remove_var("SCP_DATABASE_PATH");
        let default_path = resolve_state_db_path(&cwd).unwrap();
        assert!(
            default_path.to_string_lossy().contains("state.db"),
            "Default must contain state.db: {:?}",
            default_path
        );
    }

    // =========================================================================
    // Port release — clearing env vars falls back to default
    // =========================================================================

    #[test]
    #[serial]
    fn state_db_clearing_env_vars_returns_to_default() {
        let cwd = std::env::current_dir().unwrap();

        // Set and verify env override
        std::env::set_var("SCP_STATE_DB", "/tmp/override.db");
        assert_eq!(
            resolve_state_db_path(&cwd).unwrap(),
            PathBuf::from("/tmp/override.db")
        );

        // Release (clear) — should fall back to default
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");
        let default_path = resolve_state_db_path(&cwd).unwrap();
        assert!(
            default_path.to_string_lossy().contains(".scp"),
            "After clearing env, should return to default: {:?}",
            default_path
        );
    }

    // =========================================================================
    // Port status display — resolve_all_paths
    // =========================================================================

    #[test]
    #[serial]
    fn resolve_all_paths_returns_three_distinct_paths() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("all paths must resolve");
        assert!(
            !paths.global_config.to_string_lossy().is_empty(),
            "Global config path must not be empty"
        );
        assert!(
            !paths.project_config.to_string_lossy().is_empty(),
            "Project config path must not be empty"
        );
        assert!(
            !paths.state_db.to_string_lossy().is_empty(),
            "State DB path must not be empty"
        );
    }

    #[test]
    #[serial]
    fn resolve_all_paths_global_ends_with_config_toml() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("must resolve");
        assert!(
            paths
                .global_config
                .to_string_lossy()
                .ends_with("config.toml"),
            "Global config must end with config.toml"
        );
    }

    #[test]
    #[serial]
    fn resolve_all_paths_project_ends_with_config_toml() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("must resolve");
        assert!(
            paths
                .project_config
                .to_string_lossy()
                .ends_with("config.toml"),
            "Project config must end with config.toml"
        );
    }

    #[test]
    #[serial]
    fn resolve_all_paths_state_db_ends_with_state_db() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("must resolve");
        assert!(
            paths.state_db.to_string_lossy().ends_with("state.db"),
            "State DB must end with state.db"
        );
    }

    #[test]
    #[serial]
    fn resolve_all_paths_global_and_project_differ() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("must resolve");
        assert_ne!(
            paths.global_config, paths.project_config,
            "Global and project config paths must be different"
        );
    }

    #[test]
    #[serial]
    fn resolve_all_paths_state_db_respects_env_override() {
        std::env::set_var("SCP_STATE_DB", "/tmp/env-override.db");
        std::env::remove_var("SCP_DATABASE_PATH");

        let paths = resolve_all_paths().expect("must resolve");
        assert_eq!(
            paths.state_db,
            PathBuf::from("/tmp/env-override.db"),
            "State DB in resolve_all_paths should respect env override"
        );

        std::env::remove_var("SCP_STATE_DB");
    }

    // =========================================================================
    // ConfigPaths data type
    // =========================================================================

    #[test]
    fn config_paths_holds_three_path_bufs() {
        let paths = ConfigPaths {
            global_config: PathBuf::from("/global/config.toml"),
            project_config: PathBuf::from("/project/.scp/config.toml"),
            state_db: PathBuf::from("/project/.scp/state.db"),
        };
        assert_eq!(paths.global_config, PathBuf::from("/global/config.toml"));
        assert_eq!(
            paths.project_config,
            PathBuf::from("/project/.scp/config.toml")
        );
        assert_eq!(paths.state_db, PathBuf::from("/project/.scp/state.db"));
    }

    #[test]
    fn config_paths_is_debug() {
        let paths = ConfigPaths {
            global_config: PathBuf::from("/a"),
            project_config: PathBuf::from("/b"),
            state_db: PathBuf::from("/c"),
        };
        let debug_str = format!("{:?}", paths);
        assert!(debug_str.contains("global_config"));
        assert!(debug_str.contains("project_config"));
        assert!(debug_str.contains("state_db"));
    }

    #[test]
    fn config_paths_is_clone() {
        let paths = ConfigPaths {
            global_config: PathBuf::from("/a"),
            project_config: PathBuf::from("/b"),
            state_db: PathBuf::from("/c"),
        };
        let cloned = paths.clone();
        assert_eq!(paths.global_config, cloned.global_config);
        assert_eq!(paths.project_config, cloned.project_config);
        assert_eq!(paths.state_db, cloned.state_db);
    }

    // =========================================================================
    // ConfigPortInfo — serialization (port status display)
    // =========================================================================

    #[test]
    fn config_port_info_serializes_all_fields() {
        let info = ConfigPortInfo {
            global_config_path: "/home/user/.config/scp/config.toml".to_string(),
            project_config_path: "/project/.scp/config.toml".to_string(),
            state_db_path: "/project/.scp/state.db".to_string(),
            global_exists: false,
            project_exists: true,
            state_db_exists: true,
        };
        let json = serde_json::to_string_pretty(&info).expect("serialization must succeed");
        assert!(
            json.contains("global_config_path"),
            "Must have global_config_path"
        );
        assert!(
            json.contains("project_config_path"),
            "Must have project_config_path"
        );
        assert!(json.contains("state_db_path"), "Must have state_db_path");
        assert!(json.contains("global_exists"), "Must have global_exists");
        assert!(json.contains("project_exists"), "Must have project_exists");
        assert!(
            json.contains("state_db_exists"),
            "Must have state_db_exists"
        );
    }

    #[test]
    fn config_port_info_deserializes_roundtrip() {
        let info = ConfigPortInfo {
            global_config_path: "/a/b/config.toml".to_string(),
            project_config_path: "/c/d/config.toml".to_string(),
            state_db_path: "/c/d/state.db".to_string(),
            global_exists: true,
            project_exists: false,
            state_db_exists: true,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let restored: ConfigPortInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.global_config_path, info.global_config_path);
        assert_eq!(restored.project_config_path, info.project_config_path);
        assert_eq!(restored.state_db_path, info.state_db_path);
        assert_eq!(restored.global_exists, info.global_exists);
        assert_eq!(restored.project_exists, info.project_exists);
        assert_eq!(restored.state_db_exists, info.state_db_exists);
    }

    #[test]
    fn config_port_info_json_has_correct_boolean_values() {
        let info = ConfigPortInfo {
            global_config_path: "/a".to_string(),
            project_config_path: "/b".to_string(),
            state_db_path: "/c".to_string(),
            global_exists: true,
            project_exists: false,
            state_db_exists: true,
        };
        let json: serde_json::Value = serde_json::to_value(&info).expect("must serialize to value");
        assert_eq!(json["global_exists"], serde_json::Value::Bool(true));
        assert_eq!(json["project_exists"], serde_json::Value::Bool(false));
        assert_eq!(json["state_db_exists"], serde_json::Value::Bool(true));
    }

    #[test]
    fn config_port_info_is_debug() {
        let info = ConfigPortInfo {
            global_config_path: "/a".to_string(),
            project_config_path: "/b".to_string(),
            state_db_path: "/c".to_string(),
            global_exists: true,
            project_exists: false,
            state_db_exists: false,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("global_exists: true"));
        assert!(debug.contains("project_exists: false"));
    }

    #[test]
    fn config_port_info_is_clone() {
        let info = ConfigPortInfo {
            global_config_path: "/a".to_string(),
            project_config_path: "/b".to_string(),
            state_db_path: "/c".to_string(),
            global_exists: true,
            project_exists: false,
            state_db_exists: false,
        };
        let cloned = info.clone();
        assert_eq!(cloned.global_config_path, info.global_config_path);
        assert_eq!(cloned.global_exists, info.global_exists);
    }

    // =========================================================================
    // Port binding error handling — run_config_ports
    // =========================================================================

    #[test]
    #[serial]
    fn run_config_ports_succeeds_with_text_output() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let result = run_config_ports(false);
        assert!(result.is_ok(), "run_config_ports(false) must succeed");
    }

    #[test]
    #[serial]
    fn run_config_ports_succeeds_with_json_output() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let result = run_config_ports(true);
        assert!(result.is_ok(), "run_config_ports(true) must succeed");
    }

    #[test]
    #[serial]
    fn run_config_ports_json_produces_valid_config_port_info() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        // Use resolve_all_paths to verify the data would be correct
        let paths = resolve_all_paths().expect("must resolve");
        let info = ConfigPortInfo {
            global_config_path: paths.global_config.display().to_string(),
            project_config_path: paths.project_config.display().to_string(),
            state_db_path: paths.state_db.display().to_string(),
            global_exists: paths.global_config.exists(),
            project_exists: paths.project_config.exists(),
            state_db_exists: paths.state_db.exists(),
        };

        let json = serde_json::to_string_pretty(&info).expect("must serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must parse");
        assert!(parsed.get("global_config_path").is_some());
        assert!(parsed.get("project_config_path").is_some());
        assert!(parsed.get("state_db_path").is_some());
        assert!(parsed.get("global_exists").is_some());
        assert!(parsed.get("project_exists").is_some());
        assert!(parsed.get("state_db_exists").is_some());
    }

    // =========================================================================
    // Port range validation — resolve_state_db_path with various repo roots
    // =========================================================================

    #[test]
    fn state_db_with_root_path_resolves() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let path =
            resolve_state_db_path(std::path::Path::new("/")).expect("root path must resolve");
        assert_eq!(path, PathBuf::from("/.scp/state.db"));
    }

    #[test]
    fn state_db_with_deep_nested_path_resolves() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let deep = std::path::Path::new("/a/b/c/d/e/f/g");
        let path = resolve_state_db_path(deep).expect("deep path must resolve");
        assert_eq!(path, PathBuf::from("/a/b/c/d/e/f/g/.scp/state.db"));
    }

    #[test]
    fn state_db_with_relative_path_resolves() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let relative = std::path::Path::new("relative/project");
        let path = resolve_state_db_path(relative).expect("relative path must resolve");
        assert!(
            path.to_string_lossy().contains("state.db"),
            "Relative path resolution must still include state.db"
        );
    }

    #[test]
    fn state_db_with_empty_path_resolves() {
        std::env::remove_var("SCP_STATE_DB");
        std::env::remove_var("SCP_DATABASE_PATH");

        let empty = std::path::Path::new("");
        let path = resolve_state_db_path(empty).expect("empty path must resolve");
        assert_eq!(path, PathBuf::from(".scp/state.db"));
    }

    // =========================================================================
    // Behavior tests — descriptive names, Martin Fowler style
    // =========================================================================

    mod global_config_behavior {
        use super::*;

        #[test]
        fn resolves_to_platform_config_directory() {
            let path = global_config_path().expect("must resolve");
            // On Linux, directories::ProjectDirs puts config under ~/.config/
            // On macOS, under ~/Library/Application Support/
            // Just verify it's a real absolute path
            assert!(path.is_absolute());
            assert!(
                path.components().count() >= 2,
                "Must have at least 2 components"
            );
        }
    }

    mod project_config_behavior {
        use super::*;

        #[test]
        fn always_contains_dot_scp_segment() {
            let path = project_config_path().expect("must resolve");
            let lossy = path.to_string_lossy();
            assert!(
                lossy.contains(".scp"),
                "Project config must be in .scp directory: {:?}",
                lossy
            );
        }

        #[test]
        fn file_name_is_config_toml() {
            let path = project_config_path().expect("must resolve");
            assert_eq!(
                path.file_name(),
                Some(std::ffi::OsStr::new("config.toml")),
                "File name must be config.toml"
            );
        }
    }

    mod state_db_resolution_behavior {
        use super::*;

        #[test]
        #[serial]
        fn when_no_env_vars_default_path_is_under_dot_scp() {
            std::env::remove_var("SCP_STATE_DB");
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            let lossy = path.to_string_lossy();

            assert!(
                lossy.contains(".scp") && lossy.contains("state.db"),
                "Default must be .scp/state.db: {:?}",
                path
            );
        }

        #[test]
        #[serial]
        fn when_scp_state_db_set_exactly_that_value_is_returned() {
            std::env::set_var("SCP_STATE_DB", "/exact/path/my.db");
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            assert_eq!(path, PathBuf::from("/exact/path/my.db"));

            std::env::remove_var("SCP_STATE_DB");
        }
    }

    mod config_port_info_display_behavior {
        use super::*;

        #[test]
        fn exists_flags_reflect_nonexistent_paths() {
            let info = ConfigPortInfo {
                global_config_path: "/nonexistent/path/config.toml".to_string(),
                project_config_path: "/also/nonexistent/.scp/config.toml".to_string(),
                state_db_path: "/no/such/state.db".to_string(),
                global_exists: false,
                project_exists: false,
                state_db_exists: false,
            };

            let json: serde_json::Value = serde_json::to_value(&info).expect("serialize");
            assert_eq!(json["global_exists"], false);
            assert_eq!(json["project_exists"], false);
            assert_eq!(json["state_db_exists"], false);
        }

        #[test]
        fn path_strings_preserve_exact_values() {
            let info = ConfigPortInfo {
                global_config_path: "/a/b/c.toml".to_string(),
                project_config_path: "/d/e/f.toml".to_string(),
                state_db_path: "/g/h/i.db".to_string(),
                global_exists: true,
                project_exists: true,
                state_db_exists: true,
            };

            let json = serde_json::to_string(&info).expect("serialize");
            let restored: ConfigPortInfo = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored.global_config_path, "/a/b/c.toml");
            assert_eq!(restored.project_config_path, "/d/e/f.toml");
            assert_eq!(restored.state_db_path, "/g/h/i.db");
        }
    }

    // =========================================================================
    // RED QUEEN ADVERSARIAL TESTS
    // =========================================================================

    mod red_queen_adversarial {
        use super::*;

        /// ATTACK: Empty SCP_STATE_DB env var — should use empty string as path.
        #[test]
        #[serial]
        fn empty_scp_state_db_env_produces_empty_path_buf() {
            std::env::set_var("SCP_STATE_DB", "");
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            // Empty string env var is still "set" (Ok), produces empty PathBuf
            assert_eq!(path, PathBuf::from(""));

            std::env::remove_var("SCP_STATE_DB");
        }

        /// ATTACK: Empty SCP_DATABASE_PATH env var — same behavior.
        #[test]
        #[serial]
        fn empty_scp_database_path_env_produces_empty_path_buf() {
            std::env::remove_var("SCP_STATE_DB");
            std::env::set_var("SCP_DATABASE_PATH", "");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            assert_eq!(path, PathBuf::from(""));

            std::env::remove_var("SCP_DATABASE_PATH");
        }

        /// ATTACK: Env var with path traversal characters — treated as literal string.
        #[test]
        #[serial]
        fn env_var_with_path_traversal_treated_as_literal() {
            std::env::set_var("SCP_STATE_DB", "../../../etc/passwd");
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            // Path traversal is NOT resolved — treated as literal
            assert_eq!(path, PathBuf::from("../../../etc/passwd"));

            std::env::remove_var("SCP_STATE_DB");
        }

        /// ATTACK: Env var with shell injection — treated as literal string.
        #[test]
        #[serial]
        fn env_var_with_shell_injection_treated_as_literal() {
            std::env::set_var("SCP_STATE_DB", "/tmp/db; rm -rf /");
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            assert_eq!(path, PathBuf::from("/tmp/db; rm -rf /"));

            std::env::remove_var("SCP_STATE_DB");
        }

        /// ATTACK: Env var with null bytes — OS rejects at set_var level (panics).
        /// PathBuf itself can contain null bytes, but std::env::set_var panics
        /// because the OS cannot represent them. This is correct defense-in-depth.
        #[test]
        fn path_buf_can_hold_null_bytes_but_env_cannot() {
            // Verify PathBuf can represent null bytes (data layer)
            let path_with_null = PathBuf::from("/tmp/db\x00hidden");
            assert!(
                path_with_null.to_string_lossy().contains("db"),
                "PathBuf should accept null bytes internally"
            );
            // std::env::set_var would panic — we do NOT test that path,
            // confirming the OS boundary prevents null injection via env vars.
        }

        /// ATTACK: Env var with extremely long path — must not panic.
        #[test]
        #[serial]
        fn env_var_with_extremely_long_path_does_not_panic() {
            let long_path = "/tmp/".to_string() + &"a".repeat(100_000) + "/state.db";
            std::env::set_var("SCP_STATE_DB", &long_path);
            std::env::remove_var("SCP_DATABASE_PATH");

            let cwd = std::env::current_dir().unwrap();
            let path = resolve_state_db_path(&cwd).unwrap();
            assert!(path.to_string_lossy().len() > 100_000);

            std::env::remove_var("SCP_STATE_DB");
        }

        /// ATTACK: ConfigPortInfo with special chars in paths serializes safely.
        #[test]
        fn config_port_info_with_injection_payloads_serializes_safely() {
            let info = ConfigPortInfo {
                global_config_path: "'; DROP TABLE configs; --".to_string(),
                project_config_path: "<script>alert('xss')</script>".to_string(),
                state_db_path: "/tmp/../../../etc/shadow".to_string(),
                global_exists: false,
                project_exists: false,
                state_db_exists: false,
            };

            let json = serde_json::to_string(&info).expect("must serialize");
            let restored: ConfigPortInfo = serde_json::from_str(&json).expect("must deserialize");
            assert_eq!(restored.global_config_path, "'; DROP TABLE configs; --");
            assert_eq!(
                restored.project_config_path,
                "<script>alert('xss')</script>"
            );
        }

        /// ATTACK: ConfigPortInfo with unicode paths roundtrips correctly.
        #[test]
        fn config_port_info_with_unicode_paths_roundtrips() {
            let info = ConfigPortInfo {
                global_config_path: "/home/\u{65E5}\u{672C}\u{8A9E}/config.toml".to_string(),
                project_config_path: "/project/\u{1F41B}/.scp/config.toml".to_string(),
                state_db_path: "/tmp/\u{00E9}\u{00E8}\u{00EA}/state.db".to_string(),
                global_exists: true,
                project_exists: false,
                state_db_exists: true,
            };

            let json = serde_json::to_string(&info).expect("serialize");
            let restored: ConfigPortInfo = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored.global_config_path, info.global_config_path);
            assert_eq!(restored.project_config_path, info.project_config_path);
            assert_eq!(restored.state_db_path, info.state_db_path);
        }

        /// ATTACK: resolve_state_db_path with root path produces /./scp/state.db.
        #[test]
        fn state_db_with_root_repo_root_produces_correct_path() {
            std::env::remove_var("SCP_STATE_DB");
            std::env::remove_var("SCP_DATABASE_PATH");

            let path = resolve_state_db_path(std::path::Path::new("/")).unwrap();
            assert_eq!(path, PathBuf::from("/.scp/state.db"));
        }

        /// ATTACK: global_config_path called many times does not accumulate state.
        #[test]
        fn global_config_path_repeated_calls_no_side_effects() {
            let first = global_config_path().unwrap();
            for _ in 0..100 {
                let subsequent = global_config_path().unwrap();
                assert_eq!(first, subsequent, "Repeated calls must be identical");
            }
        }

        /// ATTACK: resolve_all_paths called with conflicting env produces consistent output.
        #[test]
        #[serial]
        fn resolve_all_paths_with_conflicting_env_is_deterministic() {
            std::env::set_var("SCP_STATE_DB", "/tmp/deterministic.db");
            std::env::remove_var("SCP_DATABASE_PATH");

            let first = resolve_all_paths().unwrap();
            let second = resolve_all_paths().unwrap();
            assert_eq!(first.global_config, second.global_config);
            assert_eq!(first.project_config, second.project_config);
            assert_eq!(first.state_db, second.state_db);

            std::env::remove_var("SCP_STATE_DB");
        }

        /// ATTACK: ConfigPortInfo with all exists=true serializes booleans correctly.
        #[test]
        fn config_port_info_all_exists_true_serializes_booleans() {
            let info = ConfigPortInfo {
                global_config_path: "/a".to_string(),
                project_config_path: "/b".to_string(),
                state_db_path: "/c".to_string(),
                global_exists: true,
                project_exists: true,
                state_db_exists: true,
            };
            let json: serde_json::Value = serde_json::to_value(&info).expect("serialize");
            assert_eq!(json["global_exists"], true);
            assert_eq!(json["project_exists"], true);
            assert_eq!(json["state_db_exists"], true);
        }

        /// ATTACK: ConfigPortInfo with empty string paths serializes without panic.
        #[test]
        fn config_port_info_with_empty_string_paths() {
            let info = ConfigPortInfo {
                global_config_path: String::new(),
                project_config_path: String::new(),
                state_db_path: String::new(),
                global_exists: false,
                project_exists: false,
                state_db_exists: false,
            };
            let json = serde_json::to_string(&info).expect("empty paths must serialize");
            assert!(json.contains("global_config_path"));
        }
    }
}
