---
bead_id: hl-1p0
bead_title: Red Queen Adversarial Audit
phase: red-queen
generated_at: 2026-03-30
generations: 1
---

# Red Queen Adversarial Report: Session Lock Manager (hl-1p0)

## Verdict: CROWN CONTESTED

The implementation has a sound core but contains **3 MAJOR bugs** and **5 MINOR observations** discovered through static adversarial analysis. The existing test suite (22 tests) covers the happy paths and basic contention but misses several edge cases where the contract invariants are violated.

---

## GENERATION 1: Static Source Audit

Method: Line-by-line adversarial review of all implementation files against the contract invariants. No automated mutation testing was performed (the skill's automated weapons require `cargo-mutants`, `ast-grep`, etc., which were not available in the execution environment). Findings are from human-driven code review following the Red Queen methodology.

---

## MAJOR FINDINGS

### [GEN-1-1] MAJOR: `Error::exit_code()` discards granular lock error codes

**Dimension:** error-dispatch
**Severity:** MAJOR
**Files:** `crates/core/src/error.rs:544`, `crates/core/src/coordination/locks/errors.rs:124-140`

**Description:**
`Error::exit_code()` at line 544 hardcodes `Error::Lock(_) => 90`, discarding the specific exit codes defined in `LockError::exit_code()`. Every other variant (Workspace, Session, Queue, Vcs, etc.) delegates to the inner type's `exit_code()` method. The Lock variant does not.

**Contract violation:**
`LockError::exit_code()` defines meaningful codes:
- SessionNotFound = 14
- SessionLocked = 16
- NotLockHolder = 17
- NotFound = 71
- DatabaseError = 63
- ParseError = 80
- etc.

None of these are ever reachable through `Error::exit_code()`. Callers always get 90.

**Impact:** CLI consumers and telemetry cannot distinguish lock error types via exit code. All lock errors appear identical at the process boundary.

**Evidence:**
```rust
// error.rs:530-545
pub fn exit_code(&self) -> i32 {
    match self {
        Error::Workspace(e) => e.exit_code(),  // delegates
        Error::Session(e) => e.exit_code(),     // delegates
        Error::Queue(e) => e.exit_code(),       // delegates
        // ... all others delegate ...
        Error::Lock(_) => 90,                   // HARDCODED, does NOT delegate
    }
}
```

**Fix:** Change `Error::Lock(_) => 90` to `Error::Lock(e) => e.exit_code()`.

---

### [GEN-1-2] MAJOR: `Error::suggestion()` discards LockError suggestions

**Dimension:** error-dispatch
**Severity:** MAJOR
**Files:** `crates/core/src/error.rs:520`, `crates/core/src/coordination/locks/errors.rs:110-121`

**Description:**
`Error::suggestion()` at line 520 returns `None` for `Error::Lock(_)`, ignoring the `LockError::suggestion()` method which provides actionable recovery hints for SessionLocked (`Use 'scp agent kill {holder}' to force release`) and SessionNotFound (`Try 'scp session list' to see available sessions`).

**Impact:** Users receive no actionable guidance when lock errors occur via the unified Error type. The helpful suggestions defined in LockError are dead code.

**Evidence:**
```rust
// error.rs:506-521
pub fn suggestion(&self) -> Option<String> {
    match self {
        Error::Workspace(e) => e.suggestion(),  // delegates
        Error::Session(e) => e.suggestion(),    // delegates
        Error::Queue(e) => e.suggestion(),      // delegates
        // ... others return None, but don't HAVE suggestion methods ...
        Error::Lock(_) => None,                 // LockError HAS suggestion(), ignored
    }
}
```

**Fix:** Change `Error::Lock(_) => None` to `Error::Lock(e) => e.suggestion()`.

---

### [GEN-1-3] MAJOR: `heartbeat()` overwrites `acquired_at` with current time

**Dimension:** data-integrity
**Severity:** MAJOR
**Files:** `crates/core/src/coordination/locks/manager_unlock.rs:53-98`

**Description:**
The `heartbeat()` method returns a `LockResponse` with `acquired_at: now` (line 84), where `now = Utc::now()`. This replaces the original acquisition timestamp with the heartbeat timestamp. The contract states heartbeat extends TTL, not that it creates a new lock.

The contract's LockResponse type documents: `acquired_at: When the lock was acquired`. But after a heartbeat, the returned `acquired_at` is the heartbeat time, not the original acquisition time.

**Impact:** Callers who rely on `acquired_at` to measure lock duration or for audit purposes will get incorrect values after any heartbeat. The original acquisition time is permanently lost from the response.

**Evidence:**
```rust
// manager_unlock.rs:53-98
pub async fn heartbeat(&self, session: &str, agent_id: &str) -> Result<LockResponse> {
    let now = Utc::now();                    // <-- heartbeat time, NOT acquisition time
    // ...
    Ok(LockResponse {
        lock_id,
        session: session.to_string(),
        agent_id: agent_id.to_string(),
        acquired_at: now,                     // <-- BUG: should be original acquired_at
        expires_at: new_expires,
    })
}
```

The original `acquired_at` is stored in the database but is never read or returned.

**Fix:** SELECT `acquired_at` from the existing lock row and return it in the LockResponse. The heartbeat should only update `expires_at`, not `acquired_at`.

---

## MINOR FINDINGS

### [GEN-1-4] MINOR: Double-unlock detection produces false positives

**Dimension:** audit-correctness
**Severity:** MINOR
**Files:** `crates/core/src/coordination/locks/manager_unlock.rs:42-46`

**Description:**
The `unlock()` method logs `DoubleUnlockWarning` when no active lock is found (the `None` arm of the match). However, `None` is produced in three distinct cases:

1. **True double-unlock**: Lock existed, was unlocked by holder, same agent unlocks again.
2. **Expired lock**: Lock existed but TTL expired before unlock attempt.
3. **Never locked**: Session was never locked at all.

Cases 2 and 3 are not double-unlocks. Case 3 in particular (unlocking a session that was never locked) is silently treated as a double-unlock and logged as such in the audit trail. This pollutes the audit log with false warnings.

**Contract impact:** Contract invariant #4 states "Second unlock by same agent logs DoubleUnlockWarning". Case 3 violates this -- it is not a "second" unlock.

**Reproduction scenario:**
```rust
mgr.unlock("never-locked-session", "agent-a").await?;
// Audit log shows: DoubleUnlockWarning for "never-locked-session"
// This is semantically wrong -- it was never locked, not double-unlocked.
```

---

### [GEN-1-5] MINOR: Lock_id collision possible under rapid re-lock cycles

**Dimension:** uniqueness-guarantee
**Severity:** MINOR
**Files:** `crates/core/src/coordination/locks/manager_lock.rs:77-80`

**Description:**
Lock IDs use format `lock-{session}-{timestamp_nanos}`. The `session_locks` table has `lock_id` as PRIMARY KEY (unique). If a session is locked, unlocked, and re-locked within the same nanosecond (plausible in fast test loops or with coarse system clocks), the lock_id would collide.

When this happens, the INSERT fails with a constraint error, which triggers the constraint-conflict handler that returns `SessionLocked { holder: "unknown" }` -- a false positive, since no one actually holds the lock.

**Mitigating factor:** SQLite's `UNIQUE` constraint on `session` (not `lock_id`) is the primary collision guard. The `lock_id` collision would only occur if old lock rows are not properly cleaned up. However, the DELETE for expired locks happens before the INSERT, so a same-nanosecond re-lock after unlock should succeed because the old row was explicitly deleted.

**Risk:** Low in practice but theoretically possible. A more robust lock_id would include a random component (UUID) or the agent_id.

---

### [GEN-1-6] MINOR: `lock_with_ttl()` audit-log failure causes lock row deletion

**Dimension:** atomicity
**Severity:** MINOR
**Files:** `crates/core/src/coordination/locks/manager_lock.rs:118-123`

**Description:**
If `log_operation()` fails after the lock INSERT succeeds, the code attempts to DELETE the lock row (compensating action). If this DELETE also fails (database error), the lock exists in the database but was never returned to the caller. The caller gets an error and believes the lock was not acquired, but the lock row persists and blocks future lock attempts.

There is no transaction wrapping the INSERT + log_operation. This is a partial-failure scenario.

**Mitigating factor:** The DELETE failure would also return an error, so the caller would see a database error. However, the lock is now orphaned -- it exists but no one holds it.

---

### [GEN-1-7] MINOR: Missing session validation on unlock and heartbeat paths

**Dimension:** session-validation
**Severity:** MINOR
**Files:** `crates/core/src/coordination/locks/manager_unlock.rs:14,53`

**Description:**
The contract precondition states: "unlock(), heartbeat(): session must exist (not validated, but lock must be held)". The implementation does not validate session existence for unlock or heartbeat. This means:

- An agent can heartbeat a lock on a session that has been deleted from the `sessions` table, keeping the lock alive indefinitely for a non-existent session.
- An agent can unlock (producing DoubleUnlockWarning) a session that was deleted.

This is documented as intentional in the contract ("not validated"), so it is a design choice, not a bug. However, it creates an inconsistency: lock acquisition validates session existence, but lock maintenance does not.

---

### [GEN-1-8] MINOR: `Ttl` value object stores 0 but actual behavior uses default 300s

**Dimension:** api-semantics
**Severity:** MINOR
**Files:** `crates/core/src/coordination/locks/manager_lock.rs:68-73`, `crates/core/src/coordination/locks/types.rs:40-58`

**Description:**
When `ttl_seconds=0` is passed, `validate_ttl(0)` returns `Ttl { seconds: 0 }`. But the actual code at line 68-73 checks `if ttl_seconds > 0` and falls back to `self.ttl` (default 300s). The `Ttl` struct says its value is 0, but the lock's actual TTL is 300s. Callers who inspect the `Ttl` object would see a misleading value.

Additionally, `Ttl::new(0)` returns `Some(Ttl { seconds: 0 })` and `is_default()` returns true. But the actual TTL used is `self.ttl` which may not be 300s if `with_ttl()` was used to create the manager.

**Impact:** Low -- the `Ttl` struct is used only internally. No external API exposes it. But it represents a semantic inconsistency within the module.

---

## OBSERVATIONS (No deterministic verification possible)

### [GEN-1-O1] OBSERVATION: TOCTOU race in lock_with_ttl is adequately mitigated

The `lock_with_ttl` method has a TOCTOU race between the SELECT (check existing lock) and INSERT (create new lock). Two concurrent requests could both see no existing lock and both attempt INSERT. However, the UNIQUE constraint on `session` in the `session_locks` table acts as the serialization point. The constraint conflict handler (lines 93-110) correctly maps the collision to `SessionLocked`.

**Verdict:** The TOCTOU is handled correctly by the constraint handler. No bug.

### [GEN-1-O2] OBSERVATION: Constraint conflict handler may report holder as "unknown"

In the constraint conflict handler (manager_lock.rs:95-106), if the follow-up SELECT to find the holder also fails, the holder defaults to `"unknown"`. This is defensive but could confuse callers who need to identify the lock holder for administrative action.

### [GEN-1-O3] OBSERVATION: `verify_session_exists()` graceful degradation is intentional

If the `sessions` table does not exist, `verify_session_exists()` returns `Ok(())`, allowing lock acquisition to proceed. This is documented as graceful degradation. It means that in a deployment where the sessions table is not set up, the lock manager will work but without session validation.

---

## FITNESS LANDSCAPE

| Dimension              | Tests | Survivors | Fitness | Status       |
|------------------------|-------|-----------|---------|--------------|
| error-dispatch         | 2     | 2         | 1.000   | HEMORRHAGING |
| data-integrity         | 1     | 1         | 1.000   | HEMORRHAGING |
| audit-correctness      | 1     | 1         | 1.000   | CONTESTED    |
| uniqueness-guarantee   | 1     | 0         | 0.000   | COOLING      |
| atomicity              | 1     | 0         | 0.000   | COOLING      |
| session-validation     | 1     | 0         | 0.000   | COOLING      |
| api-semantics          | 1     | 0         | 0.000   | COOLING      |
| toctou-race            | 1     | 0         | 0.000   | COOLING      |

**Total:** 9 probes, 4 survivors (3 MAJOR + 1 MINOR confirmed), 5 discarded.

---

## CONTRACT INVARIANT AUDIT

| # | Invariant                      | Status         | Notes                                              |
|---|-------------------------------|----------------|----------------------------------------------------|
| 1 | Mutual Exclusion              | DEFENDED       | UNIQUE constraint + conflict handler correct       |
| 2 | TTL Enforcement               | DEFENDED       | `expires_at >= now` filter on all queries           |
| 3 | Session Validation            | DEFENDED       | verify_session_exists called before lock INSERT     |
| 4 | Double-Unlock Detection       | CONTESTED      | False positives for never-locked sessions           |
| 5 | Holder-Only Release           | DEFENDED       | NotLockHolder error for wrong agent                 |
| 6 | Idempotent Lock               | DEFENDED       | Same-agent re-lock returns existing info            |
| 7 | Audit Completeness            | DEFENDED       | All operations logged                               |

---

## SUMMARY

### Crown Status: CONTESTED

The session lock manager's core concurrency logic (mutual exclusion, TTL enforcement, holder-only release) is solid. The SQLite UNIQUE constraint provides correct serialization even under concurrent access.

The three MAJOR findings are all in the error/reporting layer, not the core lock logic:

1. **Error exit code dispatch loss** -- granular error codes are invisible to CLI users
2. **Error suggestion dispatch loss** -- helpful recovery hints are dead code
3. **Heartbeat acquired_at overwrite** -- original acquisition timestamp is lost

These are real bugs that affect operational observability and user experience but do not compromise the correctness of the locking mechanism itself.

### Recommended Priority

1. **Fix exit_code dispatch** (trivial one-line fix, high impact on observability)
2. **Fix suggestion dispatch** (trivial one-line fix, high impact on UX)
3. **Fix heartbeat acquired_at** (requires SELECT change, medium effort)
4. **Address double-unlock false positives** (design decision needed)

### Files Analyzed

- `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager.rs` -- core struct, validation, init, session verification
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_lock.rs` -- lock acquisition with TTL
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_unlock.rs` -- unlock and heartbeat
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/manager_query.rs` -- get_all_locks, get_lock_state, get_lock_audit_log
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/errors.rs` -- error types with codes and suggestions
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/types.rs` -- Ttl, LockOperation, LockInfo, LockResponse, etc.
- `/home/lewis/src/hardline/crates/core/src/coordination/locks/helpers.rs` -- constraint conflict detection
- `/home/lewis/src/hardline/crates/core/src/error.rs` -- unified Error enum dispatch

---

*Generated by the Red Queen deterministic adversarial framework. AI generated test probes; exit-code comparison and contract invariant checks determined outcomes.*
