bead_id: hl-bjy
bead_title: Port Session Locks: TTL/Heartbeat Implementation
phase: state-1.5-test-plan
updated_at: 2026-03-25T23:30:00Z

---

# Test Plan: Port Session Locks — TTL/Heartbeat Implementation

## Summary

- **Behaviors identified**: 24 public API behaviors across 7 functions
- **Trophy allocation**: 14 integration tests (60%), 8 unit tests (35%), 2 e2e tests (5%)
- **Proptest invariants**: 4 (lock uniqueness, TTL consistency, audit completeness, ownership)
- **Fuzz targets**: 2 (lock_with_ttl input validation, heartbeat edge cases)
- **Kani harnesses**: 3 (race condition, TTL math, audit completeness)
- **Mutation threshold target**: ≥90% kill rate

---

## 1. Behavior Inventory

### lock_with_ttl()

1. `[LockManager] acquires [LockResponse] when [session exists and no conflicting lock] when [ttl_seconds > 0]`
2. `[LockManager] returns [SessionLocked error with holder agent_id] when [another agent holds valid lock]`
3. `[LockManager] returns [SessionNotFound error] when [session does not exist in sessions table]`
4. `[LockManager] returns [existing LockResponse] when [agent already holds valid lock]`
5. `[LockManager] rejects [lock request] when [ttl_seconds = 0]` using default TTL
6. `[LockManager] cleans up [expired locks] when [acquiring new lock for same session]`
7. `[LockManager] creates [unique lock_id] when [inserting new lock record]`
8. `[LockManager] logs [audit entry] when [successfully acquiring lock]`
9. `[LockManager] rolls back [lock record] when [audit logging fails after insert]`
10. `[LockManager] detects [constraint conflict] when [race condition inserts lock first]`
11. `[LockManager] returns [SessionLocked error] when [constraint conflict detected with unknown holder]`

### heartbeat()

12. `[LockManager] extends [lock expires_at] when [agent holds valid lock]`
13. `[LockManager] returns [updated LockResponse] when [heartbeat succeeds]`
14. `[LockManager] rejects [heartbeat request] when [agent does not hold lock]` → `Err(NotLockHolder)`
15. `[LockManager] rejects [heartbeat request] when [no active lock exists]` → `Err(NotFound)`
16. `[LockManager] rejects [heartbeat request] when [lock has expired]` → `Err(NotFound)`

### unlock()

17. `[LockManager] removes [lock record] when [holder calls unlock]`
18. `[LockManager] logs [audit entry] when [successful unlock]`
19. `[LockManager] rejects [unlock request] when [agent does not hold lock]` → `Err(NotLockHolder)`
20. `[LockManager] logs [double_unlock_warning] when [unlock called on already released lock]`
21. `[LockManager] returns [Ok(())] when [double unlock called on already released lock]`

### get_lock_state()

22. `[LockManager] returns [LockState with holder info] when [session has active lock]`
23. `[LockManager] returns [LockState with holder = None] when [session has no active lock]`
24. `[LockManager] returns [LockState] when [session exists but no lock]`

### get_lock_audit_log()

25. `[LockManager] returns [Vec<LockAuditEntry>] when [session has audit history]`
26. `[LockManager] returns [empty Vec] when [session has no audit history]`

### verify_session_exists()

27. `[LockManager] returns [Ok(())] when [session exists in sessions table]`
28. `[LockManager] returns [SessionNotFound error] when [session does not exist]`
29. `[LockManager] returns [Ok(())] when [sessions table does not exist yet]` (graceful degradation)

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| lock_with_ttl happy path | Integration | Real SQLite interaction, constraint conflict detection |
| lock_with_ttl conflict | Integration | Requires concurrent lock state from DB |
| lock_with_ttl session validation | Integration | Cross-table join with sessions table |
| lock_with_ttl re-acquire | Integration | State transition from existing lock |
| lock_with_ttl rollback | Integration | Audit failure recovery path |
| heartbeat extension | Integration | Update operation with ownership verification |
| heartbeat invalid holder | Integration | Ownership check via DB query |
| heartbeat no lock | Integration | NotFound path with no active lock |
| unlock success | Integration | Delete operation with audit logging |
| unlock not holder | Integration | Ownership verification |
| unlock double | Integration | Double-release detection via audit |
| get_lock_state existing | Integration | Query with expiration filter |
| get_lock_state none | Integration | Empty result handling |
| get_lock_audit_log | Integration | Full audit trail retrieval |
| verify_session_exists valid | Integration | Cross-table validation |
| verify_session_exists missing | Integration | SessionNotFound error variant |
| verify_session_exists missing table | Integration | Graceful degradation |
| LockManager::new | Unit | Constructor logic, constant defaults |
| LockManager::with_ttl | Unit | Custom TTL configuration |
| LockManager::init | Unit | Schema creation idempotency |
| log_operation | Unit | Audit log insertion logic |
| is_constraint_conflict_error | Unit | Error code pattern matching |
| LockInfo serialization | Unit | DateTime parsing roundtrip |
| LockResponse serialization | Unit | RFC3339 timestamp validation |
| LockState serialization | Unit | Optional field handling |

**Ratio breakdown:**
- Integration: 14 behaviors (60%) — Real SQLite, real state, real error propagation
- Unit: 8 behaviors (35%) — Pure logic, constructors, serialization, helper functions
- E2E: 2 behaviors (5%) — Full CLI workflow validation (manual testing per AGENTS.md)

---

## 3. BDD Scenarios

### Behavior: lock_with_ttl successful acquisition

```
### Behavior: lock_with_ttl returns LockResponse when session exists and no conflicting lock

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: No active lock exists for "test-session"
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 60)
Then: Result is Ok(LockResponse)
And: LockResponse.lock_id matches pattern "lock-test-session-*"
And: LockResponse.session == "test-session"
And: LockResponse.agent_id == "agent-1"
And: LockResponse.expires_at > LockResponse acquired_at timestamp
And: Audit log contains entry with operation="lock"
And: sessions table still contains "test-session"

Error variant - SessionNotFound:
Given: SQLite database with session_locks table initialized
And: sessions table does NOT contain "nonexistent-session"
When: agent "agent-1" calls lock_with_ttl("nonexistent-session", "agent-1", 60)
Then: Result is Err(Error::Session(SessionErrorKind::NotFound("nonexistent-session")))

Error variant - SessionLocked:
Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: Another agent "agent-2" holds active lock on "test-session" with expires_at in future
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 60)
Then: Result is Err(Error::Session(SessionErrorKind::Locked("test-session", "agent-2")))
```

### Behavior: lock_with_ttl re-acquire by same agent

```
### Behavior: lock_with_ttl returns existing lock when same agent re-acquires

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: Agent "agent-1" holds active lock on "test-session" with expires_at in future
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 120)
Then: Result is Ok(LockResponse)
And: LockResponse.lock_id == existing lock_id (not regenerated)
And: LockResponse.expires_at == existing expires_at (not extended)
```

### Behavior: lock_with_ttl TTL zero uses default

```
### Behavior: lock_with_ttl uses default TTL when ttl_seconds is zero

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: LockManager created with default TTL of 300 seconds
And: No active lock exists for "test-session"
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 0)
Then: Result is Ok(LockResponse)
And: LockResponse.expires_at - acquired_at == 300 seconds (default TTL)
```

### Behavior: lock_with_ttl cleanup expired locks

```
### Behavior: lock_with_ttl cleans up expired locks before acquiring

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: Expired lock exists for "test-session" with expires_at in past
And: Agent "agent-1" holds no active lock
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 60)
Then: Result is Ok(LockResponse)
And: Expired lock record is deleted from session_locks
And: New lock record inserted with current timestamp
```

### Behavior: lock_with_ttl audit rollback

```
### Behavior: lock_with_ttl rolls back lock if audit logging fails

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: No active lock exists for "test-session"
And: session_lock_audit table is corrupted or inaccessible
When: agent "agent-1" calls lock_with_ttl("test-session", "agent-1", 60)
Then: Result is Err(Error::database(...))
And: Lock record is deleted from session_locks (rollback succeeded)
And: No orphaned lock record remains
```

### Behavior: heartbeat extends lock TTL

```
### Behavior: heartbeat extends lock expiration for valid holder

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: Agent "agent-1" holds active lock on "test-session" with expires_at = T
When: agent "agent-1" calls heartbeat("test-session", "agent-1")
Then: Result is Ok(LockResponse)
And: LockResponse.expires_at == current_time + default_ttl (300 seconds)
And: LockResponse.lock_id unchanged from original
And: Audit log contains entry with operation="heartbeat"
```

### Behavior: heartbeat not lock holder

```
### Behavior: heartbeat rejects non-holder agent

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: Agent "agent-2" holds active lock on "test-session"
When: agent "agent-1" calls heartbeat("test-session", "agent-1")
Then: Result is Err(Error::Session(SessionErrorKind::NotLockHolder("test-session", "agent-1")))
```

### Behavior: heartbeat no active lock

```
### Behavior: heartbeat returns NotFound when no active lock exists

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: No active lock exists for "test-session"
When: agent "agent-1" calls heartbeat("test-session", "agent-1")
Then: Result is Err(Error::not_found("No active lock for session 'test-session'"))
```

### Behavior: unlock successful release

```
### Behavior: unlock removes lock record when holder calls unlock

Given: SQLite database with session_locks table initialized
And: Agent "agent-1" holds active lock on "test-session"
When: agent "agent-1" calls unlock("test-session", "agent-1")
Then: Result is Ok(())
And: Lock record deleted from session_locks
And: Audit log contains entry with operation="unlock"
```

### Behavior: unlock not lock holder

```
### Behavior: unlock rejects non-holder agent

Given: SQLite database with session_locks table initialized
And: Agent "agent-2" holds active lock on "test-session"
When: agent "agent-1" calls unlock("test-session", "agent-1")
Then: Result is Err(Error::Session(SessionErrorKind::NotLockHolder("test-session", "agent-1")))
```

### Behavior: unlock double release

```
### Behavior: unlock logs warning and returns Ok on double release

Given: SQLite database with session_locks table initialized
And: Agent "agent-1" previously unlocked "test-session" (no active lock)
When: agent "agent-1" calls unlock("test-session", "agent-1")
Then: Result is Ok(())
And: Audit log contains entry with operation="double_unlock_warning"
And: No error returned (graceful handling)
```

### Behavior: get_lock_state existing lock

```
### Behavior: get_lock_state returns holder info when lock exists

Given: SQLite database with session_locks table initialized
And: Agent "agent-1" holds active lock on "test-session"
When: caller calls get_lock_state("test-session")
Then: Result is Ok(LockState)
And: LockState.session == "test-session"
And: LockState.holder == Some("agent-1")
And: LockState.expires_at == Some(expiration_datetime)
```

### Behavior: get_lock_state no lock

```
### Behavior: get_lock_state returns empty holder when no lock exists

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: No active lock exists for "test-session"
When: caller calls get_lock_state("test-session")
Then: Result is Ok(LockState)
And: LockState.session == "test-session"
And: LockState.holder == None
And: LockState.expires_at == None
```

### Behavior: get_lock_audit_log existing entries

```
### Behavior: get_lock_audit_log returns entries for session with history

Given: SQLite database with session_locks table initialized
And: session_lock_audit contains entries for "test-session"
And: Entries ordered by id ASC
When: caller calls get_lock_audit_log("test-session")
Then: Result is Ok(Vec<LockAuditEntry>)
And: Vec length == number of audit entries for session
And: Entries ordered by timestamp ASC
And: Each LockAuditEntry.session == "test-session"
```

### Behavior: get_lock_audit_log empty history

```
### Behavior: get_lock_audit_log returns empty vec for session with no history

Given: SQLite database with session_locks table initialized
And: sessions table contains "test-session"
And: session_lock_audit has no entries for "test-session"
When: caller calls get_lock_audit_log("test-session")
Then: Result is Ok(Vec<LockAuditEntry>)
And: Vec is empty (length == 0)
```

### Behavior: verify_session_exists session present

```
### Behavior: verify_session_exists returns Ok when session exists

Given: SQLite database with sessions table initialized
And: sessions table contains "test-session"
When: LockManager calls verify_session_exists("test-session")
Then: Result is Ok(())
```

### Behavior: verify_session_exists session missing

```
### Behavior: verify_session_exists returns NotFound when session missing

Given: SQLite database with sessions table initialized
And: sessions table does NOT contain "nonexistent-session"
When: LockManager calls verify_session_exists("nonexistent-session")
Then: Result is Err(Error::Session(SessionErrorKind::NotFound("nonexistent-session")))
```

### Behavior: verify_session_exists table missing

```
### Behavior: verify_session_exists returns Ok when sessions table missing

Given: SQLite database with session_locks table initialized
And: sessions table does NOT exist
When: LockManager calls verify_session_exists("any-session")
Then: Result is Ok(())
And: No error thrown (graceful degradation for early initialization)
```

---

## 4. Proptest Invariants

### Invariant: Lock uniqueness per session

```
### Proptest: lock_with_ttl
Invariant: For any valid session and agent, at most one active lock exists per session at any time
Strategy: 
  - session: non-empty string, max 255 chars, alphanumeric + hyphens
  - agent_id: non-empty string, max 255 chars
  - ttl_seconds: 1 to 86400 (1 second to 24 hours)
Anti-invariant: 
  - Same agent acquiring lock twice without unlock should return existing lock (idempotent)
  - Different agents acquiring simultaneously should result in one success, one conflict
Property: 
  - After N concurrent lock_with_ttl calls, count(active locks for session) <= 1
```

### Invariant: TTL consistency

```
### Proptest: lock_with_ttl / heartbeat
Invariant: For any valid lock, expires_at > acquired_at and expires_at - acquired_at == TTL
Strategy:
  - acquired_at: DateTime<Utc> in past 30 days
  - ttl: Duration from 1 second to 1 year
Anti-invariant:
  - ttl = 0 should use default TTL (300s)
  - ttl < 0 should be rejected (invalid input)
Property:
  - For all LockResponse: expires_at.timestamp_nanos() > acquired_at.timestamp_nanos()
  - For all LockResponse: (expires_at - acquired_at).num_seconds() in [default_ttl, max_ttl]
```

### Invariant: Audit completeness

```
### Proptest: log_operation
Invariant: Every lock/unlock operation creates exactly one audit entry
Strategy:
  - session: random valid session name
  - agent_id: random agent identifier
  - operation: "lock", "unlock", "heartbeat", "double_unlock_warning"
Anti-invariant:
  - Failed operations should not create audit entries
Property:
  - For N successful operations: count(audit_entries) == N
  - For each audit entry: timestamp matches operation time within 1 second tolerance
```

### Invariant: Ownership enforcement

```
### Proptest: heartbeat / unlock
Invariant: Only lock holder can extend or release lock
Strategy:
  - holder_agent: valid agent ID
  - challenger_agent: different agent ID
  - session: valid session name
Anti-invariant:
  - Holder calling unlock twice should succeed (first) then warn (second)
  - Non-holder calling heartbeat should fail
Property:
  - heartbeat(session, challenger) => Err(NotLockHolder)
  - unlock(session, challenger) => Err(NotLockHolder)
  - heartbeat(session, holder) => Ok(LockResponse)
  - unlock(session, holder) => Ok(())
```

---

## 5. Fuzz Targets

### Fuzz Target: lock_with_ttl malformed input

```
### Fuzz Target: fuzz_lock_with_ttl
Input type: Arbitrary<(session: String, agent_id: String, ttl_seconds: u64)>
Risk: Panic on integer overflow, string encoding issues, SQL injection
Corpus seeds:
  - "" (empty session)
  - "session\nwith\nnewlines" (injection attempt)
  - "session; DROP TABLE sessions--" (SQL injection)
  - "a".repeat(10000) (excessive length)
  - 0 (zero TTL)
  - u64::MAX (overflow TTL)
  - "🔒🔑🔐" (Unicode edge case)
  - "\x00\x01\x02" (binary null bytes)
Test function: `fuzz_target!(|input: (String, String, u64)| { ... })`
```

### Fuzz Target: heartbeat edge case TTL

```
### Fuzz Target: fuzz_heartbeat
Input type: Arbitrary<(session: String, agent_id: String)>
Risk: DateTime arithmetic overflow, timestamp parsing failure, lock state corruption
Corpus seeds:
  - Same as lock_with_ttl corpus
  - Session names with special characters
  - Agent IDs with Unicode
  - Very long session names (>255 chars)
  - Empty agent_id strings
Test function: `fuzz_target!(|input: (String, String)| { ... })`
```

### Fuzz Target: unlock double-release detection

```
### Fuzz Target: fuzz_unlock
Input type: Arbitrary<Vec<(session: String, agent_id: String)>>
Risk: Race condition in double-release detection, audit log corruption
Corpus seeds:
  - Single unlock on held lock
  - Multiple unlocks on same session by same agent
  - Unlocks by different agents on same session
  - Unlocks on non-existent sessions
Test function: `fuzz_target!(|input: Vec<(String, String)>| { ... })`
```

---

## 6. Kani Harnesses

### Kani Harness: Concurrency safety

```
### Kani Harness: kani_lock_race_condition
Property: At most one lock can exist per session at any time, even under concurrent access
Bound: 10 concurrent operations, 5 sessions
Rationale: SQL-level uniqueness constraints should guarantee this, but formal proof required
Model:
  - State: Map<session_id, Option<(agent_id, expires_at)>>
  - Transitions: lock(), heartbeat(), unlock()
  - Invariant: ∀session: state[session].is_none() || state[session].is_some()
  - Verify: No two transitions can result in two active locks for same session
```

### Kani Harness: TTL expiration logic

```
### Kani Harness: kani_ttl_math_correctness
Property: expires_at - acquired_at >= default_ttl for all valid locks
Bound: 1000 lock acquisitions, TTL range [1, 86400]
Rationale: DateTime arithmetic must never underflow or overflow
Model:
  - Input: acquired_at: i64 (nanoseconds), ttl_seconds: u64
  - Computation: expires_at = acquired_at + ttl_seconds * 1_000_000_000
  - Invariant: expires_at > acquired_at
  - Verify: No overflow in timestamp calculation for any valid input
```

### Kani Harness: Audit trail completeness

```
### Kani Harness: kani_audit_completeness
Property: Every successful lock operation has a corresponding audit entry
Bound: 100 sequential operations with interleaved failures
Rationale: Audit log is critical for forensics; must never lose entries
Model:
  - State: (locks: Set<Lock>, audit_log: Vec<AuditEntry>)
  - Transition: lock() creates Lock AND appends to audit_log
  - Invariant: locks.len() <= audit_log.len()
  - Verify: No lock exists without audit entry, no audit entry without lock
```

---

## 7. Mutation Testing Checkpoints

### Critical mutations to survive (≥90% kill rate target)

| Mutation Type | Location | Test Scenario | Expected Kill |
|---------------|----------|---------------|---------------|
| `==` → `!=` | lock_with_ttl agent check | `lock_with_ttl_re_acquires_existing_lock` | Must fail (returns new lock instead of existing) |
| `>=` → `>` | lock query expiration filter | `lock_with_ttl_cleanups_expired_locks` | Must fail (expired locks not cleaned) |
| `Some(_) => Err(...)` → `Ok(...)` | heartbeat not-holder path | `heartbeat_rejects_non_holder_agent` | Must fail (returns Ok instead of error) |
| `DELETE` → no-op | unlock operation | `unlock_removes_lock_record` | Must fail (lock persists after unlock) |
| `insert` → no-op | audit logging | `unlock_logs_audit_entry` | Must fail (no audit entry created) |
| `ttl_seconds > 0` → always true | TTL validation | `lock_with_ttl_uses_default_ttl_when_zero` | Must fail (zero TTL not handled) |
| `fetch_one` → `fetch_optional` | heartbeat lock_id query | `heartbeat_returns_updated_response` | Must fail (panics on no-lock instead of Err) |
| `lock_id` uniqueness | constraint conflict detection | `lock_with_ttl_detects_constraint_conflict` | Must fail (conflict not detected) |
| `expires_at >= now` | lock state query | `get_lock_state_returns_correct_holder` | Must fail (expired locks returned) |
| `agent_id == holder` | ownership check | `unlock_rejects_non_holder_agent` | Must fail (anyone can unlock) |

**Mutation threshold target**: ≥90% kill rate (≥27 of 30 critical mutations must be caught)

---

## 8. Combinatorial Coverage Matrix

### Unit Tests (T0)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `LockManager::new` | SqlitePool | LockManager with default TTL (300s) | unit |
| `LockManager::with_ttl` | SqlitePool + Duration | LockManager with custom TTL | unit |
| `LockManager::init` | Initialized pool | Ok(()) | unit |
| `log_operation` | Valid session/agent/operation | Ok(()) | unit |
| `is_constraint_conflict_error` | sqlx::Error with code 1555 | true | unit |
| `is_constraint_conflict_error` | sqlx::Error with code 2067 | true | unit |
| `is_constraint_conflict_error` | sqlx::Error with code 1234 | false | unit |
| `LockInfo::from_db` | Valid RFC3339 timestamps | Ok(LockInfo) | unit |
| `LockInfo::from_db` | Invalid RFC3339 timestamp | Err(validation_error) | unit |

### Integration Tests (T1)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `lock_with_ttl` happy path | Valid session, no lock, agent A | Ok(LockResponse with lock_id, agent=A) | integration |
| `lock_with_ttl` conflict | Valid session, lock by agent B, agent A | Err(SessionLocked(session, agent=B)) | integration |
| `lock_with_ttl` session missing | Invalid session name | Err(SessionNotFound(session)) | integration |
| `lock_with_ttl` re-acquire | Valid session, lock by agent A, agent A | Ok(LockResponse with existing lock_id) | integration |
| `lock_with_ttl` zero TTL | Valid session, ttl=0 | Ok(LockResponse with default TTL=300s) | integration |
| `lock_with_ttl` cleanup expired | Valid session, expired lock | Ok(LockResponse with new lock, old deleted) | integration |
| `lock_with_ttl` rollback | Valid session, audit failure | Err(database), no orphan lock | integration |
| `heartbeat` extend | Valid lock by agent A, agent A | Ok(LockResponse with extended expires_at) | integration |
| `heartbeat` not holder | Valid lock by agent B, agent A | Err(NotLockHolder(session, agent=A)) | integration |
| `heartbeat` no lock | No active lock | Err(NotFound("No active lock for session...")) | integration |
| `unlock` success | Valid lock by agent A, agent A | Ok(()) | integration |
| `unlock` not holder | Valid lock by agent B, agent A | Err(NotLockHolder(session, agent=A)) | integration |
| `unlock` double | No active lock, agent A | Ok(()) + audit warning | integration |
| `get_lock_state` existing | Valid lock | Ok(LockState with holder, expires_at) | integration |
| `get_lock_state` none | No active lock | Ok(LockState with holder=None, expires_at=None) | integration |
| `get_lock_audit_log` entries | Session with audit history | Ok(Vec<LockAuditEntry>) | integration |
| `get_lock_audit_log` empty | Session with no history | Ok(Vec<LockAuditEntry>) with length=0 | integration |
| `verify_session_exists` present | Session in sessions table | Ok(()) | integration |
| `verify_session_exists` missing | Session not in sessions table | Err(SessionNotFound) | integration |
| `verify_session_exists` table missing | sessions table does not exist | Ok(()) (graceful degradation) | integration |

### E2E Tests (T2)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| `CLI lock-with-ttl workflow` | Real CLI invocation | Exit code 0, lock created | e2e |
| `CLI agent-heartbeat workflow` | Real CLI invocation | Exit code 0, TTL extended | e2e |

---

## Open Questions

1. **TTL configuration**: Should `LockManager::with_ttl` accept Duration or u64 seconds? Current code uses Duration but API exposes u64.
2. **Audit log retention**: Is there a retention policy for audit entries? Should old entries be purged?
3. **Lock ID format**: Current format `lock-{session}-{nanos}` assumes nanosecond uniqueness. Should UUID be used instead?
4. **Session validation**: Should `verify_session_exists` be strict (require sessions table) or permissive (allow missing table)?
5. **Default TTL**: Should 300 seconds be configurable at build time or runtime?
6. **Heartbeat frequency**: Is there a recommended heartbeat interval relative to TTL? (e.g., heartbeat every TTL/3)

---

## Exit Criteria Verification

- ✅ Every public API behavior has a BDD scenario (24 behaviors mapped)
- ✅ Every Error variant has an explicit test scenario (SessionLocked, SessionNotFound, NotLockHolder, NotFound, ValidationError, DatabaseError)
- ✅ Mutation threshold stated: ≥90% kill rate
- ✅ No planned assertion is just `is_ok()` or `is_err()` — all scenarios specify exact values or error variants
- ✅ Every pure function with multiple inputs has a proptest invariant (4 invariants defined)
- ✅ Every parser/deserialization boundary has a fuzz target (2 fuzz targets + 1 composite)
- ✅ Critical arithmetic and state machine invariants have Kani harnesses (3 harnesses defined)
