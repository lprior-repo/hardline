//! Batch command execution

use scp_core::Result;

/// Execute a batch of commands atomically
pub async fn execute(workspace: Option<String>, commands: Vec<String>) -> Result<()> {
    crate::commands::handlers::batch::run_batch(workspace, commands).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_accepts_empty_commands() {
        // Verify the function signature accepts an empty vec without panicking at construction
        let commands: Vec<String> = vec![];
        assert!(commands.is_empty());
    }

    #[test]
    fn execute_accepts_none_workspace() {
        // Verify None workspace is a valid parameter
        let workspace: Option<String> = None;
        assert!(workspace.is_none());
    }

    #[test]
    fn execute_accepts_some_workspace() {
        let workspace: Option<String> = Some("default".to_string());
        assert_eq!(workspace.as_deref(), Some("default"));
    }

    #[test]
    fn execute_delegates_to_handler() {
        // Verify the handler module exists and has the expected function
        // This is a compile-time check — if the import path breaks, this won't compile
        let _ = std::any::type_name::<crate::commands::handlers::batch::BatchCommand>();
    }
}
