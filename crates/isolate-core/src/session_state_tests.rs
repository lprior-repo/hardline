use super::session_state::SessionState;

// Behavior: Created can transition to Active or Failed
#[test]
fn given_created_state_when_check_transitions_then_active_and_failed_valid() {
    assert!(SessionState::Created.can_transition_to(SessionState::Active));
    assert!(SessionState::Created.can_transition_to(SessionState::Failed));
    assert!(!SessionState::Created.can_transition_to(SessionState::Syncing));
    assert!(!SessionState::Created.can_transition_to(SessionState::Completed));
}

// Behavior: Active can transition to Syncing, Paused, or Completed
#[test]
fn given_active_state_when_check_transitions_then_three_options() {
    assert!(SessionState::Active.can_transition_to(SessionState::Syncing));
    assert!(SessionState::Active.can_transition_to(SessionState::Paused));
    assert!(SessionState::Active.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Active.can_transition_to(SessionState::Created));
    assert!(!SessionState::Active.can_transition_to(SessionState::Failed));
}

// Behavior: Syncing can transition to Synced or Failed
#[test]
fn given_syncing_state_when_check_transitions_then_synced_or_failed() {
    assert!(SessionState::Syncing.can_transition_to(SessionState::Synced));
    assert!(SessionState::Syncing.can_transition_to(SessionState::Failed));
    assert!(!SessionState::Syncing.can_transition_to(SessionState::Active));
    assert!(!SessionState::Syncing.can_transition_to(SessionState::Completed));
}

// Behavior: Synced can transition to Active, Paused, or Completed
#[test]
fn given_synced_state_when_check_transitions_then_three_options() {
    assert!(SessionState::Synced.can_transition_to(SessionState::Active));
    assert!(SessionState::Synced.can_transition_to(SessionState::Paused));
    assert!(SessionState::Synced.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Synced.can_transition_to(SessionState::Created));
    assert!(!SessionState::Synced.can_transition_to(SessionState::Failed));
}

// Behavior: Paused can transition to Active or Completed
#[test]
fn given_paused_state_when_check_transitions_then_active_or_completed() {
    assert!(SessionState::Paused.can_transition_to(SessionState::Active));
    assert!(SessionState::Paused.can_transition_to(SessionState::Completed));
    assert!(!SessionState::Paused.can_transition_to(SessionState::Created));
    assert!(!SessionState::Paused.can_transition_to(SessionState::Syncing));
}

// Behavior: Completed can transition back to Created
#[test]
fn given_completed_state_when_check_transitions_then_only_created() {
    assert!(SessionState::Completed.can_transition_to(SessionState::Created));
    assert!(!SessionState::Completed.can_transition_to(SessionState::Active));
    assert!(!SessionState::Completed.can_transition_to(SessionState::Failed));
}

// Behavior: Failed can transition back to Created
#[test]
fn given_failed_state_when_check_transitions_then_only_created() {
    assert!(SessionState::Failed.can_transition_to(SessionState::Created));
    assert!(!SessionState::Failed.can_transition_to(SessionState::Active));
    assert!(!SessionState::Failed.can_transition_to(SessionState::Syncing));
}

// Behavior: valid_next_states returns correct list for Created
#[test]
fn given_created_state_when_get_valid_states_then_returns_list() {
    let states = SessionState::Created.valid_next_states();
    assert_eq!(states.len(), 2);
    assert!(states.contains(&SessionState::Active));
    assert!(states.contains(&SessionState::Failed));
}

// Behavior: valid_next_states returns correct list for Active
#[test]
fn given_active_state_when_get_valid_states_then_returns_three() {
    let states = SessionState::Active.valid_next_states();
    assert_eq!(states.len(), 3);
    assert!(states.contains(&SessionState::Syncing));
    assert!(states.contains(&SessionState::Paused));
    assert!(states.contains(&SessionState::Completed));
}

// Behavior: is_terminal returns false for all states
#[test]
fn given_all_states_when_check_is_terminal_then_all_false() {
    for state in SessionState::all_states() {
        assert!(!state.is_terminal());
    }
}

// Behavior: all_states returns all seven states
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

// Behavior: SessionState serializes to lowercase
#[test]
fn given_session_state_when_serialize_then_lowercase() {
    let state = SessionState::Active;
    let serialized = serde_json::to_string(&state).ok();
    assert!(serialized.is_some());

    if let Some(s) = serialized {
        assert!(s.contains("active"));
    }
}

// Behavior: SessionState deserializes from lowercase
#[test]
fn given_lowercase_string_when_deserialize_then_state() {
    let json = "\"syncing\"";
    let deserialized: Result<SessionState, _> = serde_json::from_str(json);
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.ok(), Some(SessionState::Syncing));
}

// Behavior: SessionState equality works
#[test]
fn given_two_same_states_when_compare_then_equal() {
    let state1 = SessionState::Active;
    let state2 = SessionState::Active;
    assert_eq!(state1, state2);
}

// Behavior: SessionState inequality works
#[test]
fn given_two_different_states_when_compare_then_not_equal() {
    let state1 = SessionState::Active;
    let state2 = SessionState::Paused;
    assert_ne!(state1, state2);
}

// Behavior: SessionState can be cloned
#[test]
fn given_state_when_clone_then_independent() {
    let state1 = SessionState::Created;
    let state2 = state1;
    assert_eq!(state1, state2);
}

// Behavior: SessionState can be copied
#[test]
fn given_state_when_copy_then_independent() {
    let state1 = SessionState::Failed;
    let state2 = state1;
    assert_eq!(state1, state2);
}

// Behavior: SessionState Debug formatting works
#[test]
fn given_state_when_debug_format_then_no_panic() {
    let state = SessionState::Synced;
    let debug_str = format!("{:?}", state);
    assert!(!debug_str.is_empty());
}

// Behavior: State transitions are bidirectional where expected
#[test]
fn given_paused_and_active_when_check_bidirectional_transitions() {
    // Active -> Paused is valid
    assert!(SessionState::Active.can_transition_to(SessionState::Paused));
    // Paused -> Active is valid
    assert!(SessionState::Paused.can_transition_to(SessionState::Active));
}
