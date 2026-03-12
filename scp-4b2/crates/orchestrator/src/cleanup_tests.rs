//! Tests for cleanup and rollback handling

use crate::cleanup::{CleanupContext, CleanupManager, CleanupResult, PhaseType, ResourceId};
use crate::state::PipelineId;

#[test]
fn test_cleanup_context() {
    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);

    assert_eq!(ctx.failed_phase, PhaseType::UniverseSetup);
    assert!(ctx.created_resources.is_empty());
}

#[test]
fn test_cleanup_result() {
    let result = CleanupResult::success();
    assert!(result.success);

    let result = CleanupResult::success()
        .with_resource(ResourceId::new("test"))
        .with_error("test error".to_string());

    assert!(!result.success);
    assert_eq!(result.cleaned_resources.len(), 1);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_cleanup_manager() {
    let manager = CleanupManager::new();

    // Test that we can get handlers
    let handler = manager.get_handler(PhaseType::UniverseSetup);
    assert!(handler.is_some());

    // Test cleanup
    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    let result = manager.cleanup(&ctx);
    assert!(result.success);
}

#[test]
fn test_phase_type_from_state() {
    use crate::state::PipelineState;

    assert_eq!(
        PhaseType::from_state(PipelineState::SpecReview),
        Some(PhaseType::SpecReview)
    );
    assert_eq!(PhaseType::from_state(PipelineState::Pending), None);
}
