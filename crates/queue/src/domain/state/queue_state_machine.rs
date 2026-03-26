use crate::domain::entities::QueueStatus;
use crate::error::QueueError;

pub struct QueueStateMachine;

impl QueueStateMachine {
    pub fn can_transition(from: QueueStatus, to: QueueStatus) -> bool {
        match (from, to) {
            (QueueStatus::Pending, QueueStatus::Claimed) => true,
            (QueueStatus::Claimed, QueueStatus::Rebasing) => true,
            (QueueStatus::Rebasing, QueueStatus::Testing) => true,
            (QueueStatus::Testing, QueueStatus::ReadyToMerge) => true,
            (QueueStatus::Testing, QueueStatus::FailedRetryable) => true,
            (QueueStatus::Testing, QueueStatus::FailedTerminal) => true,
            (QueueStatus::ReadyToMerge, QueueStatus::Merging) => true,
            (QueueStatus::Merging, QueueStatus::Merged) => true,
            (_, QueueStatus::Cancelled) => true,
            _ => false,
        }
    }

    pub fn validate_transition(from: QueueStatus, to: QueueStatus) -> Result<(), QueueError> {
        if Self::can_transition(from, to) {
            Ok(())
        } else {
            Err(QueueError::InvalidStateTransition {
                from: format!("{:?}", from),
                to: format!("{:?}", to),
            })
        }
    }

    pub fn is_terminal(status: QueueStatus) -> bool {
        matches!(
            status,
            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
        )
    }

    pub fn is_active(status: QueueStatus) -> bool {
        matches!(
            status,
            QueueStatus::Claimed
                | QueueStatus::Rebasing
                | QueueStatus::Testing
                | QueueStatus::ReadyToMerge
                | QueueStatus::Merging
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_machine_pending_to_claimed_is_valid() {
        assert!(QueueStateMachine::can_transition(
            QueueStatus::Pending,
            QueueStatus::Claimed
        ));
    }

    #[test]
    fn state_machine_pending_to_merged_is_invalid() {
        assert!(!QueueStateMachine::can_transition(
            QueueStatus::Pending,
            QueueStatus::Merged
        ));
    }

    #[test]
    fn state_machine_cancelled_is_terminal() {
        assert!(QueueStateMachine::is_terminal(QueueStatus::Cancelled));
    }

    #[test]
    fn state_machine_claimed_is_active() {
        assert!(QueueStateMachine::is_active(QueueStatus::Claimed));
    }

    #[test]
    fn state_machine_pending_is_not_active() {
        assert!(!QueueStateMachine::is_active(QueueStatus::Pending));
    }
}

#[cfg(test)]
mod proptest_state_machine_tests {
    use crate::domain::entities::{QueueEntry, QueueEntryId, QueueStatus};
    use crate::domain::value_objects::Priority;
    use crate::error::QueueError;
    use proptest::prelude::{any, prop, proptest};
    use proptest::state_machine::{StateMachine, Transition};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, Copy)]
    enum QueueAction {
        Enqueue { session_id: String, priority: u8 },
        Claim { entry_id: u32 },
        StartRebase { entry_id: u32 },
        StartTesting { entry_id: u32 },
        MarkReadyToMerge { entry_id: u32 },
        StartMerging { entry_id: u32 },
        MarkMerged { entry_id: u32 },
        MarkFailedRetryable { entry_id: u32 },
        MarkFailedTerminal { entry_id: u32 },
        Cancel { entry_id: u32 },
    }

    impl QueueAction {
        fn priority(&self) -> u8 {
            match self {
                QueueAction::Enqueue { priority, .. } => *priority,
                _ => 200,
            }
        }
    }

    #[derive(Debug, Clone)]
    struct QueueState {
        entries: BTreeMap<u32, QueueEntry>,
        next_id: u32,
        last_error: Option<String>,
    }

    impl QueueState {
        fn new() -> Self {
            Self {
                entries: BTreeMap::new(),
                next_id: 1,
                last_error: None,
            }
        }

        fn find_entry_by_local_id(&self, local_id: u32) -> Option<&QueueEntry> {
            self.entries.get(&local_id)
        }

        fn find_entry_by_local_id_mut(&mut self, local_id: u32) -> Option<&mut QueueEntry> {
            self.entries.get_mut(&local_id)
        }

        fn valid_actions(&self) -> Vec<QueueAction> {
            let mut actions = Vec::new();
            actions.push(QueueAction::Enqueue {
                session_id: format!("session-{}", self.next_id),
                priority: 200,
            });
            for (id, entry) in &self.entries {
                let local_id = *id;
                match entry.status {
                    QueueStatus::Pending => {
                        actions.push(QueueAction::Claim { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::Claimed => {
                        actions.push(QueueAction::StartRebase { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::Rebasing => {
                        actions.push(QueueAction::StartTesting { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::Testing => {
                        actions.push(QueueAction::MarkReadyToMerge { entry_id: local_id });
                        actions.push(QueueAction::MarkFailedRetryable { entry_id: local_id });
                        actions.push(QueueAction::MarkFailedTerminal { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::ReadyToMerge => {
                        actions.push(QueueAction::StartMerging { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::Merging => {
                        actions.push(QueueAction::MarkMerged { entry_id: local_id });
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::FailedRetryable => {
                        if entry.retry_count < 3 {
                            actions.push(QueueAction::Claim { entry_id: local_id });
                        }
                        actions.push(QueueAction::Cancel { entry_id: local_id });
                    }
                    QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled => {}
                }
            }
            actions
        }
    }

    struct QueueSM;

    impl StateMachine for QueueSM {
        type Action = QueueAction;
        type State = QueueState;
        type SystemUnderTest = QueueState;

        fn generate_action(state: &Self::State) -> Self::Action {
            let actions = state.valid_actions();
            if actions.is_empty() {
                QueueAction::Enqueue {
                    session_id: format!("session-{}", state.next_id),
                    priority: 200,
                }
            } else {
                *proptest::sample::select(actions.as_slice())
            }
        }

        fn apply(state: &mut Self::State, action: Self::Action) {
            state.last_error = None;
            match action {
                QueueAction::Enqueue {
                    session_id,
                    priority,
                } => {
                    let priority = Priority::new(priority).unwrap_or_default();
                    let entry = QueueEntry::enqueue(session_id, None, priority);
                    match entry {
                        Ok(e) => {
                            state.entries.insert(state.next_id, e);
                            state.next_id += 1;
                        }
                        Err(e) => {
                            state.last_error = Some(format!("{:?}", e));
                        }
                    }
                }
                QueueAction::Claim { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.claim() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::StartRebase { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.start_rebase() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::StartTesting { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.start_testing() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::MarkReadyToMerge { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.mark_ready_to_merge() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::StartMerging { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.start_merging() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::MarkMerged { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.mark_merged() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::MarkFailedRetryable { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.mark_failed_retryable("Test failure".into()) {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::MarkFailedTerminal { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.mark_failed_terminal("Test failure".into()) {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
                QueueAction::Cancel { entry_id } => {
                    if let Some(entry) = state.find_entry_by_local_id_mut(entry_id) {
                        match entry.cancel() {
                            Ok(new_entry) => {
                                *entry = new_entry;
                            }
                            Err(e) => {
                                state.last_error = Some(format!("{:?}", e));
                            }
                        }
                    }
                }
            }
        }

        fn initial_state() -> Self::State {
            QueueState::new()
        }

        fn new_system_under_test(init: &Self::State) -> Self::SystemUnderTest {
            init.clone()
        }

        fn reset(&self, _sut: &mut Self::SystemUnderTest) {}
    }

    proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 100,
            ..Default::default()
        })]

        #[test]
        fn test_queue_state_machine_100_runs(actions in proptest::collection::vec(any::<QueueAction>(), 0..50)) {
            let mut state = QueueSM::initial_state();
            for action in &actions {
                QueueSM::apply(&mut state, action.clone());
            }
            let _sut = QueueSM::new_system_under_test(&state);
        }

        #[test]
        fn test_queue_state_machine_respects_valid_transitions(actions in proptest::collection::vec(any::<QueueAction>(), 0..30)) {
            let mut state = QueueSM::initial_state();
            for action in &actions {
                let before = state.clone();
                QueueSM::apply(&mut state, action.clone());
                if let Some(error) = &state.last_error {
                    let valid_actions = before.valid_actions();
                    assert!(
                        valid_actions.contains(action),
                        "Action {:?} should be valid in state {:?}, but got error: {}",
                        action,
                        before,
                        error
                    );
                }
            }
        }

        #[test]
        fn test_queue_state_machine_terminal_states_are_stable(actions in proptest::collection::vec(any::<QueueAction>(), 0..20)) {
            let mut state = QueueSM::initial_state();
            for action in &actions {
                QueueSM::apply(&mut state, action.clone());
            }
            for entry in state.entries.values() {
                if entry.is_terminal() {
                    assert!(
                        matches!(
                            entry.status,
                            QueueStatus::Merged | QueueStatus::FailedTerminal | QueueStatus::Cancelled
                        ),
                        "Terminal entry should have terminal status"
                    );
                }
            }
        }
    }

    #[test]
    fn test_queue_sm_initial_state_is_empty() {
        let state = QueueSM::initial_state();
        assert!(state.entries.is_empty());
        assert_eq!(state.next_id, 1);
    }

    #[test]
    fn test_queue_sm_enqueue_creates_entry() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.next_id, 2);
    }

    #[test]
    fn test_queue_sm_claim_transitions_pending_to_claimed() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        QueueSM::apply(&mut state, QueueAction::Claim { entry_id: 1 });
        let entry = state.entries.get(&1).unwrap();
        assert_eq!(entry.status, QueueStatus::Claimed);
    }

    #[test]
    fn test_queue_sm_full_happy_path() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        QueueSM::apply(&mut state, QueueAction::Claim { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartRebase { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartTesting { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::MarkReadyToMerge { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartMerging { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::MarkMerged { entry_id: 1 });
        let entry = state.entries.get(&1).unwrap();
        assert_eq!(entry.status, QueueStatus::Merged);
        assert!(entry.is_terminal());
    }

    #[test]
    fn test_queue_sm_failure_path() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        QueueSM::apply(&mut state, QueueAction::Claim { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartRebase { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartTesting { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::MarkFailedTerminal { entry_id: 1 });
        let entry = state.entries.get(&1).unwrap();
        assert_eq!(entry.status, QueueStatus::FailedTerminal);
        assert!(entry.is_terminal());
    }

    #[test]
    fn test_queue_sm_cancel_from_any_non_terminal() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        QueueSM::apply(&mut state, QueueAction::Cancel { entry_id: 1 });
        let entry = state.entries.get(&1).unwrap();
        assert_eq!(entry.status, QueueStatus::Cancelled);
        assert!(entry.is_terminal());
    }

    #[test]
    fn test_queue_sm_retryable_failure_allows_retry() {
        let mut state = QueueSM::initial_state();
        QueueSM::apply(
            &mut state,
            QueueAction::Enqueue {
                session_id: "test-session".into(),
                priority: 200,
            },
        );
        QueueSM::apply(&mut state, QueueAction::Claim { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartRebase { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::StartTesting { entry_id: 1 });
        QueueSM::apply(&mut state, QueueAction::MarkFailedRetryable { entry_id: 1 });
        let entry = state.entries.get(&1).unwrap();
        assert_eq!(entry.status, QueueStatus::FailedRetryable);
        assert!(!entry.is_terminal());
        assert!(entry.can_retry());
    }
}
