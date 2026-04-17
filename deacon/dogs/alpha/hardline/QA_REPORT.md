# QA Attack Report: Hardline Codebase

## Executive Summary

Comprehensive adversarial testing of the Hardline (SCP) codebase reveals **7 critical issues**, **4 major issues**, and numerous minor issues across CLI commands, test infrastructure, and code organization.

**Test Execution Date:** March 19, 2026  
**Tester:** QA Enforcer + Red Queen  
**Build Status:** Compiles with warnings  
**Test Status:** 1030 core tests pass, but queue tests don't compile

---

## CRITICAL ISSUES (P0)

### 1. TASK-STORE-PERSISTENCE: Task commands non-functional

**Severity:** CRITICAL  
**Category:** State Management  
**Location:** `crates/cli/src/commands/task.rs`

**Problem:**  
The task commands use a global in-memory `TASK_STORE` singleton. Each CLI invocation creates a NEW PROCESS with a FRESH store. Demo tasks are only created when `task list` is called in a process, but subsequent commands like `task show task-001` run in new processes with empty stores.

**Reproduction:**
```bash
$ scp task list
Tasks (3):
  task-001 [-] Open - -
  task-002 [-] Open - -
  task-003 [-] Open - -

$ scp task show task-001
Error: Task not found: task-001
EXIT: 60
```

**Root Cause:**  
Static variables in Rust don't persist across process boundaries:
```rust
static TASK_STORE: LazyLock<Arc<TaskStore>> = LazyLock::new(|| Arc::new(TaskStore::new()));
```

**Fix Required:**  
Tasks must be persisted to disk/database, not kept in-memory per-process.

---

### 2. QUEUE-TEST-COMPILE: Queue tests don't compile

**Severity:** CRITICAL  
**Category:** Test Infrastructure  
**Location:** `crates/queue/src/domain/tests/ports_tests.rs`

**Problem:**  
Test file uses trait methods directly on `InMemoryQueueRepository` without importing the `QueueRepository` trait.

**Error:**
```
error[E0599]: no method named 'enqueue' found for struct 'InMemoryQueueRepository'
help: trait 'QueueRepository' which provides 'enqueue' is implemented but not in scope; 
perhaps you want to import it
```

**Reproduction:**
```bash
$ cargo test -p scp-queue
error: could not compile `scp-queue` (lib test) due to 20 previous errors
```

**Fix Required:**  
Add `use crate::domain::ports::QueueRepository;` to imports.

---

### 3. DEAD-CLI-COMMANDS: batch/util/version not connected

**Severity:** CRITICAL  
**Category:** Code Quality  
**Location:** `crates/cli/src/main.rs` vs `crates/cli/src/commands/`

**Problem:**  
Commands exist in code but are NOT registered in the CLI argument parser:

| Command File | Function | CLI Status |
|--------------|----------|------------|
| `batch.rs` | `batch::run()` | NOT CONNECTED |
| `util.rs` | `util::run()` | NOT CONNECTED |
| `version.rs` | `version::run()` | NOT CONNECTED |
| `abs.rs` | `abs::run()` | NOT CONNECTED |
| `cat.rs` | `cat::run()` | NOT CONNECTED |
| `conflicts.rs` | `conflicts::list()`, `conflicts::resolve()` | NOT CONNECTED |
| `move_duplicate.rs` | `move_duplicate::move_changes()`, `duplicate()` | NOT CONNECTED |
| `rebase.rs` | `restack()`, `rebase()`, `mv()`, `duplicate()` | NOT CONNECTED |
| `wait.rs` | `wait::run()` | NOT CONNECTED |

**Reproduction:**
```bash
$ scp batch
error: unrecognized subcommand 'batch'
  tip: some similar subcommands exist: 'switch', 'fetch'

$ scp util
error: unrecognized subcommand 'util'

$ scp version
error: unrecognized subcommand 'version'
```

**Impact:**  
~1000 lines of dead code. Commands cannot be used.

---

### 4. CLIPPY-ERROR: scp-error fails to compile

**Severity:** CRITICAL  
**Category:** Build  
**Location:** `crates/scp-error/src/lib.rs`

**Problem:**  
Clippy denies `similar_names` at warning level, causing compilation to fail:

```
error: binding's name is too similar to existing binding
  --> crates/scp-error/src/lib.rs
```

**Reproduction:**
```bash
$ cargo clippy
error: failed to compile scp-error
```

---

### 5. DOC-TEST-FAILURES: 16 doctests fail in core

**Severity:** CRITICAL  
**Category:** Test Infrastructure  
**Location:** `crates/core/src/output_jsonl/mod.rs`, `crates/core/src/coordination/conflict_resolutions.rs`

**Problem:**  
Doc tests reference items that don't exist in the expected modules:

```
error[E0432]: unresolved imports `scp_core::output::emit_stdout`, 
`scp_core::output::OutputLine`, `scp_core::output::SessionOutput`
```

**Reproduction:**
```bash
$ cargo test --doc -p scp-core
failures:
    crates/core/src/output_jsonl/mod.rs - output_jsonl (line 43)
    crates/core/src/coordination/conflict_resolutions.rs (5 failures)
    crates/core/src/domain/builders.rs
    crates/core/src/domain/events.rs
    ...
16 failed, 6 passed
```

---

## MAJOR ISSUES (P1)

### 6. INIT-DIRECTORY-BUG: init initializes wrong directory

**Severity:** MAJOR  
**Category:** CLI Behavior  
**Location:** `crates/cli/src/commands/init.rs`

**Problem:**  
When running `scp init --vcs git` in `/tmp/test-scp/`, it initializes git in `/tmp/` instead.

**Reproduction:**
```bash
$ mkdir /tmp/test-scp && cd /tmp/test-scp
$ scp init --vcs git
Initializing Source Control Plane...
✓ Initialized Git in "/tmp"   # WRONG - should be /tmp/test-scp
```

**Root Cause:**  
The init command seems to use `current_dir()` from the process start, not the cwd at invocation time.

---

### 7. UNWRAP-IN-CLI: unwrap() in production code

**Severity:** MAJOR  
**Category:** Code Quality  
**Location:** `crates/cli/src/commands/workspace.rs:47`

**Problem:**  
```rust
let first = chars.next().unwrap();
```

**Reproduction:**
```bash
$ grep -n "\.unwrap()" crates/cli/src/commands/*.rs | grep -v "test"
```

**Impact:**  
This can panic on empty strings.

---

## MINOR ISSUES (P2)

### 8. TASK-CLAIM-INJECTION: No input validation on task claim user

**Severity:** MINOR  
**Category:** Input Validation  
**Reproduction:**
```bash
$ scp task claim task-001 --user "test; rm -rf /"
Error: Task not found: task-001  # But the injection attempt succeeded as a string
```

---

### 9. QUEUE-LARGE-INPUT: No limit on queue item size

**Severity:** MINOR  
**Category:** Input Validation  
**Reproduction:**
```bash
$ scp queue enqueue "$(python3 -c 'print("A"*10000)')"
✓ Added 'AAAA...AAAA' to queue  # 10000 chars accepted
```

---

### 10. EXIT-CODE-DUPLICATION: Error exit codes not unique

**Severity:** MINOR  
**Category:** CLI Design  
**Observation:**
- Exit 60 used for: Task not found, VCS error
- Exit 30 used for: Workspace not initialized
- Exit 20 used for: Queue empty

Exit codes should follow consistent pattern (10=success, 1-138=errors).

---

## INPUT VALIDATION SUMMARY

| Command | Empty Input | Injection | Large Input | Negative |
|---------|------------|-----------|-------------|----------|
| `workspace spawn ""` | ✓ Rejected | ✓ Sanitized | N/A | N/A |
| `workspace spawn "test; rm"` | ✓ Rejected | ✓ Sanitized | N/A | N/A |
| `queue enqueue "test; rm"` | ✓ Accepted | ✗ No validation | ✓ Accepted (10K+) | N/A |
| `config set key val` | ✓ Works | ✗ No validation | ✓ Accepted | N/A |
| `queue insert -1` | N/A | N/A | ✓ Accepted (99999999) | ✓ Rejected by clap |

---

## STUB VERIFICATION

| Stub Area | Current Behavior | Expected | Status |
|-----------|-----------------|----------|--------|
| `snapshot::storage` | Returns NotFound | NotFound | ✓ Correct |
| `stack::engine` | Returns NotFound | NotFound | ✓ Correct |
| `github::client` | Returns NotYetImplemented | NotYetImplemented | ✓ Correct |
| `gix::stash` | Returns NotYetImplemented | NotYetImplemented | ✓ Correct |
| `gix::remote` | Returns Network error | Network error | ✓ Correct |

---

## BUILD & TEST STATUS

### Compilation
- ✓ Release build succeeds (with warnings)
- ✗ `cargo clippy` fails due to scp-error similar_names
- ✗ Queue tests don't compile (missing trait import)
- ✗ 16 doc tests fail (wrong imports)

### Tests
- ✓ 1030 core tests pass
- ✓ 5 VCS tests pass
- ✗ Queue tests don't compile
- ✗ 16 doc tests fail

### Warnings
- 60 dead code warnings in scp-cli
- 42 unused variable/import warnings
- 1 clippy error blocking compilation

---

## RECOMMENDATIONS

### Immediate (P0 - Block Merge)
1. Fix queue test compilation (add trait import)
2. Fix scp-error clippy error (rename binding)
3. Fix doc test imports
4. Connect dead CLI commands OR remove dead code
5. Implement task persistence

### Soon (P1)
6. Fix init directory bug
7. Remove unwrap from workspace.rs
8. Add input validation to queue/config

### Eventually (P2)
9. Standardize exit codes
10. Add input size limits
11. Clean up dead code warnings

---

## TEST COVERAGE GAPS

| Area | Current | Needed |
|------|---------|--------|
| CLI commands | 6 help tests | 40+ full tests |
| State machines | Some unit tests | Exhaust transition matrix |
| VCS backends | 5 tests | 50+ tests |
| Queue operations | Don't compile | 20+ tests |
| Error handling | Minimal | Adversarial testing |

---

## EVIDENCE APPENDIX

### Command Exit Codes
```bash
scp (no args)          EXIT: 2
scp --version          EXIT: 0
scp --help             EXIT: 0
scp init               EXIT: 60 (IO error)
scp doctor             EXIT: 90 (internal error)
scp status             EXIT: 30 (not initialized)
scp config list        EXIT: 0
scp config get foo     EXIT: 40 (not found)
scp config set k v     EXIT: 0
scp queue list         EXIT: 0
scp queue enqueue x    EXIT: 0
scp queue dequeue      EXIT: 20 (empty)
scp task list         EXIT: 0
scp task show task-001 EXIT: 60 (not found - NEW PROCESS)
```

### Build Warnings Summary
```
scp-cli: 60 warnings (unused code/dead code)
scp-core: compilation succeeds
scp-queue: 0 warnings but tests don't compile
scp-vcs: 0 warnings
scp-error: 1 error (similar_names)
```

---

*Report generated by QA Enforcer + Red Queen adversarial testing*
*Findings should be converted to beads for tracking and regression*
