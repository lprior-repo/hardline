//! Domain layer - Business logic and entities

pub mod absolute_path;
pub mod branch_name;
pub mod errors;
pub mod worktree;
pub mod worktree_id;
pub mod worktree_name;
pub mod worktree_state;
pub mod worktree_type_enum;

// Re-export commonly used types
pub use absolute_path::AbsolutePath;
pub use branch_name::BranchName;
pub use errors::WorktreeDomainError;
pub use worktree::Worktree;
pub use worktree_id::WorktreeId;
pub use worktree_name::WorktreeName;
pub use worktree_state::WorktreeState;
pub use worktree_type_enum::WorktreeTypeEnum;
