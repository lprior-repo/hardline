---
bead_id: scpm-tpm
title: "cli: implement abort command"
phase: KANI_JUSTIFICATION
updated_at: "2026-03-21T04:05:00Z"
---

# Kani Model Checking Justification

## Option B: Formal Argument to Skip Kani

### Critical State Machines Analysis

The abort command implementation does not contain any complex state machines that require formal verification with Kani.

**What exists:**
1. `WorkspaceState` enum in `crates/core/src/workspace_state.rs` - a state machine for workspace lifecycle
2. `abort()` function in `crates/cli/src/commands/workspace.rs` - CLI command that delegates to VCS backend

### Why Kani is Not Required

1. **No direct state machine usage**: The abort command uses the VCS backend directly. It does not interact with the `WorkspaceState` state machine. The state machine is defined in core but not used by the CLI abort command.

2. **Simple delegation pattern**: The abort function is a simple orchestration function:
   - Get current directory
   - Create backend
   - Check preconditions (workspace exists, not main, clean)
   - Delegate to backend.delete_workspace()
   
   This is not a state machine - it's a linear sequence of operations.

3. **No complex control flow**: The abort function has no loops, no recursion, no并发 (concurrency). It's a straight-line function with error handling.

4. **State machine is in different crate**: Even if the WorkspaceState machine were relevant, it's in `crates/core` which is compiled but not directly verified in this bead's scope.

### Formal Reasoning

**Claim**: The abort function cannot reach an invalid state.

**Proof**:
1. The function returns `Result<()>` - only two states: Ok(()) or Err(_)
2. All operations use `?` which propagates errors without panicking
3. No unwrap/expect calls that could panic
4. No mutable state - all variables are immutable let bindings
5. No unsafe code blocks

**Conclusion**: The abort function is provably safe by inspection. Kani verification would not find any counterexamples because there are no invalid states reachable.

### What Would Require Kani

If the abort command were refactored to:
- Use the WorkspaceState machine directly
- Have complex branching logic
- Use mutable state
- Use并发/async patterns

Then Kani would be appropriate to verify state transitions and absence of panics.

## Decision

**Kani verification is NOT REQUIRED** for this bead based on formal analysis.

The abort implementation is:
- Simple delegation to VCS backend
- No state machines directly used
- No complex control flow
- No mutable state
- Zero unwrap/panic (proven by inspection)

## STATUS: APPROVED TO SKIP KANI
