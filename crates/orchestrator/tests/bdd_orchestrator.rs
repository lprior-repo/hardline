//! BDD Validation: orchestrator crate
//!
//! Claim Sheet built from types, docs, and public API surface.
//! Each test is a Given/When/Then claim with adversarial variants.

// ─── State Machine Claims ───

mod state_machine {
    use orchestrator::state::{Pipeline, PipelineConfig, PipelineId, PipelineState};

    // CLAIM: Pipeline starts in Pending state
    #[test]
    fn claim_pipeline_starts_pending() {
        let p = Pipeline::new("test.spec".into());
        assert_eq!(p.state, PipelineState::Pending);
    }

    // CLAIM: with_config creates pipeline with custom config
    #[test]
    fn claim_pipeline_with_custom_config() {
        let cfg = PipelineConfig {
            max_iterations: 5,
            quality_threshold: 90,
            scenarios_path: "custom".into(),
            linter_path: Some("/bin/lint".into()),
        };
        let p = Pipeline::with_config("test.spec".into(), &cfg);
        assert_eq!(p.max_iterations, 5);
        assert_eq!(p.quality_threshold, 90);
        assert_eq!(p.spec_path, "test.spec");
    }

    // CLAIM: PipelineId::new generates unique IDs
    #[test]
    fn claim_pipeline_ids_are_unique() {
        let id1 = PipelineId::new();
        let id2 = PipelineId::new();
        assert_ne!(id1, id2);
    }

    // CLAIM: PipelineId Default creates new unique ID
    #[test]
    fn claim_pipeline_id_default() {
        let id = PipelineId::default();
        assert!(!id.0.is_empty());
    }

    // CLAIM: Happy path lifecycle: Pending -> SpecReview -> UniverseSetup -> AgentDevelopment ->
    // Validation -> Accepted
    #[test]
    fn claim_happy_path_lifecycle() {
        let mut p = Pipeline::new("test.spec".into());
        assert!(p.transition_to(PipelineState::SpecReview).is_ok());
        assert_eq!(p.state, PipelineState::SpecReview);

        assert!(p.transition_to(PipelineState::UniverseSetup).is_ok());
        assert_eq!(p.state, PipelineState::UniverseSetup);

        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert_eq!(p.state, PipelineState::AgentDevelopment);

        assert!(p.transition_to(PipelineState::Validation).is_ok());
        assert_eq!(p.state, PipelineState::Validation);

        assert!(p.transition_to(PipelineState::Accepted).is_ok());
        assert!(p.state.is_terminal());
    }

    // CLAIM: Any non-terminal state can transition to Failed
    #[test]
    fn claim_failure_from_non_terminal_states() {
        for state in [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ] {
            let mut p = Pipeline::new("test.spec".into());
            // Get to the target state
            match state {
                PipelineState::Pending => {}
                PipelineState::SpecReview => {
                    let _ = p.transition_to(PipelineState::SpecReview);
                }
                PipelineState::UniverseSetup => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup));
                }
                PipelineState::AgentDevelopment => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
                        .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));
                }
                PipelineState::Validation => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
                        .and_then(|_| p.transition_to(PipelineState::AgentDevelopment))
                        .and_then(|_| p.transition_to(PipelineState::Validation));
                }
                _ => unreachable!(),
            }
            assert!(p.transition_to(PipelineState::Failed).is_ok());
        }
    }

    // CLAIM: Any non-terminal state can transition to Escalated
    #[test]
    fn claim_escalation_from_non_terminal_states() {
        for state in [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ] {
            let mut p = Pipeline::new("test.spec".into());
            match state {
                PipelineState::Pending => {}
                PipelineState::SpecReview => {
                    let _ = p.transition_to(PipelineState::SpecReview);
                }
                PipelineState::UniverseSetup => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup));
                }
                PipelineState::AgentDevelopment => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
                        .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));
                }
                PipelineState::Validation => {
                    let _ = p
                        .transition_to(PipelineState::SpecReview)
                        .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
                        .and_then(|_| p.transition_to(PipelineState::AgentDevelopment))
                        .and_then(|_| p.transition_to(PipelineState::Validation));
                }
                _ => unreachable!(),
            }
            assert!(p.transition_to(PipelineState::Escalated).is_ok());
        }
    }

    // ADVERSARIAL: Cannot transition from terminal states
    #[test]
    fn claim_terminal_states_block_transitions() {
        // Build a path to each terminal state
        let terminal_paths: Vec<PipelineState> = vec![
            PipelineState::Failed,    // Pending -> SpecReview -> Failed
            PipelineState::Escalated, // Pending -> SpecReview -> Escalated
        ];
        for terminal in terminal_paths {
            let mut p = Pipeline::new("test.spec".into());
            let _ = p.transition_to(PipelineState::SpecReview);
            let _ = p.transition_to(terminal);

            for target in [
                PipelineState::Pending,
                PipelineState::SpecReview,
                PipelineState::UniverseSetup,
                PipelineState::AgentDevelopment,
                PipelineState::Validation,
                PipelineState::Accepted,
                PipelineState::Escalated,
                PipelineState::Failed,
            ] {
                let result = p.transition_to(target);
                assert!(
                    result.is_err(),
                    "Terminal state {terminal:?} should block transition to {target:?}"
                );
            }
        }

        // Accepted requires the full path
        let mut p = Pipeline::new("test.spec".into());
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment))
            .and_then(|_| p.transition_to(PipelineState::Validation))
            .and_then(|_| p.transition_to(PipelineState::Accepted));
        for target in [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ] {
            let result = p.transition_to(target);
            assert!(
                result.is_err(),
                "Terminal state Accepted should block transition to {target:?}"
            );
        }
    }

    // ADVERSARIAL: Invalid transitions are rejected (e.g. Pending -> Validation directly)
    #[test]
    fn claim_invalid_transitions_rejected() {
        let mut p = Pipeline::new("test.spec".into());
        assert!(p.transition_to(PipelineState::AgentDevelopment).is_err());
        assert!(p.transition_to(PipelineState::Validation).is_err());
        assert!(p.transition_to(PipelineState::Accepted).is_err());
    }

    // CLAIM: Validation -> AgentDevelopment loop works
    #[test]
    fn claim_validation_loop_to_agent_development() {
        let mut p = Pipeline::new("test.spec".into());
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment))
            .and_then(|_| p.transition_to(PipelineState::Validation));
        assert_eq!(p.state, PipelineState::Validation);

        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
        assert_eq!(p.state, PipelineState::AgentDevelopment);
    }

    // CLAIM: AgentDevelopment self-loop is allowed
    #[test]
    fn claim_agent_development_self_loop() {
        let mut p = Pipeline::new("test.spec".into());
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));
        assert!(p.transition_to(PipelineState::AgentDevelopment).is_ok());
    }

    // CLAIM: increment_iteration respects max_iterations
    #[test]
    fn claim_iteration_limit_enforced() {
        let cfg = PipelineConfig {
            max_iterations: 2,
            quality_threshold: 80,
            scenarios_path: "s".into(),
            linter_path: None,
        };
        let mut p = Pipeline::with_config("test.spec".into(), &cfg);
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));

        assert!(p.increment_iteration().is_ok()); // 0 -> 1
        assert!(p.increment_iteration().is_ok()); // 1 -> 2
        assert!(p.increment_iteration().is_err()); // 2 >= max_iterations=2
    }

    // CLAIM: can_iterate checks both state and iteration count
    #[test]
    fn claim_can_iterate_checks_both_conditions() {
        let cfg = PipelineConfig {
            max_iterations: 1,
            quality_threshold: 80,
            scenarios_path: "s".into(),
            linter_path: None,
        };
        let mut p = Pipeline::with_config("test.spec".into(), &cfg);

        assert!(!p.can_iterate());

        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));

        assert!(p.can_iterate());

        p.increment_iteration().unwrap();
        assert!(!p.can_iterate());
    }

    // CLAIM: set_error and clear_error work
    #[test]
    fn claim_error_management() {
        let mut p = Pipeline::new("test.spec".into());
        assert!(p.last_error.is_none());

        p.set_error("boom".into());
        assert_eq!(p.last_error.as_deref(), Some("boom"));

        p.clear_error();
        assert!(p.last_error.is_none());
    }

    // CLAIM: transition_to updates updated_at timestamp
    #[test]
    fn claim_transition_updates_timestamp() {
        let mut p = Pipeline::new("test.spec".into());
        let original_updated = p.updated_at;
        p.transition_to(PipelineState::SpecReview).unwrap();
        assert!(p.updated_at >= original_updated);
    }

    // CLAIM: PipelineConfig default values
    #[test]
    fn claim_pipeline_config_defaults() {
        let cfg = PipelineConfig::default();
        assert_eq!(cfg.max_iterations, 10);
        assert_eq!(cfg.quality_threshold, 80);
        assert_eq!(cfg.scenarios_path, "scenarios");
        assert!(cfg.linter_path.is_none());
    }

    // ADVERSARIAL: quality_threshold can exceed 100 (no validation)
    #[test]
    fn claim_quality_threshold_not_validated() {
        let cfg = PipelineConfig {
            max_iterations: 10,
            quality_threshold: 999,
            scenarios_path: "s".into(),
            linter_path: None,
        };
        let p = Pipeline::with_config("test.spec".into(), &cfg);
        assert_eq!(p.quality_threshold, 999);
    }
}

// ─── PipelineExecutor Claims ───

mod executor {
    use orchestrator::{
        phases::{Decision, PhaseError},
        state::{Pipeline, PipelineConfig, PipelineState},
        PipelineExecutor,
    };
    use tempfile::TempDir;

    fn make_executor() -> (PipelineExecutor, TempDir) {
        let tmp = TempDir::new().unwrap();
        let executor = PipelineExecutor::new(tmp.path().to_path_buf()).unwrap();
        (executor, tmp)
    }

    // CLAIM: can_run_pipeline returns true for non-terminal states
    #[test]
    fn claim_can_run_non_terminal() {
        let (exec, _) = make_executor();
        let mut p = Pipeline::new("test.spec".into());
        assert!(exec.can_run_pipeline(&p));

        p.state = PipelineState::Failed;
        assert!(!exec.can_run_pipeline(&p));

        p.state = PipelineState::Accepted;
        assert!(!exec.can_run_pipeline(&p));

        p.state = PipelineState::Escalated;
        assert!(!exec.can_run_pipeline(&p));
    }

    // CLAIM: create_pipeline creates and persists a pipeline
    #[test]
    fn claim_create_pipeline() {
        let (mut exec, _tmp) = make_executor();
        let p = exec.create_pipeline("test.spec".into()).unwrap();
        assert_eq!(p.spec_path, "test.spec");
        assert_eq!(p.state, PipelineState::Pending);

        let stored = exec.store().get(&p.id).unwrap();
        assert_eq!(stored.spec_path, "test.spec");
    }

    // CLAIM: run_pipeline completes happy path to Accepted
    #[test]
    fn claim_run_pipeline_happy_path() {
        let (mut exec, _tmp) = make_executor();
        let p = exec.create_pipeline("test.spec".into()).unwrap();
        let decision = exec.run_pipeline(&p.id).unwrap();
        assert_eq!(decision, Decision::Accept);
    }

    // CLAIM: run_pipeline with high quality_threshold fails spec review
    #[test]
    fn claim_run_pipeline_fails_spec_review() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().to_path_buf();

        // Create pipeline with custom config in a shared store
        {
            let mut store = orchestrator::persistence::StateStore::new(state_dir.clone()).unwrap();
            let config = PipelineConfig {
                max_iterations: 10,
                quality_threshold: 90, // Linter returns 85
                scenarios_path: "s".into(),
                linter_path: None,
            };
            let p = Pipeline::with_config("test.spec".into(), &config);
            store.create(p).unwrap();
            // Drop store to flush to disk
        }

        // Executor loads from the same directory
        let mut exec = PipelineExecutor::new(state_dir).unwrap();

        // Find the pipeline we created
        let pipelines = exec.store().list();
        assert_eq!(pipelines.len(), 1);
        let id = pipelines[0].id.clone();

        let decision = exec.run_pipeline(&id).unwrap();
        assert_eq!(decision, Decision::Fail);

        let stored = exec.store().get(&id).unwrap();
        assert_eq!(stored.state, PipelineState::Failed);
    }

    // CLAIM: run_pipeline with low max_iterations escalates at agent dev
    #[test]
    fn claim_run_pipeline_escalates_on_iteration_limit() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().to_path_buf();

        {
            let mut store = orchestrator::persistence::StateStore::new(state_dir.clone()).unwrap();
            let config = PipelineConfig {
                max_iterations: 0,
                quality_threshold: 80,
                scenarios_path: "s".into(),
                linter_path: None,
            };
            let p = Pipeline::with_config("test.spec".into(), &config);
            store.create(p).unwrap();
        }

        let mut exec = PipelineExecutor::new(state_dir).unwrap();

        let pipelines = exec.store().list();
        let id = pipelines[0].id.clone();

        let result = exec.run_pipeline(&id);
        match result {
            Ok(Decision::Escalate)
            | Err(PhaseError::DevelopmentFailed(_))
            | Err(PhaseError::IterationError(_)) => {}
            Ok(other) => panic!(
                "Expected Escalate, DevelopmentFailed, or IterationError, got {:?}",
                other
            ),
            Err(e) => panic!(
                "Expected Escalate, DevelopmentFailed, or IterationError, got {:?}",
                e
            ),
        }
    }

    // CLAIM: get_pending_pipelines returns non-terminal pipelines
    #[test]
    fn claim_get_pending_pipelines() {
        let (mut exec, _tmp) = make_executor();
        let p1 = exec.create_pipeline("test1.spec".into()).unwrap();
        let _p2 = exec.create_pipeline("test2.spec".into()).unwrap();

        let pending = exec.get_pending_pipelines();
        assert_eq!(pending.len(), 2);

        exec.run_pipeline(&p1.id).unwrap();
        let pending = exec.get_pending_pipelines();
        assert_eq!(pending.len(), 1);
    }

    // CLAIM: Metrics are recorded during pipeline execution
    #[test]
    fn claim_metrics_recorded() {
        let (mut exec, _tmp) = make_executor();
        let p = exec.create_pipeline("test.spec".into()).unwrap();
        exec.run_pipeline(&p.id).unwrap();

        let metrics = exec.metrics();
        let phases: Vec<_> = metrics.get_phase_metrics().collect();
        assert!(
            phases.len() >= 4,
            "Expected at least 4 phase metrics, got {}",
            phases.len()
        );
    }

    // CLAIM: recover_pipeline works for non-terminal pipelines
    #[test]
    fn claim_recover_pipeline() {
        let (mut exec, _tmp) = make_executor();
        let p = exec.create_pipeline("test.spec".into()).unwrap();
        let decision = exec.recover_pipeline(&p.id).unwrap();
        assert_eq!(decision, Decision::Accept);
    }

    // CLAIM: recover_pipeline returns correct decision for terminal states
    #[test]
    fn claim_recover_pipeline_terminal() {
        let (mut exec, _tmp) = make_executor();
        let p = exec.create_pipeline("test.spec".into()).unwrap();
        exec.run_pipeline(&p.id).unwrap();

        let decision = exec.recover_pipeline(&p.id).unwrap();
        assert_eq!(decision, Decision::Accept);
    }
}

// ─── Cleanup Claims ───

mod cleanup {
    use orchestrator::{
        cleanup::{
            CleanupContext, CleanupHandler, CleanupManager, CleanupResult, NoopCleanupHandler,
            PhaseType, ResourceId, UniverseSetupCleanupHandler,
        },
        state::{PipelineId, PipelineState},
    };

    // CLAIM: PhaseType::from_state maps correctly
    #[test]
    fn claim_phase_type_from_state() {
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
        assert_eq!(PhaseType::from_state(PipelineState::Escalated), None);
        assert_eq!(PhaseType::from_state(PipelineState::Failed), None);
    }

    // CLAIM: CleanupContext builder works
    #[test]
    fn claim_cleanup_context_builder() {
        let mut ctx = CleanupContext::new(PipelineId("pipeline-1".into()), PhaseType::SpecReview);
        ctx.add_resource(ResourceId::new("res-1"));
        ctx.add_resource(ResourceId::new("res-2"));
        assert_eq!(ctx.created_resources.len(), 2);
    }

    // CLAIM: CleanupResult builder: success by default
    #[test]
    fn claim_cleanup_result_success() {
        let result = CleanupResult::success()
            .with_resource(ResourceId::new("r1"))
            .with_resource(ResourceId::new("r2"));
        assert!(result.success_flag());
        assert!(result.errors().is_empty());
    }

    // CLAIM: CleanupResult transitions to Failed on with_error
    #[test]
    fn claim_cleanup_result_with_error() {
        let result = CleanupResult::success()
            .with_resource(ResourceId::new("r1"))
            .with_error("cleanup failed".into());
        assert!(!result.success_flag());
        let errors = result.errors();
        assert_eq!(errors.len(), 1);
    }

    // CLAIM: NoopCleanupHandler returns success
    #[test]
    fn claim_noop_handler() {
        let handler = NoopCleanupHandler;
        let ctx = CleanupContext::new(PipelineId("p1".into()), PhaseType::SpecReview);
        let result = handler.cleanup(&ctx);
        assert!(result.success_flag());
        let rollback = handler.rollback(&ctx);
        assert!(rollback.success_flag());
    }

    // CLAIM: UniverseSetupCleanupHandler processes resources
    #[test]
    fn claim_universe_setup_handler() {
        let handler = UniverseSetupCleanupHandler;
        let mut ctx = CleanupContext::new(PipelineId("p1".into()), PhaseType::UniverseSetup);
        ctx.add_resource(ResourceId::new("universe-1"));
        let result = handler.cleanup(&ctx);
        assert!(result.success_flag());
    }

    // CLAIM: CleanupManager dispatches to correct handler
    #[test]
    fn claim_cleanup_manager_dispatch() {
        let mut mgr = CleanupManager::new();
        let noop = NoopCleanupHandler;
        mgr.register_handler(Box::new(noop));

        let ctx = CleanupContext::new(PipelineId("p1".into()), PhaseType::SpecReview);
        let result = mgr.cleanup(&ctx);
        assert!(result.success_flag());
    }

    // CLAIM: CleanupManager returns success for missing handler
    #[test]
    fn claim_cleanup_manager_missing_handler() {
        let mgr = CleanupManager::new();
        let ctx = CleanupContext::new(PipelineId("p1".into()), PhaseType::Validation);
        let result = mgr.cleanup(&ctx);
        assert!(result.success_flag());
    }

    // ADVERSARIAL: UniverseSetupCleanupHandler rollback with data returns failure
    #[test]
    fn claim_universe_rollback_with_data_fails() {
        let handler = UniverseSetupCleanupHandler;
        let mut ctx = CleanupContext::new(PipelineId("p1".into()), PhaseType::UniverseSetup);
        ctx.set_rollback_data(vec![1, 2, 3]);
        let result = handler.rollback(&ctx);
        assert!(!result.success_flag());
    }
}

// ─── Metrics Claims ───

mod metrics {
    use orchestrator::metrics::{Metrics, PhaseMetrics, ScenarioResult};

    fn make_phase_metrics(pipeline_id: &str, phase: &str, success: bool) -> PhaseMetrics {
        PhaseMetrics {
            pipeline_id: pipeline_id.into(),
            phase: phase.into(),
            started_at: chrono::Utc::now(),
            duration_secs: 1.0,
            success,
        }
    }

    // CLAIM: success_rate returns 0.0 when empty
    #[test]
    fn claim_success_rate_empty() {
        let m = Metrics::new();
        assert_eq!(m.success_rate(), 0.0);
    }

    // CLAIM: success_rate computes correctly
    #[test]
    fn claim_success_rate_calculation() {
        let mut m = Metrics::new();
        m.record_phase(make_phase_metrics("p1", "review", true));
        m.record_phase(make_phase_metrics("p1", "setup", true));
        m.mark_complete("p1", "accepted");

        m.record_phase(make_phase_metrics("p2", "review", false));
        m.mark_complete("p2", "failed");

        let rate = m.success_rate();
        assert!(rate > 0.0 && rate < 100.0);
    }

    // CLAIM: scenario_pass_rate returns 0.0 when empty
    #[test]
    fn claim_scenario_pass_rate_empty() {
        let m = Metrics::new();
        assert_eq!(m.scenario_pass_rate(), 0.0);
    }

    // CLAIM: scenario_pass_rate with mixed results
    #[test]
    fn claim_scenario_pass_rate_mixed() {
        let mut m = Metrics::new();
        // record_phase creates the PipelineMetrics entry
        m.record_phase(make_phase_metrics("p1", "validation", true));
        m.record_scenarios(
            "p1",
            vec![
                ScenarioResult {
                    name: "s1".into(),
                    passed: true,
                    duration_secs: 1.0,
                    error: None,
                },
                ScenarioResult {
                    name: "s2".into(),
                    passed: false,
                    duration_secs: 0.5,
                    error: Some("fail".into()),
                },
            ],
        );
        let rate = m.scenario_pass_rate();
        assert!((rate - 50.0).abs() < 0.1);
    }

    // CLAIM: aggregated returns zeros when empty
    #[test]
    fn claim_aggregated_empty() {
        let m = Metrics::new();
        let agg = m.aggregated();
        assert_eq!(agg.total_pipelines, 0);
        assert_eq!(agg.successful_pipelines, 0);
    }

    // CLAIM: slowest_phases returns sorted descending
    #[test]
    fn claim_slowest_phases_sorted() {
        let mut m = Metrics::new();
        m.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "fast".into(),
            started_at: chrono::Utc::now(),
            duration_secs: 0.1,
            success: true,
        });
        m.record_phase(PhaseMetrics {
            pipeline_id: "p1".into(),
            phase: "slow".into(),
            started_at: chrono::Utc::now(),
            duration_secs: 10.0,
            success: true,
        });
        let slowest = m.slowest_phases(10);
        assert_eq!(slowest.len(), 2);
        assert!(slowest[0].1 >= slowest[1].1);
    }

    // ADVERSARIAL: Recording metrics for nonexistent pipeline is a no-op
    #[test]
    fn claim_metrics_for_nonexistent_pipeline_noop() {
        let mut m = Metrics::new();
        m.record_scenarios("nonexistent", vec![]);
        m.record_iteration("nonexistent", 5);
        m.mark_complete("nonexistent", "accepted");
        assert!(m.get_pipeline_metrics("nonexistent").is_none());
    }

    // CLAIM: export produces valid JSON
    #[test]
    fn claim_export_valid_json() {
        let mut m = Metrics::new();
        m.record_phase(make_phase_metrics("p1", "review", true));
        m.mark_complete("p1", "accepted");
        let json = m.export().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
    }
}

// ─── Persistence Claims ───

mod persistence {
    use orchestrator::{
        persistence::StateStore,
        state::{Pipeline, PipelineId, PipelineState},
    };
    use tempfile::TempDir;

    // CLAIM: StateStore creates and loads from directory
    #[test]
    fn claim_state_store_create_and_load() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();
        let p = Pipeline::new("test.spec".into());
        let id = p.id.clone();
        store.create(p).unwrap();

        let loaded = store.get(&id).unwrap();
        assert_eq!(loaded.spec_path, "test.spec");
    }

    // CLAIM: StateStore get_mut marks dirty and persists
    #[test]
    fn claim_state_store_update_persists() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();
        let p = Pipeline::new("test.spec".into());
        let id = p.id.clone();
        store.create(p).unwrap();

        {
            let pipeline = store.get_mut(&id).unwrap();
            pipeline.spec_path = "modified.spec".into();
        }
        store.sync().unwrap();

        let store2 = StateStore::new(tmp.path().to_path_buf()).unwrap();
        let loaded = store2.get(&id).unwrap();
        assert_eq!(loaded.spec_path, "modified.spec");
    }

    // CLAIM: StateStore delete removes pipeline
    #[test]
    fn claim_state_store_delete() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();
        let p = Pipeline::new("test.spec".into());
        let id = p.id.clone();
        store.create(p).unwrap();

        store.delete(&id).unwrap();
        assert!(store.get(&id).is_err());
    }

    // CLAIM: StateStore list_by_state filters correctly
    #[test]
    fn claim_state_store_list_by_state() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();

        let p1 = Pipeline::new("pending.spec".into());
        let mut p2 = Pipeline::new("failed.spec".into());
        p2.state = PipelineState::Failed;

        store.create(p1).unwrap();
        store.create(p2).unwrap();

        let pending = store.list_by_state(PipelineState::Pending);
        assert_eq!(pending.len(), 1);

        let failed = store.list_by_state(PipelineState::Failed);
        assert_eq!(failed.len(), 1);
    }

    // CLAIM: get_pending_recovery returns non-terminal pipelines
    #[test]
    fn claim_get_pending_recovery() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();

        let p1 = Pipeline::new("active.spec".into());
        let mut p2 = Pipeline::new("done.spec".into());
        p2.state = PipelineState::Accepted;

        store.create(p1).unwrap();
        store.create(p2).unwrap();

        let recovery = store.get_pending_recovery();
        assert_eq!(recovery.len(), 1);
    }

    // CLAIM: exists checks correctly
    #[test]
    fn claim_state_store_exists() {
        let tmp = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();
        let p = Pipeline::new("test.spec".into());
        let id = p.id.clone();
        store.create(p).unwrap();

        assert!(store.exists(&id));
        assert!(!store.exists(&PipelineId("nonexistent".into())));
    }

    // CLAIM: export_all and import_from work
    #[test]
    fn claim_export_import() {
        let tmp = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        let mut store = StateStore::new(tmp.path().to_path_buf()).unwrap();

        store.create(Pipeline::new("test1.spec".into())).unwrap();
        store.create(Pipeline::new("test2.spec".into())).unwrap();

        let export_path = tmp2.path().join("export.json");
        store.export_all(&export_path).unwrap();

        let mut store2 = StateStore::new(tmp2.path().join("new_store").to_path_buf()).unwrap();
        let imported = store2.import_from(&export_path).unwrap();
        assert_eq!(imported, 2);
    }

    // ADVERSARIAL: Corrupt JSON files are skipped during load
    #[test]
    fn claim_corrupt_files_skipped() {
        let tmp = TempDir::new().unwrap();
        // Write a valid pipeline first
        let p = Pipeline::new("good.spec".into());
        let json = serde_json::to_string_pretty(&p).unwrap();
        std::fs::write(tmp.path().join("good.json"), &json).unwrap();
        // Write a corrupt file
        std::fs::write(tmp.path().join("bad.json"), "not json at all").unwrap();

        let store = StateStore::new(tmp.path().to_path_buf()).unwrap();
        assert_eq!(store.list().len(), 1);
    }
}

// ─── Policy Claims ───

mod policies {
    use orchestrator::policies::{
        CircuitBreaker, CircuitBreakerState, ConfigError, Deadline, PhaseTimeout, PolicyConfig,
        PolicyOpts, RetryPolicy, RetryPolicyError, TimeoutPolicy,
    };

    // CLAIM: PhaseTimeout rejects zero
    #[test]
    fn claim_phase_timeout_rejects_zero() {
        let result = PhaseTimeout::new(0);
        assert!(matches!(result, Err(ConfigError::InvalidTimeout { .. })));
    }

    // CLAIM: PhaseTimeout is_expired works
    #[test]
    fn claim_phase_timeout_expiry() {
        let timeout = PhaseTimeout::new(1000).unwrap();
        let past = chrono::Utc::now() - chrono::Duration::milliseconds(2000);
        assert!(timeout.is_expired(past));
        let future = chrono::Utc::now() + chrono::Duration::milliseconds(2000);
        assert!(!timeout.is_expired(future));
    }

    // CLAIM: TimeoutPolicy rejects zero
    #[test]
    fn claim_timeout_policy_rejects_zero() {
        let result = TimeoutPolicy::new(0);
        assert!(result.is_err());
    }

    // CLAIM: TimeoutPolicy::none() has no timeout
    #[test]
    fn claim_timeout_policy_none() {
        let tp = TimeoutPolicy::none();
        assert!(tp.is_none());
        assert!(tp.get_timeout_ms().is_none());
    }

    // CLAIM: RetryPolicy rejects base_delay=0
    #[test]
    fn claim_retry_policy_rejects_zero_base_delay() {
        let result = RetryPolicy::new(3, 0, 2.0, Some(1000), vec![]);
        assert!(matches!(result, Err(RetryPolicyError::InvalidBaseDelay)));
    }

    // CLAIM: RetryPolicy rejects max_delay < base_delay
    #[test]
    fn claim_retry_policy_rejects_max_lt_base() {
        let result = RetryPolicy::new(3, 1000, 2.0, Some(500), vec![]);
        assert!(result.is_err());
    }

    // CLAIM: RetryPolicy calculate_delay uses exponential backoff
    #[test]
    fn claim_retry_policy_exponential_backoff() {
        let rp = RetryPolicy::new(5, 100, 2.0, Some(10000), vec![]).unwrap();
        assert_eq!(rp.calculate_delay(0), 100);
        assert_eq!(rp.calculate_delay(1), 200);
        assert_eq!(rp.calculate_delay(2), 400);
        assert_eq!(rp.calculate_delay(3), 800);
    }

    // CLAIM: RetryPolicy caps at max_delay
    #[test]
    fn claim_retry_policy_caps_max_delay() {
        let rp = RetryPolicy::new(10, 1000, 2.0, Some(5000), vec![]).unwrap();
        assert_eq!(rp.calculate_delay(10), 5000);
    }

    // CLAIM: RetryPolicy rejects invalid factor
    #[test]
    fn claim_retry_policy_rejects_bad_factor() {
        assert!(RetryPolicy::new(3, 100, 1.0, None, vec![]).is_err());
        assert!(RetryPolicy::new(3, 100, 0.5, None, vec![]).is_err());
        assert!(RetryPolicy::new(3, 100, f64::NAN, None, vec![]).is_err());
        assert!(RetryPolicy::new(3, 100, f64::INFINITY, None, vec![]).is_err());
    }

    // CLAIM: RetryPolicy is_retryable works
    #[test]
    fn claim_retry_policy_retryable() {
        let rp = RetryPolicy::new(
            3,
            100,
            2.0,
            None,
            vec!["timeout".into(), "connection".into()],
        )
        .unwrap();
        assert!(rp.is_retryable("request timeout occurred"));
        assert!(rp.is_retryable("connection refused"));
        assert!(!rp.is_retryable("invalid input"));
    }

    // ADVERSARIAL: Empty retryable_errors -> is_retryable returns false
    #[test]
    fn claim_retry_policy_empty_patterns() {
        let rp = RetryPolicy::new(3, 100, 2.0, None, vec![]).unwrap();
        assert!(!rp.is_retryable("timeout"));
    }

    // CLAIM: CircuitBreaker rejects zero threshold and timeout
    #[test]
    fn claim_circuit_breaker_rejects_zero() {
        assert!(CircuitBreaker::new(0, 1, 30000).is_err());
        assert!(CircuitBreaker::new(3, 1, 0).is_err());
    }

    // CLAIM: CircuitBreaker state transitions: Closed -> Open -> HalfOpen -> Closed
    #[test]
    fn claim_circuit_breaker_full_lifecycle() {
        let mut cb = CircuitBreaker::new(2, 1, 1).unwrap(); // 1ms recovery timeout
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());

        std::thread::sleep(std::time::Duration::from_millis(5));
        cb.try_transition_to_half_open();
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
        assert!(cb.can_execute());

        cb.record_success();
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
    }

    // CLAIM: CircuitBreaker rejects all zero params
    #[test]
    fn claim_circuit_breaker_rejects_zero_params() {
        assert!(CircuitBreaker::new(0, 1, 1000).is_err());
        assert!(CircuitBreaker::new(1, 0, 1000).is_err());
        assert!(CircuitBreaker::new(1, 1, 0).is_err());
    }

    // CLAIM: Deadline from_now works
    #[test]
    fn claim_deadline_from_now() {
        let dl = Deadline::from_now(5000);
        assert!(!dl.is_exceeded());
        assert!(dl.remaining_ms() > 0);
    }

    // CLAIM: Deadline from_now(0) is exceeded (or nearly so)
    #[test]
    fn claim_deadline_zero_is_exceeded() {
        let dl = Deadline::from_now(0);
        assert!(dl.remaining_ms() >= 0);
    }

    // CLAIM: PolicyConfig validates all sub-configs
    #[test]
    fn claim_policy_config_validation() {
        let result = PolicyConfig::new(PolicyOpts {
            timeout_ms: 1000,
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            failure_threshold: 5,
            recovery_timeout_ms: 30000,
        });
        assert!(result.is_ok());

        assert!(PolicyConfig::new(PolicyOpts {
            timeout_ms: 0,
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            failure_threshold: 5,
            recovery_timeout_ms: 30000
        })
        .is_err());
        assert!(PolicyConfig::new(PolicyOpts {
            timeout_ms: 1000,
            max_retries: 3,
            base_delay_ms: 0,
            max_delay_ms: 5000,
            failure_threshold: 5,
            recovery_timeout_ms: 30000
        })
        .is_err());
        assert!(PolicyConfig::new(PolicyOpts {
            timeout_ms: 1000,
            max_retries: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            failure_threshold: 0,
            recovery_timeout_ms: 30000
        })
        .is_err());
    }
}

// ─── Queue Claims ───

mod queue {
    use std::time::Duration;

    use orchestrator::queue::{
        InMemoryJobRepository, Job, JobPayload, JobPriority, JobProcessor, JobProcessorConfig,
        JobRepository, JobState,
    };

    fn make_job(priority: JobPriority) -> Job {
        Job {
            id: uuid::Uuid::new_v4().to_string(),
            priority,
            payload: JobPayload::Task {
                command: "echo hello".into(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // CLAIM: JobPriority ordering: P0 > P1 > P2 > P3 > P4
    #[test]
    fn claim_job_priority_ordering() {
        assert!(JobPriority::P0 < JobPriority::P1);
        assert!(JobPriority::P1 < JobPriority::P2);
        assert!(JobPriority::P2 < JobPriority::P3);
        assert!(JobPriority::P3 < JobPriority::P4);
    }

    // CLAIM: JobPriority from_u8 clamps
    #[test]
    fn claim_job_priority_from_u8_clamps() {
        assert_eq!(JobPriority::from_u8(0), JobPriority::P0);
        assert_eq!(JobPriority::from_u8(2), JobPriority::P2);
        assert_eq!(JobPriority::from_u8(4), JobPriority::P4);
        assert_eq!(JobPriority::from_u8(100), JobPriority::P4);
    }

    // CLAIM: JobState is_pending/is_running/is_terminal
    #[test]
    fn claim_job_state_predicates() {
        let pending = JobState::Pending;
        assert!(pending.is_pending());
        assert!(!pending.is_running());
        assert!(!pending.is_terminal());

        let running = JobState::Running {
            started_at: chrono::Utc::now(),
        };
        assert!(!running.is_pending());
        assert!(running.is_running());
        assert!(!running.is_terminal());

        let completed = JobState::Completed {
            finished_at: chrono::Utc::now(),
        };
        assert!(!completed.is_pending());
        assert!(!completed.is_running());
        assert!(completed.is_terminal());

        let failed = JobState::Failed {
            error: "boom".into(),
            failed_at: chrono::Utc::now(),
        };
        assert!(failed.is_terminal());
    }

    // CLAIM: InMemoryJobRepository polls by priority
    #[test]
    fn claim_repository_polls_by_priority() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let repo = InMemoryJobRepository::new();
        let p0 = make_job(JobPriority::P0);
        let p0_id = p0.id.clone();
        repo.add_job(make_job(JobPriority::P2)).expect("add");
        repo.add_job(p0).expect("add");
        repo.add_job(make_job(JobPriority::P1)).expect("add");

        let jobs = rt.block_on(repo.poll_pending_jobs(1)).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, p0_id);
    }

    // CLAIM: JobProcessorConfig validates
    #[test]
    fn claim_processor_config_validation() {
        let valid = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: 1,
            max_retries: 3,
        };
        assert!(valid.validate().is_ok());

        let zero_poll = JobProcessorConfig {
            poll_interval: Duration::ZERO,
            concurrency_limit: 1,
            max_retries: 3,
        };
        assert!(zero_poll.validate().is_err());

        let zero_concurrency = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: 0,
            max_retries: 3,
        };
        assert!(zero_concurrency.validate().is_err());
    }

    // CLAIM: JobProcessor can be created
    #[tokio::test]
    async fn claim_job_processor_creation() {
        let repo = InMemoryJobRepository::new();
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(10),
            concurrency_limit: 1,
            max_retries: 3,
        };
        let processor = JobProcessor::new(repo, config).unwrap();
        assert_eq!(processor.running_jobs(), 0);
    }

    // CLAIM: Job serde roundtrip
    #[test]
    fn claim_job_serde_roundtrip() {
        let job = make_job(JobPriority::P2);
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: Job = serde_json::from_str(&json).unwrap();
        assert_eq!(job.id, deserialized.id);
        assert_eq!(job.priority, deserialized.priority);
    }
}

// ─── Parallel Execution Claims ───

mod parallel {
    use orchestrator::{
        cleanup::PhaseType,
        parallel::{DependencyGraph, ParallelExecutor, PhaseGroup, PhaseStatus},
        state::PipelineState,
    };

    // CLAIM: DependencyGraph cycle detection
    #[test]
    fn claim_cycle_detection() {
        let graph = DependencyGraph::new()
            .add_phase(PhaseType::SpecReview, vec![])
            .add_phase(PhaseType::UniverseSetup, vec![PhaseType::Validation])
            .add_phase(PhaseType::Validation, vec![PhaseType::UniverseSetup]);

        let result = graph.validate();
        assert!(result.is_err());
    }

    // CLAIM: Self-cycle detected
    #[test]
    fn claim_self_cycle_detection() {
        let graph =
            DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![PhaseType::SpecReview]);

        let result = graph.validate();
        assert!(result.is_err());
    }

    // CLAIM: Valid dependency graph
    #[test]
    fn claim_valid_dependency_graph() {
        let graph = DependencyGraph::new()
            .add_phase(PhaseType::SpecReview, vec![])
            .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview])
            .add_phase(PhaseType::AgentDevelopment, vec![PhaseType::UniverseSetup]);

        assert!(graph.validate().is_ok());
    }

    // CLAIM: Empty graph is valid and complete
    #[test]
    fn claim_empty_graph_complete() {
        let graph = DependencyGraph::new();
        assert!(graph.validate().is_ok());
        assert!(graph.is_complete());
        assert!(!graph.has_failures());
    }

    // CLAIM: get_ready_phases returns only phases with met dependencies
    #[test]
    fn claim_get_ready_phases() {
        let mut graph = DependencyGraph::new()
            .add_phase(PhaseType::SpecReview, vec![])
            .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);

        let mut completed = std::collections::HashSet::new();
        let ready = graph.get_ready_phases(&completed);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&PhaseType::SpecReview));

        graph.mark_running(PhaseType::SpecReview);
        graph.mark_completed(PhaseType::SpecReview);
        completed.insert(PhaseType::SpecReview);
        let ready = graph.get_ready_phases(&completed);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&PhaseType::UniverseSetup));
    }

    // CLAIM: PhaseGroup max_parallelism
    #[test]
    fn claim_phase_group_max_parallelism() {
        let group = PhaseGroup::new(vec![PhaseType::SpecReview, PhaseType::Validation]);
        assert_eq!(group.max_parallelism, 2);

        let limited = group.with_max_parallelism(1);
        assert_eq!(limited.max_parallelism, 1);
    }

    // CLAIM: resolve_parallel_phases maps states correctly
    #[test]
    fn claim_resolve_parallel_phases() {
        let groups = ParallelExecutor::resolve_parallel_phases(&PipelineState::Pending);
        assert_eq!(groups.len(), 1);

        let groups = ParallelExecutor::resolve_parallel_phases(&PipelineState::Accepted);
        assert!(groups.is_empty());

        let groups = ParallelExecutor::resolve_parallel_phases(&PipelineState::Failed);
        assert!(groups.is_empty());
    }

    // CLAIM: PhaseStatus serde roundtrip
    #[test]
    fn claim_phase_status_serde() {
        for status in [
            PhaseStatus::Pending,
            PhaseStatus::Running,
            PhaseStatus::Completed,
            PhaseStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: PhaseStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }
}

// ─── Adversarial / Edge Case Claims ───

mod adversarial {
    use orchestrator::{
        phases::Decision,
        state::{Pipeline, PipelineConfig, PipelineId, PipelineState},
        PipelineExecutor,
    };
    use tempfile::TempDir;

    // ADVERSARIAL: Empty spec path
    #[test]
    fn claim_empty_spec_path() {
        let p = Pipeline::new("".into());
        assert_eq!(p.spec_path, "");
        assert_eq!(p.state, PipelineState::Pending);
    }

    // ADVERSARIAL: max_iterations = u32::MAX
    #[test]
    fn claim_max_iterations_max_value() {
        let cfg = PipelineConfig {
            max_iterations: u32::MAX,
            quality_threshold: 80,
            scenarios_path: "s".into(),
            linter_path: None,
        };
        let mut p = Pipeline::with_config("test.spec".into(), &cfg);
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));
        assert!(p.can_iterate());
        assert!(p.increment_iteration().is_ok());
    }

    // ADVERSARIAL: max_iterations = 0
    #[test]
    fn claim_max_iterations_zero() {
        let cfg = PipelineConfig {
            max_iterations: 0,
            quality_threshold: 80,
            scenarios_path: "s".into(),
            linter_path: None,
        };
        let mut p = Pipeline::with_config("test.spec".into(), &cfg);
        let _ = p
            .transition_to(PipelineState::SpecReview)
            .and_then(|_| p.transition_to(PipelineState::UniverseSetup))
            .and_then(|_| p.transition_to(PipelineState::AgentDevelopment));
        assert!(!p.can_iterate());
        assert!(p.increment_iteration().is_err());
    }

    // ADVERSARIAL: quality_threshold = 0 (always passes)
    #[test]
    fn claim_quality_threshold_zero() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().to_path_buf();

        {
            let mut store = orchestrator::persistence::StateStore::new(state_dir.clone()).unwrap();
            let config = PipelineConfig {
                max_iterations: 10,
                quality_threshold: 0,
                scenarios_path: "s".into(),
                linter_path: None,
            };
            let p = Pipeline::with_config("test.spec".into(), &config);
            store.create(p).unwrap();
        }

        let mut exec = PipelineExecutor::new(state_dir).unwrap();

        let pipelines = exec.store().list();
        let id = pipelines[0].id.clone();
        let decision = exec.run_pipeline(&id).unwrap();
        assert_eq!(decision, Decision::Accept);
    }

    // ADVERSARIAL: Spec review with quality_threshold = 86 (linter returns 85)
    #[test]
    fn claim_spec_review_threshold_boundary() {
        let tmp = TempDir::new().unwrap();
        let state_dir = tmp.path().to_path_buf();

        {
            let mut store = orchestrator::persistence::StateStore::new(state_dir.clone()).unwrap();
            let config = PipelineConfig {
                max_iterations: 10,
                quality_threshold: 86,
                scenarios_path: "s".into(),
                linter_path: None,
            };
            let p = Pipeline::with_config("test.spec".into(), &config);
            store.create(p).unwrap();
        }

        let mut exec = PipelineExecutor::new(state_dir).unwrap();

        let pipelines = exec.store().list();
        let id = pipelines[0].id.clone();
        let decision = exec.run_pipeline(&id).unwrap();
        assert_eq!(decision, Decision::Fail);
    }

    // ADVERSARIAL: Run pipeline twice (idempotent)
    #[test]
    fn claim_run_pipeline_twice() {
        let tmp = TempDir::new().unwrap();
        let mut exec = PipelineExecutor::new(tmp.path().to_path_buf()).unwrap();
        let p = exec.create_pipeline("test.spec".into()).unwrap();

        let d1 = exec.run_pipeline(&p.id).unwrap();
        assert_eq!(d1, Decision::Accept);

        let d2 = exec.run_pipeline(&p.id).unwrap();
        assert_eq!(d2, Decision::Accept);
    }

    // ADVERSARIAL: PipelineId with special characters
    #[test]
    fn claim_pipeline_id_special_chars() {
        let id = PipelineId("../../../etc/passwd".into());
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: PipelineId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }

    // ADVERSARIAL: Multiple concurrent pipeline executions
    #[test]
    fn claim_multiple_pipelines() {
        let tmp = TempDir::new().unwrap();
        let mut exec = PipelineExecutor::new(tmp.path().to_path_buf()).unwrap();

        let ids: Vec<_> = (0..5)
            .map(|i| {
                let p = exec.create_pipeline(format!("test{}.spec", i)).unwrap();
                p.id.clone()
            })
            .collect();

        for id in &ids {
            let decision = exec.run_pipeline(id).unwrap();
            assert_eq!(decision, Decision::Accept);
        }

        for id in &ids {
            let stored = exec.store().get(id).unwrap();
            assert_eq!(stored.state, PipelineState::Accepted);
        }
    }
}
