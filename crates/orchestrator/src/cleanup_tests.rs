//! Tests for cleanup and rollback handling

use crate::cleanup::{
    AgentDevelopmentCleanupHandler, CleanupContext, CleanupError, CleanupHandler, CleanupManager,
    CleanupResult, CleanupStatus, NoopCleanupHandler, PhaseType, ResourceId,
};
use crate::state::{PipelineId, PipelineState};

#[test]
fn test_cleanup_context() {
    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);

    assert_eq!(ctx.failed_phase, PhaseType::UniverseSetup);
    assert!(ctx.created_resources.is_empty());
    assert!(ctx.rollback_data.is_empty());
}

#[test]
fn test_cleanup_result() {
    let result = CleanupResult::success();
    assert!(result.success_flag());

    let result = CleanupResult::success()
        .with_resource(ResourceId::new("test"))
        .with_error("test error".to_string());

    assert!(!result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 1);
    assert_eq!(result.errors().len(), 1);
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
    assert!(result.success_flag());
}

#[test]
fn test_phase_type_from_state() {
    assert_eq!(
        PhaseType::from_state(PipelineState::SpecReview),
        Some(PhaseType::SpecReview)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::UniverseSetup),
        Some(PhaseType::UniverseSetup)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::AgentDevelopment),
        Some(PhaseType::AgentDevelopment)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::Validation),
        Some(PhaseType::Validation)
    );
    assert_eq!(PhaseType::from_state(PipelineState::Pending), None);
    assert_eq!(PhaseType::from_state(PipelineState::Accepted), None);
    assert_eq!(PhaseType::from_state(PipelineState::Failed), None);
    assert_eq!(PhaseType::from_state(PipelineState::Escalated), None);
}

// --- CleanupContext mutations ---

#[test]
fn test_cleanup_context_add_resource() {
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    ctx.add_resource(ResourceId::new("res-1"));
    ctx.add_resource(ResourceId::new("res-2"));
    assert_eq!(ctx.created_resources.len(), 2);
}

#[test]
fn test_cleanup_context_set_rollback_data() {
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    assert!(ctx.rollback_data.is_empty());
    ctx.set_rollback_data(vec![1, 2, 3]);
    assert_eq!(ctx.rollback_data, vec![1, 2, 3]);
}

// --- CleanupResult builder ---

#[test]
fn test_cleanup_result_multiple_resources() {
    let result = CleanupResult::success()
        .with_resource(ResourceId::new("r1"))
        .with_resource(ResourceId::new("r2"))
        .with_resource(ResourceId::new("r3"));

    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 3);
}

#[test]
fn test_cleanup_result_success_flag() {
    let result = CleanupResult::success();
    assert!(result.success_flag());

    let result = CleanupResult::success().with_error("fail".to_string());
    assert!(!result.success_flag());
}

#[test]
fn test_cleanup_result_errors_on_success() {
    let result = CleanupResult::success();
    assert!(result.errors().is_empty());
}

#[test]
fn test_cleanup_result_multiple_errors() {
    let result = CleanupResult::success()
        .with_error("error 1".to_string())
        .with_error("error 2".to_string());

    assert_eq!(result.errors().len(), 2);
    assert!(!result.success_flag());
}

// --- CleanupStatus ---

#[test]
fn test_cleanup_status_variants() {
    let success = CleanupStatus::Success;
    let failed = CleanupStatus::Failed(vec!["err".to_string()]);

    match success {
        CleanupStatus::Success => {}
        CleanupStatus::Failed(_) => panic!("expected Success"),
    }

    match failed {
        CleanupStatus::Success => panic!("expected Failed"),
        CleanupStatus::Failed(errs) => assert_eq!(errs.len(), 1),
    }
}

// --- CleanupError display ---

#[test]
fn test_cleanup_error_display() {
    let err = CleanupError::NotImplemented("validation".to_string());
    assert!(format!("{err}").contains("validation"));

    let err = CleanupError::ResourceNotFound("r-42".to_string());
    assert!(format!("{err}").contains("r-42"));

    let err = CleanupError::CleanupFailed("disk full".to_string());
    assert!(format!("{err}").contains("disk full"));

    let err = CleanupError::RollbackFailed("data corrupt".to_string());
    assert!(format!("{err}").contains("data corrupt"));
}

// --- CleanupHandler implementations ---

#[test]
fn test_noop_cleanup_handler() {
    let handler = NoopCleanupHandler;
    assert_eq!(handler.phase_type(), PhaseType::SpecReview);

    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::SpecReview);
    let cleanup_result = handler.cleanup(&ctx);
    assert!(cleanup_result.success_flag());

    let rollback_result = handler.rollback(&ctx);
    assert!(rollback_result.success_flag());
}

#[test]
fn test_agent_dev_cleanup_handler() {
    let handler = AgentDevelopmentCleanupHandler;
    assert_eq!(handler.phase_type(), PhaseType::AgentDevelopment);

    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);
    ctx.add_resource(ResourceId::new("workspace-1"));
    ctx.add_resource(ResourceId::new("workspace-2"));

    let result = handler.cleanup(&ctx);
    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 2);
}

#[test]
fn test_agent_dev_rollback_with_data() {
    let handler = AgentDevelopmentCleanupHandler;
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);
    ctx.set_rollback_data(vec![0xDE, 0xAD]);

    let result = handler.rollback(&ctx);
    assert!(!result.success_flag());
    assert_eq!(result.errors().len(), 1);
}

#[test]
fn test_agent_dev_rollback_without_data() {
    let handler = AgentDevelopmentCleanupHandler;
    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);

    let result = handler.rollback(&ctx);
    assert!(result.success_flag());
}

// --- CleanupManager operations ---

#[test]
fn test_cleanup_manager_get_all_handler_types() {
    let manager = CleanupManager::new();

    // All four phase types should have handlers
    for phase in [
        PhaseType::SpecReview,
        PhaseType::UniverseSetup,
        PhaseType::AgentDevelopment,
        PhaseType::Validation,
    ] {
        assert!(
            manager.get_handler(phase).is_some(),
            "Expected handler for {:?}",
            phase
        );
    }
}

#[test]
fn test_cleanup_manager_cleanup_with_resources() {
    let manager = CleanupManager::new();
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    ctx.add_resource(ResourceId::new("temp-dir"));

    let result = manager.cleanup(&ctx);
    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 1);
}

#[test]
fn test_cleanup_manager_rollback_with_data() {
    let manager = CleanupManager::new();
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    ctx.set_rollback_data(vec![1, 2, 3]);

    let result = manager.rollback(&ctx);
    assert!(!result.success_flag());
}

#[test]
fn test_cleanup_manager_rollback_without_data() {
    let manager = CleanupManager::new();
    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);

    let result = manager.rollback(&ctx);
    assert!(result.success_flag());
}

#[test]
fn test_cleanup_manager_register_handler() {
    let mut manager = CleanupManager::new();

    struct CustomHandler;
    impl crate::cleanup::CleanupHandler for CustomHandler {
        fn phase_type(&self) -> PhaseType {
            PhaseType::Validation
        }
        fn cleanup(&self, _context: &CleanupContext) -> CleanupResult {
            CleanupResult::success().with_resource(ResourceId::new("custom-cleaned"))
        }
        fn rollback(&self, _context: &CleanupContext) -> CleanupResult {
            CleanupResult::success()
        }
    }

    manager.register_handler(Box::new(CustomHandler));
    let handler = manager.get_handler(PhaseType::Validation);
    assert!(handler.is_some());

    let ctx = CleanupContext::new(PipelineId::new(), PhaseType::Validation);
    let result = handler.unwrap().cleanup(&ctx);
    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 1);
}

#[test]
fn test_cleanup_manager_default() {
    let manager = CleanupManager::default();
    assert!(manager.get_handler(PhaseType::SpecReview).is_some());
}

// --- ResourceId ---

#[test]
fn test_resource_id_new() {
    let rid = ResourceId::new("abc-123");
    assert_eq!(rid.0, "abc-123");
}

#[test]
fn test_resource_id_equality() {
    let a = ResourceId::new("same");
    let b = ResourceId::new("same");
    assert_eq!(a, b);
}

// --- Serde roundtrips ---

#[test]
fn test_phase_type_serde_roundtrip_all_variants() {
    use crate::cleanup::PhaseType;
    let phases = [
        PhaseType::SpecReview,
        PhaseType::UniverseSetup,
        PhaseType::AgentDevelopment,
        PhaseType::Validation,
    ];
    for phase in &phases {
        let json = serde_json::to_string(phase).expect("serialize");
        let deserialized: PhaseType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*phase, deserialized);
    }
}

#[test]
fn test_phase_type_serde_snake_case_all_variants() {
    assert_eq!(
        serde_json::to_string(&crate::cleanup::PhaseType::SpecReview).expect("serialize"),
        "\"spec_review\""
    );
    assert_eq!(
        serde_json::to_string(&crate::cleanup::PhaseType::UniverseSetup).expect("serialize"),
        "\"universe_setup\""
    );
    assert_eq!(
        serde_json::to_string(&crate::cleanup::PhaseType::AgentDevelopment).expect("serialize"),
        "\"agent_development\""
    );
    assert_eq!(
        serde_json::to_string(&crate::cleanup::PhaseType::Validation).expect("serialize"),
        "\"validation\""
    );
}

#[test]
fn test_phase_type_debug_format_all_variants() {
    assert_eq!(
        format!("{:?}", crate::cleanup::PhaseType::SpecReview),
        "SpecReview"
    );
    assert_eq!(
        format!("{:?}", crate::cleanup::PhaseType::UniverseSetup),
        "UniverseSetup"
    );
    assert_eq!(
        format!("{:?}", crate::cleanup::PhaseType::AgentDevelopment),
        "AgentDevelopment"
    );
    assert_eq!(
        format!("{:?}", crate::cleanup::PhaseType::Validation),
        "Validation"
    );
}

#[test]
fn test_phase_type_rejects_invalid_deserialization() {
    let result = serde_json::from_str::<crate::cleanup::PhaseType>("\"not_a_phase\"");
    assert!(result.is_err());
}

#[test]
fn test_phase_type_copy_and_clone() {
    let a = crate::cleanup::PhaseType::SpecReview;
    let b = a; // Copy semantics
    let c = a; // Still valid after copy (Copy trait)
    assert_eq!(a, b);
    assert_eq!(a, c);

    let d = crate::cleanup::PhaseType::Validation.clone();
    assert_eq!(d, crate::cleanup::PhaseType::Validation);
}

#[test]
fn test_phase_type_equality_and_hash() {
    use std::collections::HashSet;
    let all = [
        crate::cleanup::PhaseType::SpecReview,
        crate::cleanup::PhaseType::UniverseSetup,
        crate::cleanup::PhaseType::AgentDevelopment,
        crate::cleanup::PhaseType::Validation,
    ];
    // All variants are distinct
    let set: HashSet<_> = all.iter().copied().collect();
    assert_eq!(set.len(), 4);
    // Self-equality
    for phase in &all {
        assert_eq!(*phase, *phase);
    }
}

#[test]
fn test_resource_id_serde_roundtrip() {
    let rid = ResourceId::new("res-xyz");
    let json = serde_json::to_string(&rid).expect("serialize");
    let deserialized: ResourceId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(rid, deserialized);
}

#[test]
fn test_cleanup_context_serde_roundtrip() {
    let mut ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::UniverseSetup,
    );
    ctx.add_resource(ResourceId::new("r1"));
    ctx.set_rollback_data(vec![1, 2, 3]);
    let json = serde_json::to_string(&ctx).expect("serialize");
    let deserialized: CleanupContext = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(ctx.failed_phase, deserialized.failed_phase);
    assert_eq!(
        ctx.created_resources.len(),
        deserialized.created_resources.len()
    );
    assert_eq!(ctx.rollback_data, deserialized.rollback_data);
}

#[test]
fn test_cleanup_status_serde_roundtrip() {
    let success = CleanupStatus::Success;
    let json = serde_json::to_string(&success).expect("serialize");
    assert_eq!(json, "\"success\"");

    let failed = CleanupStatus::Failed(vec!["err1".to_string(), "err2".to_string()]);
    let json = serde_json::to_string(&failed).expect("serialize");
    let deserialized: CleanupStatus = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(failed, deserialized);
}

#[test]
fn test_cleanup_result_serde_roundtrip() {
    let result = CleanupResult::success()
        .with_resource(ResourceId::new("r1"))
        .with_resource(ResourceId::new("r2"));
    let json = serde_json::to_string(&result).expect("serialize");
    let deserialized: CleanupResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(result.success_flag(), deserialized.success_flag());
    assert_eq!(
        result.cleaned_resources.len(),
        deserialized.cleaned_resources.len()
    );
}

// --- PhaseType from_state for all states ---

#[test]
fn test_phase_type_from_state_exhaustive() {
    use crate::cleanup::PhaseType;
    use crate::state::PipelineState;

    assert_eq!(PhaseType::from_state(PipelineState::Pending), None);
    assert_eq!(
        PhaseType::from_state(PipelineState::SpecReview),
        Some(PhaseType::SpecReview)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::UniverseSetup),
        Some(PhaseType::UniverseSetup)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::AgentDevelopment),
        Some(PhaseType::AgentDevelopment)
    );
    assert_eq!(
        PhaseType::from_state(PipelineState::Validation),
        Some(PhaseType::Validation)
    );
    assert_eq!(PhaseType::from_state(PipelineState::Accepted), None);
    assert_eq!(PhaseType::from_state(PipelineState::Escalated), None);
    assert_eq!(PhaseType::from_state(PipelineState::Failed), None);
}

// --- CleanupError variants ---

#[test]
fn test_cleanup_error_variants_display() {
    let errors = [
        CleanupError::NotImplemented("phase_x".to_string()),
        CleanupError::ResourceNotFound("r-999".to_string()),
        CleanupError::CleanupFailed("disk full".to_string()),
        CleanupError::RollbackFailed("data corrupt".to_string()),
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty());
    }
}

// --- CleanupResult builder edge cases ---

#[test]
fn test_cleanup_result_with_error_after_success_transitions_to_failed() {
    let result = CleanupResult::success()
        .with_resource(ResourceId::new("r1"))
        .with_error("err".to_string())
        .with_resource(ResourceId::new("r2")); // Can still add resources after error

    assert!(!result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 2);
    assert_eq!(result.errors().len(), 1);
}

#[test]
fn test_cleanup_result_multiple_errors_accumulate() {
    let result = CleanupResult::success()
        .with_error("err1".to_string())
        .with_error("err2".to_string())
        .with_error("err3".to_string());

    assert!(!result.success_flag());
    assert_eq!(result.errors().len(), 3);
    assert_eq!(result.errors()[0], "err1");
    assert_eq!(result.errors()[2], "err3");
}

// --- CleanupManager register and replace handler ---

#[test]
fn test_cleanup_manager_register_replaces_existing_handler() {
    let mut manager = CleanupManager::new();

    struct HandlerV1;
    impl CleanupHandler for HandlerV1 {
        fn phase_type(&self) -> crate::cleanup::PhaseType {
            crate::cleanup::PhaseType::SpecReview
        }
        fn cleanup(&self, _ctx: &CleanupContext) -> CleanupResult {
            CleanupResult::success()
        }
        fn rollback(&self, _ctx: &CleanupContext) -> CleanupResult {
            CleanupResult::success()
        }
    }

    struct HandlerV2;
    impl CleanupHandler for HandlerV2 {
        fn phase_type(&self) -> crate::cleanup::PhaseType {
            crate::cleanup::PhaseType::SpecReview
        }
        fn cleanup(&self, _ctx: &CleanupContext) -> CleanupResult {
            CleanupResult::success().with_resource(ResourceId::new("v2-cleaned"))
        }
        fn rollback(&self, _ctx: &CleanupContext) -> CleanupResult {
            CleanupResult::success()
        }
    }

    manager.register_handler(Box::new(HandlerV1));
    manager.register_handler(Box::new(HandlerV2)); // Should replace V1

    let handler = manager
        .get_handler(crate::cleanup::PhaseType::SpecReview)
        .expect("handler");
    let result = handler.cleanup(&CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::SpecReview,
    ));
    assert_eq!(result.cleaned_resources.len(), 1);
    assert_eq!(result.cleaned_resources[0].0, "v2-cleaned");
}

// --- CleanupManager cleanup for unknown phase ---

#[test]
fn test_cleanup_manager_cleanup_unknown_phase_succeeds() {
    // The CleanupManager handles unknown phases gracefully (returns success)
    let manager = CleanupManager::new();
    let ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::Validation,
    );
    let result = manager.cleanup(&ctx);
    // Validation uses NoopCleanupHandler by default
    assert!(result.success_flag());
}

// --- ResourceId inequality ---

#[test]
fn test_resource_id_inequality() {
    let a = ResourceId::new("alpha");
    let b = ResourceId::new("beta");
    assert_ne!(a, b);
}

// --- ResourceId can be used in HashSet ---

#[test]
fn test_resource_id_in_hashset() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ResourceId::new("r1"));
    set.insert(ResourceId::new("r1")); // duplicate
    set.insert(ResourceId::new("r2"));
    assert_eq!(set.len(), 2);
}

// --- UniverseSetupCleanupHandler ---

#[test]
fn test_universe_setup_cleanup_handler_with_resources() {
    let handler = crate::cleanup::UniverseSetupCleanupHandler;
    let mut ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::UniverseSetup,
    );
    ctx.add_resource(ResourceId::new("dir-1"));
    ctx.add_resource(ResourceId::new("dir-2"));
    ctx.add_resource(ResourceId::new("dir-3"));

    let result = handler.cleanup(&ctx);
    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 3);
}

#[test]
fn test_universe_setup_cleanup_handler_no_resources() {
    let handler = crate::cleanup::UniverseSetupCleanupHandler;
    let ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::UniverseSetup,
    );

    let result = handler.cleanup(&ctx);
    assert!(result.success_flag());
    assert!(result.cleaned_resources.is_empty());
}

#[test]
fn test_universe_setup_rollback_with_data() {
    let handler = crate::cleanup::UniverseSetupCleanupHandler;
    let mut ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::UniverseSetup,
    );
    ctx.set_rollback_data(vec![1, 2, 3]);

    let result = handler.rollback(&ctx);
    assert!(!result.success_flag());
    assert_eq!(result.errors().len(), 1);
}

#[test]
fn test_universe_setup_rollback_without_data() {
    let handler = crate::cleanup::UniverseSetupCleanupHandler;
    let ctx = CleanupContext::new(
        PipelineId("test".to_string()),
        crate::cleanup::PhaseType::UniverseSetup,
    );

    let result = handler.rollback(&ctx);
    assert!(result.success_flag());
}

// --- Full cleanup lifecycle integration ---

#[test]
fn test_cleanup_lifecycle_full_integration() {
    let manager = CleanupManager::new();
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);

    ctx.add_resource(ResourceId::new("resource-1"));
    ctx.add_resource(ResourceId::new("resource-2"));
    ctx.add_resource(ResourceId::new("resource-3"));

    let result = manager.cleanup(&ctx);
    assert!(result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 3);
    assert_eq!(result.cleaned_resources[0].0, "resource-1");
    assert_eq!(result.cleaned_resources[1].0, "resource-2");
    assert_eq!(result.cleaned_resources[2].0, "resource-3");
}

#[test]
fn test_cleanup_lifecycle_rollback_with_error_aggregation() {
    let manager = CleanupManager::new();
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    ctx.set_rollback_data(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let result = manager.rollback(&ctx);
    assert!(!result.success_flag());
    let errors = result.errors();
    assert!(!errors.is_empty());
    assert!(errors[0].contains("not implemented"));
}

#[test]
fn test_cleanup_manager_orchestration_multiple_phases() {
    let manager = CleanupManager::new();

    let universe_ctx = {
        let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
        ctx.add_resource(ResourceId::new("universe-res"));
        ctx
    };

    let agent_ctx = {
        let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);
        ctx.add_resource(ResourceId::new("agent-res-1"));
        ctx.add_resource(ResourceId::new("agent-res-2"));
        ctx
    };

    let universe_result = manager.cleanup(&universe_ctx);
    assert!(universe_result.success_flag());
    assert_eq!(universe_result.cleaned_resources.len(), 1);

    let agent_result = manager.cleanup(&agent_ctx);
    assert!(agent_result.success_flag());
    assert_eq!(agent_result.cleaned_resources.len(), 2);

    let spec_result = manager.cleanup(&CleanupContext::new(
        PipelineId::new(),
        PhaseType::SpecReview,
    ));
    assert!(spec_result.success_flag());
    assert!(spec_result.cleaned_resources.is_empty());
}

#[test]
fn test_cleanup_ordering_preserved_through_handler() {
    let manager = CleanupManager::new();
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);

    let resource_names = ["alpha", "beta", "gamma", "delta"];
    for name in resource_names {
        ctx.add_resource(ResourceId::new(name));
    }

    let result = manager.cleanup(&ctx);
    assert!(result.success_flag());

    let cleaned: Vec<&str> = result
        .cleaned_resources
        .iter()
        .map(|r| r.0.as_str())
        .collect();
    assert_eq!(cleaned, resource_names);
}

#[test]
fn test_cleanup_error_aggregation_across_operations() {
    let result = CleanupResult::success()
        .with_resource(ResourceId::new("r1"))
        .with_error("error-alpha".to_string())
        .with_resource(ResourceId::new("r2"))
        .with_error("error-beta".to_string())
        .with_resource(ResourceId::new("r3"));

    assert!(!result.success_flag());
    assert_eq!(result.cleaned_resources.len(), 3);
    assert_eq!(result.errors().len(), 2);
    assert_eq!(result.errors()[0], "error-alpha");
    assert_eq!(result.errors()[1], "error-beta");
}

#[test]
fn test_cleanup_context_clone_preserves_ordering() {
    let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
    ctx.add_resource(ResourceId::new("first"));
    ctx.add_resource(ResourceId::new("second"));
    ctx.set_rollback_data(vec![1, 2, 3]);

    let cloned = ctx.clone();
    assert_eq!(cloned.failed_phase, ctx.failed_phase);
    assert_eq!(cloned.pipeline_id.0, ctx.pipeline_id.0);
    assert_eq!(cloned.created_resources.len(), 2);
    assert_eq!(cloned.created_resources[0].0, "first");
    assert_eq!(cloned.created_resources[1].0, "second");
    assert_eq!(cloned.rollback_data, vec![1, 2, 3]);
}

#[test]
fn test_cleanup_manager_default_is_initialized_with_handlers() {
    let manager = CleanupManager::default();
    assert!(manager.get_handler(PhaseType::SpecReview).is_some());
    assert!(manager.get_handler(PhaseType::UniverseSetup).is_some());
    assert!(manager.get_handler(PhaseType::AgentDevelopment).is_some());
    assert!(manager.get_handler(PhaseType::Validation).is_some());
}
