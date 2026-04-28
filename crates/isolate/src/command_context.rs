//! Command context for tracking command execution with unique IDs.
//!
//! Provides task-local state for generating unique command IDs during execution.

use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::task_local;

#[derive(Debug, Clone)]
struct CommandState {
    base_id: String,
    sequence: u64,
}

impl CommandState {
    fn next_id(&mut self, action: &str, target: &str) -> String {
        self.sequence = self.sequence.saturating_add(1);
        format!("{}:{}:{}:{}", self.base_id, self.sequence, action, target)
    }
}

task_local! {
    static COMMAND_STATE: std::cell::RefCell<CommandState>;
}

static COMMAND_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Resolve a base command ID from an explicit value or generate a default.
#[must_use]
pub fn resolve_base_command_id(explicit: Option<&str>) -> String {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(default_base_command_id, std::string::ToString::to_string)
}

fn default_base_command_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());
    let pid = std::process::id();
    let counter = COMMAND_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{pid}-{now}-{counter}")
}

/// Execute a future with a command context.
pub async fn with_command_context<T, F>(base_id: String, future: F) -> T
where
    F: Future<Output = T>,
{
    COMMAND_STATE
        .scope(
            std::cell::RefCell::new(CommandState {
                base_id,
                sequence: 0,
            }),
            future,
        )
        .await
}

/// Generate the next write command ID if within a command context.
#[must_use]
pub fn next_write_command_id(action: &str, target: &str) -> Option<String> {
    COMMAND_STATE
        .try_with(|state| state.borrow_mut().next_id(action, target))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_base_command_id_explicit() {
        let id = resolve_base_command_id(Some("my-command"));
        assert_eq!(id, "my-command");
    }

    #[test]
    fn test_resolve_base_command_id_empty() {
        let id = resolve_base_command_id(Some(""));
        assert!(!id.is_empty());
        assert!(id.starts_with("cmd-"));
    }

    #[test]
    fn test_resolve_base_command_id_whitespace() {
        let id = resolve_base_command_id(Some("  "));
        assert!(!id.is_empty());
        assert!(id.starts_with("cmd-"));
    }

    #[test]
    fn test_resolve_base_command_id_none() {
        let id = resolve_base_command_id(None);
        assert!(!id.is_empty());
        assert!(id.starts_with("cmd-"));
    }

    #[tokio::test]
    async fn test_with_command_context() {
        let result = with_command_context("test-cmd".to_string(), async {
            next_write_command_id("action", "target")
        })
        .await;
        assert!(result.is_some());
        let id = result.unwrap();
        assert!(id.contains("test-cmd"));
        assert!(id.contains("action"));
        assert!(id.contains("target"));
    }

    #[tokio::test]
    async fn test_next_write_command_id_outside_context() {
        let result = next_write_command_id("action", "target");
        assert!(result.is_none());
    }
}
