# Martin Fowler Test Plan

## Overview

This test plan provides comprehensive Given-When-Then scenarios for validating the WorkspaceState and AgentState state machines.

## Happy Path Tests

### WorkspaceState Happy Path

- **test_workspace_transitions_from_created_to_working**
  - Given: workspace in `Created` state
  - When: transition to `Working` is attempted
  - Then: returns `Ok(WorkspaceState::Working)`

- **test_workspace_transitions_from_working_to_ready**
  - Given: workspace in `Working` state
  - When: transition to `Ready` is attempted
  - Then: returns `Ok(WorkspaceState::Ready)`

- **test_workspace_transitions_from_ready_to_merged**
  - Given: workspace in `Ready` state
  - When: transition to `Merged` is attempted
  - Then: returns `Ok(WorkspaceState::Merged)`

- **test_workspace_transitions_from_ready_to_conflict**
  - Given: workspace in `Ready` state
  - When: transition to `Conflict` is attempted
  - Then: returns `Ok(WorkspaceState::Conflict)`

- **test_workspace_transitions_from_ready_to_abandoned**
  - Given: workspace in `Ready` state
  - When: transition to `Abandoned` is attempted
  - Then: returns `Ok(WorkspaceState::Abandoned)`

### AgentState Happy Path

- **test_agent_transitions_from_idle_to_active**
  - Given: agent in `Idle` state
  - When: transition to `Active` is attempted
  - Then: returns `Ok(AgentState::Active)`

- **test_agent_transitions_from_active_to_idle**
  - Given: agent in `Active` state
  - When: transition to `Idle` is attempted
  - Then: returns `Ok(AgentState::Idle)`

- **test_agent_transitions_from_any_to_offline**
  - Given: agent in any non-terminal state
  - When: transition to `Offline` is attempted
  - Then: returns `Ok(AgentState::Offline)`

- **test_agent_transitions_from_any_to_error**
  - Given: agent in any non-terminal state
  - When: transition to `Error` is attempted
  - Then: returns `Ok(AgentState::Error)`

- **test_agent_transitions_from_offline_to_idle**
  - Given: agent in `Offline` state
  - When: transition to `Idle` is attempted
  - Then: returns `Ok(AgentState::Idle)`

## Error Path Tests

### WorkspaceState Error Path

- **test_workspace_invalid_transition_from_created_to_merged**
  - Given: workspace in `Created` state
  - When: transition to `Merged` is attempted (skipping Working→Ready)
  - Then: returns `Err(Error::InvalidTransition { from: Created, to: Merged })`

- **test_workspace_invalid_transition_from_ready_to_created**
  - Given: workspace in `Ready` state
  - When: transition back to `Created` is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Ready, to: Created })`

- **test_workspace_invalid_transition_from_merged_to_any**
  - Given: workspace in terminal `Merged` state
  - When: transition to any other state is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Merged, to: ... })`

- **test_workspace_invalid_transition_from_conflict_to_any**
  - Given: workspace in terminal `Conflict` state
  - When: transition to any other state is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Conflict, to: ... })`

- **test_workspace_invalid_transition_from_abandoned_to_any**
  - Given: workspace in terminal `Abandoned` state
  - When: transition to any other state is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Abandoned, to: ... })`

### AgentState Error Path

- **test_agent_invalid_transition_from_error_to_active**
  - Given: agent in terminal `Error` state
  - When: transition to `Active` is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Error, to: Active })`

- **test_agent_invalid_transition_from_error_to_idle**
  - Given: agent in terminal `Error` state
  - When: transition to `Idle` is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Error, to: Idle })`

- **test_agent_invalid_transition_from_offline_to_active**
  - Given: agent in terminal `Offline` state
  - When: transition to `Active` is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Offline, to: Active })`

- **test_agent_invalid_self_loop**
  - Given: agent in `Active` state
  - When: transition to `Active` (self-loop) is attempted
  - Then: returns `Err(Error::InvalidTransition { from: Active, to: Active })`

## Edge Case Tests

- **test_workspace_can_check_transition_validity_before_attempting**
  - Given: workspace in `Created` state
  - When: `can_transition_to(Working)` is checked
  - Then: returns `true`

- **test_workspace_cannot_transition_to_invalid_state**
  - Given: workspace in `Created` state
  - When: `can_transition_to(Abandoned)` is checked
  - Then: returns `false`

- **test_agent_can_check_transition_validity_before_attempting**
  - Given: agent in `Idle` state
  - When: `can_transition_to(Active)` is checked
  - Then: returns `true`

- **test_workspace_terminal_states_are_correctly_identified**
  - Given: workspace in `Merged`, `Conflict`, or `Abandoned` state
  - When: `is_terminal()` is checked
  - Then: returns `true` for all three

- **test_workspace_non_terminal_states_return_false**
  - Given: workspace in `Created`, `Working`, or `Ready` state
  - When: `is_terminal()` is checked
  - Then: returns `false` for all three

- **test_agent_terminal_states_are_correctly_identified**
  - Given: agent in `Offline` or `Error` state
  - When: `is_terminal()` is checked
  - Then: returns `true` for both

- **test_agent_non_terminal_states_return_false**
  - Given: agent in `Idle` or `Active` state
  - When: `is_terminal()` is checked
  - Then: returns `false` for both

## Contract Verification Tests

### WorkspaceState Contract Tests

- **test_precondition_p1_valid_transition_succeeds**
  - Given: valid state pair (Created → Working)
  - When: transition is attempted
  - Then: returns Ok(new_state)

- **test_precondition_p1_invalid_transition_fails**
  - Given: invalid state pair (Created → Merged)
  - When: transition is attempted
  - Then: returns Err(InvalidTransition)

- **test_invariant_i3_valid_transitions_only**
  - Given: all possible state pairs
  - When: can_transition_to is checked
  - Then: returns true only for valid pairs

### AgentState Contract Tests

- **test_precondition_p2_valid_transition_succeeds**
  - Given: valid state pair (Idle → Active)
  - When: transition is attempted
  - Then: returns Ok(new_state)

- **test_precondition_p2_invalid_transition_fails**
  - Given: invalid state pair (Error → Active)
  - When: transition is attempted
  - Then: returns Err(InvalidTransition)

- **test_invariant_i3_agent_valid_transitions_only**
  - Given: all possible state pairs for agent
  - When: can_transition_to is checked
  - Then: returns true only for valid pairs

## Contract Violation Tests

These tests verify that violation examples from contract.md are correctly handled:

- **test_violates_p1_workspace_created_to_merged_returns_error**
  - Given: workspace in `Created` state
  - When: `transition(Created, Merged)` is called
  - Then: returns `Err(Error::InvalidTransition)`

- **test_violates_p2_agent_error_to_active_returns_error**
  - Given: agent in `Error` state
  - When: `transition(Error, Active)` is called
  - Then: returns `Err(Error::InvalidTransition)`

- **test_violates_q2_workspace_ready_to_created_returns_error**
  - Given: workspace in `Ready` state
  - When: `transition(Ready, Created)` is called
  - Then: returns `Err(Error::InvalidTransition)`

## Given-When-Then Scenarios

### Scenario 1: Complete Workspace Lifecycle Happy Path
- **Scenario**: A workspace goes through full lifecycle from creation to merge
- **Given**: A new workspace in `Created` state
- **When**: 
  - Transition to `Working`
  - Transition to `Ready`
  - Transition to `Merged`
- **Then**: 
  - Each transition returns Ok with correct state
  - Final state is `Merged`
  - `is_terminal()` returns true

### Scenario 2: Workspace Merge Conflict
- **Scenario**: A workspace has a merge conflict
- **Given**: A workspace in `Ready` state
- **When**: Transition to `Conflict` is attempted
- **Then**: 
  - Returns `Ok(WorkspaceState::Conflict)`
  - `is_terminal()` returns true

### Scenario 3: Agent Becomes Unavailable
- **Scenario**: An agent goes offline unexpectedly
- **Given**: An agent in `Active` state
- **When**: Transition to `Offline` is attempted
- **Then**: 
  - Returns `Ok(AgentState::Offline)`
  - `is_terminal()` returns true

### Scenario 4: Agent Recovers from Offline
- **Scenario**: An offline agent comes back online
- **Given**: An agent in `Offline` state
- **When**: Transition to `Idle` is attempted
- **Then**: 
  - Returns `Ok(AgentState::Idle)`
  - `is_terminal()` returns false

### Scenario 5: Agent Encounters Error
- **Scenario**: An agent encounters an error during work
- **Given**: An agent in `Active` state
- **When**: Transition to `Error` is attempted
- **Then**: 
  - Returns `Ok(AgentState::Error)`
  - `is_terminal()` returns true

## Implementation Checklist

- [ ] WorkspaceState enum with all 6 variants
- [ ] AgentState enum with all 4 variants  
- [ ] WorkspaceStateMachine::transition function
- [ ] WorkspaceStateMachine::can_transition_to function
- [ ] WorkspaceStateMachine::is_terminal function
- [ ] WorkspaceStateMachine::is_ready function
- [ ] AgentStateMachine::transition function
- [ ] AgentStateMachine::can_transition_to function
- [ ] AgentStateMachine::is_terminal function
- [ ] AgentStateMachine::is_available function
- [ ] All happy path tests pass
- [ ] All error path tests pass
- [ ] All contract violation tests pass
