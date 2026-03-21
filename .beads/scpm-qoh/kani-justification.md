# Kani Justification: JJ Backend (scpm-qoh)

## Formal Argument to Skip Kani Model Checking

### 1. What Critical State Machines Exist?

The JjBackend implements the VcsBackend trait with the following operations:
- current_branch(), list_branches(), create_branch(), switch_branch()
- push(), pull(), rebase(), merge()
- log(), status(), is_initialized()
- create_workspace(), switch_workspace(), list_workspaces(), delete_workspace()
- fork_workspace(), merge_workspace()

**State Machine Analysis:**
- JjBackend is **stateless** after construction
- All operations are **idempotent queries** or **single atomic mutations**
- No multi-step state transitions
- No conditional branching based on internal state
- No retry loops or complex error recovery

### 2. Why Those State Machines Cannot Reach Invalid States

**Reasoning:**

1. **Immutable Backend**: `JjBackend { repo_path: PathBuf }` has only one field, set at construction and never mutated.

2. **Stateless Operations**: Each trait method:
   - Takes `&self` (no mutation of backend)
   - Spawns a new `std::process::Command`
   - Returns `Result<T, VcsError>` with typed errors
   - No internal state is modified

3. **Single Atomic Operations**: Operations like `rebase()`, `merge()`, `push()`, `pull()`:
   - Execute a single jj CLI command
   - Return success or typed error
   - No partial state if jj fails mid-operation

4. **No Invalid States**: Since there's no mutable state:
   - Cannot have "inconsistent" state
   - Cannot have "half-completed" operations
   - Each operation is independent

### 3. What Guarantees the Contract/Tests Provide

**Contract Guarantees (contract.md):**
- All operations return `Result<T, VcsError>` - never panic
- Preconditions enforced at runtime (JJ CLI installed, valid repo)
- Postconditions: operations either succeed or return typed error

**Tests Provide:**
- Compilation verification (cargo check)
- Unit test: backend creation works
- Integration tests: Git operations work (JJ tests limited by test environment)
- No unwrap/panic in source code (verified by grep)

### 4. Formal Reasoning

```
Given:
- JjBackend::new(path) -> JjBackend where repo_path = path
- run_jj(args) -> Result<Output, VcsError::Io>
- All trait methods: &self -> Result<T, VcsError>

Invariant:
- repo_path is never mutated after construction

Proof by Construction:
1. JjBackend contains only repo_path: PathBuf (immutable after new())
2. run_jj() creates fresh Command each call, no shared state
3. Each VcsBackend method produces output or error, doesn't modify self
4. Therefore: JjBackend cannot reach an "invalid state" because it has no variable state

Conclusion:
- No state machine exists that could be in an invalid state
- Kani model checking would not find counterexamples
- Formal verification of "no panic states" is provided by:
  a) Zero unwrap/panic in source (grep verified)
  b) All operations return Result (type system enforced)
  c) Single atomic CLI invocations (no partial failures)
```

### 5. Kani Would Verify

If we ran Kani, it would verify:
```
// Pseudo-property Kani would check:
assert!(JjBackend::new(path).run_jj(args).is_ok() || is_err())
```

But this is trivially true because:
- `Command::output()` returns `io::Result<Output>`
- `map_err(VcsError::Io)` converts io errors
- jj CLI can succeed or fail, both are handled

### 6. Decision

**Recommendation**: Skip Kani

**Justification**: The JjBackend is a thin CLI wrapper with no complex state machines. The absence of panics is already verified by:
1. Grep showing no unwrap/expect/panic in source
2. All operations returning Result<T, VcsError>
3. Single atomic command invocations

Kani would provide minimal additional assurance for this simple infrastructure code.
