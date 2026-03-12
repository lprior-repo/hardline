use crate::session_state::SessionState;

#[test]
fn given_created_state_when_check_transitions_then_active_and_failed_valid() {
    assert!(SessionState::Created.can_transition_to(SessionState::Active));
    assert!(SessionState::Created.can_transition_to(SessionState::Failed));
    assert!(!SessionState::Created.can_transition_to(SessionState::Syncing));
    assert!(!SessionState::Created.can_transition_to(SessionState::Completed));
}

#[test]
fn given_active_state_when_check_transitions_then_three_options() {
    assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
    assert!(SessionState::Active.can_transition_to(SessionState::Paused));
    assert!(SessionState::Active.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Active.can_transition_to(SessionState::Created));
    assert!(!SessionState::Active.can_transition_to(SessionState::Failed));
}

#[test]
fn given_syncing_state_when_check_transitions_then_synced_or_failed() {
    assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
    assert!(SessionState::Syncing.can_transition_to(SessionState::Failed));
    assert!(!SessionState::Syncing.can_transition_to(SessionState::Active));
    assert!(!SessionState::Syncing.can_transition_to(SessionState::Completed));
}

#[test]
fn given_synced_state_when_check_transitions_then_three_options() {
    assert!(SessionState::Synced.can_transition_to(SessionState::Active));
    assert!(SessionState::Synced.can_transition_to(SessionState::Paused));
    assert!(SessionState::Synced.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Synced.can_transition_to(SessionState::Created));
    assert!(!SessionState::Synced.can_transition_to(SessionState::Failed));
}

#[test]
fn given_paused_state_when_check_transitions_then_active_or_completed() {
    assert!(SessionState::Paused.can_transition_to(SessionState::Active));
    assert!(SessionState::Paused.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Paused.can_transition_to(SessionState::Created));
    assert!(!SessionState::Paused.can_transition_to(SessionState::Syncing));
}

#[test]
fn given_completed_state_when_check_transitions_then_only_created() {
    assert!(SessionState::Completed.can_transition_to(SessionState::Created));
    assert!(!SessionState::Completed.can_transition_to(SessionState::Active));
    assert!(!SessionState::Completed.can_transition_to(SessionState::Failed));
}

#[test]
fn given_failed_state_when_check_transitions_then_only_created() {
    assert!(SessionState::Failed.can_transition_to(SessionState::Created));
    assert!(!SessionState::Failed.can_transition_to(SessionState::Active));
    assert!(!SessionState::Failed.can_transition_to(SessionState::Syncing));
}

#[test]
fn given_created_state_when_get_valid_states_then_returns_list() {
    let states = SessionState::Created.valid_next_states();
    assert_eq!(states.len(), 2);
    assert!(states.contains(&SessionState::Active));
    assert!(states.contains(&SessionState::Failed));
}

#[test]
fn given_active_state_when_get_valid_states_then_returns_three() {
    let states = SessionState::Active.valid_next_states();
    assert_eq!(states.len(), 3);
    assert!(states.contains(&SessionState::Syncing));
    assert!(states.contains(&SessionState::Paused));
    assert!(states.contains(&SessionState::Completed));
}

#[test]
fn given_all_states_when_check_is_terminal_then_all_false() {
    for state in SessionState::all_states() {
        assert!(!state.is_terminal());
    }
}

#[test]
fn given_all_states_when_called_then_returns_seven_states() {
    let states = SessionState::all_states();
    assert_eq!(states.len(), 7);

    let state_values = [
        SessionState::Created,
        SessionState::Active,
        SessionState::Syncing,
        SessionState::Synced,
        SessionState::Paused,
        SessionState::Completed,
        SessionState::Failed,
    ];

    for expected in state_values {
        assert!(states.contains(&expected), "Missing state: {:?}", expected);
    }
}

#[test]
fn given_session_state_when_serialize_then_lowercase() {
    let state = SessionState::Active;
    let serialized = serde_json::to_string(&state).ok();
    assert!(serialized.is_some());

    if let Some(s) = serialized {
        assert!(s.contains("active"));
    }
}

#[test]
fn given_lowercase_string_when_deserialize_then_state() {
    let json = "\"syncing\"";
    let deserialized: Result<SessionState, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.ok(), Some(SessionState::Syncing));
}

#[test]
fn given_two_same_states_when_compare_then_equal() {
    let state1 = SessionState::Active;
    let state2 = SessionState::Active;
    assert_eq!(state1, state2);
}

#[test]
fn given_two_different_states_when_compare_then_not_equal() {
    let state1 = SessionState::Active;
    let state2 = SessionState::Paused;
    assert_ne!(state1, state2);
}

#[test]
fn given_state_when_clone_then_independent() {
    let state1 = SessionState::Created;
    let state2 = state1.clone();
    assert_eq!(state1, state2);
}

#[test]
fn given_state_when_copy_then_independent() {
    let state1 = SessionState::Failed;
    let state2 = state1;
    assert_eq!(state1, state2);
}

#[test]
fn given_state_when_debug_format_then_no_panic() {
    let state = SessionState::Synced;
    let debug_str = format!("{:?}", state);
    assert!(!debug_str.is_empty());
}

#[test]
fn given_paused_and_active_when_check_bidirectional_transitions() {
    assert!(SessionState::Active.can_transition_to(SessionState::Paused));
    assert!(SessionState::Paused.can_transition_to(SessionState::Active));
}

#[test]
fn test_state_transition_created_to_active() {
    use crate::session_state::StateTransition;
    let transition =
        StateTransition::new(SessionState::Created, SessionState::Active, "activation");
    assert_eq!(transition.from, SessionState::Created);
    assert_eq!(transition.to, SessionState::Active);
}

#[test]
fn test_state_validation_prevents_invalid_created_to_paused() {
    use crate::session_state::StateTransition;
    let transition = StateTransition::new(SessionState::Created, SessionState::Paused, "invalid");
    assert!(transition.validate().is_err());
}

#[test]
fn test_state_validation_allows_valid_created_to_active() {
    use crate::session_state::StateTransition;
    let transition = StateTransition::new(SessionState::Created, SessionState::Active, "valid");
    assert!(transition.validate().is_ok());
}
