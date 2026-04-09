//! Comprehensive tests for StateStore — save/load round-trip,
//! persistence across store instances, not-found handling,
//! and concurrent read/write patterns.

use std::fs;
use std::sync::Arc;
use std::thread;

use tempfile::TempDir;

use crate::persistence::{StateStore, StoreError};
use crate::state::{Pipeline, PipelineConfig, PipelineId, PipelineState};

fn create_temp_store() -> (StateStore, TempDir) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let store = StateStore::new(temp_dir.path().to_path_buf()).expect("failed to create store");
    (store, temp_dir)
}

fn make_pipeline(spec: &str) -> Pipeline {
    Pipeline::new(spec.to_string())
}

fn advance_to_state(pipeline: &mut Pipeline, target: PipelineState) {
    let transitions = match target {
        PipelineState::Pending => vec![],
        PipelineState::SpecReview => vec![PipelineState::SpecReview],
        PipelineState::UniverseSetup => {
            vec![PipelineState::SpecReview, PipelineState::UniverseSetup]
        }
        PipelineState::AgentDevelopment => vec![
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
        ],
        PipelineState::Validation => vec![
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
        ],
        PipelineState::Accepted => vec![
            PipelineState::SpecReview,
            PipelineState::UniverseSetup,
            PipelineState::AgentDevelopment,
            PipelineState::Validation,
            PipelineState::Accepted,
        ],
        PipelineState::Escalated => vec![PipelineState::Escalated],
        PipelineState::Failed => vec![PipelineState::Failed],
    };
    for t in transitions {
        pipeline
            .transition_to(t)
            .expect("transition should succeed");
    }
}

// ============================================================
// Section 1: Save/Load Round-Trip
// ============================================================

#[test]
fn save_load_roundtrip_preserves_spec_path() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/round-trip.yaml");
    let id = pipeline.id.clone();

    store.create(pipeline).expect("create should succeed");
    let loaded = store.get(&id).expect("get should succeed");
    assert_eq!(loaded.spec_path, "specs/round-trip.yaml");
}

#[test]
fn save_load_roundtrip_preserves_state() {
    let (mut store, _temp) = create_temp_store();
    let mut pipeline = make_pipeline("specs/state-check.yaml");
    advance_to_state(&mut pipeline, PipelineState::AgentDevelopment);
    let id = pipeline.id.clone();

    store.create(pipeline).expect("create should succeed");
    let loaded = store.get(&id).expect("get should succeed");
    assert_eq!(loaded.state, PipelineState::AgentDevelopment);
}

#[test]
fn save_load_roundtrip_preserves_iteration_count() {
    let (mut store, _temp) = create_temp_store();
    let mut pipeline = make_pipeline("specs/iter.yaml");
    advance_to_state(&mut pipeline, PipelineState::AgentDevelopment);
    for _ in 0..5 {
        pipeline.increment_iteration().expect("increment");
    }
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.iteration, 5);
}

#[test]
fn save_load_roundtrip_preserves_max_iterations() {
    let (mut store, _temp) = create_temp_store();
    let config = PipelineConfig {
        max_iterations: 42,
        quality_threshold: 80,
        scenarios_path: "scenarios".to_string(),
        linter_path: None,
    };
    let pipeline = Pipeline::with_config("specs/max-iter.yaml".to_string(), &config);
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.max_iterations, 42);
}

#[test]
fn save_load_roundtrip_preserves_quality_threshold() {
    let (mut store, _temp) = create_temp_store();
    let config = PipelineConfig {
        max_iterations: 10,
        quality_threshold: 95,
        scenarios_path: "scenarios".to_string(),
        linter_path: None,
    };
    let pipeline = Pipeline::with_config("specs/quality.yaml".to_string(), &config);
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.quality_threshold, 95);
}

#[test]
fn save_load_roundtrip_preserves_last_error() {
    let (mut store, _temp) = create_temp_store();
    let mut pipeline = make_pipeline("specs/err.yaml");
    pipeline.set_error("catastrophic failure".to_string());
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.last_error.as_deref(), Some("catastrophic failure"));
}

#[test]
fn save_load_roundtrip_preserves_nil_last_error() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/no-err.yaml");
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert!(loaded.last_error.is_none());
}

#[test]
fn save_load_roundtrip_preserves_created_at() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/ts.yaml");
    let expected_created = pipeline.created_at;
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.created_at, expected_created);
}

#[test]
fn save_load_roundtrip_after_update_preserves_new_state() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/update-rt.yaml");
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let p = store.get_mut(&id).expect("get_mut");
    advance_to_state(p, PipelineState::SpecReview);
    let _ = p;

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.state, PipelineState::SpecReview);
}

#[test]
fn save_load_roundtrip_for_each_pipeline_state() {
    let all_states = [
        PipelineState::Pending,
        PipelineState::SpecReview,
        PipelineState::UniverseSetup,
        PipelineState::AgentDevelopment,
        PipelineState::Validation,
        PipelineState::Accepted,
        PipelineState::Escalated,
        PipelineState::Failed,
    ];

    for state in &all_states {
        let (mut store, _temp) = create_temp_store();
        let mut pipeline = make_pipeline("specs/all-states.yaml");
        advance_to_state(&mut pipeline, *state);
        let id = pipeline.id.clone();

        store.create(pipeline).expect("create");
        let loaded = store.get(&id).expect("get");
        assert_eq!(
            loaded.state, *state,
            "round-trip should preserve state {:?}",
            state
        );
    }
}

#[test]
fn save_load_roundtrip_with_multiple_pipelines() {
    let (mut store, _temp) = create_temp_store();
    let mut ids = Vec::new();

    for i in 0..10 {
        let mut p = make_pipeline(&format!("specs/multi-{i}.yaml"));
        advance_to_state(
            &mut p,
            match i % 4 {
                0 => PipelineState::Pending,
                1 => PipelineState::SpecReview,
                2 => PipelineState::AgentDevelopment,
                _ => PipelineState::Validation,
            },
        );
        ids.push(p.id.clone());
        store.create(p).expect("create");
    }

    for (i, id) in ids.iter().enumerate() {
        let loaded = store.get(id).expect("get");
        assert_eq!(
            loaded.spec_path,
            format!("specs/multi-{i}.yaml"),
            "pipeline {} should have correct spec_path",
            i
        );
    }
}

#[test]
fn update_then_get_roundtrip_preserves_changes() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/up-down.yaml");
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    let p = store.get_mut(&id).expect("get_mut");
    advance_to_state(p, PipelineState::Validation);
    for _ in 0..3 {
        let _ = p.increment_iteration();
    }
    p.set_error("validation failed".to_string());
    let _ = p;

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.state, PipelineState::Validation);
    assert_eq!(loaded.iteration, 3);
    assert_eq!(loaded.last_error.as_deref(), Some("validation failed"));
}

// ============================================================
// Section 2: Persistence Across Store Instances
// ============================================================

#[test]
fn persistence_across_instances_single_pipeline() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let mut pipeline = make_pipeline("specs/persist-1.yaml");
        advance_to_state(&mut pipeline, PipelineState::SpecReview);
        let id = pipeline.id.clone();
        store1.create(pipeline).expect("create");
        store1.sync().expect("sync");
        drop(store1);
        id
    };

    let store2 = StateStore::new(path.clone()).expect("store2");
    let loaded = store2
        .get(&id)
        .expect("pipeline should persist across instances");
    assert_eq!(loaded.spec_path, "specs/persist-1.yaml");
    assert_eq!(loaded.state, PipelineState::SpecReview);
}

#[test]
fn persistence_across_instances_preserves_all_fields() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let config = PipelineConfig {
        max_iterations: 25,
        quality_threshold: 99,
        scenarios_path: "custom/scenarios".to_string(),
        linter_path: Some("/usr/bin/lint".to_string()),
    };

    let (id, created_at) = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let mut pipeline = Pipeline::with_config("specs/full-persist.yaml".to_string(), &config);
        advance_to_state(&mut pipeline, PipelineState::AgentDevelopment);
        for _ in 0..7 {
            pipeline.increment_iteration().expect("inc");
        }
        pipeline.set_error("intermediate error".to_string());
        let id = pipeline.id.clone();
        let created_at = pipeline.created_at;
        store1.create(pipeline).expect("create");
        store1.sync().expect("sync");
        drop(store1);
        (id, created_at)
    };

    let store2 = StateStore::new(path.clone()).expect("store2");
    let loaded = store2.get(&id).expect("get");
    assert_eq!(loaded.spec_path, "specs/full-persist.yaml");
    assert_eq!(loaded.state, PipelineState::AgentDevelopment);
    assert_eq!(loaded.iteration, 7);
    assert_eq!(loaded.max_iterations, 25);
    assert_eq!(loaded.quality_threshold, 99);
    assert_eq!(loaded.last_error.as_deref(), Some("intermediate error"));
    assert_eq!(loaded.created_at, created_at);
}

#[test]
fn persistence_across_instances_multiple_pipelines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let ids = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let mut ids = Vec::new();
        for i in 0..5 {
            let mut p = make_pipeline(&format!("specs/persist-multi-{i}.yaml"));
            advance_to_state(&mut p, PipelineState::UniverseSetup);
            ids.push(p.id.clone());
            store1.create(p).expect("create");
        }
        store1.sync().expect("sync");
        drop(store1);
        ids
    };

    let store2 = StateStore::new(path.clone()).expect("store2");
    assert_eq!(store2.list().len(), 5);
    for (i, id) in ids.iter().enumerate() {
        let loaded = store2.get(id).expect("get");
        assert_eq!(loaded.spec_path, format!("specs/persist-multi-{i}.yaml"));
    }
}

#[test]
fn persistence_after_delete_removes_from_disk() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let pipeline = make_pipeline("specs/to-delete.yaml");
        let id = pipeline.id.clone();
        store1.create(pipeline).expect("create");
        store1.sync().expect("sync");
        drop(store1);
        id
    };

    {
        let mut store2 = StateStore::new(path.clone()).expect("store2");
        assert!(store2.exists(&id));
        store2.delete(&id).expect("delete");
        drop(store2);
    }

    let store3 = StateStore::new(path.clone()).expect("store3");
    assert!(!store3.exists(&id));
    assert!(store3.list().is_empty());
}

#[test]
fn persistence_after_update_reflects_new_state() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let pipeline = make_pipeline("specs/updated.yaml");
        let id = pipeline.id.clone();
        store1.create(pipeline).expect("create");

        let p = store1.get_mut(&id).expect("get_mut");
        advance_to_state(p, PipelineState::Accepted);
        let _ = p;

        store1.sync().expect("sync");
        drop(store1);
        id
    };

    let store2 = StateStore::new(path.clone()).expect("store2");
    let loaded = store2.get(&id).expect("get");
    assert_eq!(loaded.state, PipelineState::Accepted);
}

#[test]
fn persistence_file_format_is_valid_json() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store1 = StateStore::new(path.clone()).expect("store1");
        let pipeline = make_pipeline("specs/json-check.yaml");
        let id = pipeline.id.clone();
        store1.create(pipeline).expect("create");
        store1.sync().expect("sync");
        drop(store1);
        id
    };

    let file_path = path.join(format!("{}.json", id.0));
    let content = fs::read_to_string(&file_path).expect("read file");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse json");
    assert!(parsed.is_object());
    assert_eq!(parsed["spec_path"], "specs/json-check.yaml");
}

#[test]
fn persistence_empty_dir_opens_clean_store() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let store = StateStore::new(path.clone()).expect("store");
    assert!(store.list().is_empty());
}

#[test]
fn persistence_drop_auto_syncs() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store = StateStore::new(path.clone()).expect("store");
        let pipeline = make_pipeline("specs/auto-sync.yaml");
        let id = pipeline.id.clone();
        store.create(pipeline).expect("create");
        // No explicit sync — Drop should handle it
        id
    };

    let store2 = StateStore::new(path.clone()).expect("store2");
    let loaded = store2.get(&id).expect("should survive drop auto-sync");
    assert_eq!(loaded.spec_path, "specs/auto-sync.yaml");
}

// ============================================================
// Section 3: Not-Found Handling
// ============================================================

#[test]
fn get_returns_not_found_error_variant_for_missing_id() {
    let (store, _temp) = create_temp_store();
    let missing_id = PipelineId("does-not-exist".to_string());

    let result = store.get(&missing_id);
    assert!(result.is_err());
    let err = result.err().expect("should have error");
    assert!(
        matches!(err, StoreError::NotFound(ref id) if id == "does-not-exist"),
        "expected NotFound error with correct id, got {:?}",
        err
    );
}

#[test]
fn get_returns_not_found_with_uuid_style_id() {
    let (store, _temp) = create_temp_store();
    let missing_id = PipelineId::new();

    let result = store.get(&missing_id);
    assert!(result.is_err());
    let err = result.err().expect("error");
    assert!(matches!(err, StoreError::NotFound(_)));
}

#[test]
fn get_mut_returns_not_found_for_missing_id() {
    let (mut store, _temp) = create_temp_store();
    let missing_id = PipelineId("ghost".to_string());

    let result = store.get_mut(&missing_id);
    assert!(result.is_err());
    let err = result.err().expect("error");
    assert!(
        matches!(err, StoreError::NotFound(ref id) if id == "ghost"),
        "expected NotFound for get_mut, got {:?}",
        err
    );
}

#[test]
fn delete_returns_not_found_for_missing_id() {
    let (mut store, _temp) = create_temp_store();
    let missing_id = PipelineId("phantom".to_string());

    let result = store.delete(&missing_id);
    assert!(result.is_err());
    let err = result.err().expect("error");
    assert!(
        matches!(err, StoreError::NotFound(ref id) if id == "phantom"),
        "expected NotFound for delete, got {:?}",
        err
    );
}

#[test]
fn not_found_error_message_contains_id() {
    let (store, _temp) = create_temp_store();
    let missing_id = PipelineId("error-msg-check".to_string());

    let err = store.get(&missing_id).err().expect("error");
    let msg = err.to_string();
    assert!(
        msg.contains("error-msg-check"),
        "error message should contain the id, got: {}",
        msg
    );
}

#[test]
fn exists_returns_false_for_missing_id() {
    let (store, _temp) = create_temp_store();
    assert!(!store.exists(&PipelineId("nope".to_string())));
}

#[test]
fn not_found_after_delete() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/temp.yaml");
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");
    store.delete(&id).expect("delete");

    let result = store.get(&id);
    assert!(result.is_err());
    assert!(matches!(
        result.err().expect("error"),
        StoreError::NotFound(_)
    ));
}

#[test]
fn get_after_clear_returns_not_found() {
    let (mut store, _temp) = create_temp_store();
    let pipeline = make_pipeline("specs/clear-me.yaml");
    let id = pipeline.id.clone();
    store.create(pipeline).expect("create");

    store.clear().expect("clear");

    let result = store.get(&id);
    assert!(result.is_err());
    assert!(matches!(
        result.err().expect("error"),
        StoreError::NotFound(_)
    ));
}

// ============================================================
// Section 4: Concurrent Read/Write Patterns
// ============================================================

#[test]
fn concurrent_reads_see_consistent_data() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let id = {
        let mut store = StateStore::new(path.clone()).expect("store");
        let pipeline = make_pipeline("specs/concurrent.yaml");
        let id = pipeline.id.clone();
        store.create(pipeline).expect("create");
        store.sync().expect("sync");
        drop(store);
        id
    };

    let path = Arc::new(path);
    let id = Arc::new(id);
    let mut handles = Vec::new();

    for i in 0..8 {
        let p = Arc::clone(&path);
        let id_clone = Arc::clone(&id);
        handles.push(thread::spawn(move || {
            let store = StateStore::new((*p).clone()).expect("store in thread");
            let loaded = store.get(&id_clone).expect("get in thread");
            assert_eq!(loaded.spec_path, "specs/concurrent.yaml", "thread {}", i);
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }
}

#[test]
fn concurrent_writes_to_different_pipelines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = Arc::new(temp_dir.path().to_path_buf());
    let mut handles = Vec::new();

    for i in 0..8 {
        let p = Arc::clone(&path);
        handles.push(thread::spawn(move || {
            let mut store = StateStore::new((*p).clone()).expect("store");
            let pipeline = make_pipeline(&format!("specs/conc-write-{i}.yaml"));
            let id = pipeline.id.clone();
            store.create(pipeline).expect("create");
            let loaded = store.get(&id).expect("get");
            assert_eq!(
                loaded.spec_path,
                format!("specs/conc-write-{i}.yaml"),
                "thread {}",
                i
            );
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }
}

#[test]
fn concurrent_create_and_separate_read() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = Arc::new(temp_dir.path().to_path_buf());

    let mut ids: Vec<Arc<PipelineId>> = Vec::new();
    for _ in 0..4 {
        let p = make_pipeline("specs/create-read.yaml");
        ids.push(Arc::new(p.id.clone()));
    }

    let mut handles = Vec::new();
    for (i, id) in ids.into_iter().enumerate() {
        let p = Arc::clone(&path);
        handles.push(thread::spawn(move || {
            let mut store = StateStore::new((*p).clone()).expect("store");
            let pipeline = make_pipeline(&format!("specs/create-read-{i}.yaml"));
            let arc_id = Arc::clone(&id);
            let mut pipeline = pipeline;
            let pipeline_id = (*arc_id).clone();
            pipeline.id = pipeline_id;
            store.create(pipeline).expect("create");
            drop(store);

            let store2 = StateStore::new((*p).clone()).expect("store2");
            let loaded = store2.get(&arc_id).expect("get");
            assert_eq!(loaded.spec_path, format!("specs/create-read-{i}.yaml"));
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }
}

#[test]
fn concurrent_delete_different_pipelines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let ids: Vec<PipelineId> = {
        let mut store = StateStore::new(path.clone()).expect("store");
        let mut ids = Vec::new();
        for i in 0..8 {
            let p = make_pipeline(&format!("specs/del-conc-{i}.yaml"));
            ids.push(p.id.clone());
            store.create(p).expect("create");
        }
        store.sync().expect("sync");
        drop(store);
        ids
    };

    for id in &ids {
        let mut store = StateStore::new(path.clone()).expect("store");
        store.delete(id).expect("delete");
    }

    let final_store = StateStore::new(path.clone()).expect("final store");
    assert_eq!(final_store.list().len(), 0);
}

// ============================================================
// Section 5: Export/Import Round-Trip
// ============================================================

#[test]
fn export_import_roundtrip_preserves_all_pipelines() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let mut store1 = StateStore::new(path.clone()).expect("store1");
    let mut ids = Vec::new();
    for i in 0..5 {
        let mut p = make_pipeline(&format!("specs/export-{i}.yaml"));
        advance_to_state(&mut p, PipelineState::Validation);
        ids.push((p.id.clone(), p.spec_path.clone(), p.state));
        store1.create(p).expect("create");
    }

    let export_path = temp_dir.path().join("export.json");
    store1.export_all(&export_path).expect("export");

    let temp_dir2 = TempDir::new().expect("temp dir 2");
    let mut store2 = StateStore::new(temp_dir2.path().to_path_buf()).expect("store2");
    let count = store2.import_from(&export_path).expect("import");

    assert_eq!(count, 5);
    assert_eq!(store2.list().len(), 5);

    for (id, spec_path, state) in &ids {
        let loaded = store2.get(id).expect("get after import");
        assert_eq!(loaded.spec_path, *spec_path);
        assert_eq!(loaded.state, *state);
    }
}

#[test]
fn export_import_roundtrip_single_pipeline() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let mut store1 = StateStore::new(path.clone()).expect("store1");
    let mut pipeline = make_pipeline("specs/single-export.yaml");
    advance_to_state(&mut pipeline, PipelineState::AgentDevelopment);
    for _ in 0..3 {
        pipeline.increment_iteration().expect("inc");
    }
    let id = pipeline.id.clone();
    store1.create(pipeline).expect("create");

    let export_path = temp_dir.path().join("single.json");
    store1.export_all(&export_path).expect("export");

    let temp_dir2 = TempDir::new().expect("temp dir 2");
    let mut store2 = StateStore::new(temp_dir2.path().to_path_buf()).expect("store2");
    store2.import_from(&export_path).expect("import");

    let loaded = store2.get(&id).expect("get");
    assert_eq!(loaded.spec_path, "specs/single-export.yaml");
    assert_eq!(loaded.state, PipelineState::AgentDevelopment);
    assert_eq!(loaded.iteration, 3);
}

// ============================================================
// Section 6: Edge Cases
// ============================================================

#[test]
fn store_creates_state_dir_if_missing() {
    let temp_dir = TempDir::new().expect("temp dir");
    let nested = temp_dir.path().join("a").join("b").join("c");
    let store = StateStore::new(nested.clone()).expect("store should create nested dirs");
    assert!(nested.exists());
    assert!(store.list().is_empty());
}

#[test]
fn create_overwrites_existing_pipeline_with_same_id() {
    let (mut store, _temp) = create_temp_store();
    let id = PipelineId("overwrite-test".to_string());

    let mut p1 = make_pipeline("specs/original.yaml");
    p1.id = id.clone();
    store.create(p1).expect("create p1");

    let mut p2 = make_pipeline("specs/replaced.yaml");
    p2.id = id.clone();
    store.create(p2).expect("create p2 should overwrite");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.spec_path, "specs/replaced.yaml");
}

#[test]
fn update_saves_pipeline_not_in_cache() {
    let (mut store, _temp) = create_temp_store();
    let mut pipeline = make_pipeline("specs/direct-update.yaml");
    advance_to_state(&mut pipeline, PipelineState::Failed);

    let id = pipeline.id.clone();
    store
        .update(pipeline)
        .expect("update should succeed even if not cached");

    let loaded = store.get(&id).expect("get");
    assert_eq!(loaded.state, PipelineState::Failed);
}

#[test]
fn list_by_state_returns_empty_for_unmatched_state() {
    let (mut store, _temp) = create_temp_store();
    let mut p = make_pipeline("specs/no-match.yaml");
    advance_to_state(&mut p, PipelineState::Failed);
    store.create(p).expect("create");

    let accepted = store.list_by_state(PipelineState::Accepted);
    assert!(accepted.is_empty());
}

#[test]
fn get_pending_recovery_returns_only_non_terminal() {
    let (mut store, _temp) = create_temp_store();

    let p1 = make_pipeline("specs/pending.yaml");
    store.create(p1).expect("create");

    let mut p2 = make_pipeline("specs/running.yaml");
    advance_to_state(&mut p2, PipelineState::AgentDevelopment);
    store.create(p2).expect("create");

    let mut p3 = make_pipeline("specs/done.yaml");
    advance_to_state(&mut p3, PipelineState::Accepted);
    store.create(p3).expect("create");

    let mut p4 = make_pipeline("specs/broke.yaml");
    advance_to_state(&mut p4, PipelineState::Failed);
    store.create(p4).expect("create");

    let recovery = store.get_pending_recovery();
    assert_eq!(recovery.len(), 2);
    for p in &recovery {
        assert!(!p.state.is_terminal());
    }
}

#[test]
fn clear_removes_all_disk_files() {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().to_path_buf();

    let mut store = StateStore::new(path.clone()).expect("store");
    for i in 0..3 {
        store
            .create(make_pipeline(&format!("specs/clear-{i}.yaml")))
            .expect("create");
    }
    assert_eq!(store.list().len(), 3);

    store.clear().expect("clear");
    assert!(store.list().is_empty());

    let json_files: Vec<_> = fs::read_dir(&path)
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert!(
        json_files.is_empty(),
        "no json files should remain after clear"
    );
}

#[test]
fn sync_is_noop_when_not_dirty() {
    let (mut store, _temp) = create_temp_store();
    store.sync().expect("sync on clean store");
    store.sync().expect("second sync still ok");
}

// ============================================================
// Section 7: StoreError Variants
// ============================================================

#[test]
fn store_error_io_variant_message() {
    let err = StoreError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "no access",
    ));
    assert!(err.to_string().contains("no access"));
}

#[test]
fn store_error_serialization_variant_message() {
    let json_err = serde_json::from_str::<serde_json::Value>("not json");
    let err = StoreError::Serialization(json_err.err().expect("json error"));
    assert!(err.to_string().contains("Serialization"));
}

#[test]
fn store_error_not_found_variant_message() {
    let err = StoreError::NotFound("test-id-42".to_string());
    let msg = err.to_string();
    assert!(msg.contains("test-id-42"));
}

#[test]
fn store_error_invalid_state_variant_message() {
    let err = StoreError::InvalidState("corrupt data".to_string());
    let msg = err.to_string();
    assert!(msg.contains("corrupt data"));
}

// ============================================================
// Section 8: Property-Based Tests (proptest)
// ============================================================

use proptest::prelude::*;

prop_compose! {
    fn arb_pipeline_state()(
        state_idx in 0..8usize
    ) -> PipelineState {
        match state_idx {
            0 => PipelineState::Pending,
            1 => PipelineState::SpecReview,
            2 => PipelineState::UniverseSetup,
            3 => PipelineState::AgentDevelopment,
            4 => PipelineState::Validation,
            5 => PipelineState::Accepted,
            6 => PipelineState::Escalated,
            _ => PipelineState::Failed,
        }
    }
}

prop_compose! {
    fn arb_pipeline()(
        spec_path in "[a-z]{3,10}\\.yaml",
        state in arb_pipeline_state(),
        iteration in 0..20u32,
        max_iterations in 1..50u32,
    ) -> Pipeline {
        let mut p = Pipeline::new(format!("specs/{}", spec_path));
        p.state = state;
        p.iteration = iteration.min(max_iterations);
        p.max_iterations = max_iterations;
        p
    }
}

proptest! {
    #[test]
    fn proptest_save_load_roundtrip_preserves_spec_path(
        pipeline in arb_pipeline()
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut store = StateStore::new(temp_dir.path().to_path_buf()).expect("store");
        let expected_path = pipeline.spec_path.clone();
        let id = pipeline.id.clone();
        store.create(pipeline).expect("create");

        let loaded = store.get(&id).expect("get");
        prop_assert_eq!(&loaded.spec_path, &expected_path);
    }

    #[test]
    fn proptest_save_load_roundtrip_preserves_state(
        pipeline in arb_pipeline()
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut store = StateStore::new(temp_dir.path().to_path_buf()).expect("store");
        let expected_state = pipeline.state;
        let id = pipeline.id.clone();
        store.create(pipeline).expect("create");

        let loaded = store.get(&id).expect("get");
        prop_assert_eq!(loaded.state, expected_state);
    }

    #[test]
    fn proptest_save_load_roundtrip_preserves_iteration(
        pipeline in arb_pipeline()
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut store = StateStore::new(temp_dir.path().to_path_buf()).expect("store");
        let expected_iter = pipeline.iteration;
        let id = pipeline.id.clone();
        store.create(pipeline).expect("create");

        let loaded = store.get(&id).expect("get");
        prop_assert_eq!(loaded.iteration, expected_iter);
    }

    #[test]
    fn proptest_persistence_across_instances(
        pipeline in arb_pipeline()
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let path = temp_dir.path().to_path_buf();
        let expected_spec = pipeline.spec_path.clone();
        let expected_state = pipeline.state;
        let id = pipeline.id.clone();

        {
            let mut store1 = StateStore::new(path.clone()).expect("store1");
            store1.create(pipeline).expect("create");
            store1.sync().expect("sync");
        }

        let store2 = StateStore::new(path.clone()).expect("store2");
        let loaded = store2.get(&id).expect("get after reopen");
        prop_assert_eq!(&loaded.spec_path, &expected_spec);
        prop_assert_eq!(loaded.state, expected_state);
    }

    #[test]
    fn proptest_not_found_for_random_ids(
        random_id in "[a-f0-9\\-]{10,50}"
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = StateStore::new(temp_dir.path().to_path_buf()).expect("store");
        let id = PipelineId(random_id.clone());
        let result = store.get(&id);
        prop_assert!(result.is_err());
        prop_assert!(matches!(result.err().expect("error"), StoreError::NotFound(missing) if missing == random_id));
    }

    #[test]
    fn proptest_export_import_roundtrip(
        pipelines in prop::collection::vec(arb_pipeline(), 1..10)
    ) {
        let temp_dir = TempDir::new().expect("temp dir");
        let mut store1 = StateStore::new(temp_dir.path().to_path_buf()).expect("store1");
        let mut expected = Vec::new();

        for p in pipelines {
            expected.push((p.id.clone(), p.spec_path.clone(), p.state));
            store1.create(p).expect("create");
        }

        let export_path = temp_dir.path().join("export.json");
        store1.export_all(&export_path).expect("export");

        let temp_dir2 = TempDir::new().expect("temp dir2");
        let mut store2 = StateStore::new(temp_dir2.path().to_path_buf()).expect("store2");
        let count = store2.import_from(&export_path).expect("import");

        prop_assert_eq!(count, expected.len());

        for (id, spec_path, state) in &expected {
            let loaded = store2.get(id).expect("get after import");
            prop_assert_eq!(&loaded.spec_path, spec_path);
            prop_assert_eq!(loaded.state, *state);
        }
    }
}
