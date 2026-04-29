//! Action functions for the retry command handler (Tier 3).
//!
//! I/O operations that orchestrate retrying the last failed VCS operation.

use scp_core::Result;

use super::data::{RetryOptions, RetryOutput};

// ============================================================================
// Public API
// ============================================================================

/// Retry the last failed VCS operation.
///
/// # Errors
///
/// Returns errors for VCS operation failures after all retry attempts
/// are exhausted, or for underlying I/O errors.
pub fn run_retry(_opts: RetryOptions) -> Result<RetryOutput> {
    // For now, return a stub that reports no operation to retry.
    // The full implementation needs VCS operation logging (Phase 5).
    Ok(RetryOutput {
        success: false,
        attempts: 0,
        message: "No failed operation to retry".to_string(),
    })
}
