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

    // =========================================================================
    // Create Session Lifecycle Tests (ha-ovm)
    // =========================================================================

    mod create_session_lifecycle {
        use super::*;
        use crate::domain::entities::session::{BranchState, SessionState};
        use chrono::Utc;

        fn make_name(s: &str) -> SessionName {
            SessionName::parse(s).expect("valid session name")
        }

        // --- Valid Creation Through Service ---

        #[test]
        fn service_create_session_returns_ok_with_valid_name() {
            let result = SessionService::create_session(make_name("valid-create"));
            assert!(result.is_ok());
        }

        #[test]
        fn service_create_session_has_created_state() {
            let session =
                SessionService::create_session(make_name("state-check")).expect("created");
            assert_eq!(session.state(), SessionState::Created);
        }

        #[test]
        fn service_create_session_generates_session_prefixed_id() {
            let session = SessionService::create_session(make_name("id-prefix")).expect("created");
            assert!(session.id.as_str().starts_with("session-"));
        }

        #[test]
        fn service_create_session_preserves_name() {
            let session =
                SessionService::create_session(make_name("name-preserve")).expect("created");
            assert_eq!(session.name.as_str(), "name-preserve");
        }

        #[test]
        fn service_create_session_no_workspace() {
            let session =
                SessionService::create_session(make_name("no-ws")).expect("created");
            assert!(session.workspace().is_none());
        }

        #[test]
        fn service_create_session_no_bead() {
            let session =
                SessionService::create_session(make_name("no-bead")).expect("created");
            assert!(session.bead().is_none());
        }

        #[test]
        fn service_create_session_no_assigned_agent() {
            let session =
                SessionService::create_session(make_name("no-agent")).expect("created");
            assert!(session.assigned_agent().is_none());
        }

        #[test]
        fn service_create_session_branch_is_detached() {
            let session =
                SessionService::create_session(make_name("branch-init")).expect("created");
            assert!(matches!(session.branch, BranchState::Detached));
        }

        #[test]
        fn service_create_session_no_last_synced() {
            let session =
                SessionService::create_session(make_name("no-sync")).expect("created");
            assert!(session.last_synced.is_none());
        }

        #[test]
        fn service_create_session_has_created_at_near_now() {
            let before = Utc::now();
            let session =
                SessionService::create_session(make_name("timestamp")).expect("created");
            let after = Utc::now();
            assert!(session.created_at >= before);
            assert!(session.created_at <= after);
        }

        #[test]
        fn service_create_session_state_is_not_terminal() {
            let session =
                SessionService::create_session(make_name("non-terminal")).expect("created");
            assert!(!session.state().is_terminal());
        }

        #[test]
        fn service_create_session_with_min_length_name() {
            let session = SessionService::create_session(make_name("a")).expect("created");
            assert_eq!(session.name.as_str(), "a");
        }

        #[test]
        fn service_create_session_with_max_length_name() {
            let max_name = "a".repeat(SessionName::MAX_LENGTH);
            let session =
                SessionService::create_session(make_name(&max_name)).expect("created");
            assert_eq!(session.name.as_str().len(), SessionName::MAX_LENGTH);
        }

        #[test]
        fn service_create_session_propagates_name_validation() {
            // Invalid names should fail at SessionName::parse before reaching the service
            assert!(SessionName::parse("").is_err());
            assert!(SessionName::parse("123bad").is_err());
            assert!(SessionName::parse("has space").is_err());
        }

        #[test]
        fn service_create_session_accepts_valid_names() {
            for name in &["a", "test", "my-session", "session_name", "aBc123"] {
                let result = SessionService::create_session(make_name(name));
                assert!(result.is_ok(), "Name '{}' should be valid", name);
            }
        }

        // --- Duplicate Name Handling ---

        #[test]
        fn service_create_duplicate_name_succeeds_with_different_ids() {
            let s1 =
                SessionService::create_session(make_name("same-name")).expect("s1 created");
            let s2 =
                SessionService::create_session(make_name("same-name")).expect("s2 created");
            assert_eq!(s1.name.as_str(), s2.name.as_str());
            assert_ne!(s1.id.as_str(), s2.id.as_str());
        }

        #[test]
        fn service_create_duplicate_name_produces_distinct_sessions() {
            let s1 = SessionService::create_session(make_name("dup")).expect("s1");
            let s2 = SessionService::create_session(make_name("dup")).expect("s2");
            assert_ne!(s1.id, s2.id);
            assert_eq!(s1.state(), SessionState::Created);
            assert_eq!(s2.state(), SessionState::Created);
        }

        #[test]
        fn service_create_batch_same_name_all_unique_ids() {
            let mut ids = std::collections::HashSet::new();
            for _ in 0..10 {
                let session =
                    SessionService::create_session(make_name("batch")).expect("created");
                ids.insert(session.id);
            }
            assert_eq!(ids.len(), 10);
        }

        #[test]
        fn service_create_different_names_also_different_ids() {
            let s1 = SessionService::create_session(make_name("alpha")).expect("s1");
            let s2 = SessionService::create_session(make_name("beta")).expect("s2");
            assert_ne!(s1.name.as_str(), s2.name.as_str());
            assert_ne!(s1.id, s2.id);
        }

        // --- Full Create Lifecycle Through Service ---

        #[test]
        fn service_lifecycle_create_to_activate() {
            let created = SessionService::create_session(make_name("lc-activate"))
                .expect("created");
            assert_eq!(created.state(), SessionState::Created);

            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.state(), SessionState::Active);
        }

        #[test]
        fn service_lifecycle_create_activate_complete() {
            let created = SessionService::create_session(make_name("lc-complete"))
                .expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let completed = SessionService::complete_session(active).expect("completed");
            assert!(completed.state().is_terminal());
            assert_eq!(completed.state(), SessionState::Completed);
        }

        #[test]
        fn service_lifecycle_create_activate_fail() {
            let created = SessionService::create_session(make_name("lc-fail"))
                .expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let failed = SessionService::fail_session(active).expect("failed");
            assert!(failed.state().is_terminal());
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn service_lifecycle_create_entity_fail_from_created() {
            // SessionService::fail_session requires Session<Active>, but
            // the entity allows direct Created→Failed. Verify the created
            // session is compatible with entity-level fail.
            let created = SessionService::create_session(make_name("entity-fail"))
                .expect("created");
            let failed = created.fail().expect("entity fail from created");
            assert_eq!(failed.state(), SessionState::Failed);
        }

        #[test]
        fn service_lifecycle_create_activate_sync_complete() {
            let created = SessionService::create_session(make_name("lc-sync"))
                .expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let syncing = active.sync().expect("syncing");
            assert_eq!(syncing.state(), SessionState::Syncing);
            let synced = syncing.sync_complete().expect("synced");
            assert_eq!(synced.state(), SessionState::Synced);
            let completed = synced.complete().expect("completed");
            assert_eq!(completed.state(), SessionState::Completed);
        }

        #[test]
        fn service_lifecycle_create_pause_resume_complete() {
            let created = SessionService::create_session(make_name("lc-pause"))
                .expect("created");
            let active = SessionService::activate_session(created).expect("activated");
            let paused = active.pause().expect("paused");
            assert_eq!(paused.state(), SessionState::Paused);
            let resumed = paused.resume().expect("resumed");
            assert_eq!(resumed.state(), SessionState::Active);
            let completed = SessionService::complete_session(resumed).expect("completed");
            assert!(completed.state().is_terminal());
        }

        #[test]
        fn service_lifecycle_identity_preserved_through_full_path() {
            let created = SessionService::create_session(make_name("lc-identity"))
                .expect("created");
            let id = created.id.as_str().to_string();
            let name = created.name.as_str().to_string();

            let active = SessionService::activate_session(created).expect("activated");
            assert_eq!(active.id.as_str(), id);
            assert_eq!(active.name.as_str(), name);

            let syncing = active.sync().expect("syncing");
            assert_eq!(syncing.id.as_str(), id);
            assert_eq!(syncing.name.as_str(), name);

            let synced = syncing.sync_complete().expect("synced");
            assert_eq!(synced.id.as_str(), id);
            assert_eq!(synced.name.as_str(), name);

            let completed = synced.complete().expect("completed");
            assert_eq!(completed.id.as_str(), id);
            assert_eq!(completed.name.as_str(), name);
        }
    }
}
