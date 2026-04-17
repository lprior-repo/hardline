bead_id: ha-g1t4
bead_title: Test: CircuitBreaker — closed→open→half-open→closed cycle
phase: p4
updated_at: 2026-04-06T04:00:00Z

# Exhaustive Test Plan

## Target: circuit_breaker.rs (primary)

### Unit Tests
1. `test_exact_boundary_elapsed_equals_open_duration` — verify transition at exactly open_duration ms
2. `test_failure_count_clamps_at_threshold` — failure_count stops at threshold after opening
3. `test_multiple_complete_lifecycle_cycles` — closed→open→halfopen→closed × 5
4. `test_halfopen_success_count_preserved_across_partial_successes` — interleaved success/failure in halfopen
5. `test_open_state_failure_count_unchanged` — verify failure_count frozen in open state
6. `test_repeated_open_halfopen_cycles_without_recovery` — open→halfopen→open × N without closing
7. `test_success_threshold_1_single_success_closes` — boundary: success_threshold=1
8. `test_large_threshold_many_failures_before_open` — stress: threshold=1000
9. `test_success_in_closed_resets_accumulated_failures` — interleaved success resets count
10. `test_halfopen_to_closed_resets_both_counters` — verify both counts zero after recovery

### Proptests
1. `prop_arbitrary_operation_sequence_maintains_state_invariants` — random record_success/record_failure/check_and_transition sequences never violate state machine
2. `prop_full_lifecycle_with_random_thresholds` — random thresholds, verify full cycle completes
3. `prop_closed_state_failure_count_bounded_by_threshold` — failure_count never exceeds threshold in closed state
4. `prop_check_and_transition_only_transitions_from_open` — never transitions from Closed or HalfOpen

### BDD Scenarios
- Given circuit in Closed, When failures reach threshold, Then state is Open and execution blocked
- Given circuit in Open, When elapsed < open_duration, Then state stays Open
- Given circuit in Open, When elapsed >= open_duration, Then state is HalfOpen and execution allowed
- Given circuit in HalfOpen, When successes reach threshold, Then state is Closed and counts reset
- Given circuit in HalfOpen, When any failure recorded, Then state is Open and success_count reset

## Target: circuit.rs (secondary)

### Unit Tests
1. `test_can_execute_always_true_in_closed` — verify can_execute in Closed
2. `test_failure_count_increments_without_premature_open` — failures below threshold stay closed
3. `test_record_failure_updates_last_failure_at_each_time` — timestamp updated per failure
4. `test_repeated_lifecycle_cycles` — full cycle × 3
5. `test_can_execute_in_halfopen_returns_true` — verify probe allowed

### Proptests
1. `prop_failure_threshold_never_opens_before_threshold` — parametric threshold/failures
2. `prop_success_resets_failure_count` — arbitrary failures then success resets count

## Trophy Allocation
- Unit: 70% (deterministic state transitions, boundary tests)
- Property: 20% (invariant checking across parameter space)
- Integration: 5% (multi-cycle scenarios)
- Fuzz: 5% (not applicable — deterministic state machine, no unbounded input)
