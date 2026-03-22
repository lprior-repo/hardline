//! Backend type definition
//!
//! This module provides `BackendType` - enumeration distinguishing Git vs JJ repositories.

use serde::{Deserialize, Serialize};

/// Version control system backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    /// Git repository (contains .git directory)
    Git,
    /// Jujutsu repository (contains .jj directory)
    Jj,
}
