//! Application layer - Use cases and business logic orchestration

pub mod commands;
pub mod hooks;
pub mod repositories;
pub mod services;

// Re-export commonly used types
pub use commands::*;
pub use hooks::{
    HookContext, HookOutcome, NoOpWorktreeHooks, ShellWorktreeHooks, WorktreeHookEvent,
    WorktreeHooks,
};
pub use repositories::WorktreeRepository;
pub use services::WorktreeService;
