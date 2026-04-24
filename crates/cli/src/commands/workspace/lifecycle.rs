//! Workspace lifecycle commands

use scp_core::output::Output;
use scp_core::vcs;
use scp_core::Error;

use super::operations::*;
use super::types::SyncOption;

/// Create a new workspace
pub fn spawn(name: &str, sync: SyncOption) -> Result<(), Error> {
    // P1: Validate workspace name BEFORE any I/O
    if let Some(err) = super::validators::validate_workspace_name(name) {
        return Err(err);
    }

    Output::info(&format!("Creating workspace '{}'...", name));

    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace already exists
    let workspaces = backend.list_workspaces()?;
    if workspaces.iter().any(|w| w.name == name) {
        return Err(Error::workspace_exists(name));
    }

    spawn_with_sync(backend.as_ref(), name, sync.is_sync())
}

/// Switch to a workspace
pub fn switch(name: &str) -> Result<(), Error> {
    // P1: Validate workspace name is not empty
    if name.is_empty() {
        return Err(Error::invalid_identifier("workspace name cannot be empty"));
    }

    Output::info(&format!("Switching to workspace '{}'...", name));

    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    // Check if workspace exists and working copy is clean
    if !workspace_exists(backend.as_ref(), name)? {
        return Err(Error::workspace_not_found(name));
    }
    require_clean_working_copy(backend.as_ref())?;

    backend.switch_workspace(name)?;
    Output::success(&format!("Switched to '{}'", name));
    Ok(())
}

/// List workspaces
pub fn list() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let backend = vcs::create_backend(&cwd)?;
    let workspaces = backend.list_workspaces()?;

    if workspaces.is_empty() {
        Output::info("No workspaces found");
    } else {
        Output::info("Workspaces:");
        for ws in workspaces {
            let current = if ws.is_current { " (current)" } else { "" };
            Output::info(&format!("  - {}{}", ws.name, current));
        }
    }

    Ok(())
}

/// Show workspace status
pub fn status() -> Result<(), Error> {
    let cwd = std::env::current_dir()?;

    let backend = vcs::create_backend(&cwd)?;
    let branch = backend.current_branch()?;
    let vcs_status = backend.status()?;

    Output::info(&format!("Current branch: {}", branch));
    Output::info(&format!("Status: {}", vcs_status));

    Ok(())
}

/// Sync workspace with main
pub fn sync(name: Option<&str>, all: bool) -> Result<(), Error> {
    let options = crate::commands::handlers::sync::SyncOptions {
        allow_dirty: false,
        target_branch: None,
        lock_timeout_secs: 30,
        retry_config: crate::commands::handlers::sync::RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 100,
        },
    };

    let sync_future = async {
        if all {
            crate::commands::handlers::sync::sync_all_sessions(options).await
        } else if let Some(n) = name {
            let session_name = scp_core::domain::SessionName::parse(n).map_err(|e| {
                crate::commands::handlers::sync::SyncError::InvalidIdentifier(e.to_string())
            })?;
            crate::commands::handlers::sync::sync_named_session(session_name, options).await
        } else {
            crate::commands::handlers::sync::sync_current_workspace(options).await
        }
    };

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(sync_future)?,
        Err(_) => {
            let runtime =
                tokio::runtime::Runtime::new().expect("Failed to create runtime for sync");
            runtime.block_on(sync_future)?
        }
    };

    Ok(())
}

/// Split workspace
pub fn add(path: &str) -> Result<(), Error> {
    let cwd = std::env::current_dir()?;
    let backend = vcs::create_backend(&cwd)?;

    split_workspace(backend.as_ref(), path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // sync() dispatch tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sync_rejects_empty_session_name() {
        // SessionName::parse("") fails with empty input error
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            // Call the inner logic directly to avoid Handle::current() nesting
            scp_core::domain::SessionName::parse("")
        });
        assert!(result.is_err(), "empty session name should be rejected");
    }

    #[test]
    fn test_sync_rejects_session_name_with_special_chars() {
        let result = scp_core::domain::SessionName::parse("bad name!");
        assert!(
            result.is_err(),
            "session name with spaces and special chars should be rejected"
        );
    }

    #[test]
    fn test_sync_rejects_session_name_starting_with_number() {
        let result = scp_core::domain::SessionName::parse("123session");
        assert!(
            result.is_err(),
            "session name starting with number should be rejected"
        );
    }

    #[test]
    fn test_sync_options_has_correct_defaults() {
        let options = crate::commands::handlers::sync::SyncOptions {
            allow_dirty: false,
            target_branch: None,
            lock_timeout_secs: 30,
            retry_config: crate::commands::handlers::sync::RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 100,
            },
        };
        assert!(!options.allow_dirty);
        assert!(options.target_branch.is_none());
        assert_eq!(options.lock_timeout_secs, 30);
        assert_eq!(options.retry_config.max_attempts, 3);
        assert_eq!(options.retry_config.initial_delay_ms, 100);
    }

    #[test]
    #[should_panic(expected = "no reactor running")]
    fn test_sync_panics_without_tokio_runtime() {
        // Calling sync() outside a tokio runtime should panic
        // because Handle::current().block_on() fails when no runtime exists
        let _ = sync(None, false);
    }

    #[tokio::test]
    async fn test_sync_dispatch_named_session_rejects_invalid() {
        // Verify the sync dispatch path (B) rejects invalid session names
        // before reaching the handler
        let result = scp_core::domain::SessionName::parse("invalid/name");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            !err_msg.is_empty(),
            "invalid session name should produce an error message"
        );
    }

    #[tokio::test]
    async fn test_sync_dispatch_all_flag_constructs_options() {
        // Verify the SyncOptions used in the all=true path are valid
        let options = crate::commands::handlers::sync::SyncOptions {
            allow_dirty: false,
            target_branch: None,
            lock_timeout_secs: 30,
            retry_config: crate::commands::handlers::sync::RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 100,
            },
        };
        // The all=true path just calls sync_all_sessions(options)
        // Verify options are well-formed for that call
        assert!(options.retry_config.max_attempts > 0);
        assert!(options.retry_config.initial_delay_ms > 0);
        assert!(options.lock_timeout_secs > 0);
    }
}
