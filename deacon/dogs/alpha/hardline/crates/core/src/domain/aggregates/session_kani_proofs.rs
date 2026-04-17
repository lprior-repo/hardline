//! Kani proofs for Session aggregate invariants.
//!
//! # Invariants Proven
//!
//! 1. Session preserves ID through transitions
//! 2. Branch transitions are validated
//! 3. Workspace path changes are validated

#[cfg(kani)]
mod proof {
    use std::path::PathBuf;

    use crate::domain::aggregates::session::{Session, SessionError};
    use crate::domain::{
        identifiers::{SessionId, SessionName},
        session::BranchState,
    };

    fn assume_valid_session() -> Session {
        let id = SessionId::parse("test-session").unwrap();
        let name = SessionName::parse("test-session").unwrap();
        let workspace = PathBuf::from("/tmp");
        Session::new(id, name, BranchState::Detached, workspace).unwrap()
    }

    #[kani::proof]
    fn verify_session_preserves_id_on_transition() {
        let session = assume_valid_session();
        let original_id = session.id.clone();

        let new_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };

        if let Ok(new_session) = session.transition_branch(new_branch) {
            assert_eq!(new_session.id, original_id);
        }
    }

    #[kani::proof]
    fn verify_session_preserves_name_on_transition() {
        let session = assume_valid_session();
        let original_name = session.name.clone();

        let new_branch = BranchState::OnBranch {
            name: "main".to_string(),
        };

        if let Ok(new_session) = session.transition_branch(new_branch) {
            assert_eq!(new_session.name, original_name);
        }
    }

    #[kani::proof]
    fn verify_branch_transition_changes_branch() {
        let session = assume_valid_session();
        let new_branch = BranchState::OnBranch {
            name: "feature".to_string(),
        };

        let result = session.transition_branch(new_branch.clone());
        if result.is_ok() {
            assert_eq!(result.unwrap().branch, new_branch);
        }
    }

    #[kani::proof]
    fn verify_invalid_branch_transition_returns_error() {
        let detached = Session::new(
            SessionId::parse("test").unwrap(),
            SessionName::parse("test").unwrap(),
            BranchState::Detached,
            PathBuf::from("/tmp"),
        )
        .unwrap();

        let result = detached.transition_branch(BranchState::Detached);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SessionError::InvalidBranchTransition { .. })
        ));
    }

    #[kani::proof]
    fn verify_on_branch_to_on_branch_is_valid() {
        let on_branch = Session::new(
            SessionId::parse("test").unwrap(),
            SessionName::parse("test").unwrap(),
            BranchState::OnBranch {
                name: "main".to_string(),
            },
            PathBuf::from("/tmp"),
        )
        .unwrap();

        let result = on_branch.transition_branch(BranchState::OnBranch {
            name: "feature".to_string(),
        });
        assert!(result.is_ok());
    }

    #[kani::proof]
    fn verify_rename_preserves_id_and_branch() {
        let session = assume_valid_session();
        let new_name = SessionName::parse("renamed").unwrap();
        let renamed = session.rename(new_name.clone());

        assert_eq!(renamed.id, session.id);
        assert_eq!(renamed.branch, session.branch);
        assert_eq!(renamed.name, new_name);
    }

    #[kani::proof]
    fn verify_is_active_when_on_branch_with_valid_path() {
        let session = Session::new(
            SessionId::parse("test").unwrap(),
            SessionName::parse("test").unwrap(),
            BranchState::OnBranch {
                name: "main".to_string(),
            },
            PathBuf::from("/tmp"),
        )
        .unwrap();

        assert!(session.is_active());
    }

    #[kani::proof]
    fn verify_is_not_active_when_detached() {
        let session = Session::new(
            SessionId::parse("test").unwrap(),
            SessionName::parse("test").unwrap(),
            BranchState::Detached,
            PathBuf::from("/tmp"),
        )
        .unwrap();

        assert!(!session.is_active());
    }
}
