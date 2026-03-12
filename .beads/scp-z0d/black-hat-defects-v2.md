# Black Hat Review: scp-z0d - Rollback and Cleanup on Failure

## 🔴 PHASE 1: Contract Violations

### DEFECT 1.1: Contract Signature Mismatch
- **Location**: `crates/orchestrator/src/phases.rs:111`
- **Issue**: Contract specifies `cleanup_after_failure(&self, pipeline: &Pipeline)` but implementation uses `cleanup_after_failure(&self, pipeline_id: &PipelineId)`
- **Impact**: API diverges from contract-spec.md lines 64-65
- **Severity**: HIGH - breaks contract parity

### DEFECT 1.2: Function Exceeds 25 Lines (Hard Constraint Violation)
- **Location**: `crates/orchestrator/src/phases.rs:111-141`
- **Issue**: `cleanup_after_failure` is 31 lines (exceeds 25-line limit)
- **Severity**: HIGH - violates Farley hard constraint

---

## 🟠 PHASE 2: Farley Rigor Flaws

### DEFECT 2.1: Multiple Functions Exceed 25 Lines
| Function | Lines | Location |
|----------|-------|----------|
| run_validation_phase | 60 | phases.rs:257-316 |
| handle_setup_failure | 37 | phases.rs:532-567 |
| handle_dev_failure | 32 | phases.rs:570-600 |
| spec_review | 37 | phases.rs:318-355 |
| universe_setup | 24 | phases.rs:363-386 |

**Severity**: HIGH - 5 functions violate hard constraint

---

## 🟡 PHASE 3: Functional Rust Flaws (The Big 6)

### DEFECT 3.1: CleanupResult.success Boolean Flag
- **Location**: `crates/orchestrator/src/cleanup.rs:90-94`
- **Issue**: `success: bool` field violates "Types as Documentation" principle
- **Should be**: Use `enum CleanupStatus { Success, Failed(Vec<String>) }`
- **Severity**: MEDIUM

### DEFECT 3.2: Pipeline.last_error Uses Option<String>
- **Location**: `crates/orchestrator/src/state.rs:121`
- **Issue**: Using `Option<String>` for error tracking is primitive obsession
- **Should be**: Dedicated error struct with type-safe error details
- **Severity**: LOW (acceptable for simple errors)

---

## 🔵 PHASE 4: Simplicity & DDD Failures

### DEFECT 4.1: Unwrap in Handler Code Path
- **Location**: `crates/orchestrator/src/phases.rs:512-521`
- **Issue**: Uses `self.store.get_mut(id).ok()` pattern with implicit unwrap of Option, then calls `.update()` which can fail
- **Code**:
  ```rust
  let pipeline_opt = self.store.get_mut(id).ok().map(|p| { ... });
  if let Some(pipeline) = pipeline_opt {
      self.store.update(pipeline).map_err(...)?;
  }
  ```
- **Severity**: LOW (uses safe Option handling, not unwrap/panic)

---

## 🟣 PHASE 5: The Bitter Truth (Cleverness & Bloat)

### DEFECT 5.1: No Major Issues Detected
- Code is generally readable and follows domain patterns
- No obvious YAGNI violations
- No clever one-liners hiding logic
- Cleanup handlers are appropriately decomposed

---

## Verdict

**REJECTED** - The code has contract signature mismatch (DEFECT 1.1) and violates Farley hard constraints with 5 functions exceeding 25 lines (DEFECT 2.1). While the implementation correctly handles cleanup/rollback workflow and addresses previous Q4 violations (cleanup now persists after cleanup completes), the hard constraint violations and contract parity issues must be resolved before approval.

### Required Fixes:
1. Change `cleanup_after_failure` to accept `&Pipeline` per contract, OR update contract to accept `&PipelineId`
2. Decompose functions exceeding 25 lines into smaller units
3. Replace `CleanupResult.success: bool` with enum-based status
