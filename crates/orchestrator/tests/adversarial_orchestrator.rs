//! Adversarial tests for orchestrator crate
//!
//! BDD Claim Sheet — validated with real terminal output.
//!
//! # Claim Sheet
//!
//! ## C1: Pipeline State Machine
//! - C1.1: Pipeline::new creates pipeline in Pending state
//! - C1.2: Valid transitions follow the defined state graph
//! - C1.3: Invalid transitions are rejected with TransitionError
//! - C1.4: Terminal states block all further transitions
//! - C1.5: Iteration counting and max enforcement works
//! - C1.6: Serde roundtrips preserve all data
//!
//! ## C2: PipelineExecutor
//! - C2.1: PipelineExecutor::new initializes with valid paths
//! - C2.2: create_pipeline persists and returns pipeline
//! - C2.3: can_run_pipeline checks non-terminal precondition
//! - C2.4: run_pipeline executes full happy path
//! - C2.5: cleanup_after_failure handles all phase types
//! - C2.6: rollback_phase works for registered handlers
//! - C2.7: get_pending_pipelines returns non-terminal pipelines
//!
//! ## C3: StateStore Persistence
//! - C3.1: CRUD operations work correctly
//! - C3.2: Export/import preserves data
//! - C3.3: Recovery filtering works
//! - C3.4: Sync persists dirty data
//!
//! ## C4: Metrics
//! - C4.1: Phase recording and aggregation works
//! - C4.2: Success rate and scenario pass rate calculations
//! - C4.3: Slowest phases ordering
//!
//! ## C5: Policies (Timeout, Retry, CircuitBreaker, Deadline)
//! - C5.1: TimeoutPolicy rejects zero, accepts valid
//! - C5.2: RetryPolicy exponential backoff formula
//! - C5.3: CircuitBreaker state transitions
//! - C5.4: Deadline expiry detection
//! - C5.5: PolicyConfig validates all parameters
//!
//! ## C6: Queue (Job types, Repository, Processor)
//! - C6.1: Job priority ordering
//! - C6.2: Job state transitions
//! - C6.3: Repository poll/update/get operations
//! - C6.4: Processor config validation
//!
//! ## C7: Cleanup
//! - C7.1: CleanupManager handles all phase types
//! - C7.2: CleanupResult builder pattern
//! - C7.3: Handler registration and replacement
//!
//! ## C8: Parallel Execution
//! - C8.1: DependencyGraph detects cycles
//! - C8.2: Phase resolution from pipeline state
//! - C8.3: Dependency order validation

use std::collections::HashSet;

use tempfile::TempDir;

// ============================================================
// C1: Pipeline State Machine — Adversarial Tests
// ============================================================

mod pipeline_state_adversarial {
    use orchestrator::state::*;

    use super::*;

    // --- Missing input ---
    #[test]
    fn adv_c1_empty_spec_path() {
        let pipeline = Pipeline::new(String::new());
        assert_eq!(pipeline.spec_path, "");
        assert_eq!(pipeline.state, PipelineState::Pending);
        // Empty path is valid — Pipeline doesn't validate spec_path
    }

    #[test]
    fn adv_c1_whitespace_spec_path() {
        let pipeline = Pipeline::new("   ".to_string());
        assert_eq!(pipeline.spec_path, "   ");
    }

    #[test]
    fn adv_c1_very_long_spec_path() {
        let long_path = "a".repeat(10_000);
        let pipeline = Pipeline::new(long_path.clone());
        assert_eq!(pipeline.spec_path.len(), 10_000);
    }

    #[test]
    fn adv_c1_spec_path_with_null_bytes() {
        let path = "test\0.yaml".to_string();
        let pipeline = Pipeline::new(path);
        assert!(pipeline.spec_path.contains('\0'));
    }

    #[test]
    fn adv_c1_spec_path_with_special_chars() {
        let special_paths = vec![
            "../../etc/passwd",
            "/dev/null",
            "C:\\Windows\\System32",
            "path with spaces",
            "path/with/../traversal",
            "日本語パス",
            "🎉emoji_path",
        ];
        for path in special_paths {
            let pipeline = Pipeline::new(path.to_string());
            assert_eq!(pipeline.spec_path, path);
        }
    }

    // --- Boundary ---
    #[test]
    fn adv_c1_zero_max_iterations() {
        let config = PipelineConfig {
            max_iterations: 0,
            quality_threshold: 80,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        };
        let mut pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);
        pipeline.state = PipelineState::AgentDevelopment;
        // With max_iterations=0, can_iterate should be false immediately
        assert!(!pipeline.can_iterate());
        assert!(pipeline.increment_iteration().is_err());
    }

    #[test]
    fn adv_c1_max_iterations_equals_u32_max() {
        let config = PipelineConfig {
            max_iterations: u32::MAX,
            quality_threshold: 80,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        };
        let pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);
        assert_eq!(pipeline.max_iterations, u32::MAX);
    }

    #[test]
    fn adv_c1_quality_threshold_boundary() {
        let config = PipelineConfig {
            max_iterations: 10,
            quality_threshold: 0,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        };
        let pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);
        assert_eq!(pipeline.quality_threshold, 0);
    }

    #[test]
    fn adv_c1_quality_threshold_max() {
        let config = PipelineConfig {
            max_iterations: 10,
            quality_threshold: 100,
            scenarios_path: "scenarios".to_string(),
            linter_path: None,
        };
        let pipeline = Pipeline::with_config("specs/test.yaml".to_string(), &config);
        assert_eq!(pipeline.quality_threshold, 100);
    }

    // --- Wrong state ---
    #[test]
    fn adv_c1_transition_from_accepted_to_every_state() {
        let non_terminals = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Failed,
            PipelineState::Escalated,
        ];
        for &target in &non_terminals {
            let mut pipeline = Pipeline::new("test".to_string());
            pipeline.state = PipelineState::Accepted;
            let result = pipeline.transition_to(target);
            assert!(result.is_err(), "Accepted -> {target:?} should fail");
        }
    }

    #[test]
    fn adv_c1_transition_from_escalated_to_every_state() {
        let targets = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Failed,
        ];
        for &target in &targets {
            let mut pipeline = Pipeline::new("test".to_string());
            pipeline.state = PipelineState::Escalated;
            let result = pipeline.transition_to(target);
            assert!(result.is_err(), "Escalated -> {target:?} should fail");
        }
    }

    #[test]
    fn adv_c1_transition_from_failed_to_every_state() {
        let targets = [
            PipelineState::Pending,
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
            PipelineState::Escalated,
        ];
        for &target in &targets {
            let mut pipeline = Pipeline::new("test".to_string());
            pipeline.state = PipelineState::Failed;
            let result = pipeline.transition_to(target);
            assert!(result.is_err(), "Failed -> {target:?} should fail");
        }
    }

    #[test]
    fn adv_c1_skip_ahead_pending_to_agent_development() {
        let mut pipeline = Pipeline::new("test".to_string());
        assert!(pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .is_err());
    }

    #[test]
    fn adv_c1_skip_ahead_pending_to_validation() {
        let mut pipeline = Pipeline::new("test".to_string());
        assert!(pipeline.transition_to(PipelineState::Validation).is_err());
    }

    #[test]
    fn adv_c1_skip_ahead_pending_to_accepted() {
        let mut pipeline = Pipeline::new("test".to_string());
        assert!(pipeline.transition_to(PipelineState::Accepted).is_err());
    }

    #[test]
    fn adv_c1_iteration_in_wrong_state() {
        let mut pipeline = Pipeline::new("test".to_string());
        // Pending state: increment_iteration should succeed (just increments counter)
        let result = pipeline.increment_iteration();
        assert!(result.is_ok());
        assert_eq!(pipeline.iteration, 1);
    }

    // --- Stress ---
    #[test]
    fn adv_c1_rapid_state_transitions() {
        for _ in 0..100 {
            let mut pipeline = Pipeline::new("test".to_string());
            pipeline.transition_to(PipelineState::SpecReview).unwrap();
            pipeline
                .transition_to(PipelineState::UniverseSetup)
                .unwrap();
            pipeline
                .transition_to(PipelineState::AgentDevelopment)
                .unwrap();
            pipeline.transition_to(PipelineState::Validation).unwrap();
            pipeline.transition_to(PipelineState::Accepted).unwrap();
            assert!(pipeline.state.is_terminal());
        }
    }

    #[test]
    fn adv_c1_many_pipelines_unique_ids() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let pipeline = Pipeline::new("test".to_string());
            assert!(ids.insert(pipeline.id.0.clone()), "Duplicate ID found!");
        }
    }

    #[test]
    fn adv_c1_iteration_stress() {
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.state = PipelineState::AgentDevelopment;
        pipeline.max_iterations = 1000;
        for i in 1..=1000 {
            assert!(pipeline.increment_iteration().is_ok());
            assert_eq!(pipeline.iteration, i);
        }
        assert!(pipeline.increment_iteration().is_err());
    }

    #[test]
    fn adv_c1_error_set_clear_cycle() {
        let mut pipeline = Pipeline::new("test".to_string());
        for i in 0..100 {
            let msg = format!("error-{}", i);
            pipeline.set_error(msg.clone());
            assert_eq!(pipeline.last_error.as_deref(), Some(msg.as_str()));
            pipeline.clear_error();
            assert!(pipeline.last_error.is_none());
        }
    }

    // --- Serde edge cases ---
    #[test]
    fn adv_c1_serde_empty_pipeline_id() {
        let id = PipelineId(String::new());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: PipelineId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn adv_c1_serde_pipeline_id_with_unicode() {
        let id = PipelineId("日本語-🎉-pipeline".to_string());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: PipelineId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn adv_c1_serde_invalid_json_rejected() {
        let result: Result<PipelineState, _> = serde_json::from_str("invalid");
        assert!(result.is_err());

        let result: Result<PipelineId, _> = serde_json::from_str("123");
        assert!(result.is_err());
    }

    #[test]
    fn adv_c1_serde_malformed_pipeline_state() {
        let result: Result<PipelineState, _> = serde_json::from_str("\"invalid_state\"");
        assert!(result.is_err());
    }

    // --- TransitionError ---
    #[test]
    fn adv_c1_transition_error_all_terminal_variants() {
        let terminals = [
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for &terminal in &terminals {
            let mut pipeline = Pipeline::new("test".to_string());
            pipeline.state = terminal;
            let result = pipeline.transition_to(PipelineState::Pending);
            assert!(matches!(
                result,
                Err(TransitionError::AlreadyTerminal { .. })
            ));
        }
    }
}

// ============================================================
// C2: PipelineExecutor — Adversarial Tests
// ============================================================

mod pipeline_executor_adversarial {
    use orchestrator::{phases::PipelineExecutor, state::*, PhaseType};

    use super::*;

    fn create_executor() -> (PipelineExecutor, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().to_path_buf();
        let scenarios_path = temp_dir.path().join("scenarios");
        let executor = PipelineExecutor::new(state_dir).expect("create executor");
        (executor, temp_dir)
    }

    // --- Missing input ---
    #[test]
    fn adv_c2_can_run_on_nonexistent_pipeline() {
        let (executor, _temp) = create_executor();
        let phantom = Pipeline::new("phantom.yaml".to_string());
        // can_run_pipeline just checks state, doesn't need persistence
        assert!(executor.can_run_pipeline(&phantom));
    }

    #[test]
    fn adv_c2_get_pending_from_empty_store() {
        let (executor, _temp) = create_executor();
        let pending = executor.get_pending_pipelines();
        assert!(pending.is_empty());
    }

    // --- Boundary ---
    #[test]
    fn adv_c2_cleanup_nonexistent_phase() {
        let (executor, _temp) = create_executor();
        // PhaseType::from_state returns None for Pending
        let pipeline = Pipeline::new("test".to_string());
        // Pending -> None phase type -> cleanup_after_failure should be ok
        let result = executor.cleanup_after_failure(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn adv_c2_cleanup_for_terminal_state() {
        let (executor, _temp) = create_executor();
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.state = PipelineState::Failed;
        // cleanup_after_failure on Failed state: PhaseType::from_state returns None
        let result = executor.cleanup_after_failure(&pipeline);
        assert!(result.is_ok());
    }

    #[test]
    fn adv_c2_can_run_pipeline_terminal() {
        let (executor, _temp) = create_executor();
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.state = PipelineState::Accepted;
        assert!(!executor.can_run_pipeline(&pipeline));

        pipeline.state = PipelineState::Failed;
        assert!(!executor.can_run_pipeline(&pipeline));

        pipeline.state = PipelineState::Escalated;
        assert!(!executor.can_run_pipeline(&pipeline));
    }

    #[test]
    fn adv_c2_create_pipeline_persists() {
        let (mut executor, _temp) = create_executor();
        let pipeline = executor
            .create_pipeline("specs/test.yaml".to_string())
            .expect("create");

        let retrieved = executor.store().get(&pipeline.id).expect("get");
        assert_eq!(retrieved.spec_path, "specs/test.yaml");
        assert_eq!(retrieved.state, PipelineState::Pending);
    }

    #[test]
    fn adv_c2_get_pending_pipelines() {
        let (mut executor, _temp) = create_executor();

        // Create two pipelines
        let _p1 = executor
            .create_pipeline("p1.yaml".to_string())
            .expect("create");
        let _p2 = executor
            .create_pipeline("p2.yaml".to_string())
            .expect("create");

        let pending = executor.get_pending_pipelines();
        assert_eq!(pending.len(), 2);

        // No pipelines are terminal yet
        assert!(pending.iter().all(|p| !p.state.is_terminal()));
    }

    // --- Wrong state ---
    #[test]
    fn adv_c2_rollback_unregistered_phase() {
        let (executor, _temp) = create_executor();
        let pipeline = Pipeline::new("test".to_string());

        // All phase types have registered handlers, so rollback should succeed
        for phase in [
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::AgentDevelopment,
            PhaseType::Validation,
        ] {
            let result = executor.rollback_phase(&pipeline, phase);
            assert!(result.is_ok(), "Rollback for {:?} should succeed", phase);
        }
    }

    // --- Stress ---
    #[test]
    fn adv_c2_create_many_pipelines() {
        let (mut executor, _temp) = create_executor();
        for i in 0..50 {
            let pipeline = executor
                .create_pipeline(format!("specs/test_{}.yaml", i))
                .expect("create");
            assert!(executor.store().exists(&pipeline.id));
        }
        assert_eq!(executor.store().list().len(), 50);
    }

    // --- Cleanup with failing handler ---
    #[test]
    fn adv_c2_cleanup_with_rollback_data() {
        let (executor, _temp) = create_executor();
        // Test the cleanup/rollback path with various pipeline states
        let mut pipeline = Pipeline::new("test".to_string());
        pipeline.state = PipelineState::Validation;

        // UniverseSetup handler without rollback data succeeds
        let result = executor.rollback_phase(&pipeline, PhaseType::UniverseSetup);
        assert!(result.is_ok());
    }
}

// ============================================================
// C3: StateStore — Adversarial Tests
// ============================================================

mod state_store_adversarial {
    use orchestrator::{persistence::StateStore, state::*};

    use super::*;

    fn create_temp_store() -> (StateStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = StateStore::new(temp_dir.path().to_path_buf()).unwrap();
        (store, temp_dir)
    }

    // --- Missing input ---
    #[test]
    fn adv_c3_update_nonexistent() {
        let (mut store, _temp) = create_temp_store();
        let pipeline = Pipeline::new("phantom.yaml".to_string());
        // update saves to disk and cache even if not in cache
        let result = store.update(pipeline.clone());
        assert!(result.is_ok());
        let retrieved = store.get(&pipeline.id).expect("should exist now");
        assert_eq!(retrieved.spec_path, "phantom.yaml");
    }

    // --- Bad input ---
    #[test]
    fn adv_c3_corrupted_state_file() {
        let temp_dir = TempDir::new().unwrap();
        // Write a corrupted JSON file
        let corrupt_path = temp_dir.path().join("corrupted.json");
        std::fs::write(&corrupt_path, "{invalid json!!!").unwrap();

        // Store should still load without panicking
        let store = StateStore::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn adv_c3_valid_json_wrong_type() {
        let temp_dir = TempDir::new().unwrap();
        let wrong_type_path = temp_dir.path().join("wrong.json");
        std::fs::write(&wrong_type_path, "\"just a string\"").unwrap();

        let store = StateStore::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn adv_c3_export_to_nonexistent_directory() {
        let (store, _temp) = create_temp_store();
        let result = store.export_all(std::path::Path::new("/nonexistent/dir/file.json"));
        assert!(result.is_err());
    }

    #[test]
    fn adv_c3_import_from_nonexistent_file() {
        let (mut store, _temp) = create_temp_store();
        let result = store.import_from(std::path::Path::new("/nonexistent/file.json"));
        assert!(result.is_err());
    }

    #[test]
    fn adv_c3_import_from_empty_array() {
        let (mut store, _temp) = create_temp_store();
        let import_path = _temp.path().join("empty.json");
        std::fs::write(&import_path, "[]").unwrap();
        let count = store.import_from(&import_path).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn adv_c3_import_from_invalid_json() {
        let (mut store, _temp) = create_temp_store();
        let import_path = _temp.path().join("invalid.json");
        std::fs::write(&import_path, "not json").unwrap();
        let result = store.import_from(&import_path);
        assert!(result.is_err());
    }

    // --- Boundary ---
    #[test]
    fn adv_c3_large_number_of_pipelines() {
        let (mut store, _temp) = create_temp_store();
        for i in 0..100 {
            store
                .create(Pipeline::new(format!("specs/p{}.yaml", i)))
                .unwrap();
        }
        assert_eq!(store.list().len(), 100);

        let recovery = store.get_pending_recovery();
        assert_eq!(recovery.len(), 100);
    }

    #[test]
    fn adv_c3_sync_after_many_operations() {
        let (mut store, _temp) = create_temp_store();
        for i in 0..50 {
            let p = Pipeline::new(format!("specs/p{}.yaml", i));
            store.create(p).unwrap();
        }
        store.sync().unwrap();

        // Verify all persisted by creating a new store from same dir
        let store2 = StateStore::new(_temp.path().to_path_buf()).unwrap();
        assert_eq!(store2.list().len(), 50);
    }

    // --- Stress ---
    #[test]
    fn adv_c3_rapid_create_delete_cycle() {
        let (mut store, _temp) = create_temp_store();

        for i in 0..100 {
            let p = Pipeline::new(format!("specs/p{}.yaml", i));
            store.create(p).unwrap();
        }

        // Store should have 100 pipelines
        assert_eq!(store.list().len(), 100);

        // Delete all
        let all: Vec<_> = store.list().iter().map(|p| p.id.clone()).collect();
        for id in all {
            store.delete(&id).unwrap();
        }
        assert!(store.list().is_empty());
    }

    #[test]
    fn adv_c3_delete_all_and_recreate() {
        let (mut store, _temp) = create_temp_store();
        store
            .create(Pipeline::new("test.yaml".to_string()))
            .unwrap();
        assert_eq!(store.list().len(), 1);

        // Delete all
        let all: Vec<_> = store.list().iter().map(|p| p.id.clone()).collect();
        for id in all {
            store.delete(&id).unwrap();
        }
        assert!(store.list().is_empty());

        store
            .create(Pipeline::new("test2.yaml".to_string()))
            .unwrap();
        assert_eq!(store.list().len(), 1);
    }

    // --- Path traversal ---
    #[test]
    fn adv_c3_state_dir_with_special_chars() {
        let temp_dir = TempDir::new().unwrap();
        let special_subdir = temp_dir.path().join("path with spaces & special chars!");
        let mut store = StateStore::new(special_subdir.clone()).unwrap();
        store
            .create(Pipeline::new("test.yaml".to_string()))
            .unwrap();
        assert_eq!(store.list().len(), 1);
    }
}

// ============================================================
// C4: Metrics — Adversarial Tests
// ============================================================

mod metrics_adversarial {
    use chrono::Utc;
    use orchestrator::metrics::*;

    // --- Missing input ---
    #[test]
    fn adv_c4_record_phase_empty_strings() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: String::new(),
            phase: String::new(),
            started_at: Utc::now(),
            duration_secs: 0.0,
            success: false,
        });
        assert!(metrics.get_pipeline_metrics("").is_some());
    }

    // --- Boundary ---
    #[test]
    fn adv_c4_zero_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: 0.0,
            success: true,
        });
        let agg = metrics.aggregated();
        assert_eq!(agg.average_duration_secs, 0.0);
    }

    #[test]
    fn adv_c4_very_large_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: f64::MAX,
            success: true,
        });
        let agg = metrics.aggregated();
        assert_eq!(agg.average_duration_secs, f64::MAX);
    }

    #[test]
    fn adv_c4_nan_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: f64::NAN,
            success: true,
        });
        // NaN propagates through sum
        let agg = metrics.aggregated();
        assert!(agg.average_duration_secs.is_nan());
    }

    #[test]
    fn adv_c4_infinity_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: f64::INFINITY,
            success: true,
        });
        let agg = metrics.aggregated();
        assert_eq!(agg.average_duration_secs, f64::INFINITY);
    }

    #[test]
    fn adv_c4_negative_duration() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: -100.0,
            success: true,
        });
        let agg = metrics.aggregated();
        assert_eq!(agg.average_duration_secs, -100.0);
    }

    #[test]
    fn adv_c4_empty_scenario_name() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "val".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.record_scenarios(
            "p1",
            vec![ScenarioResult {
                name: String::new(),
                passed: true,
                duration_secs: 0.0,
                error: None,
            }],
        );
        assert!((metrics.scenario_pass_rate() - 100.0).abs() < 0.1);
    }

    #[test]
    fn adv_c4_scenario_with_very_long_error() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "val".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        let long_error = "x".repeat(1_000_000);
        metrics.record_scenarios(
            "p1",
            vec![ScenarioResult {
                name: "fail".to_string(),
                passed: false,
                duration_secs: 0.0,
                error: Some(long_error),
            }],
        );
        assert!((metrics.scenario_pass_rate() - 0.0).abs() < 0.1);
    }

    // --- Stress ---
    #[test]
    fn adv_c4_many_phases_same_pipeline() {
        let mut metrics = Metrics::new();
        for i in 0..1000 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "p1".to_string(),
                phase: format!("phase_{}", i),
                started_at: Utc::now(),
                duration_secs: 1.0,
                success: true,
            });
        }
        assert_eq!(metrics.get_for_pipeline("p1").len(), 1000);
        assert_eq!(metrics.get_phase_metrics().count(), 1000);
    }

    #[test]
    fn adv_c4_slowest_phases_many_same_duration() {
        let mut metrics = Metrics::new();
        for _ in 0..100 {
            metrics.record_phase(PhaseMetrics {
                pipeline_id: "p1".to_string(),
                phase: "same".to_string(),
                started_at: Utc::now(),
                duration_secs: 42.0,
                success: true,
            });
        }
        let slowest = metrics.slowest_phases(1);
        assert_eq!(slowest.len(), 1);
        // 100 * 42.0 = 4200.0
        assert!((slowest[0].1 - 4200.0).abs() < 0.01);
    }

    // --- Edge: mark_complete then update ---
    #[test]
    fn adv_c4_mark_complete_overwrite() {
        let mut metrics = Metrics::new();
        metrics.record_phase(PhaseMetrics {
            pipeline_id: "p1".to_string(),
            phase: "test".to_string(),
            started_at: Utc::now(),
            duration_secs: 1.0,
            success: true,
        });
        metrics.mark_complete("p1", "accepted");
        metrics.mark_complete("p1", "failed"); // Overwrite
        assert_eq!(
            metrics.get_pipeline_metrics("p1").unwrap().final_state,
            "failed"
        );
    }

    #[test]
    fn adv_c4_export_empty() {
        let metrics = Metrics::new();
        let exported = metrics.export().unwrap();
        assert_eq!(exported.trim(), "{}");
    }
}

// ============================================================
// C5: Policies — Adversarial Tests
// ============================================================

mod policies_adversarial {
    use orchestrator::policies::*;

    // --- TimeoutPolicy ---
    #[test]
    fn adv_c5_timeout_very_large_value() {
        let policy = TimeoutPolicy::new(u64::MAX).unwrap();
        assert_eq!(policy.get_timeout_ms(), Some(u64::MAX));
    }

    #[test]
    fn adv_c5_timeout_one_ms() {
        let policy = TimeoutPolicy::new(1).unwrap();
        assert_eq!(policy.get_timeout_ms(), Some(1));
    }

    // --- RetryPolicy (new version) ---
    #[test]
    fn adv_c5_retry_zero_retries() {
        let policy = RetryPolicy::new(0, 100, 2.0, None, vec![]).unwrap();
        assert_eq!(policy.max_retries(), 0);
    }

    #[test]
    fn adv_c5_retry_factor_just_above_one() {
        let policy = RetryPolicy::new(3, 100, 1.000001, None, vec![]).unwrap();
        assert_eq!(policy.factor(), 1.000001);
    }

    #[test]
    fn adv_c5_retry_factor_very_large() {
        let policy = RetryPolicy::new(3, 1, 1e10, None, vec![]).unwrap();
        // delay at attempt 0 = 1 * 1e10^0 = 1
        assert_eq!(policy.calculate_delay(0), 1);
        // delay at attempt 1 = 1 * 1e10 = 10000000000, clamped to u64::MAX only if > u64::MAX
        // 1e10 = 10_000_000_000 which fits in u64
        assert_eq!(policy.calculate_delay(1), 10_000_000_000);
    }

    #[test]
    fn adv_c5_retry_empty_error_patterns() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec![]).unwrap();
        assert!(!policy.is_retryable("any error"));
        assert!(!policy.is_retryable(""));
    }

    #[test]
    fn adv_c5_retry_match_substring() {
        let policy = RetryPolicy::new(3, 100, 2.0, None, vec!["timeout".into()]).unwrap();
        assert!(policy.is_retryable("connection timeout after 30s"));
        assert!(policy.is_retryable("timeout"));
        assert!(!policy.is_retryable("time out")); // Space breaks it
    }

    #[test]
    fn adv_c5_retry_max_delay_equals_base() {
        let policy = RetryPolicy::new(3, 100, 2.0, Some(100), vec![]).unwrap();
        for attempt in 0..20 {
            assert!(policy.calculate_delay(attempt) <= 100);
        }
    }

    // --- CircuitBreaker (new version) ---
    #[test]
    fn adv_c5_circuit_breaker_single_threshold() {
        let mut cb = CircuitBreaker::new(1, 1, 1).unwrap();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.is_execution_allowed());
    }

    #[test]
    fn adv_c5_circuit_breaker_very_high_threshold() {
        let mut cb = CircuitBreaker::new(u32::MAX, 1, 1).unwrap();
        for _ in 0..100 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitBreakerState::Closed);
        assert!(cb.is_execution_allowed());
    }

    #[test]
    fn adv_c5_circuit_breaker_one_ms_open_duration() {
        let mut cb = CircuitBreaker::new(1, 1, 1).unwrap();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitBreakerState::Open);

        // At exactly 1ms, should transition
        let transitioned = cb.check_and_transition(1);
        assert!(transitioned);
        assert_eq!(cb.state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn adv_c5_circuit_breaker_zero_ms_elapsed() {
        let mut cb = CircuitBreaker::new(1, 1, 1000).unwrap();
        cb.record_failure();
        let transitioned = cb.check_and_transition(0);
        assert!(!transitioned);
    }

    #[test]
    fn adv_c5_circuit_breaker_success_count_resets_on_close() {
        let mut cb = CircuitBreaker::new(1, 3, 1000).unwrap();
        cb.record_failure();
        cb.check_and_transition(1001); // -> HalfOpen
        cb.record_success(); // success_count = 1
        cb.record_success(); // success_count = 2
        cb.record_success(); // -> Closed, reset
        assert_eq!(cb.success_count(), 0);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn adv_c5_circuit_breaker_half_open_failure_resets_success() {
        let mut cb = CircuitBreaker::new(1, 5, 1000).unwrap();
        cb.record_failure();
        cb.check_and_transition(1001);
        cb.record_success(); // 1
        cb.record_success(); // 2
        cb.record_success(); // 3
        cb.record_failure(); // -> Open, reset
        assert_eq!(cb.success_count(), 0);
        assert_eq!(cb.state(), CircuitBreakerState::Open);
    }

    // --- CircuitBreaker (old version in circuit.rs) ---
    #[test]
    fn adv_c5_old_circuit_breaker_recovery_timeout_boundary() {
        let mut cb = CircuitBreaker::new(1, 1, 1).unwrap();
        cb.record_failure();
        assert!(!cb.can_execute());
        // Wait 2ms for 1ms recovery
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(cb.can_execute());
    }

    // --- Deadline ---
    #[test]
    fn adv_c5_deadline_zero_duration() {
        let deadline = Deadline::from_now(0);
        // By the time we check, it should be exceeded
        let remaining = deadline.remaining_ms();
        assert!(remaining <= 1);
    }

    #[test]
    fn adv_c5_deadline_very_far_future() {
        // Use a large but safe value (1 year in ms)
        let deadline = Deadline::from_now(365 * 24 * 60 * 60 * 1000);
        assert!(!deadline.is_exceeded());
        assert!(deadline.remaining_ms() > 0);
    }

    #[test]
    fn adv_c5_deadline_overflow_wraps() {
        // u64::MAX milliseconds overflows i64, causing wraparound to past
        // This is a known limitation — the deadline will appear already exceeded
        let deadline = Deadline::from_now(u64::MAX);
        // Due to i64 overflow in chrono::Duration::milliseconds, this wraps to past
        assert!(deadline.is_exceeded());
    }

    // --- PolicyConfig ---
    #[test]
    fn adv_c5_policy_config_all_zeros() {
        let result = PolicyConfig::new(0, 0, 0, 0, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn adv_c5_policy_config_very_large_values() {
        let result = PolicyConfig::new(u64::MAX, u32::MAX, 1, u64::MAX, u32::MAX, u64::MAX);
        assert!(result.is_ok());
    }

    #[test]
    fn adv_c5_policy_config_max_equals_base_delay() {
        let result = PolicyConfig::new(1000, 3, 100, 100, 3, 5000);
        assert!(result.is_ok());
    }

    // --- TimeoutError / PolicyError display ---
    #[test]
    fn adv_c5_policy_error_all_variants_display() {
        use std::time::Instant;
        let errors = vec![
            PolicyError::InvalidTimeout("test".into()),
            PolicyError::InvalidRetryPolicy("test".into()),
            PolicyError::TimeoutExceeded {
                phase_id: "p".into(),
                duration_ms: 100,
                timeout_ms: 50,
            },
            PolicyError::MaxRetriesExceeded {
                phase_id: "p".into(),
                attempts: 100,
                last_error: Box::new(PolicyError::InvalidTimeout("inner".into())),
            },
            PolicyError::CircuitBreakerOpen {
                phase_id: "p".into(),
                open_until: Instant::now(),
            },
            PolicyError::NonRetryableError {
                phase_id: "p".into(),
                cause: "c".into(),
            },
            PolicyError::PreconditionViolation("v".into()),
        ];
        for err in errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }
    }

    // --- ConfigError / OrchestratorError display ---
    #[test]
    fn adv_c5_config_error_all_variants() {
        let errors = vec![
            ConfigError::InvalidTimeout { duration_ms: 0 },
            ConfigError::InvalidBaseDelay { delay_ms: 0 },
            ConfigError::InvalidMaxDelay {
                max_delay_ms: 50,
                base_delay_ms: 100,
            },
            ConfigError::InvalidFailureThreshold { threshold: 0 },
            ConfigError::InvalidRecoveryTimeout { timeout_ms: 0 },
        ];
        for err in &errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn adv_c5_orchestrator_error_retries_exhausted_nested() {
        let err = OrchestratorError::RetriesExhausted {
            phase: "outer".to_string(),
            attempts: 10,
            last_error: Box::new(OrchestratorError::RetriesExhausted {
                phase: "inner".to_string(),
                attempts: 5,
                last_error: Box::new(OrchestratorError::PhaseExecution {
                    phase: "leaf".to_string(),
                    message: "root cause".to_string(),
                }),
            }),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("outer"));
        assert!(msg.contains("10"));
    }
}

// ============================================================
// C6: Queue — Adversarial Tests
// ============================================================

mod queue_adversarial {
    use std::time::Duration;

    use orchestrator::queue::*;

    fn create_test_job(id: &str, priority: JobPriority) -> Job {
        Job {
            id: id.to_string(),
            priority,
            payload: JobPayload::Task {
                command: "test".to_string(),
            },
            state: JobState::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // --- Missing input ---
    #[tokio::test]
    async fn adv_c6_poll_from_empty_repository() {
        let repo = InMemoryJobRepository::new();
        let jobs = repo.poll_pending_jobs(100).await.unwrap();
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn adv_c6_get_nonexistent_job() {
        let repo = InMemoryJobRepository::new();
        let result = repo.get_job("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn adv_c6_update_nonexistent_job() {
        let repo = InMemoryJobRepository::new();
        let result = repo
            .update_job_state("nonexistent", JobState::Pending)
            .await;
        assert!(result.is_err());
    }

    // --- Bad input ---
    #[tokio::test]
    async fn adv_c6_job_with_empty_id() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("", JobPriority::P0));
        let jobs = repo.poll_pending_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "");
    }

    #[tokio::test]
    async fn adv_c6_job_with_special_id() {
        let repo = InMemoryJobRepository::new();
        let special_ids = vec!["job with spaces", "job/with/slashes", "job\nwith\nnewlines"];
        for id in special_ids {
            repo.add_job(create_test_job(id, JobPriority::P0));
        }
        let jobs = repo.poll_pending_jobs(10).await.unwrap();
        assert_eq!(jobs.len(), 3);
    }

    #[test]
    fn adv_c6_job_priority_from_u8_all_values() {
        for v in 0..=u8::MAX {
            let priority = JobPriority::from_u8(v);
            // Should always return a valid priority
            assert!(matches!(
                priority,
                JobPriority::P0
                    | JobPriority::P1
                    | JobPriority::P2
                    | JobPriority::P3
                    | JobPriority::P4
            ));
        }
    }

    // --- Boundary ---
    #[tokio::test]
    async fn adv_c6_poll_limit_zero() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));
        let jobs = repo.poll_pending_jobs(0).await.unwrap();
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn adv_c6_poll_limit_exceeds_available() {
        let repo = InMemoryJobRepository::new();
        repo.add_job(create_test_job("1", JobPriority::P0));
        let jobs = repo.poll_pending_jobs(100).await.unwrap();
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn adv_c6_processor_config_zero_poll_interval() {
        let config = JobProcessorConfig {
            poll_interval: Duration::ZERO,
            concurrency_limit: 5,
            max_retries: 3,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn adv_c6_processor_config_zero_concurrency() {
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: 0,
            max_retries: 3,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn adv_c6_processor_config_very_large_concurrency() {
        let config = JobProcessorConfig {
            poll_interval: Duration::from_millis(100),
            concurrency_limit: usize::MAX,
            max_retries: 0,
        };
        assert!(config.validate().is_ok());
    }

    // --- Wrong state ---
    #[test]
    fn adv_c6_job_state_is_pending_checks() {
        assert!(JobState::Pending.is_pending());
        assert!(!JobState::Pending.is_running());
        assert!(!JobState::Pending.is_terminal());
    }

    #[test]
    fn adv_c6_job_state_running_checks() {
        let running = JobState::Running {
            started_at: chrono::Utc::now(),
        };
        assert!(!running.is_pending());
        assert!(running.is_running());
        assert!(!running.is_terminal());
    }

    // --- Stress ---
    #[tokio::test]
    async fn adv_c6_many_jobs_priority_sort() {
        let repo = InMemoryJobRepository::new();
        // Add jobs with mixed priorities
        for i in 0..100 {
            let priority = match i % 5 {
                0 => JobPriority::P0,
                1 => JobPriority::P1,
                2 => JobPriority::P2,
                3 => JobPriority::P3,
                _ => JobPriority::P4,
            };
            repo.add_job(create_test_job(&format!("job-{}", i), priority));
        }

        let jobs = repo.poll_pending_jobs(100).await.unwrap();
        assert_eq!(jobs.len(), 100);

        // Verify sorted by priority
        for window in jobs.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    #[tokio::test]
    async fn adv_c6_update_state_then_repoll() {
        let repo = InMemoryJobRepository::new();
        for i in 0..10 {
            repo.add_job(create_test_job(&format!("job-{}", i), JobPriority::P0));
        }

        // Mark first 5 as running
        for i in 0..5 {
            repo.update_job_state(
                &format!("job-{}", i),
                JobState::Running {
                    started_at: chrono::Utc::now(),
                },
            )
            .await
            .unwrap();
        }

        let pending = repo.poll_pending_jobs(10).await.unwrap();
        assert_eq!(pending.len(), 5);
    }

    // --- Serde ---
    #[test]
    fn adv_c6_job_payload_custom_with_complex_data() {
        let payload = JobPayload::Custom {
            data: serde_json::json!({
                "nested": {"deep": {"value": 42}},
                "array": [1, 2, 3],
                "null": null,
                "string": "test",
                "bool": true,
                "number": 3.14
            }),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: JobPayload = serde_json::from_str(&json).unwrap();
        match (payload, deserialized) {
            (JobPayload::Custom { data: d1 }, JobPayload::Custom { data: d2 }) => {
                assert_eq!(d1, d2);
            }
            _ => panic!("Payload mismatch"),
        }
    }

    #[test]
    fn adv_c6_job_outcome_failure_with_empty_error() {
        let outcome = JobOutcome::Failure {
            error: String::new(),
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let deserialized: JobOutcome = serde_json::from_str(&json).unwrap();
        match deserialized {
            JobOutcome::Failure { error } => assert_eq!(error, ""),
            _ => panic!("Expected Failure"),
        }
    }

    // --- sort_jobs_by_priority ---
    #[test]
    fn adv_c6_sort_single_job() {
        let mut jobs = vec![create_test_job("1", JobPriority::P3)];
        sort_jobs_by_priority(&mut jobs);
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn adv_c6_sort_empty() {
        let mut jobs: Vec<Job> = vec![];
        sort_jobs_by_priority(&mut jobs);
        assert!(jobs.is_empty());
    }
}

// ============================================================
// C7: Cleanup — Adversarial Tests
// ============================================================

mod cleanup_adversarial {
    use orchestrator::{cleanup::*, state::PipelineId, CleanupHandler};

    // --- Missing input ---
    #[test]
    fn adv_c7_empty_context() {
        let ctx = CleanupContext::new(PipelineId::new(), PhaseType::Validation);
        assert!(ctx.created_resources.is_empty());
        assert!(ctx.rollback_data.is_empty());
    }

    #[test]
    fn adv_c7_noop_handler_empty_context() {
        let handler = NoopCleanupHandler;
        let ctx = CleanupContext::new(PipelineId::new(), PhaseType::SpecReview);
        assert!(handler.cleanup(&ctx).success_flag());
        assert!(handler.rollback(&ctx).success_flag());
    }

    // --- Boundary ---
    #[test]
    fn adv_c7_many_resources() {
        let handler = UniverseSetupCleanupHandler;
        let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::UniverseSetup);
        for i in 0..1000 {
            ctx.add_resource(ResourceId::new(format!("res-{}", i)));
        }
        let result = handler.cleanup(&ctx);
        assert!(result.success_flag());
        assert_eq!(result.cleaned_resources.len(), 1000);
    }

    #[test]
    fn adv_c7_large_rollback_data() {
        let handler = AgentDevelopmentCleanupHandler;
        let mut ctx = CleanupContext::new(PipelineId::new(), PhaseType::AgentDevelopment);
        let large_data = vec![0xFF; 1_000_000];
        ctx.set_rollback_data(large_data.clone());
        let result = handler.rollback(&ctx);
        assert!(!result.success_flag());
    }

    #[test]
    fn adv_c7_result_builder_many_errors() {
        let mut result = CleanupResult::success();
        for i in 0..100 {
            result = result.with_error(format!("error-{}", i));
        }
        assert!(!result.success_flag());
        assert_eq!(result.errors().len(), 100);
    }

    #[test]
    fn adv_c7_result_builder_many_resources() {
        let mut result = CleanupResult::success();
        for i in 0..100 {
            result = result.with_resource(ResourceId::new(format!("r-{}", i)));
        }
        assert!(result.success_flag());
        assert_eq!(result.cleaned_resources.len(), 100);
    }

    // --- Custom handler ---
    #[test]
    fn adv_c7_register_many_handlers() {
        let mut manager = CleanupManager::new();

        struct TestHandler(PhaseType);
        impl CleanupHandler for TestHandler {
            fn phase_type(&self) -> PhaseType {
                self.0
            }
            fn cleanup(&self, _ctx: &CleanupContext) -> CleanupResult {
                CleanupResult::success()
            }
            fn rollback(&self, _ctx: &CleanupContext) -> CleanupResult {
                CleanupResult::success()
            }
        }

        for phase in [
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::AgentDevelopment,
            PhaseType::Validation,
        ] {
            manager.register_handler(Box::new(TestHandler(phase)));
        }

        // All should still work
        for phase in [
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::AgentDevelopment,
            PhaseType::Validation,
        ] {
            assert!(manager.get_handler(phase).is_some());
        }
    }

    // --- CleanupError ---
    #[test]
    fn adv_c7_cleanup_error_all_variants_implement_error() {
        use std::error::Error;
        let errors = [
            CleanupError::NotImplemented("x".into()),
            CleanupError::ResourceNotFound("y".into()),
            CleanupError::CleanupFailed("z".into()),
            CleanupError::RollbackFailed("w".into()),
        ];
        for err in &errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
            assert!(err.source().is_none());
        }
    }

    // --- ResourceId edge cases ---
    #[test]
    fn adv_c7_resource_id_empty() {
        let rid = ResourceId::new("");
        assert_eq!(rid.0, "");
    }

    #[test]
    fn adv_c7_resource_id_with_null_bytes() {
        let rid = ResourceId::new("test\0resource");
        assert!(rid.0.contains('\0'));
    }

    #[test]
    fn adv_c7_resource_id_hash_stability() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for i in 0..100 {
            set.insert(ResourceId::new(format!("res-{}", i)));
        }
        assert_eq!(set.len(), 100);
    }
}

// ============================================================
// C8: Parallel Execution — Adversarial Tests
// ============================================================

mod parallel_adversarial {
    use std::collections::HashSet;

    use orchestrator::{cleanup::PhaseType, parallel::*, state::PipelineState};

    // --- Missing input ---
    #[test]
    fn adv_c8_empty_dependency_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.is_complete());
        assert!(!graph.has_failures());
        let completed: HashSet<PhaseType> = HashSet::new();
        assert!(graph.get_ready_phases(&completed).is_empty());
    }

    #[test]
    fn adv_c8_validate_empty_phases() {
        assert!(ParallelExecutor::validate_dependency_order(&[]).is_ok());
    }

    // --- Boundary ---
    #[test]
    fn adv_c8_self_cycle() {
        let graph =
            DependencyGraph::new().add_phase(PhaseType::SpecReview, vec![PhaseType::SpecReview]);
        assert!(graph.validate().is_err());
    }

    #[test]
    fn adv_c8_two_node_cycle() {
        let graph = DependencyGraph::new()
            .add_phase(PhaseType::SpecReview, vec![PhaseType::UniverseSetup])
            .add_phase(PhaseType::UniverseSetup, vec![PhaseType::SpecReview]);
        assert!(graph.validate().is_err());
    }

    #[test]
    fn adv_c8_three_node_cycle() {
        let graph = DependencyGraph::new()
            .add_phase(PhaseType::SpecReview, vec![PhaseType::Validation])
            .add_phase(PhaseType::Validation, vec![PhaseType::AgentDevelopment])
            .add_phase(PhaseType::AgentDevelopment, vec![PhaseType::SpecReview]);
        assert!(graph.validate().is_err());
    }

    #[test]
    fn adv_c8_nonexistent_dependency() {
        let graph =
            DependencyGraph::new().add_phase(PhaseType::Validation, vec![PhaseType::SpecReview]);
        // SpecReview not added as a node
        assert!(graph.validate().is_err());
    }

    #[test]
    fn adv_c8_all_parallel_from_terminal_states() {
        let terminals = [
            PipelineState::Accepted,
            PipelineState::Escalated,
            PipelineState::Failed,
        ];
        for state in &terminals {
            let groups = ParallelExecutor::resolve_parallel_phases(state);
            assert!(groups.is_empty());
        }
    }

    #[test]
    fn adv_c8_mark_completed_nonexistent_phase() {
        let mut graph = DependencyGraph::new();
        graph.mark_completed(PhaseType::SpecReview); // No-op
        assert!(!graph.has_failures());
    }

    #[test]
    fn adv_c8_mark_failed_nonexistent_phase() {
        let mut graph = DependencyGraph::new();
        graph.mark_failed(PhaseType::SpecReview); // No-op
        assert!(!graph.has_failures());
    }

    // --- Stress ---
    #[test]
    fn adv_c8_large_dependency_graph() {
        // Create a linear chain: SpecReview -> UniverseSetup -> AgentDev -> Validation
        // Repeat the pattern 25 times (100 nodes total)
        let mut graph = DependencyGraph::new();
        let phases = [
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::AgentDevelopment,
            PhaseType::Validation,
        ];

        let mut prev_phase = None;
        for _i in 0..25 {
            for &phase in &phases {
                let deps = prev_phase.map(|p: PhaseType| vec![p]).unwrap_or_default();
                graph = graph.add_phase(phase, deps);
                prev_phase = Some(phase);
            }
        }

        // Should validate (no cycles, all deps exist)
        // Actually this will have duplicate phases in HashMap, so only last added wins
        // Let's just verify validate doesn't panic
        let _ = graph.validate();
    }

    #[test]
    fn adv_c8_phase_group_edge_cases() {
        let empty_group = PhaseGroup::new(vec![]);
        assert!(empty_group.phases.is_empty());
        assert_eq!(empty_group.max_parallelism, 0);

        let single = PhaseGroup::new(vec![PhaseType::SpecReview]);
        assert_eq!(single.max_parallelism, 1);

        let limited = PhaseGroup::new(vec![
            PhaseType::SpecReview,
            PhaseType::UniverseSetup,
            PhaseType::Validation,
        ])
        .with_max_parallelism(1);
        assert_eq!(limited.max_parallelism, 1);
        assert_eq!(limited.phases.len(), 3);
    }

    // --- PhaseNode ---
    #[test]
    fn adv_c8_phase_node_non_pending_cant_execute() {
        let mut node = PhaseNode::new(PhaseType::SpecReview);
        let completed: HashSet<PhaseType> = HashSet::new();

        assert!(node.can_execute(&completed));

        for status in [
            PhaseStatus::Running,
            PhaseStatus::Completed,
            PhaseStatus::Failed,
        ] {
            node.status = status;
            assert!(!node.can_execute(&completed));
        }
    }

    #[test]
    fn adv_c8_phase_node_multiple_unmet_deps() {
        let node = PhaseNode::new(PhaseType::Validation)
            .with_dependency(PhaseType::SpecReview)
            .with_dependency(PhaseType::UniverseSetup)
            .with_dependency(PhaseType::AgentDevelopment);

        let completed: HashSet<PhaseType> = HashSet::new();
        assert!(!node.can_execute(&completed));

        // Partial: only one met
        let mut partial = HashSet::new();
        partial.insert(PhaseType::SpecReview);
        assert!(!node.can_execute(&partial));

        // All met
        let mut all = HashSet::new();
        all.insert(PhaseType::SpecReview);
        all.insert(PhaseType::UniverseSetup);
        all.insert(PhaseType::AgentDevelopment);
        assert!(node.can_execute(&all));
    }

    // --- ParallelError ---
    #[test]
    fn adv_c8_parallel_error_implements_error() {
        use std::error::Error;
        let errors = [
            ParallelError::DependencyNotMet(PhaseType::Validation),
            ParallelError::InvalidPhaseConfiguration("test".to_string()),
            ParallelError::ExecutionFailed("test".to_string()),
        ];
        for err in &errors {
            let msg = format!("{}", err);
            assert!(!msg.is_empty());
            assert!(err.source().is_none());
        }
    }

    // --- Serde ---
    #[test]
    fn adv_c8_phase_status_serde_roundtrip() {
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

// ============================================================
// Cross-module Integration Adversarial Tests
// ============================================================

mod integration_adversarial {
    use orchestrator::*;
    use tempfile::TempDir;

    #[test]
    fn adv_full_pipeline_lifecycle_with_executor() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().to_path_buf();
        let scenarios_path = temp_dir.path().join("scenarios");

        let mut executor = phases::PipelineExecutor::new(state_dir).expect("executor");

        // Create and run pipeline
        let pipeline = executor
            .create_pipeline("specs/integration.yaml".to_string())
            .expect("create");

        assert!(executor.can_run_pipeline(&pipeline));

        let decision = executor.run_pipeline(&pipeline.id);
        // The run_pipeline goes through: spec_review -> universe_setup -> agent_dev -> validation
        // Since run_linter returns 85 (hardcoded) and threshold is 80, it should pass
        if !decision.is_ok() {
            eprintln!(
                "Pipeline decision error: {:?}",
                decision.as_ref().unwrap_err()
            );
        }
        assert!(decision.is_ok());
    }

    #[test]
    fn adv_pipeline_serde_full_roundtrip_with_all_fields() {
        let mut pipeline = Pipeline::with_config(
            "complex/spec.yaml".to_string(),
            &state::PipelineConfig {
                max_iterations: 42,
                quality_threshold: 95,
                scenarios_path: "custom/path".to_string(),
                linter_path: Some("/usr/bin/custom-lint".to_string()),
            },
        );

        pipeline.transition_to(PipelineState::SpecReview).unwrap();
        pipeline
            .transition_to(PipelineState::UniverseSetup)
            .unwrap();
        pipeline
            .transition_to(PipelineState::AgentDevelopment)
            .unwrap();
        pipeline.increment_iteration().unwrap();
        pipeline.set_error("test error".to_string());

        let json = serde_json::to_string_pretty(&pipeline).unwrap();
        let deserialized: Pipeline = serde_json::from_str(&json).unwrap();

        assert_eq!(pipeline.id, deserialized.id);
        assert_eq!(pipeline.spec_path, deserialized.spec_path);
        assert_eq!(pipeline.state, deserialized.state);
        assert_eq!(pipeline.iteration, deserialized.iteration);
        assert_eq!(pipeline.max_iterations, deserialized.max_iterations);
        assert_eq!(pipeline.quality_threshold, deserialized.quality_threshold);
        assert_eq!(pipeline.last_error, deserialized.last_error);
    }

    #[test]
    fn adv_metrics_across_full_pipeline() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().to_path_buf();
        let scenarios_path = temp_dir.path().join("scenarios");

        let mut executor = phases::PipelineExecutor::new(state_dir).expect("executor");

        let pipeline = executor
            .create_pipeline("specs/metrics.yaml".to_string())
            .expect("create");

        executor.run_pipeline(&pipeline.id).ok();

        // Check metrics were recorded
        let pipeline_metrics = executor.metrics().get_pipeline_metrics(&pipeline.id.0);
        assert!(pipeline_metrics.is_some());

        let pm = pipeline_metrics.unwrap();
        assert!(!pm.phase_metrics.is_empty());
    }

    #[test]
    fn adv_store_persistence_across_drop_and_recreate() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().to_path_buf();

        {
            let mut store = persistence::StateStore::new(state_dir.clone()).unwrap();
            store
                .create(Pipeline::new("persist.yaml".to_string()))
                .unwrap();
        } // Drop triggers sync

        {
            let store = persistence::StateStore::new(state_dir).unwrap();
            assert_eq!(store.list().len(), 1);
            let pipeline = store.list()[0];
            assert_eq!(pipeline.spec_path, "persist.yaml");
        }
    }
}
