use crate::domain::entities::session::{Active, Completed, Created, Failed, Session, SessionId};
use crate::domain::value_objects::SessionName;
use crate::error::{Result, SessionError};

pub struct SessionService;

impl SessionService {
    pub fn create_session(name: SessionName) -> Result<Session<Created>> {
        Session::create(name)
    }

    pub fn activate_session(session: Session<Created>) -> Result<Session<Active>> {
        session.activate()
    }

    pub fn complete_session(session: Session<Active>) -> Result<Session<Completed>> {
        session.complete()
    }

    pub fn fail_session(session: Session<Active>) -> Result<Session<Failed>> {
        session.fail()
    }

    pub fn list_sessions() -> Result<Vec<Session<Created>>> {
        Ok(Vec::new())
    }

    pub fn get_session(_id: SessionId) -> Result<Session<Created>> {
        Err(SessionError::NotFound("not implemented".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_name(s: &str) -> SessionName {
        SessionName::parse(s).expect("valid session name")
    }

    #[test]
    fn service_create_session_returns_ok() {
        let result = SessionService::create_session(make_name("test-session"));
        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.state(), crate::domain::entities::session::SessionState::Created);
    }

    #[test]
    fn service_create_session_generates_id() {
        let session = SessionService::create_session(make_name("id-test")).expect("created");
        assert!(!session.id.as_str().is_empty());
    }

    #[test]
    fn service_activate_session_transitions() {
        let created = SessionService::create_session(make_name("activate-test")).expect("created");
        let active = SessionService::activate_session(created).expect("activated");
        assert_eq!(active.state(), crate::domain::entities::session::SessionState::Active);
    }

    #[test]
    fn service_complete_session_transitions() {
        let created = SessionService::create_session(make_name("complete-test")).expect("created");
        let active = SessionService::activate_session(created).expect("activated");
        let completed = SessionService::complete_session(active).expect("completed");
        assert!(completed.state().is_terminal());
    }

    #[test]
    fn service_fail_session_transitions() {
        let created = SessionService::create_session(make_name("fail-test")).expect("created");
        let active = SessionService::activate_session(created).expect("activated");
        let failed = SessionService::fail_session(active).expect("failed");
        assert!(failed.state().is_terminal());
    }

    #[test]
    fn service_list_sessions_returns_empty() {
        let list = SessionService::list_sessions().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn service_get_session_returns_not_found() {
        let id = SessionId::parse("nonexistent").expect("valid id");
        let result = SessionService::get_session(id);
        assert!(result.is_err());
        match result {
            Err(SessionError::NotFound(_)) => {}
            Err(e) => panic!("Expected NotFound, got {:?}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    // =========================================================================
    // Session Service Lifecycle Path Tests
    // =========================================================================

    mod lifecycle_path_tests {
        use super::*;
        use crate::domain::entities::session::{BranchState, SessionState};

        fn make_name(s: &str) -> SessionName {
            SessionName::parse(s).expect("valid session name")
        }

        #[test]
        fn service_full_sync_path() {
            let created = SessionService::create_session(make_name("sync-path")).expect("created");
            let active = SessionService::activate_session(created).expect("active");

            // Sync path goes through the session methods directly
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");

            assert!(completed.state().is_terminal());
            assert_eq!(completed.state(), SessionState::Completed);
        }

        #[test]
        fn service_pause_resume_path() {
            let created = SessionService::create_session(make_name("pause-path")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let paused = active.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);

            let resumed = paused.resume().expect("resume");
            assert_eq!(resumed.state(), SessionState::Active);
            assert!(resumed.is_active());
        }

        #[test]
        fn service_pause_from_synced() {
            let created = SessionService::create_session(make_name("pause-synced")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let paused = synced.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);
        }

        #[test]
        fn service_complete_from_synced() {
            let created = SessionService::create_session(make_name("complete-synced")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_fail_from_active_state() {
            let created = SessionService::create_session(make_name("fail-active")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let failed = SessionService::fail_session(active).expect("failed");
            assert!(failed.state().is_terminal());
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn service_id_and_name_preserved_through_full_lifecycle() {
            let created = SessionService::create_session(make_name("persist-test")).expect("created");
            let original_id = created.id.as_str().to_string();
            let original_name = created.name.as_str().to_string();

            let active = SessionService::activate_session(created).expect("active");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");

            assert_eq!(synced.id.as_str(), original_id);
            assert_eq!(synced.name.as_str(), original_name);
        }

        #[test]
        fn service_create_multiple_sessions_have_unique_ids() {
            let s1 = SessionService::create_session(make_name("s1")).expect("created");
            let s2 = SessionService::create_session(make_name("s2")).expect("created");
            assert_ne!(s1.id.as_str(), s2.id.as_str());
        }

        #[test]
        fn service_created_session_has_no_workspace_or_bead() {
            let created = SessionService::create_session(make_name("empty-fields")).expect("created");
            assert!(created.workspace().is_none());
            assert!(created.bead().is_none());
            assert!(created.last_synced.is_none());
            assert!(matches!(created.branch, BranchState::Detached));
        }
    }
}
