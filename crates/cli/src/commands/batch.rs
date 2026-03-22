//! Batch command execution

use scp_core::Result;

/// Execute a batch of commands atomically
pub async fn execute(workspace: Option<String>, commands: Vec<String>) -> Result<()> {
    crate::commands::handlers::batch::run_batch(workspace, commands).await
}
