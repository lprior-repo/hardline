# Contract Specification: TOCTOU Race Condition in Directory Creation

```
bead_id: hl-6h1
bead_title: High: TOCTOU race condition in directory creation
phase: state-1
updated_at: 2026-03-29T19:53:00Z
```

---

## Context

### Feature
Fix a Time-Of-Check-Time-Of-Use (TOCTOU) race condition in the cross-process lock acquisition path for workspace creation. The `.isolate` data directory is created BEFORE the exclusive file lock is acquired, leaking a "half-initialized" signal to concurrent processes and recovery logic.

### Domain Terms

| Term | Definition |
|------|-----------|
| **TOCTOU** | Time-Of-Check-Time-Of-Use: a race between checking a condition and acting on it |
| **Lock file** | A file used for advisory file-level locking via `fs2::FileExt` |
| **Lock directory** | The directory containing the lock file (currently `.isolate/`) |
| **Data directory** | The `.isolate/` directory that holds runtime state, databases, and config |
| **Lock guard** | RAII handle (`File`) that releases the file lock on drop |
| **Phantom signal** | A side effect (directory creation) that persists after a crash, misleading other processes |
| **Cross-process lock** | An advisory file lock serialized across OS processes via `fs2` |
| **In-process lock** | An async `tokio::Mutex` serialized within a single process |

### Vulnerable Code Location
- **File**: `crates/core/src/jj_operation_sync/jj_lock.rs`
- **Function**: `acquire_cross_process_lock(repo_root: &Path) -> Result<File>`
- **Lines**: 131-186

### Vulnerable Sequence (Current)

```
Process A                          Process B
-----------                        -----------
acquire_cross_process_lock()
  |
  +-> create_dir_all(.isolate)    |
  |   [RACE WINDOW STARTS]        |
  |   *** CRASH HERE ***          |
  |                                acquire_cross_process_lock()
  |                                  |
  |                                +-> create_dir_all(.isolate)  [no-op, already exists]
  |                                +-> open(.isolate/workspace-create.lock)
  |                                +-> try_lock_exclusive()     [SUCCEEDS - A never locked it]
  |                                +-> [proceeds with workspace creation]
  |                                +-> [BUT .isolate was a phantom from A's crash]
```

### Actual Impact

The race window exists between:
1. `tokio::fs::create_dir_all(&lock_dir)` (line 133) - creates `.isolate`
2. `acquire_file_lock_with_timeout(&file, ...)` (line 148) - acquires file lock

If Process A crashes between these two operations:
- `.isolate` directory persists as a phantom signal
- Other code checking `.isolate` existence (`recovery.rs:57`, `workspace_integrity/checks.rs:236`) gets a false positive
- Process B acquires the file lock successfully (no contention) and proceeds
- The real danger: if B also crashes in the same window, or if the data directory is assumed to be "fully initialized" because it exists, downstream operations may operate on a corrupted/partial state

### Assumptions
- The filesystem supports advisory file locking (Linux `flock`/`fcntl`)
- `fs2::FileExt::try_lock_exclusive()` is the canonical lock mechanism
- The `.isolate` directory name is a convention, not a hard requirement for lock placement
- `OpenOptions::new().create(true)` creates the file but NOT parent directories

### Open Questions
1. Should the lock file path change from `.isolate/workspace-create.lock` to a repo-root-level path (e.g., `.scp-workspace-create.lock`) to fully decouple lock from data? **Recommendation: Yes.**
2. Should the lock acquisition failure be a hard error or trigger automatic stale-lock cleanup? **Recommendation: Hard error with descriptive message.**
3. Is there any code that relies on `.isolate` directory creation as a signal for "init in progress"? **Answer: `recovery.rs:57` checks `.isolate` existence to decide whether to log.**

---

## Preconditions

### P1: Lock Acquisition Preconditions
- **P1.1**: `repo_root` MUST be an existing, accessible directory on a filesystem that supports advisory file locking
- **P1.2**: The calling process MUST have read+write permissions on `repo_root`
- **P1.3**: No other process MAY hold the exclusive file lock on the lock file path (or the caller accepts blocking until timeout)

### P2: Data Directory Creation Preconditions (AFTER lock)
- **P2.1**: The exclusive file lock MUST be held BEFORE any data directory (`.isolate`) creation
- **P2.2**: The data directory MUST NOT exist OR the lock holder is the one who created it

### P3: Cleanup Preconditions
- **P3.1**: If lock acquisition fails, the `.isolate` directory MUST NOT have been created
- **P3.2**: If the process crashes, the file lock MUST be automatically released by the OS (guaranteed by advisory locks on process exit)

---

## Postconditions

### Q1: Lock Acquisition Postconditions
- **Q1.1**: On success, returns `Ok(File)` where the `File` holds an exclusive advisory lock
- **Q1.2**: On success, the lock file exists at the lock path
- **Q1.3**: On success, returns `Ok(File)` with exclusive lock held. Does NOT create `.isolate` directory — that is the caller's responsibility via `ensure_data_directory()`
- **Q1.4**: On success, no concurrent process can acquire the same file lock until this `File` is dropped

### Q2: Data Directory Postconditions
- **Q2.1**: `.isolate` directory exists if and only if the lock is held by the creating process
- **Q2.2**: `.isolate` directory is never in a "half-initialized" state visible to other processes

### Q3: Failure Postconditions
- **Q3.1**: On lock acquisition failure (timeout), the `.isolate` directory MUST NOT have been created by this call
- **Q3.2**: On directory creation failure after lock acquisition, the lock is released (File dropped) and error is returned
- **Q3.3**: On any error, no phantom `.isolate` directory is left behind

---

## Invariants

### I1: Lock-Before-Create Invariant (PRIMARY)
```
FOR ALL calls to acquire_cross_process_lock:
  holding(lock) IMPLIES .isolate directory exists
  .isolate directory exists IMPLIES holding(lock) OR lock was recently released
```

### I2: No Phantom Directory Invariant
```
FOR ALL failure paths:
  returned Err(_) IMPLIES .isolate directory state is unchanged from before the call
```

### I3: Atomic Visibility Invariant
```
FOR ALL concurrent processes:
  Process B observes .isolate directory
  IMPLIES
  the lock that created .isolate was either held or recently released
  (never "directory exists but lock was never acquired")
```

### I4: Idempotent Lock Acquisition
```
acquire_cross_process_lock(repo) followed by drop(lock) followed by acquire_cross_process_lock(repo)
  MUST succeed without error
```

### I5: Single-Holder Invariant
```
FOR ALL File handles returned by acquire_cross_process_lock:
  count(holders_of_same_lock_path) <= 1
```

---

## Error Taxonomy

### E1: Lock Acquisition Errors

| Error Variant | When | Category | Code |
|---|---|---|---|
| `Error::Jj(JjErrorKind::LockTimeout)` | File lock not acquired within retry budget | Transient, retryable | 37 |
| `Error::Io(IoErrorKind::IoError)` | Cannot open lock file (permission, FS error) | Permanent, check filesystem | 64 |
| `Error::Io(IoErrorKind::IoError)` | Cannot create lock directory (after lock held) | Permanent, check filesystem | 64 |
| `Error::Internal(InternalErrorKind::Internal)` | `spawn_blocking` join failure | System resource | 90 |

### E2: Lock Portability Warning (Non-Fatal)

| Condition | Behavior |
|---|---|
| Filesystem does not support advisory locks | Log warning, set `lock_supported = false`, continue unless `Isolate_STRICT_LOCKS` env var set |
| `Isolate_STRICT_LOCKS` set + unsupported FS | Return `Error::State(StateErrorKind::ValidationError)` |

### E3: New Error Variant Needed (Post-Fix)

| Error Variant | When | Purpose |
|---|---|---|
| `Error::Io(IoErrorKind::IoError("Failed to create data directory: {e}"))` | `.isolate` dir creation fails AFTER lock acquired | Distinguish from lock failures; signals lock-was-held-but-dir-failed |

---

## Contract Signatures

### Current (Vulnerable)

```rust
/// ACQUIRES lock AND creates .isolate directory as side effect
/// TOCTOU: .isolate created BEFORE lock held
pub async fn acquire_cross_process_lock(repo_root: &Path) -> Result<File>
```

### Proposed (Fixed)

```rust
/// Acquire cross-process file lock at repo root level.
/// 
/// Lock file path: {repo_root}/.scp-workspace-create.lock
/// (NOT inside .isolate/ to avoid chicken-and-egg TOCTOU)
///
/// Post-condition: Returns Ok(File) with exclusive lock held.
/// Does NOT create .isolate directory. Caller must call
/// ensure_data_directory() AFTER acquiring the lock.
pub async fn acquire_cross_process_lock(repo_root: &Path) -> Result<File>

/// Create .isolate data directory.
///
/// PRECONDITION: Caller MUST hold the cross-process lock.
/// If precondition is violated, behavior is undefined (may create
/// phantom directory visible to other processes).
///
/// POSTCONDITION: .isolate directory exists.
/// INVARIANT: Only callable while lock is held.
pub async fn ensure_data_directory(repo_root: &Path) -> Result<()>
```

### Caller Contract (create_workspace_synced)

```rust
/// Current (vulnerable):
///   1. acquire_lock_with_backoff()          // in-process lock
///   2. acquire_cross_process_lock()         // creates .isolate + locks  [TOCTOU HERE]
///   3. create_dir_all(parent)               // creates workspace dir
///   4. jj workspace add                     // creates workspace
///
/// Fixed:
///   1. acquire_lock_with_backoff()          // in-process lock
///   2. acquire_cross_process_lock()         // lock file at repo root, NO .isolate creation
///   3. ensure_data_directory()              // creates .isolate WHILE LOCKED
///   4. create_dir_all(parent)               // creates workspace dir
///   5. jj workspace add                     // creates workspace
pub async fn create_workspace_synced(
    name: &str,
    path: &Path,
    repo_root: &Path,
) -> Result<()>
```

---

## Required Changes

### Change 1: Relocate Lock File (jj_lock.rs)

**Current**: Lock file at `{repo_root}/.isolate/workspace-create.lock`
**Fixed**: Lock file at `{repo_root}/.scp-workspace-create.lock`

**Rationale**: The lock file must reside in a directory that is guaranteed to exist (`repo_root`). Placing it inside `.isolate` creates a circular dependency: you need `.isolate` to open the lock file, but you need the lock to safely create `.isolate`.

### Change 2: Remove create_dir_all from acquire_cross_process_lock (jj_lock.rs)

**Current** (lines 132-135):
```rust
let lock_dir = repo_root.join(".isolate");
tokio::fs::create_dir_all(&lock_dir).await...?;
```

**Fixed**:
```rust
let lock_path = repo_root.join(LOCK_FILE_NAME);  // .scp-workspace-create.lock
// NO create_dir_all - lock file is at repo root, parent always exists
```

### Change 3: Extract ensure_data_directory (jj_lock.rs)

New function that creates `.isolate` directory, to be called AFTER lock is held:
```rust
pub async fn ensure_data_directory(repo_root: &Path) -> Result<()> {
    let data_dir = repo_root.join(".isolate");
    tokio::fs::create_dir_all(&data_dir).await
        .map_err(|e| Error::io_error(format!("Failed to create data directory: {e}")))?;
    Ok(())
}
```

### Change 4: Update create_workspace_synced caller (jj_operations.rs)

Insert `ensure_data_directory()` call between lock acquisition and workspace directory creation.

### Change 5: Update constants (jj_lock.rs)

```rust
// Current:
pub const WORKSPACE_CREATION_LOCK_FILE: &str = "workspace-create.lock";

// Fixed:
pub const WORKSPACE_CREATION_LOCK_FILE: &str = ".scp-workspace-create.lock";
```

### Change 6: Update all tests referencing .isolate path for lock file

- `jj_lock_tests.rs:103-105` - update lock_path to use new constant
- `jj_lock_tests.rs:132-134` - update lock_path to use new constant
- `jj_operations.rs` tests - verify new call order

---

## Non-Goals

- **NG1**: Do NOT change the in-process `WORKSPACE_CREATION_LOCK` Mutex behavior
- **NG2**: Do NOT change the `acquire_file_lock_with_timeout` retry/backoff logic
- **NG3**: Do NOT change the lock portability detection (fs2 probe) behavior
- **NG4**: Do NOT change the `Isolate_STRICT_LOCKS` environment variable handling
- **NG5**: Do NOT migrate from `fs2` to a different locking library
- **NG6**: Do NOT change the coordination/locks SQLite-based session lock manager (separate system)
- **NG7**: Do NOT add distributed locking (this is single-machine, multi-process)

---

## Affected Files

| File | Change Type | Description |
|------|-------------|-------------|
| `crates/core/src/jj_operation_sync/jj_lock.rs` | MODIFY | Relocate lock file, remove `create_dir_all`, add `ensure_data_directory` |
| `crates/core/src/jj_operation_sync/jj_operations.rs` | MODIFY | Call `ensure_data_directory()` after lock acquisition |
| `crates/core/src/jj_operation_sync/jj_lock_tests.rs` | MODIFY | Update lock file path references, add TOCTOU regression tests |
| `crates/core/src/jj_operation_sync/mod.rs` | MODIFY | Export `ensure_data_directory` |

---

## Violation Examples (What the Fix Prevents)

### V1: Phantom Directory on Crash
```
Process A: create_dir_all(.isolate) -> CRASH
Process B: sees .isolate exists -> assumes init in progress -> waits/block/detects stale state
FIX: .isolate never created unless lock is held
```

### V2: Concurrent Directory Creation
```
Process A: create_dir_all(.isolate) -> (slow FS)
Process B: create_dir_all(.isolate) -> (slow FS)
Both: proceed to lock acquisition, one wins, one times out
BUT: .isolate was created by both, potential metadata corruption
FIX: Only one process holds lock when creating .isolate
```

### V3: Recovery False Positive
```
Process A: create_dir_all(.isolate) -> CRASH before lock
recovery.rs:57: sees .isolate exists -> tries to log recovery -> creates recovery.log
Doctor command: sees .isolate + recovery.log -> thinks system was initialized -> skips init
User: runs init -> "already initialized" -> confused
FIX: .isolate only exists when lock was successfully acquired
```
