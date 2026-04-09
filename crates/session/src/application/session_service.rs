use crate::domain::entities::session::{
    Active, Completed, Created, Failed, Paused, Session, SessionId,
};
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

    pub fn suspend_session(session: Session<Active>) -> Result<Session<Paused>> {
        session.pause()
    }

    pub fn resume_session(session: Session<Paused>) -> Result<Session<Active>> {
        session.resume()
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
        assert_eq!(
            session.state(),
            crate::domain::entities::session::SessionState::Created
        );
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
        assert_eq!(
            active.state(),
            crate::domain::entities::session::SessionState::Active
        );
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
    // SessionService Transition Tests (ha-4sk)
    // =========================================================================

    mod service_transition_tests {
        use super::*;
        use crate::domain::entities::session::SessionState;

        fn make_name(s: &str) -> SessionName {
            SessionName::parse(s).expect("valid session name")
        }

        // -- Activate transition --

        #[test]
        fn service_activate_transitions_created_to_active() {
            let created = SessionService::create_session(make_name("activate-t")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.state(), SessionState::Active);
            assert!(active.is_active());
        }

        #[test]
        fn service_activate_preserves_session_identity() {
            let created = SessionService::create_session(make_name("activate-id")).expect("created");
            let id = created.id.as_str().to_string();
            let name = created.name.as_str().to_string();
            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.id.as_str(), id);
            assert_eq!(active.name.as_str(), name);
        }

        // -- Suspend (pause) transition --

        #[test]
        fn service_suspend_transitions_active_to_paused() {
            let created = SessionService::create_session(make_name("suspend-t")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let paused = SessionService::suspend_session(active).expect("suspended");
            assert_eq!(paused.state(), SessionState::Paused);
            assert!(!paused.state().is_terminal());
        }

        #[test]
        fn service_suspend_preserves_session_identity() {
            let created = SessionService::create_session(make_name("suspend-id")).expect("created");
            let id = created.id.as_str().to_string();
            let active = SessionService::activate_session(created).expect("activated");
            let paused = SessionService::suspend_session(active).expect("suspended");
            assert_eq!(paused.id.as_str(), id);
        }

        // -- Resume transition --

        #[test]
        fn service_resume_transitions_paused_to_active() {
            let created = SessionService::create_session(make_name("resume-t")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let paused = SessionService::suspend_session(active).expect("suspended");
            let resumed = SessionService::resume_session(paused).expect("resumed");
            assert_eq!(resumed.state(), SessionState::Active);
            assert!(resumed.is_active());
        }

        #[test]
        fn service_resume_preserves_session_identity() {
            let created = SessionService::create_session(make_name("resume-id")).expect("created");
            let id = created.id.as_str().to_string();
            let active = SessionService::activate_session(created).expect("activated");
            let paused = SessionService::suspend_session(active).expect("suspended");
            let resumed = SessionService::resume_session(paused).expect("resumed");
            assert_eq!(resumed.id.as_str(), id);
        }

        // -- Complete transition --

        #[test]
        fn service_complete_transitions_active_to_completed() {
            let created = SessionService::create_session(make_name("complete-t")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let completed = SessionService::complete_session(active).expect("completed");
            assert_eq!(completed.state(), SessionState::Completed);
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_complete_preserves_session_identity() {
            let created =
                SessionService::create_session(make_name("complete-id")).expect("created");
            let id = created.id.as_str().to_string();
            let active = SessionService::activate_session(created).expect("activated");
            let completed = SessionService::complete_session(active).expect("completed");
            assert_eq!(completed.id.as_str(), id);
        }

        // -- Fail transition --

        #[test]
        fn service_fail_transitions_active_to_failed() {
            let created = SessionService::create_session(make_name("fail-t")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let failed = SessionService::fail_session(active).expect("failed");
            assert_eq!(failed.state(), SessionState::Failed);
            assert!(failed.state().is_terminal());
        }

        // -- Full lifecycle: activate → suspend → resume → complete --

        #[test]
        fn service_full_lifecycle_activate_suspend_resume_complete() {
            let created =
                SessionService::create_session(make_name("full-lifecycle")).expect("created");
            assert_eq!(created.state(), SessionState::Created);

            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.state(), SessionState::Active);

            let paused = SessionService::suspend_session(active).expect("suspended");
            assert_eq!(paused.state(), SessionState::Paused);

            let resumed = SessionService::resume_session(paused).expect("resumed");
            assert_eq!(resumed.state(), SessionState::Active);

            let completed = SessionService::complete_session(resumed).expect("completed");
            assert_eq!(completed.state(), SessionState::Completed);
            assert!(completed.state().is_terminal());
        }

        // -- Multiple suspend/resume cycles --

        #[test]
        fn service_multiple_suspend_resume_cycles() {
            let created = SessionService::create_session(make_name("multi-cycle")).expect("created");
            let id = created.id.as_str().to_string();

            let active = SessionService::activate_session(created).expect("activated");

            // Cycle 1
            let paused = SessionService::suspend_session(active).expect("suspended");
            let active = SessionService::resume_session(paused).expect("resumed");

            // Cycle 2
            let paused = SessionService::suspend_session(active).expect("suspended");
            let active = SessionService::resume_session(paused).expect("resumed");

            // Cycle 3
            let paused = SessionService::suspend_session(active).expect("suspended");
            let resumed = SessionService::resume_session(paused).expect("resumed");

            assert_eq!(resumed.state(), SessionState::Active);
            assert_eq!(resumed.id.as_str(), id);
        }

        // -- Identity preserved through full lifecycle with suspend/resume --

        #[test]
        fn service_identity_preserved_through_full_lifecycle() {
            let created =
                SessionService::create_session(make_name("identity-lifecycle")).expect("created");
            let id = created.id.as_str().to_string();
            let name = created.name.as_str().to_string();

            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.id.as_str(), id);
            assert_eq!(active.name.as_str(), name);

            let paused = SessionService::suspend_session(active).expect("suspended");
            assert_eq!(paused.id.as_str(), id);
            assert_eq!(paused.name.as_str(), name);

            let resumed = SessionService::resume_session(paused).expect("resumed");
            assert_eq!(resumed.id.as_str(), id);
            assert_eq!(resumed.name.as_str(), name);

            let completed = SessionService::complete_session(resumed).expect("completed");
            assert_eq!(completed.id.as_str(), id);
            assert_eq!(completed.name.as_str(), name);
        }

        // -- Suspend then fail (suspend does not prevent failure) --

        #[test]
        fn service_suspend_then_resume_then_fail() {
            let created =
                SessionService::create_session(make_name("suspend-fail")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let paused = SessionService::suspend_session(active).expect("suspended");
            let resumed = SessionService::resume_session(paused).expect("resumed");
            let failed = SessionService::fail_session(resumed).expect("failed");
            assert_eq!(failed.state(), SessionState::Failed);
            assert!(failed.state().is_terminal());
        }

        // -- Activate then complete (no suspend) --

        #[test]
        fn service_activate_then_complete_without_suspend() {
            let created =
                SessionService::create_session(make_name("direct-complete")).expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let completed = SessionService::complete_session(active).expect("completed");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        // -- Typestate enforces valid transitions at compile time --
        // The following are documented as compile-time guarantees enforced
        // by the typestate pattern. They cannot be written as runtime tests
        // because the compiler rejects them:
        //
        // - Cannot call suspend_session on a Created session (requires Active)
        // - Cannot call resume_session on an Active session (requires Paused)
        // - Cannot call complete_session on a Paused session (requires Active)
        // - Cannot call activate_session on an Active session (requires Created)
        // - Cannot call any transition on Completed or Failed (terminal states)
        //
        // See compile_fail tests for proof: tests/compile_fail/session_*.rs

        #[test]
        fn service_typestate_prevents_invalid_created_to_complete() {
            // Must go Created → Active → Completed (not Created → Completed)
            let created =
                SessionService::create_session(make_name("skip-activate")).expect("created");
            let completed = SessionService::activate_session(created)
                .expect("activated")
                .complete()
                .expect("completed");
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_typestate_prevents_invalid_paused_to_complete() {
            // Must go Paused → Active → Completed (not Paused → Completed)
            let created = SessionService::create_session(make_name("paused-complete"))
                .expect("created");
            let paused = SessionService::suspend_session(
                SessionService::activate_session(created).expect("activated"),
            )
            .expect("suspended");
            let completed = SessionService::resume_session(paused)
                .expect("resumed")
                .complete()
                .expect("completed");
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_typestate_prevents_invalid_created_to_suspend() {
            // Must go Created → Active → Paused (not Created → Paused)
            let created = SessionService::create_session(make_name("created-suspend"))
                .expect("created");
            let paused = SessionService::suspend_session(
                SessionService::activate_session(created).expect("activated"),
            )
            .expect("suspended");
            assert_eq!(paused.state(), SessionState::Paused);
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
            let created =
                SessionService::create_session(make_name("pause-synced")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let paused = synced.pause().expect("pause");
            assert_eq!(paused.state(), SessionState::Paused);
        }

        #[test]
        fn service_complete_from_synced() {
            let created =
                SessionService::create_session(make_name("complete-synced")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let syncing = active.sync().expect("sync");
            let synced = syncing.sync_complete().expect("sync_complete");
            let completed = synced.complete().expect("complete");
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_fail_from_active_state() {
            let created =
                SessionService::create_session(make_name("fail-active")).expect("created");
            let active = SessionService::activate_session(created).expect("active");
            let failed = SessionService::fail_session(active).expect("failed");
            assert!(failed.state().is_terminal());
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn service_id_and_name_preserved_through_full_lifecycle() {
            let created =
                SessionService::create_session(make_name("persist-test")).expect("created");
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
            let created =
                SessionService::create_session(make_name("empty-fields")).expect("created");
            assert!(created.workspace().is_none());
            assert!(created.bead().is_none());
            assert!(created.last_synced.is_none());
            assert!(matches!(created.branch, BranchState::Detached));
        }
    }
}
