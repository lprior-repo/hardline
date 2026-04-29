use scp_core::infrastructure::database::SqliteDatabaseService;

use crate::{
    domain::{entities::Session, value_objects::SessionName},
    infrastructure::{
        repository::SessionRepository, sqlite_session_repository::SqliteSessionRepository,
    },
};

async fn test_repository() -> SqliteSessionRepository {
    let db = SqliteDatabaseService::in_memory()
        .await
        .expect("failed to create in-memory database");
    let repo = SqliteSessionRepository::new(db);
    repo.init_schema().await.expect("failed to init schema");
    repo
}

#[tokio::test]
async fn test_saves_valid_session_and_retrieves_by_id() {
    let repo = test_repository().await;
    let name = SessionName::parse("test-session").expect("valid");
    let session = Session::create(name).expect("created");

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
    let session = Session::create(name).expect("created");

    repo.save(&session).await.expect("save failed");
    let found = repo.find_by_name(&session.name).await.expect("find failed");

    assert!(found.is_some());
    assert_eq!(found.unwrap().name.as_str(), "my-session");
}

#[tokio::test]
async fn test_lists_all_sessions() {
    let repo = test_repository().await;
    let s1 = Session::create(SessionName::parse("session-1").unwrap()).unwrap();
    let s2 = Session::create(SessionName::parse("session-2").unwrap()).unwrap();
    let s3 = Session::create(SessionName::parse("session-3").unwrap()).unwrap();

    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");
    repo.save(&s3).await.expect("save failed");

    let list = repo.list().await.expect("list failed");
    assert_eq!(list.len(), 3);
}

#[tokio::test]
async fn test_deletes_existing_session() {
    let repo = test_repository().await;
    let session = Session::create(SessionName::parse("to-delete").unwrap()).unwrap();

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
    let s1 = Session::create(SessionName::parse("s1").unwrap()).unwrap();
    let s2 = Session::create(SessionName::parse("s2").unwrap()).unwrap();
    let s3 = Session::create(SessionName::parse("s3").unwrap()).unwrap();

    repo.save(&s1).await.expect("save failed");
    repo.save(&s2).await.expect("save failed");
    repo.save(&s3).await.expect("save failed");

    repo.delete(s2.id.as_str()).await.expect("delete failed");

    let list = repo.list().await.expect("list failed");
    assert_eq!(list.len(), 2);
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

#[tokio::test]
async fn test_save_same_id_twice_updates() {
    let repo = test_repository().await;
    let name1 = SessionName::parse("session-123").unwrap();
    let session1 = Session::create(name1).unwrap();

    use crate::domain::entities::{BranchState, SessionParts, SessionState};
    let session2 = Session::from_parts(SessionParts {
        id: session1.id.clone(),
        name: SessionName::parse("updated-session").unwrap(),
        workspace: None,
        bead: None,
        branch: BranchState::Detached,
        last_synced: None,
        created_at: session1.created_at,
    });

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
