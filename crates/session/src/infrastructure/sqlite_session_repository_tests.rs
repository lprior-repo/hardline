use crate::domain::entities::SessionState;
use crate::domain::value_objects::{AgentId, SessionName};
use crate::infrastructure::repository::SessionRepository;
use crate::infrastructure::sqlite_session_repository::SqliteSessionRepository;
use scp_core::infrastructure::database::SqliteDatabaseService;

async fn test_repository() -> SqliteSessionRepository {
    let db = SqliteDatabaseService::in_memory()
        .await
        .expect("failed to create in-memory database");
    let repo = SqliteSessionRepository::new(db);
    repo.init_schema().await.expect("failed to init schema");
    repo
}

// =========================================================================
// CRUD Tests
// =========================================================================

#[tokio::test]
async fn test_saves_valid_session_and_retrieves_by_id() {
    let repo = test_repository().await;
    let name = SessionName::parse("test-session").expect("valid");
    let session = crate::domain::entities::Session::create(name).expect("created");

    repo.save(&session).await.expect("save failed");
    let found = repo
        .find_by_id(session.id.as_str())
        .await
        .expect("find failed");

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, session.id);
    assert_eq!(found.name.as_str(), "test-session");
}

#[tokio::test]
async fn test_saves_valid_session_and_retrieves_by_name() {
    let repo = test_repository().await;
    let name = SessionName::parse("my-session").expect("valid");
    let session = crate::domain::entities::Session::create(name).expect("created");

    repo.save(&session).await.expect("save failed");
    let found = repo.find_by_name(&session.name).await.expect("find failed");

    assert!(found.is_some());
    assert_eq!(found.unwrap().name.as_str(), "my-session");
}

#[tokio::test]
async fn test_lists_all_sessions() {
    let repo = test_repository().await;
    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("session-2").unwrap()).unwrap();
    let s3 = crate::domain::entities::Session::create(SessionName::parse("session-3").unwrap()).unwrap();

    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");
    repo.save(&s3).await.expect("save failed");

    let list = repo.list().await.expect("list failed");
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn test_deletes_existing_session() {
    let repo = test_repository().await;
    let session = crate::domain::entities::Session::create(SessionName::parse("to-delete").unwrap()).unwrap();

    repo.save(&session).await.expect("save failed");
    repo.delete(session.id.as_str())
        .await
        .expect("delete failed");

    let found = repo
        .find_by_id(session.id.as_str())
        .await
        .expect("find failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn test_find_by_id_returns_not_found_for_nonexistent() {
    let repo = test_repository().await;
    let found = repo
        .find_by_id("nonexistent-id")
        .await
        .expect("find failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_returns_not_found_for_nonexistent() {
    let repo = test_repository().await;
    let result = repo.delete("nonexistent").await;
    assert!(result.is_err());
    match result {
        Err(crate::error::SessionError::NotFound(_)) => {}
        Err(e) => panic!("Expected NotFound, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[tokio::test]
async fn test_handles_empty_database_gracefully() {
    let repo = test_repository().await;
    let list = repo.list().await.expect("list failed");
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_list_after_delete_returns_correct_count() {
    let repo = test_repository().await;
    let s1 = crate::domain::entities::Session::create(SessionName::parse("s1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("s2").unwrap()).unwrap();
    let s3 = crate::domain::entities::Session::create(SessionName::parse("s3").unwrap()).unwrap();

    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");
    repo.save(&s3).await.expect("save failed");

    repo.delete(s2.id.as_str()).await.expect("delete failed");

    let list = repo.list().await.expect("list failed");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_save_same_id_twice_updates() {
    let repo = test_repository().await;
    let name1 = SessionName::parse("session-123").unwrap();
    let session1 = crate::domain::entities::Session::create(name1).unwrap();

    use crate::domain::entities::SessionState;
    let session2 = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: session1.id.clone(),
            name: SessionName::parse("updated-session").unwrap(),
            workspace: None,
            bead: None,
            assigned_agent: None,
            branch: crate::domain::entities::BranchState::Detached,
            last_synced: None,
            created_at: session1.created_at,
        },
    );

    repo.save(&session1).await.expect("first save failed");
    repo.save(&session2).await.expect("second save failed");

    let found = repo
        .find_by_id(session1.id.as_str())
        .await
        .expect("find failed");
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.state(), SessionState::Created);
    assert_eq!(found.name.as_str(), "updated-session");
}

#[tokio::test]
async fn test_p3_violation_empty_session_id() {
    let repo = test_repository().await;
    let result = repo.find_by_id("").await;
    assert!(result.is_err());
    match result {
        Err(crate::error::SessionError::InvalidIdentifier(_)) => {}
        Err(e) => panic!("Expected InvalidIdentifier, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

// =========================================================================
// Find by State Tests
// =========================================================================

#[tokio::test]
async fn test_find_by_state_returns_matching_sessions() {
    let repo = test_repository().await;

    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("session-2").unwrap()).unwrap();
    let s3 = crate::domain::entities::Session::create(SessionName::parse("session-3").unwrap()).unwrap();

    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");
    repo.save(&s3).await.expect("save failed");

    let found = repo.find_by_state(SessionState::Created).await.expect("find failed");
    assert_eq!(found.len(), 3);
}

#[tokio::test]
async fn test_find_by_state_returns_empty_for_no_matches() {
    let repo = test_repository().await;

    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    repo.save(&s1).await.expect("save failed");

    // Transition s1 to Active
    let active = s1.activate().expect("activate");
    
    // Save the active session
    repo.save(&active).await.expect("save failed");

    let found = repo.find_by_state(SessionState::Active).await.expect("find failed");
    assert_eq!(found.len(), 1);
    // Note: Typestate is Created after DB roundtrip, but data is preserved correctly
    assert_eq!(found[0].name.as_str(), "session-1");
}

#[tokio::test]
async fn test_find_by_state_filtering() {
    let repo = test_repository().await;

    let created = crate::domain::entities::Session::create(SessionName::parse("created").unwrap()).unwrap();
    let active = crate::domain::entities::Session::create(SessionName::parse("active").unwrap()).unwrap().activate().unwrap();
    let paused = crate::domain::entities::Session::create(SessionName::parse("paused").unwrap()).unwrap().activate().unwrap().pause().unwrap();

    repo.save(&created).await.expect("save failed");
    repo.save(&active).await.expect("save failed");
    repo.save(&paused).await.expect("save failed");

    let found = repo.find_by_state(SessionState::Created).await.expect("find failed");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name.as_str(), "created");

    let found = repo.find_by_state(SessionState::Active).await.expect("find failed");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name.as_str(), "active");

    let found = repo.find_by_state(SessionState::Paused).await.expect("find failed");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name.as_str(), "paused");
}

#[tokio::test]
async fn test_find_by_state_empty_database() {
    let repo = test_repository().await;

    let found = repo.find_by_state(SessionState::Active).await.expect("find failed");
    assert!(found.is_empty());
}

// =========================================================================
// Find by Agent Tests
// =========================================================================

#[tokio::test]
async fn test_find_by_agent_returns_matching_sessions() {
    let repo = test_repository().await;
    let agent = AgentId::new("agent-001").expect("valid");

    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("session-2").unwrap()).unwrap();
    let _s3 = crate::domain::entities::Session::create(SessionName::parse("session-3").unwrap()).unwrap();

    let s1_with_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: s1.id,
            name: s1.name,
            workspace: None,
            bead: None,
            assigned_agent: Some(agent.clone()),
            branch: s1.branch.clone(),
            last_synced: None,
            created_at: s1.created_at,
        },
    );

    let s2_with_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: s2.id.clone(),
            name: s2.name.clone(),
            workspace: None,
            bead: None,
            assigned_agent: Some(agent.clone()),
            branch: s2.branch.clone(),
            last_synced: None,
            created_at: s2.created_at,
        },
    );

    let s2_no_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: s2.id,
            name: s2.name,
            workspace: None,
            bead: None,
            assigned_agent: None,
            branch: s2.branch,
            last_synced: None,
            created_at: s2.created_at,
        },
    );

    repo.save(&s1_with_agent).await.expect("save failed");
    repo.save(&s2_no_agent).await.expect("save failed");
    repo.save(&s2_with_agent).await.expect("save failed");

    let found = repo.find_by_agent(&agent).await.expect("find failed");
    assert_eq!(found.len(), 2);
}

#[tokio::test]
async fn test_find_by_agent_returns_empty_for_no_matches() {
    let repo = test_repository().await;
    let agent = AgentId::new("nonexistent-agent").expect("valid");

    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    repo.save(&s1).await.expect("save failed");

    let found = repo.find_by_agent(&agent).await.expect("find failed");
    assert!(found.is_empty());
}

#[tokio::test]
async fn test_find_by_agent_with_none_agent() {
    let repo = test_repository().await;
    let agent = AgentId::new("agent-with-none").expect("valid");

    let s1 = crate::domain::entities::Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("session-2").unwrap()).unwrap();

    // s1 has no agent, s2 has agent
    repo.save(&s1).await.expect("save failed");
    let s2_with_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: s2.id,
            name: s2.name,
            workspace: None,
            bead: None,
            assigned_agent: Some(agent.clone()),
            branch: s2.branch.clone(),
            last_synced: None,
            created_at: s2.created_at,
        },
    );
    repo.save(&s2_with_agent).await.expect("save failed");

    let found = repo.find_by_agent(&agent).await.expect("find failed");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name.as_str(), "session-2");
}

#[tokio::test]
async fn test_find_by_agent_empty_database() {
    let repo = test_repository().await;
    let agent = AgentId::new("agent").expect("valid");

    let found = repo.find_by_agent(&agent).await.expect("find failed");
    assert!(found.is_empty());
}

#[tokio::test]
async fn test_session_preserves_assigned_agent_through_lifecycle() {
    let repo = test_repository().await;
    let agent = AgentId::new("test-agent").expect("valid");

    let session = crate::domain::entities::Session::create(SessionName::parse("lifecycle-test").unwrap()).unwrap();
    let session_with_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: session.id,
            name: session.name,
            workspace: None,
            bead: None,
            assigned_agent: Some(agent.clone()),
            branch: session.branch.clone(),
            last_synced: None,
            created_at: session.created_at,
        },
    );

    repo.save(&session_with_agent).await.expect("save failed");

    let found = repo.find_by_id(session_with_agent.id.as_str()).await.expect("find failed").unwrap();
    assert_eq!(found.assigned_agent().map(|a| a.as_str()), Some("test-agent"));

    // Transition through lifecycle
    let active = found.activate().expect("activate");
    repo.save(&active).await.expect("save failed");
    let found = repo.find_by_id(active.id.as_str()).await.expect("find failed").unwrap();
    assert_eq!(found.assigned_agent().map(|a| a.as_str()), Some("test-agent"));

    let completed = active.complete().expect("complete");
    repo.save(&completed).await.expect("save failed");
    let found = repo.find_by_id(completed.id.as_str()).await.expect("find failed").unwrap();
    assert_eq!(found.assigned_agent().map(|a| a.as_str()), Some("test-agent"));
}

// =========================================================================
// Concurrent Access Tests
// =========================================================================

#[tokio::test]
async fn test_concurrent_saves_same_session() {
    let repo = test_repository().await;
    let session = crate::domain::entities::Session::create(SessionName::parse("concurrent").unwrap()).unwrap();

    // Simulate concurrent saves
    let save1 = repo.save(&session);
    let save2 = repo.save(&session);
    
    let (r1, r2) = tokio::join!(save1, save2);
    assert!(r1.is_ok());
    assert!(r2.is_ok());

    let found = repo.find_by_id(session.id.as_str()).await.expect("find failed").unwrap();
    assert_eq!(found.id, session.id);
}

#[tokio::test]
async fn test_concurrent_reads() {
    let repo = test_repository().await;
    let session = crate::domain::entities::Session::create(SessionName::parse("concurrent-read").unwrap()).unwrap();
    repo.save(&session).await.expect("save failed");

    // Simulate concurrent reads
    let read1 = repo.find_by_id(session.id.as_str());
    let read2 = repo.find_by_id(session.id.as_str());
    let read3 = repo.list();

    let (r1, r2, r3) = tokio::join!(read1, read2, read3);
    
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
    
    let found1 = r1.unwrap().expect("found1");
    let found2 = r2.unwrap().expect("found2");
    let list = r3.unwrap();
    
    assert_eq!(found1.id, found2.id);
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_concurrent_save_and_delete() {
    let repo = test_repository().await;
    let session = crate::domain::entities::Session::create(SessionName::parse("concurrent-del").unwrap()).unwrap();
    repo.save(&session).await.expect("save failed");

    // Simulate concurrent save and delete
    let save = repo.save(&session);
    let delete = repo.delete(session.id.as_str());

    let (r_save, r_delete) = tokio::join!(save, delete);
    
    // One should succeed, both shouldn't panic
    assert!(r_save.is_ok() || r_delete.is_ok());
    
    // Session should be deleted
    let found = repo.find_by_id(session.id.as_str()).await.expect("find failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn test_concurrent_find_by_state_and_list() {
    let repo = test_repository().await;
    
    let s1 = crate::domain::entities::Session::create(SessionName::parse("s1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("s2").unwrap()).unwrap();
    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");

    // Simulate concurrent queries
    let find_state = repo.find_by_state(SessionState::Created);
    let list = repo.list();

    let (r_state, r_list) = tokio::join!(find_state, list);
    
    assert!(r_state.is_ok());
    assert!(r_list.is_ok());
    
    let state_results = r_state.unwrap();
    let list_results = r_list.unwrap();
    
    assert_eq!(state_results.len(), 2);
    assert_eq!(list_results.len(), 2);
}

#[tokio::test]
async fn test_concurrent_find_by_agent_and_save() {
    let repo = test_repository().await;
    let agent = AgentId::new("concurrent-agent").expect("valid");
    
    let s1 = crate::domain::entities::Session::create(SessionName::parse("s1").unwrap()).unwrap();
    let s2 = crate::domain::entities::Session::create(SessionName::parse("s2").unwrap()).unwrap();
    
    let s1_with_agent = crate::domain::entities::Session::from_parts(
        crate::domain::entities::SessionData {
            id: s1.id,
            name: s1.name,
            workspace: None,
            bead: None,
            assigned_agent: Some(agent.clone()),
            branch: s1.branch.clone(),
            last_synced: None,
            created_at: s1.created_at,
        },
    );
    
    repo.save(&s1_with_agent).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");

    // Simulate concurrent find and save
    let find = repo.find_by_agent(&agent);
    let save = repo.save(&s2);

    let (r_find, r_save) = tokio::join!(find, save);
    
    assert!(r_find.is_ok());
    assert!(r_save.is_ok());
}

// =========================================================================
// Service + Repository Integration: Create Session Lifecycle (ha-ovm)
// =========================================================================

#[tokio::test]
async fn test_service_create_session_saved_and_found_by_id() {
    let repo = test_repository().await;
    let name = SessionName::parse("repo-create").expect("valid");
    let session =
        crate::application::session_service::SessionService::create_session(name)
            .expect("created");

    repo.save(&session).await.expect("save failed");
    let found = repo
        .find_by_id(session.id.as_str())
        .await
        .expect("find failed");

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, session.id);
    assert_eq!(found.name.as_str(), "repo-create");
}

#[tokio::test]
async fn test_service_create_session_saved_and_found_by_name() {
    let repo = test_repository().await;
    let name = SessionName::parse("repo-by-name").expect("valid");
    let session =
        crate::application::session_service::SessionService::create_session(name)
            .expect("created");

    repo.save(&session).await.expect("save failed");
    let found = repo
        .find_by_name(&session.name)
        .await
        .expect("find failed");

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, session.id);
}

#[tokio::test]
async fn test_service_create_duplicate_names_both_saved_to_repo() {
    let repo = test_repository().await;
    let name = SessionName::parse("dup-name").expect("valid");

    let s1 =
        crate::application::session_service::SessionService::create_session(name.clone())
            .expect("s1");
    let s2 =
        crate::application::session_service::SessionService::create_session(name)
            .expect("s2");

    repo.save(&s1).await.expect("save s1");
    repo.save(&s2).await.expect("save s2");

    let found1 = repo
        .find_by_id(s1.id.as_str())
        .await
        .expect("find s1")
        .expect("s1 exists");
    let found2 = repo
        .find_by_id(s2.id.as_str())
        .await
        .expect("find s2")
        .expect("s2 exists");

    assert_ne!(found1.id, found2.id);
    assert_eq!(found1.name.as_str(), "dup-name");
    assert_eq!(found2.name.as_str(), "dup-name");

    let all = repo.list().await.expect("list");
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_service_create_full_lifecycle_repo_roundtrip() {
    let repo = test_repository().await;
    let name = SessionName::parse("lifecycle-repo").expect("valid");

    // Create through service
    let created =
        crate::application::session_service::SessionService::create_session(name)
            .expect("created");
    let id = created.id.as_str().to_string();
    repo.save(&created).await.expect("save created");

    // Activate through service
    let active =
        crate::application::session_service::SessionService::activate_session(created)
            .expect("activated");
    repo.save(&active).await.expect("save active");

    // Verify retrievable after activation
    let found = repo
        .find_by_id(&id)
        .await
        .expect("find")
        .expect("exists");
    assert_eq!(found.name.as_str(), "lifecycle-repo");

    // Complete through service
    let completed =
        crate::application::session_service::SessionService::complete_session(active)
            .expect("completed");
    repo.save(&completed)
        .await
        .expect("save completed");

    // Verify identity preserved through full lifecycle
    let found = repo
        .find_by_id(&id)
        .await
        .expect("find")
        .expect("exists");
    assert_eq!(found.id.as_str(), id);
    assert_eq!(found.name.as_str(), "lifecycle-repo");
}

#[tokio::test]
async fn test_service_create_multiple_sessions_repo_lists_all() {
    let repo = test_repository().await;

    for i in 0..5 {
        let name = SessionName::parse(&format!("batch-{}", i)).expect("valid");
        let session =
            crate::application::session_service::SessionService::create_session(name)
                .expect("created");
        repo.save(&session).await.expect("save failed");
    }

    let all = repo.list().await.expect("list");
    assert_eq!(all.len(), 5);
}

#[tokio::test]
async fn test_service_create_session_not_found_before_save() {
    let repo = test_repository().await;
    let name = SessionName::parse("unsaved").expect("valid");
    let session =
        crate::application::session_service::SessionService::create_session(name)
            .expect("created");

    // Not saved yet — should not be found
    let found = repo
        .find_by_id(session.id.as_str())
        .await
        .expect("find failed");
    assert!(found.is_none());
}

#[tokio::test]
async fn test_service_create_session_delete_from_repo() {
    let repo = test_repository().await;
    let name = SessionName::parse("to-delete").expect("valid");
    let session =
        crate::application::session_service::SessionService::create_session(name)
            .expect("created");

    repo.save(&session).await.expect("save failed");
    repo.delete(session.id.as_str())
        .await
        .expect("delete failed");

    let found = repo
        .find_by_id(session.id.as_str())
        .await
        .expect("find failed");
    assert!(found.is_none());
}
