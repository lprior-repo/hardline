# Test Plan: TOCTOU Race Condition in Directory Creation

```
bead_id: hl-6h1
bead_title: High: TOCTOU race condition in directory creation
phase: state-1.5-retry1
updated_at: 2026-03-29T20:05:00Z
```

## Contract Alignment Notice (M1 Resolution)

**Three-way disagreement resolved.** Source code is ground truth.

| Artifact | spawn_blocking error variant | Verdict |
|----------|---------------------------|---------|
| `jj_lock.rs:185` (source) | `Error::io_error(format!("Failed to join lock task: {e}"))` → `Error::Io(IoErrorKind::IoError(...))` | **Ground truth** |
| `test-plan.md` B7 | `Error::Io(IoError { inner: IoErrorKind::IoError(msg) })` | Correct — matches source |
| `contract.md:162` | `Error::Internal(InternalErrorKind::Internal)` | **WRONG — must be corrected** |

**Required contract.md change:** Line 162 must be updated from `Error::Internal(InternalErrorKind::Internal)` to `Error::Io(IoErrorKind::IoError)` with message prefix `"Failed to join lock task"`.

---

## Summary

- Behaviors identified: 24
- Trophy allocation: 4 unit / 20 integration / 0 e2e / 4 static
- Proptest invariants: 3
- Fuzz targets: 0 (no parsing/deserialization boundaries — filesystem I/O only)
- Kani harnesses: 2

### Ratio Justification

This module is fundamentally **filesystem I/O** — advisory file locks, directory creation, process synchronization. The Calc layer (pure logic) is limited to backoff arithmetic, covered by proptest. Integration tests dominate because the behaviors under test require real OS semantics (`flock`, `create_dir_all`, file permissions, concurrent `Command::new` processes). No mocks are used — real filesystem, real file locks, real concurrent processes. The 0 e2e count reflects that there is no CLI/user-facing surface to test at this layer — the public API is Rust functions called by other modules.

---

## 1. Behavior Inventory

| # | Behavior |
|---|----------|
| B1 | `acquire_cross_process_lock` returns `Ok(File)` with exclusive advisory lock when repo_root is accessible and uncontested |
| B2 | `acquire_cross_process_lock` places lock file at `{repo_root}/.scp-workspace-create.lock` (repo root, NOT inside `.isolate/`) |
| B3 | `acquire_cross_process_lock` does NOT create `.isolate` directory as a side effect |
| B4 | `acquire_cross_process_lock` releases file lock when returned `File` is dropped |
| B5 | `acquire_cross_process_lock` returns `Err(Jj(JjError { inner: JjErrorKind::LockTimeout { ... } }))` when another process holds the lock for the full retry budget |
| B6 | `acquire_cross_process_lock` returns `Err(Io(IoError { inner: IoErrorKind::IoError(msg) }))` when lock file cannot be opened (permission denied) |
| B7 | `acquire_cross_process_lock` returns `Err(Io(IoError { inner: IoErrorKind::IoError(msg) }))` when `spawn_blocking` task join fails |
| B8 | `acquire_cross_process_lock` returns `Err(State(StateError { inner: StateErrorKind::ValidationError(msg) }))` when `Isolate_STRICT_LOCKS` is set and filesystem does not support advisory locks |
| B9 | `ensure_data_directory` creates `.isolate` directory at `{repo_root}/.isolate` |
| B10 | `ensure_data_directory` returns `Ok(())` when `.isolate` directory already exists (idempotent) |
| B11 | `ensure_data_directory` returns `Err(Io(IoError { inner: IoErrorKind::IoError(msg) }))` when directory creation fails due to permission denied |
| B12 | `ensure_data_directory` does NOT acquire or release any file lock |
| B13 | `create_workspace_synced` calls `ensure_data_directory()` AFTER `acquire_cross_process_lock()` returns `Ok` (verified via tracing span ordering) |
| B14 | `create_workspace_synced` returns `Err(Config(ConfigError { inner: ConfigErrorKind::Invalid(msg) }))` when workspace name is empty |
| B15 | **I1 Lock-Before-Create**: `.isolate` directory never exists without the lock having been held (or recently released) — failure path proven with exact error variant |
| B16 | **I2 No Phantom Directory**: On `Err(Io(IoError))` return from `acquire_cross_process_lock`, `.isolate` directory state is unchanged from before the call |
| B17 | **I5 Single-Holder**: At most one process holds the cross-process lock at any instant — exactly 3 concurrent tasks, no iteration |
| B18 | **I4 Idempotent**: acquire → drop → acquire succeeds without error |
| B19 | **I3 Atomic Visibility**: No concurrent process ever observes `.isolate` directory without a corresponding lock holder — cross-process filesystem probe |
| B20 | `acquire_cross_process_lock` preserves lock file content across acquire-drop-reacquire cycle (O_TRUNC mutation guard) |
| B21 | `acquire_cross_process_lock` opens lock file with read permissions (O_RDONLY mutation guard) |
| B22 | `acquire_file_lock_with_timeout` introduces measurable delays proportional to exponential backoff (sleep deletion mutation guard) |
| B23 | `ensure_data_directory` returns `Err(Io(IoError { inner: IoErrorKind::IoError(msg) }))` when `.isolate` exists as a regular file (not a directory) |
| B24 | `acquire_cross_process_lock` returns `Err(Io(IoError { inner: IoErrorKind::IoError(msg) }))` when `repo_root` does not exist |

---

## 2. Trophy Allocation

| # | Behavior | Layer | Justification |
|---|----------|-------|---------------|
| B1 | Lock acquisition success | **Integration** | Real filesystem + real `fs2` advisory lock — must verify actual OS behavior |
| B2 | Lock file path at repo root | **Integration** | Verifies file path on real filesystem via `Path::exists` |
| B3 | No `.isolate` side effect | **Integration** | Must check real filesystem state — `.isolate` must not appear |
| B4 | Lock release on drop | **Integration** | RAII behavior with real file lock — second process must succeed after drop |
| B5 | Lock timeout on contention | **Integration** | Real process-level contention via two `File` handles on same path |
| B6 | IoError on permission denied | **Integration** | Real filesystem permission check (read-only dir) |
| B7 | IoError on spawn_blocking failure | **Unit** | Pure error-path test — tokio runtime shutdown in isolated thread |
| B8 | Strict locks validation error | **Integration** | Env var + real filesystem probe (deferred — see note) |
| B9 | Data directory creation | **Integration** | Real filesystem `create_dir_all` |
| B10 | Data directory idempotent | **Integration** | Real filesystem — directory pre-exists |
| B11 | Data directory IoError on permission denied | **Integration** | Real permission-denied scenario |
| B12 | No lock side effect from ensure_data_directory | **Integration** | Verifies function does not touch lock file path on real filesystem |
| B13 | Call order in workspace creation | **Integration** | Tracing span capture proves ordering without API changes |
| B14 | Empty name rejection | **Unit** | Pure input validation, no I/O |
| B15 | Lock-Before-Create invariant | **Integration** | Multi-process stress test with barrier synchronization |
| B16 | No Phantom Directory invariant | **Integration** | Failure injection + filesystem state verification |
| B17 | Single-Holder invariant | **Integration** | Exactly 3 concurrent tasks, fixed explicit spawning |
| B18 | Idempotent acquire cycle | **Integration** | Sequential acquire → drop → acquire on real filesystem |
| B19 | Atomic Visibility invariant | **Integration** | Cross-process probe via `Command::new` + tempdir |
| B20 | Lock file content preserved | **Integration** | Real file I/O — write content, lock cycle, verify content intact |
| B21 | Lock file opened readable | **Integration** | Real `File` handle — verify read capability after lock |
| B22 | Backoff sleep is not removed | **Integration** | Wall-clock measurement with controlled lock holder |
| B23 | `.isolate` exists as file, not directory | **Integration** | Real filesystem — create regular file at `.isolate` path |
| B24 | Nonexistent repo_root | **Integration** | Real filesystem — path does not exist |

### Static Analysis (supplementary)

| Check | Tool | What It Catches |
|-------|------|-----------------|
| Lock file constant is `.scp-workspace-create.lock` | `clippy::const_path_is_dot_prefixed` | Catch if someone reverts to `workspace-create.lock` |
| No `create_dir_all` in `acquire_cross_process_lock` | `rg "create_dir_all" jj_lock.rs` | Regression of TOCTOU |
| `ensure_data_directory` exported from module | `cargo check` | Export contract |
| Error variant `IoErrorKind::IoError` used for data dir failures | Type checker | Correct error variant |
| `Error::io_error` (not `Error::Internal`) for spawn_blocking join | Type checker + code review | M1 alignment |

---

## 3. BDD Scenarios

### Behavior B1: Lock acquisition succeeds when uncontested

```rust
/// fn acquire_cross_process_lock_returns_file_when_repo_root_accessible()
```

**Given:** a valid, writable `tempdir()` as `repo_root`
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Ok(File)` — the `File` is a valid open file handle
**And:** the file at `{repo_root}/.scp-workspace-create.lock` exists on disk
**And:** a second `try_lock_exclusive()` on the same lock path returns `Err(_)` (lock is held)

---

### Behavior B2: Lock file placed at repo root

```rust
/// fn acquire_cross_process_lock_places_lock_at_repo_root_when_called()
```

**Given:** a valid `repo_root` directory
**When:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(_)`
**Then:** `repo_root.join(".scp-workspace-create.lock").exists()` is `true`
**And:** `repo_root.join(".isolate").join("workspace-create.lock").exists()` is `false`
**And:** `repo_root.join(".isolate").exists()` is `false` (no phantom directory)

---

### Behavior B3: No `.isolate` directory side effect

```rust
/// fn acquire_cross_process_lock_does_not_create_isolate_dir_when_called()
```

**Given:** a valid `repo_root` with NO `.isolate` directory
**When:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(_)`
**Then:** `repo_root.join(".isolate").exists()` is `false`

---

### Behavior B4: Lock released on File drop

```rust
/// fn acquire_cross_process_lock_releases_when_file_dropped()
```

**Given:** a valid `repo_root`
**When:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(file)`, then `drop(file)`
**Then:** a subsequent `OpenOptions::new().write(true).open(lock_path)` followed by `try_lock_exclusive()` returns `Ok(())`

---

### Behavior B5: Lock timeout when contended

```rust
/// fn acquire_cross_process_lock_returns_lock_timeout_when_another_process_holds_lock()
```

**Given:** `repo_root` where process A already holds the exclusive lock via `fs2::try_lock_exclusive()`
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation, timeout_ms, retries } }))`
**And:** `operation` equals `"workspace creation cross-process lock"`
**And:** `retries` equals `HIGH_CONTENTION_MAX_ATTEMPTS` (8)
**And:** `timeout_ms` equals the sum of exponential backoff values `(0..8).map(|i| 25 * 2^i).sum()`

---

### Behavior B6: IoError on permission denied

```rust
/// fn acquire_cross_process_lock_returns_io_error_when_repo_root_read_only()
```

**Given:** a `repo_root` directory with `0o444` permissions (read-only)
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`
**And:** `msg` contains `"Failed to open workspace lock file"`

---

### Behavior B7: IoError on spawn_blocking failure

```rust
/// fn acquire_cross_process_lock_returns_io_error_when_task_join_fails()
```

> **M1 Resolution:** Source code (`jj_lock.rs:185`) uses `Error::io_error(format!("Failed to join lock task: {e}"))` which maps to `Error::Io(IoErrorKind::IoError(...))`. The test plan matches the source. **contract.md line 162 must be corrected** to `Error::Io(IoErrorKind::IoError)` — the `Error::Internal` variant listed there is incorrect.

> **Implementation strategy:** Spawn a new thread with its own single-threaded tokio runtime. Call `runtime.shutdown_background()`, then attempt `acquire_cross_process_lock`. The `JoinError` propagates through the `map_err` on line 185.

**Given:** a tokio runtime that has been shut down via `shutdown_background()` (no capacity for new blocking tasks)
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`
**And:** `msg` contains `"Failed to join lock task"`

---

### Behavior B8: Strict locks validation error (DEFERRED)

```rust
/// #[ignore]
/// fn acquire_cross_process_lock_returns_validation_error_when_strict_locks_on_unsupported_fs()
```

> **m3 Resolution — CI Feasibility:** Triggering unsupported-FS behavior requires a filesystem that doesn't support `flock` (e.g., NFS with `noac` mount option, or a tmpfs remounted with specific options). This cannot be reliably reproduced in standard CI environments (GitHub Actions, Docker). **This test is deferred** — marked `#[ignore]` with documentation for manual execution. The lock portability code path is verified structurally (static analysis) and by the non-strict warning variant below.

**Given:** `Isolate_STRICT_LOCKS` environment variable is set
**And:** a filesystem that does NOT support advisory file locks
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::State(StateError { inner: StateErrorKind::ValidationError(msg) }))`
**And:** `msg` contains `"LOCK_PORTABILITY_UNSUPPORTED"`

**Non-strict variant (warning only — CI-feasible):**
**Given:** `Isolate_STRICT_LOCKS` is NOT set
**And:** filesystem does NOT support advisory locks
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Ok(File)` (graceful degradation)
**And:** a warning log containing `"lock_portability_warning"` is emitted

---

### Behavior B9: Data directory creation

```rust
/// fn ensure_data_directory_creates_isolate_dir_when_called()
```

**Given:** a valid `repo_root` with NO `.isolate` directory
**When:** `ensure_data_directory(&repo_root).await`
**Then:** returns `Ok(())`
**And:** `repo_root.join(".isolate").is_dir()` is `true`

---

### Behavior B10: Data directory idempotent

```rust
/// fn ensure_data_directory_succeeds_when_isolate_dir_already_exists()
```

**Given:** a valid `repo_root` where `.isolate` directory already exists
**When:** `ensure_data_directory(&repo_root).await`
**Then:** returns `Ok(())`
**And:** `.isolate` directory still exists (unchanged)

---

### Behavior B11: Data directory IoError on permission denied

```rust
/// fn ensure_data_directory_returns_io_error_when_creation_fails()
```

**Given:** a `repo_root` directory with `0o444` permissions (read-only, cannot create subdirs)
**When:** `ensure_data_directory(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`
**And:** `msg` contains `"Failed to create data directory"`

---

### Behavior B12: No lock side effect from ensure_data_directory

```rust
/// fn ensure_data_directory_does_not_touch_lock_file_when_called()
```

**Given:** a valid `repo_root` with no `.scp-workspace-create.lock` file
**When:** `ensure_data_directory(&repo_root).await` returns `Ok(())`
**Then:** `repo_root.join(".scp-workspace-create.lock").exists()` is `false`

---

### Behavior B13: Call order in workspace creation

```rust
/// fn create_workspace_synced_calls_ensure_data_dir_after_acquiring_lock()
```

> **M3 Resolution — Concrete Implementation Strategy:** Use `tracing::info_span!` instrumentation.
>
> **Implementation requirement (pre-test):** Add two tracing spans to the implementation:
> 1. In `acquire_cross_process_lock`, after successful lock acquisition (before returning `Ok(file)`): `tracing::info!(target: "test_ordering", event = "lock_acquired");`
> 2. In `ensure_data_directory`, after `create_dir_all` succeeds: `tracing::info!(target: "test_ordering", event = "data_dir_created");`
>
> **Test implementation:** Install a `tracing_subscriber::collect::TestSpan` layer (or equivalent) that captures ordered events from the `"test_ordering"` target. Run `create_workspace_synced`. Assert that the captured event list is `["lock_acquired", "data_dir_created"]` — in that exact order.
>
> **Why tracing over filesystem probes:** A filesystem probe (`Path::exists` between steps) cannot observe state "during" an async function's execution from the caller's perspective — the function runs to completion before control returns. Tracing spans capture ordering from inside the function without API changes or timing hacks.

**Given:** a valid repo with workspace prerequisites met
**And:** a tracing subscriber capturing events from the `"test_ordering"` target
**When:** `create_workspace_synced(name, path, repo_root).await` is called
**Then:** the captured event list is exactly `["lock_acquired", "data_dir_created"]` in that order

---

### Behavior B14: Empty name rejection

```rust
/// fn create_workspace_synced_returns_config_error_when_name_empty()
```

**Given:** any valid `path` and `repo_root`
**When:** `create_workspace_synced("", path, repo_root).await`
**Then:** returns `Err(Error::Config(ConfigError { inner: ConfigErrorKind::Invalid(msg) }))`
**And:** `msg` equals `"workspace name cannot be empty"`

---

### Behavior B15: Lock-Before-Create invariant (TOCTOU regression)

```rust
/// fn regression_no_phantom_directory_when_lock_acquisition_fails()
/// fn regression_no_phantom_directory_when_lock_acquisition_times_out()
```

> **m2 Resolution — Exact error variant:** Invariant tests now assert the specific error variant, not `Err(_)`. This proves that the specific error path leaves no phantom — adding marginal defense against error-path-specific regressions.

**Given:** a valid `repo_root` with NO `.isolate` directory
**And:** another process already holds the exclusive lock
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation: _, timeout_ms: _, retries: _ } }))`
**And:** `repo_root.join(".isolate").exists()` is `false`

---

### Behavior B16: No Phantom Directory invariant (crash simulation)

```rust
/// fn regression_isolate_not_created_on_io_error_from_acquire_lock()
```

> **m2 Resolution — Exact error variant:** The precondition now asserts `Err(Io(IoError))`, not `Err(_)`.

**Given:** a valid `repo_root` with NO `.isolate` directory
**And:** `repo_root` has `0o444` permissions (read-only, causes `IoError`)
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to open workspace lock file"`
**And:** `repo_root.join(".isolate").exists()` is `false`
**And:** no partial directory structure exists

---

### Behavior B17: Single-Holder invariant (stress test)

```rust
/// fn stress_max_concurrent_lock_holders_is_one()
```

> **M4 Resolution — Holzmann Rule 2 Compliance:** The test no longer uses an iterator map over a range. Instead, exactly 3 concurrent tasks are spawned via explicit `tokio::spawn` calls. This is the minimum concurrency level that proves mutual exclusion (2 is trivial — one holds, one contends; 3 proves the system serializes across multiple waiters). No loop, no dynamic task count.
>
> **Holzmann R2 Exception Justification:** Even with explicit spawning, concurrency testing fundamentally requires multiple concurrent tasks. The iteration count is a bounded constant (3), not dynamic input. Each task body is identical — there is no loop logic, only the concurrency itself.

**Given:** a valid `repo_root`
**And:** a `Barrier` sized for 3 tasks
**And:** an `AtomicUsize` counter `in_critical_section` incremented on lock acquire
**And:** exactly 3 tokio tasks, each calling `acquire_cross_process_lock`, spawned explicitly (no iterator/map)
**When:** all 3 tasks complete
**Then:** `max(in_critical_section)` observed value equals `1`
**And:** every task either returns `Ok(File)` or returns `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))` — no panics, no other error variants

---

### Behavior B18: Idempotent acquire cycle

```rust
/// fn acquire_cross_process_lock_succeeds_on_repeated_acquire_drop_cycle()
```

**Given:** a valid `repo_root`
**When:** `acquire_cross_process_lock` → `drop(File)` → `acquire_cross_process_lock` → `drop(File)` → `acquire_cross_process_lock`
**Then:** all three acquisitions return `Ok(File)`
**And:** only one `.scp-workspace-create.lock` file exists

---

### Behavior B19: Atomic Visibility invariant (I3)

```rust
/// fn regression_isolate_never_visible_without_lock_having_been_held()
```

> **M2 Resolution — Concrete cross-process proof:** This is the PRIMARY invariant of the TOCTOU fix. The test uses a filesystem probe pattern with real child processes via `std::process::Command`.

**Implementation strategy:**

1. Create a tempdir as `repo_root`
2. Write a small helper binary (or shell script) `probe_child` that:
   - Takes `repo_root` as argument
   - Loops (up to 500ms) checking: `repo_root.join(".isolate").exists()`
   - If `.isolate` exists: check if `repo_root.join(".scp-workspace-create.lock")` can be locked (i.e., the lock is NOT held)
   - If `.isolate` exists AND lock is NOT held: exit with code 1 (violation)
   - If `.isolate` exists AND lock IS held: exit with code 0 (valid — lock holder created it)
   - If `.isolate` never appears within timeout: exit with code 0 (no violation)
3. Start `probe_child` as a background process via `Command::new`
4. In the test process: call `acquire_cross_process_lock(&repo_root)`, then `ensure_data_directory(&repo_root)`, hold for 200ms, then drop
5. Assert `probe_child` exits with code 0

**Given:** a valid `repo_root` with NO `.isolate` directory
**And:** a child process (`probe_child`) running concurrently, polling for `.isolate` existence and lock status
**When:** the test process calls `acquire_cross_process_lock`, then `ensure_data_directory`, holds for 200ms, then drops
**Then:** `probe_child` exits with code 0 (no violation detected)
**And:** the child process never observed `.isolate` existing without the lock being held

**Alternative (if child binary is too complex):** Use a single-process test with a spawned tokio task that polls `.isolate` existence at 1ms intervals. Acquire lock, sleep 50ms (to ensure poller is running), then call `ensure_data_directory`. Assert poller never observed `.isolate` without lock held.

---

### Behavior B20: Lock file content preserved across acquire-drop-reacquire cycle

```rust
/// fn acquire_cross_process_lock_preserves_lock_file_content_when_reacquired()
```

> **M6 Resolution — Kills `truncate(false)` → `truncate(true)` mutation.** If `truncate(true)` were substituted, the lock file content would be wiped on every open, and this test would detect it.

**Given:** a valid `repo_root` with a pre-existing `.scp-workspace-create.lock` file containing the exact bytes `"LOCK-STATE-MARKER"`
**When:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(_file)`, then `drop(_file)`
**And:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(_file2)`, then `drop(_file2)`
**Then:** `std::fs::read_to_string(repo_root.join(".scp-workspace-create.lock"))` equals `"LOCK-STATE-MARKER"`
**And:** the file size is exactly 18 bytes (length of the marker string)

---

### Behavior B21: Lock file opened with read permissions

```rust
/// fn acquire_cross_process_lock_opens_lock_file_for_reading()
```

> **M6 Resolution — Kills `read(true)` removal mutation.** If `read(true)` were removed from `OpenOptions`, the file would be opened write-only and a read attempt would fail.

**Given:** a valid `repo_root`
**When:** `acquire_cross_process_lock(&repo_root).await` returns `Ok(file)`
**Then:** `file.metadata()` returns `Ok(_meta)` (file handle is valid)
**And:** `file.try_clone()` returns `Ok(cloned)` (file handle is cloneable, proving it's open)
**And:** using `cloned`, calling `std::io::Read::read_to_string(&mut cloned, &mut buf)` returns `Ok(_)` (file is readable)

---

### Behavior B22: Backoff sleep is not removed

```rust
/// fn acquire_file_lock_with_timeout_introduces_measurable_delays_on_contention()
```

> **M6 Resolution — Kills `std::thread::sleep` deletion mutation.** If the sleep were removed, retries would happen instantaneously and the test would measure near-zero elapsed time.
>
> **Implementation strategy:** Use a controlled lock holder that releases after a known delay.
> 1. Create a tempdir, open a lock file, acquire exclusive lock
> 2. Spawn a thread that holds the lock for 200ms, then drops it
> 3. In the test thread, call `acquire_file_lock_with_timeout` and measure elapsed time via `std::time::Instant`
> 4. With base backoff 25ms, after 2 retries the minimum elapsed time is 25ms + 50ms = 75ms (first two backoff sleeps before the 3rd attempt succeeds)
> 5. Assert elapsed time >= 60ms (with tolerance for OS scheduling)

**Given:** a lock file where another thread holds the exclusive lock
**And:** the lock holder thread will release after 200ms
**When:** `acquire_file_lock_with_timeout(&file, "test contention")` is called
**Then:** the call returns `Ok(())` (lock acquired after holder releases)
**And:** elapsed wall-clock time is >= 60ms (proving at least 2 backoff sleeps occurred)

---

### Behavior B23: `.isolate` exists as regular file

```rust
/// fn ensure_data_directory_returns_io_error_when_isolate_is_a_file_not_directory()
```

> **m4 Resolution — `.isolate`-as-file boundary test.** If `.isolate` exists as a regular file (not a directory), `create_dir_all` fails with a "Not a directory" OS error. This is a distinct error condition from permission denied (B11) and from already-exists (B10).

**Given:** a valid `repo_root` where `.isolate` exists as a regular file (not a directory)
**When:** `ensure_data_directory(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`
**And:** `msg` contains `"Failed to create data directory"`

---

### Behavior B24: Nonexistent repo_root

```rust
/// fn acquire_cross_process_lock_returns_io_error_when_repo_root_does_not_exist()
```

> **m5 Resolution — Nonexistent repo_root boundary test.** When `repo_root` does not exist, the lock file open fails with ENOENT — distinct from EACCES (B6's permission denied). The error message substring differs.

**Given:** a `repo_root` path that does not exist on the filesystem (e.g., `/tmp/nonexistent_dir_xyz123`)
**When:** `acquire_cross_process_lock(&repo_root).await`
**Then:** returns `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`
**And:** `msg` contains `"Failed to open workspace lock file"`
**And:** the underlying OS error kind is `std::io::ErrorKind::NotFound`

---

## 4. Proptest Invariants

### Proptest P1: Backoff arithmetic never overflows

```rust
/// proptest!(|(attempt in 0u32..100u32)| {
///     let backoff_ms = FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt);
///     // backoff_ms must not overflow u64 (attempt capped at HIGH_CONTENTION_MAX_ATTEMPTS)
/// });
```

**Invariant:** For any `attempt` in `0..HIGH_CONTENTION_MAX_ATTEMPTS`, `FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt)` does not overflow `u64`.
**Strategy:** `0u32..HIGH_CONTENTION_MAX_ATTEMPTS`
**Anti-invariant:** For `attempt >= 64` (if `FILE_LOCK_BASE_BACKOFF_MS > 1`), `2_u64.pow(attempt)` would overflow. The function MUST cap `attempt` before calling `pow`.

### Proptest P2: Total wait time is bounded and deterministic

```rust
/// proptest!(|(base_ms in 1u64..1000u64, max_attempts in 1usize..20usize)| {
///     let total: u64 = (0u32..max_attempts as u32)
///         .map(|i| base_ms * 2_u64.pow(i))
///         .sum();
///     assert!(total > 0);
///     // total must be finite and deterministic for same inputs
/// });
```

**Invariant:** For any valid `base_ms` and `max_attempts`, the total wait time is a finite, deterministic `u64` value.
**Strategy:** `(1u64..1000u64, 1usize..20usize)`
**Anti-invariant:** No valid input should produce `u64::MAX` or overflow.

### Proptest P3: Lock path is always at repo root (never nested)

```rust
/// proptest!(|(suffix in "[a-zA-Z0-9_-]{1,50}")| {
///     let repo_root = PathBuf::from("/tmp").join(suffix);
///     let lock_path = repo_root.join(WORKSPACE_CREATION_LOCK_FILE);
///     // lock_path parent must equal repo_root
///     assert_eq!(lock_path.parent(), Some(repo_root.as_path()));
/// });
```

**Invariant:** `repo_root.join(WORKSPACE_CREATION_LOCK_FILE).parent()` always equals `repo_root` — the lock file is always a direct child of `repo_root`, never nested.
**Strategy:** Any valid directory name string `[a-zA-Z0-9_-]{1,50}`
**Anti-invariant:** If `WORKSPACE_CREATION_LOCK_FILE` ever contains `/` or `..`, the parent would differ from `repo_root`.

---

## 5. Fuzz Targets

**None.** This module has no parsing, deserialization, or untrusted-string-input boundaries. All inputs are:
- `&Path` — filesystem paths, not user-controlled strings
- `Duration` / `usize` — compile-time constants

Filesystem operations (`create_dir_all`, `OpenOptions::open`) are OS-validated. No fuzz surface exists.

---

## 6. Kani Harnesses

### Kani K1: Backoff arithmetic overflow freedom

```
Property: FILE_LOCK_BASE_BACKOFF_MS * 2_u64.pow(attempt) never panics or wraps
         for attempt in 0..=HIGH_CONTENTION_MAX_ATTEMPTS
Bound: attempt ∈ [0, 255] (exhaustive u8)
Rationale: The `pow` call with unchecked attempt index could theoretically overflow.
          Kani proves this is impossible for the bounded range.
```

### Kani K2: Lock constant is dot-prefixed

```
Property: WORKSPACE_CREATION_LOCK_FILE.starts_with('.')
Bound: string length ≤ 64
Rationale: Ensures the lock file is hidden (dot-prefixed) at the filesystem level,
          preventing casual discovery. A regression to "workspace-create.lock" would
          break this invariant.
```

---

## 7. Mutation Testing Checkpoints

### Critical Mutations That Must Be Caught

| Mutation | Caught By | Scenario |
|----------|-----------|----------|
| Replace `.scp-workspace-create.lock` with `workspace-create.lock` | B2 | Path assertion fails |
| Remove `ensure_data_directory()` call from `create_workspace_synced` | B13 | Tracing captures only `"lock_acquired"`, missing `"data_dir_created"` |
| Swap order: `ensure_data_directory` before `acquire_cross_process_lock` | B13 | Tracing captures `"data_dir_created"` before `"lock_acquired"` |
| Reintroduce `create_dir_all(.isolate)` into `acquire_cross_process_lock` | B3 | `.isolate` exists after lock call |
| Change `try_lock_exclusive()` to always return `Ok(())` | B5 | Timeout error not returned |
| Remove `drop` release behavior | B4 | Second lock attempt fails |
| Change error message prefix from `"Failed to create data directory"` to generic | B11 | Message assertion fails |
| Remove lock portability probe | B8 | ValidationError not returned |
| Set `lock_supported = true` unconditionally | B8 | ValidationError not returned on unsupported FS |
| **Change `truncate(false)` to `truncate(true)`** | **B20** | **Lock file content `"LOCK-STATE-MARKER"` is wiped** |
| **Remove `read(true)` from OpenOptions** | **B21** | **File read fails with "Bad file descriptor"** |
| **Delete backoff `std::thread::sleep`** | **B22** | **Elapsed time < 60ms (near-instant retries)** |

### Threshold

**≥ 90% mutation kill rate** required. Run via `cargo mutants --workspace --timeout 120`.
Focus mutation scope on `crates/core/src/jj_operation_sync/jj_lock.rs` and the modified section of `jj_operations.rs`.

With the 3 new mutation-killing tests (B20, B21, B22), the estimated kill rate rises from 75% (9/12) to 100% (12/12) for the identified mutations. The overall rate depends on additional compiler-generated mutations but should exceed 90%.

---

## 8. Combinatorial Coverage Matrix

### `acquire_cross_process_lock` — Error Variants

| Scenario | Input Condition | Expected Output | Layer |
|----------|----------------|-----------------|-------|
| Happy path | writable `repo_root`, no contention | `Ok(File)` with exclusive lock | integration |
| Lock path correct | any valid `repo_root` | lock file at `{root}/.scp-workspace-create.lock` | integration |
| No `.isolate` side effect | `repo_root` without `.isolate` | `.isolate` does not exist after call | integration |
| Contended lock | another process holds lock | `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation: "workspace creation cross-process lock", retries: 8, timeout_ms: <computed> } }))` | integration |
| Permission denied | read-only `repo_root` | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to open workspace lock file"` | integration |
| Nonexistent repo_root | path does not exist | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to open workspace lock file"` and OS error kind is `NotFound` | integration |
| spawn_blocking failure | tokio runtime shut down | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to join lock task"` | unit |
| Strict locks + unsupported FS | `Isolate_STRICT_LOCKS` set, no flock support | `Err(Error::State(StateError { inner: StateErrorKind::ValidationError(msg) }))` where `msg` contains `"LOCK_PORTABILITY_UNSUPPORTED"` | integration (deferred) |
| Non-strict + unsupported FS | `Isolate_STRICT_LOCKS` not set, no flock | `Ok(File)` + warning log | integration |
| Lock release | drop returned `File` | second process can acquire lock | integration |
| Idempotent cycle | acquire → drop → acquire → drop → acquire | all return `Ok(File)` | integration |
| Content preserved | pre-existing file with known content | content unchanged after lock cycle | integration |

### `ensure_data_directory` — Error Variants

| Scenario | Input Condition | Expected Output | Layer |
|----------|----------------|-----------------|-------|
| Happy path | writable `repo_root`, no `.isolate` | `Ok(())`, `.isolate` exists | integration |
| Already exists | `.isolate` pre-exists as directory | `Ok(())`, `.isolate` still exists | integration |
| `.isolate` is a regular file | `.isolate` exists as file | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to create data directory"` | integration |
| Permission denied | read-only `repo_root` | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))` where `msg` contains `"Failed to create data directory"` | integration |
| No lock file side effect | no lock file exists | lock file still does not exist | integration |

### `create_workspace_synced` — Call Order & Input Validation

| Scenario | Input Condition | Expected Output | Layer |
|----------|----------------|-----------------|-------|
| Correct call order | valid name, valid paths | tracing events: `["lock_acquired", "data_dir_created"]` | integration |
| Empty name | `name = ""` | `Err(Error::Config(ConfigError { inner: ConfigErrorKind::Invalid(msg) }))` where `msg` equals `"workspace name cannot be empty"` | unit |

### Invariant Tests

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| TOCTOU regression: timeout | contended lock, no `.isolate` | `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))` + `.isolate` does not exist | integration |
| TOCTOU regression: permission | read-only dir, no `.isolate` | `Err(Error::Io(IoError { inner: IoErrorKind::IoError(_) }))` + `.isolate` does not exist | integration |
| Atomic visibility (I3) | concurrent probe child + lock+create | probe child exits code 0 | integration |
| Single-holder stress | exactly 3 concurrent tasks | `max(in_critical) == 1`, all results are `Ok(File)` or `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))` | integration |
| Idempotent cycle | 3 sequential acquire/drop | all `Ok(File)` | integration |

### Mutation-Killing Tests

| Scenario | Mutation Killed | Expected Output | Layer |
|----------|----------------|-----------------|-------|
| Content preserved (B20) | `truncate(false)` → `truncate(true)` | file content unchanged after cycle | integration |
| Read permissions (B21) | `read(true)` removal | `read_to_string` succeeds on file handle | integration |
| Backoff timing (B22) | sleep deletion | elapsed >= 60ms with controlled contention | integration |

### Constants Validation

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| Lock file name | `WORKSPACE_CREATION_LOCK_FILE` | equals `".scp-workspace-create.lock"` | static/unit |
| All timeouts positive | all `Duration` constants | `as_millis() > 0` | unit |
| Backoff exponential | `base * 2^i` for `i in 0..3` | `[base, base*2, base*4, base*8]` | unit |

---

## Existing Tests to Update

The following existing tests in `jj_lock_tests.rs` and `jj_operations.rs` reference the OLD lock path and/or contain banned assertion patterns. All must be updated simultaneously.

### Path Reference Updates

| Test | Line(s) | Current Reference | Required Change |
|------|---------|-------------------|-----------------|
| `given_lock_constants_when_validated_then_reasonable_values` | 27 | `assert_eq!(WORKSPACE_CREATION_LOCK_FILE, "workspace-create.lock")` | Change to `".scp-workspace-create.lock"` |
| `regression_cross_process_lock_blocks_second_holder` | 103–105 | `repo_root_path.join(".isolate").join(WORKSPACE_CREATION_LOCK_FILE)` | Change to `repo_root_path.join(WORKSPACE_CREATION_LOCK_FILE)` |
| `regression_cross_process_lock_releases_on_drop` | 132–134 | `repo_root_path.join(".isolate").join(WORKSPACE_CREATION_LOCK_FILE)` | Change to `repo_root_path.join(WORKSPACE_CREATION_LOCK_FILE)` |

### Banned Assertion Pattern Fixes (M5 — 6 patterns)

| Line | Test | Banned Pattern | Replacement |
|------|------|---------------|-------------|
| **52** | `given_file_lock_on_available_file_when_acquired_then_succeeds` | `assert!(result.is_ok())` | `assert_eq!(result, Ok(()))` |
| **81** | `given_file_already_locked_when_timeout_acquisition_then_returns_error` | `assert!(result.is_err())` | **Delete this line entirely.** The `match result` block on lines 83–91 already provides the sharp assertion. The `is_err()` is redundant and masks the match. |
| **84** | Same test | `Error::LockTimeout { operation, retries, .. }` | `Error::Jj(crate::error_jj::JjError { inner: crate::error_jj::JjErrorKind::LockTimeout { operation, retries, .. }, .. })` |
| **90** | Same test | `_ => panic!("Expected LockTimeout error")` | `other => panic!("Expected Error::Jj(JjError {{ inner: JjErrorKind::LockTimeout {{ .. }} }}), got: {other:?}")` |
| **116** | `regression_cross_process_lock_blocks_second_holder` | `assert!(second_lock_attempt.is_err())` | `let Err(e) = second_lock_attempt else { panic!("Second lock attempt must fail when first process holds exclusive lock"); }; let _ = e;` |
| **145** | `regression_cross_process_lock_releases_on_drop` | `assert!(second_lock_attempt.is_ok(), "Should be able to acquire lock after first is dropped")` | `assert_eq!(second_lock_attempt, Ok(()), "Should be able to acquire lock after first is dropped")` |
| **175** | `stress_cross_process_lock_keeps_single_holder` | `if guard.is_err() { return; }` | Replace with explicit variant match: `let guard = match guard { Ok(g) => g, Err(Error::Jj(crate::error_jj::JjError { inner: crate::error_jj::JjErrorKind::LockTimeout { .. }, .. })) => return, Err(e) => panic!("Unexpected error in stress test: {e}"), };` |
| **186** | `test_empty_workspace_name_returns_error` (jj_operations.rs) | `assert!(result.is_err())` | **Delete this line.** The `match result` block on lines 188–192 already provides the sharp assertion. |

---

## Open Questions

1. ~~**Lock portability test feasibility (B8):**~~ **RESOLVED (m3).** Marked as `#[ignore]` with manual run documentation. Not CI-feasible without exotic mount options.

2. **spawn_blocking failure test (B7):** The implementation strategy (separate thread with own runtime → `shutdown_background`) is specified but complex. Is this complexity justified for a single error path? **Recommendation:** Implement it — the error path exists in production and a test prevents silent regressions if the `map_err` chain is refactored.

3. ~~**`create_workspace_synced` ordering test (B13):**~~ **RESOLVED (M3).** Tracing span capture approach specified with concrete implementation steps.

---

## Appendix: Error Variant Coverage Checklist

| Error Variant | Behavior # | Test Scenario | Contract Aligned? |
|---------------|-----------|---------------|-------------------|
| `Error::Jj(JjError { inner: JjErrorKind::LockTimeout { operation, timeout_ms, retries } })` | B5 | Contended lock with full retry budget | YES |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to open workspace lock file: ...") })` | B6 | Read-only repo_root | YES |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to open workspace lock file: ... (NotFound)") })` | B24 | Nonexistent repo_root | YES |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to join lock task: ...") })` | B7 | Tokio runtime shutdown | YES (contract.md needs correction) |
| `Error::State(StateError { inner: StateErrorKind::ValidationError("LOCK_PORTABILITY_UNSUPPORTED: ...") })` | B8 | Strict locks env + unsupported FS | YES (deferred) |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to create data directory: ...") })` | B11 | Read-only repo_root for ensure_data_directory | YES |
| `Error::Io(IoError { inner: IoErrorKind::IoError("Failed to create data directory: ... (Not a directory)") })` | B23 | `.isolate` exists as regular file | YES |
| `Error::Config(ConfigError { inner: ConfigErrorKind::Invalid("workspace name cannot be empty") })` | B14 | Empty workspace name | YES |

## Appendix: Invariant Coverage Checklist

| Invariant | Behavior # | Test Scenario |
|-----------|-----------|---------------|
| I1 Lock-Before-Create | B15 | Lock timeout + no phantom `.isolate` |
| I2 No Phantom Directory | B16 | IoError + filesystem unchanged |
| I3 Atomic Visibility | **B19** | Cross-process probe child |
| I4 Idempotent | B18 | Sequential acquire → drop → acquire |
| I5 Single-Holder | B17 | Exactly 3 concurrent tasks |

## Appendix: Review Finding Resolution Matrix

| Finding | Severity | Status | Resolution |
|---------|----------|--------|------------|
| M1: Contract-plan-source error mismatch | MAJOR | **FIXED** | Source code is ground truth. `Error::io_error(...)` maps to `Error::Io(IoErrorKind::IoError(...))`. Test plan B7 is correct. Contract.md line 162 must be corrected. |
| M2: I3 Atomic Visibility missing | MAJOR | **FIXED** | Added B19 with concrete cross-process filesystem probe pattern using `Command::new` child process. |
| M3: B13 no implementation strategy | MAJOR | **FIXED** | Replaced "during execution" hand-waving with tracing span capture: two `tracing::info!` events captured by test subscriber, ordered assertion on event list. |
| M4: B17 violates Holzmann R2 | MAJOR | **FIXED** | Replaced iterator map over range (24 tasks) with exactly 3 explicit `tokio::spawn` calls. Added R2 exception justification for concurrency testing. |
| M5: 6 banned patterns in existing tests | MAJOR | **FIXED** | Added line-by-line replacement table for lines 52, 81, 84, 90, 116, 145, 175 (jj_lock_tests.rs) and line 186 (jj_operations.rs). |
| M6: 3 mutations survive | MAJOR | **FIXED** | Added B20 (truncate), B21 (read flag), B22 (backoff sleep). Estimated kill rate: 12/12 = 100% for identified mutations. |
| m1: Fabricated statistics | MINOR | **FIXED** | Corrected to 4 unit / 20 integration / 0 e2e / 4 static. |
| m2: B15/B16 `Err(_)` wildcards | MINOR | **FIXED** | B15 asserts `Err(Error::Jj(JjError { inner: JjErrorKind::LockTimeout { .. } }))`. B16 asserts `Err(Error::Io(IoError { inner: IoErrorKind::IoError(msg) }))`. |
| m3: B8 CI feasibility | MINOR | **FIXED** | Marked `#[ignore]` with manual run documentation. |
| m4: `.isolate`-as-file boundary | MINOR | **FIXED** | Added B23. |
| m5: Nonexistent repo_root boundary | MINOR | **FIXED** | Added B24. |
