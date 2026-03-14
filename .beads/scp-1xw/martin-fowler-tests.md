---
bead_id: scp-1xw
bead_title: Create BeadState enum in domain
phase: 1
updated_at: "2026-03-13T00:00:00Z"
---

# Martin Fowler Test Plan

## Happy Path Tests

### test_bead_create_returns_open_state
Given: Valid BeadId and BeadTitle
When: Bead::create() is called
Then: Returns Bead with state == Open

### test_bead_transition_from_open_to_claimed
Given: Bead in Open state
When: bead.transition(BeadState::Claimed) is called
Then: Returns Ok(new_bead) with state == Claimed

### test_bead_transition_from_claimed_to_in_progress
Given: Bead in Claimed state
When: bead.transition(BeadState::InProgress) is called
Then: Returns Ok(new_bead) with state == InProgress

### test_bead_transition_from_in_progress_to_ready
Given: Bead in InProgress state
When: bead.transition(BeadState::Ready) is called
Then: Returns Ok(new_bead) with state == Ready

### test_bead_transition_from_ready_to_merged
Given: Bead in Ready state
When: bead.transition(BeadState::Merged) is called
Then: Returns Ok(new_bead) with state == Merged

### test_bead_transition_from_ready_to_abandoned
Given: Bead in Ready state
When: bead.transition(BeadState::Abandoned) is called
Then: Returns Ok(new_bead) with state == Abandoned

### test_bead_claim_sets_claimed_by
Given: Bead in Open state, valid AgentId
When: bead.claim(agent_id) is called
Then: Returns Ok(new_bead) with state == Claimed and claimed_by == Some(agent_id)

## Error Path Tests

### test_bead_transition_invalid_open_to_ready
Given: Bead in Open state
When: bead.transition(BeadState::Ready) is called (skipping Claimed, InProgress)
Then: Returns Err(Error::InvalidStateTransition { from: Open, to: Ready })

### test_bead_transition_invalid_open_to_open_self_loop
Given: Bead in Open state
When: bead.transition(BeadState::Open) is called (self-loop)
Then: Returns Err(Error::InvalidStateTransition { from: Open, to: Open })

### test_bead_transition_from_merged_to_open_invalid
Given: Bead in Merged state (terminal)
When: bead.transition(BeadState::Open) is called
Then: Returns Err(Error::InvalidStateTransition { from: Merged, to: Open })

### test_bead_transition_from_abandoned_to_claimed_invalid
Given: Bead in Abandoned state (terminal)
When: bead.transition(BeadState::Claimed) is called
Then: Returns Err(Error::InvalidStateTransition { from: Abandoned, to: Claimed })

### test_bead_claim_already_claimed_bead
Given: Bead in Claimed state
When: bead.claim(agent_id) is called
Then: Returns Err(Error::BeadAlreadyClaimed { bead_id: ... })

### test_bead_claim_open_bead
Given: Bead in Open state
When: bead.claim(agent_id) is called
Then: Returns Ok with state == Claimed

## Edge Case Tests

### test_bead_state_is_terminal_for_merged
Given: BeadState::Merged
When: is_terminal() is called
Then: Returns true

### test_bead_state_is_terminal_for_abandoned
Given: BeadState::Abandoned
When: is_terminal() is called
Then: Returns true

### test_bead_state_is_not_terminal_for_open
Given: BeadState::Open
When: is_terminal() is called
Then: Returns false

### test_bead_state_is_not_terminal_for_in_progress
Given: BeadState::InProgress
When: is_terminal() is called
Then: Returns false

### test_bead_valid_transitions_from_open
Given: BeadState::Open
When: valid_transitions() is called
Then: Returns vec![BeadState::Claimed]

### test_bead_valid_transitions_from_claimed
Given: BeadState::Claimed
When: valid_transitions() is called
Then: Returns vec![BeadState::InProgress]

### test_bead_valid_transitions_from_ready
Given: BeadState::Ready
When: valid_transitions() is called
Then: Returns vec![BeadState::Merged, BeadState::Abandoned]

### test_bead_state_display_implementation
Given: BeadState::Open
When: format!("{}", state) is called
Then: Returns "open"

## Contract Verification Tests

### test_contract_precondition_p2_valid_transition_check
Given: Bead in Open state, target InProgress
When: can_transition_to(InProgress) is called
Then: Returns false (must go through Claimed first)

### test_contract_postcondition_q4_terminal_state_no_transitions
Given: Bead in Merged state
When: can_transition_to(Open) is called
Then: Returns false

### test_contract_invariant_i3_linear_progression
Given: All state combinations
When: Checking transition graph
Then: Only valid path is Open->Claimed->InProgress->Ready->(Merged|Abandoned)

### test_contract_invariant_i4_no_self_loops
Given: Any state
When: can_transition_to(same_state) is called
Then: Returns false for all variants

## Contract Violation Tests

### test_violation_p2_self_loop_returns_error
Given: Bead in Open state
When: bead.transition(BeadState::Open)
Then: returns Err(InvalidStateTransition { from: Open, to: Open }) -- NOT a panic

### test_violation_p2_skip_intermediate_states_returns_error
Given: Bead in Open state
When: bead.transition(BeadState::Ready)
Then: returns Err(InvalidStateTransition { from: Open, to: Ready }) -- NOT a panic

### test_violation_p2_terminal_to_non_terminal_returns_error
Given: Bead in Merged state
When: bead.transition(BeadState::Open)
Then: returns Err(InvalidStateTransition { from: Merged, to: Open }) -- NOT a panic

### test_violation_p4_claim_already_claimed_returns_error
Given: Bead in Claimed state
When: bead.claim(agent_id)
Then: returns Err(BeadAlreadyClaimed { ... }) -- NOT a panic

## Given-When-Then Scenarios

### Scenario 1: Agent claims a bead
Given: A bead exists in Open state
When: Agent calls claim(agent_id)
Then:
- Bead transitions to Claimed state
- Bead.claimed_by is set to Some(agent_id)
- No other fields change

### Scenario 2: Agent completes work and marks ready
Given: A bead in InProgress state
When: Agent calls mark_ready()
Then:
- Bead transitions to Ready state
- claimed_by remains unchanged

### Scenario 3: Merge successful
Given: A bead in Ready state
When: Agent calls merge()
Then:
- Bead transitions to Merged state
- is_terminal() returns true
- Further transitions return errors

### Scenario 4: Work abandoned
Given: A bead in Ready state
When: Agent calls abandon()
Then:
- Bead transitions to Abandoned state
- is_terminal() returns true
- Further transitions return errors
