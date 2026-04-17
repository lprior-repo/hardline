# BLACK HAT REVIEW: scp-core (crates/core/)

**Auditor**: hardline/polecats/alpha
**Date**: 2026-04-17
**Scope**: 93,377 lines across 300+ .rs files
**Verdict**: REJECT — Mandate remediation of CRITICAL and HIGH findings before proceeding

---

## EXECUTIVE SUMMARY

scp-core is a crate with solid architectural intent (DDD layers, functional core, design-by-contract macros) but suffers from **systemic duplication**. Three parallel type systems, three parallel error systems, and three parallel contract systems coexist, creating ~15,000 lines of dead or speculative code. The domain layer is clean (no I/O in domain), but boundary types leak unvalidated data through `From<String>` impls and serde deserialization.

---

## PHASE 1: CONTRACT & BEAD PARITY — 3 CRITICAL, 5 HIGH

### CRITICAL-001: From<String> bypasses validation on SessionName
- `crates/core/src/type_session_name.rs:70-72`
- `SessionName::from("123-invalid")` silently succeeds. The `parse()` and `new()` methods validate, but `From<String>` skips all checks.

### CRITICAL-002: From<String> bypasses validation on AbsolutePath
- `crates/core/src/type_session_path.rs:36-38`
- `AbsolutePath::from("relative/path")` succeeds — relative paths injected through generic conversions.

### CRITICAL-003: Serde Deserialize bypasses validation
- `crates/core/src/type_session_name.rs:12` and `type_session_path.rs:12`
- Both derive `#[derive(Deserialize)]` without `#[serde(try_from = "String")]`. The domain layer's `SessionName` at `domain/identifiers/session_name.rs:33` does this correctly — proving the root-level type is the broken one.

### HIGH-004: Duplicate SessionName types with inconsistent contracts
- Root: `type_session_name.rs` (broken From<String>, broken Deserialize)
- Domain: `domain/identifiers/session_name.rs` (correct serde try_from)
- lib.rs re-exports only the broken root-level type.

### HIGH-005: AgentId::new() bypasses validation
- `crates/core/src/agent.rs:22-26` — `new()` allows empty strings; `new_checked()` validates. Unvalidated constructor is more ergonomic.

### HIGH-006: Session.validate() checks only timestamps
- `crates/core/src/type_session.rs:55-67` — Does NOT validate status/state compatibility, branch validity, or metadata consistency.

### HIGH-007: WorkspaceStateTransition::new() accepts any from/to combo
- `crates/core/src/workspace_state.rs:144-153` — validate() must be called separately; type allows invalid transitions.

### HIGH-008: VcsBackend trait has 18 methods with zero contract tests
- `crates/core/src/trait_.rs` — No tests verify implementations satisfy the trait contract.

---

## PHASE 2: FARLEY ENGINEERING RIGOR — 21 violations

### Function Length (>25 lines): 21 violations

Worst offenders:
| Function | File | Lines |
|----------|------|-------|
| config_set | config/command_types.rs | 106 |
| ConfigKey::try_from | config/command_types.rs | 99 |
| apply_structured_sections | config/command_types.rs | 86 |
| load_with_layers | config/command_types.rs | 72 |
| vcs_context_map | error.rs | 51 |
| acquire | lock.rs | 47 |

### I/O in Domain: CLEAN — zero violations found in domain/

### Parameter Count (>5): CLEAN — zero violations found

---

## PHASE 3: NASA-LEVEL FUNCTIONAL RUST — 2 CRITICAL, 10 HIGH

### CRITICAL-009: Builder Option Soup
- `domain/aggregates/session.rs:224-229` and `workspace_builder.rs:25-29`
- Builders use Option<T> for all fields, allowing the "all-None" state. Typed builders would make incomplete states unrepresentable.

### CRITICAL-010: Production unwrap() in workflow executor
- `domain/workflow/executor.rs:195,199,301` — Three `#[allow(clippy::unwrap_used)]` annotations. Claims safety via "Vec cannot exceed u32::MAX" but this assumption breaks under adversarial input.

### HIGH-011: RepoStatus allows contradictory bool fields
- `vcs_types.rs:215-227` — `clean: true` + `has_conflicts: true` is representable. A VcsStatus enum already exists.

### HIGH-012: Domain aggregates perform filesystem I/O
- `domain/aggregates/session.rs:95-97` and `workspace.rs:64-66` — Session::new() and Workspace::create() call path.exists(). Domain should be pure.

### HIGH-013: unreachable!() in production config code
- `config/command_types.rs:839` — ConfigScope::Env => unreachable!() should be a proper error.

### HIGH-014: unwrap_or("unknown") silently swallows errors
- `domain/session_remove.rs:166` and `domain/session_focus.rs:249` — Mask data integrity bugs.

### HIGH-015: 38 #[allow(dead_code)] annotations in production
- Across 15+ files — dead code suppressed instead of removed.

### HIGH-016: Raw primitives where newtypes exist
- Commit (vcs_types.rs:10-17), Branch (vcs_types.rs:20-25), Workspace (vcs_types.rs:28-33), QueueItem (queue.rs:48-59), BeadsIssue (type_beads_issue.rs:15-24) — all use raw String where CommitId/BranchName/BeadId exist.

### HIGH-017: Duplicate AgentId types with different validation
- agent.rs:17-41 (no validation in new()) vs domain/identifiers/agent_id.rs (full validation). Callers must choose.

### HIGH-018: Dual Session types (18 fields vs 4 fields)
- type_session.rs (18 fields, application layer) vs domain/aggregates/session.rs (4 fields, domain layer with I/O in constructor). Severe DDD violation.

---

## PHASE 4: RUTHLESS SIMPLICITY & DDD — violations folded into Phase 3

Key additions:
- Boolean parameters: Verbosity::set(verbose: bool, quiet: bool) at output.rs:26 — mutually exclusive states that should be an enum
- Option state machines: WorkspaceStateTransition.agent_id: Option<String> at workspace_state.rs:139
- todo!() in production: infrastructure/restate/clients.rs:169
- _force parameter ignored in validate_removal_preconditions (YAGNI)

---

## PHASE 5: THE BITTER TRUTH — ~15,000 lines of dead/duplicate code

### CRITICAL-019: cli_contracts/cli_contracts/ — 12 empty ghost files (0 bytes each)
- Entire directory should be deleted.

### CRITICAL-020: Three parallel type systems for same domain concepts
- type_*.rs vs domain/identifiers/ vs cli_contracts/domain_types/
- AbsolutePath wraps String in domain but PathBuf in type_ layer — fundamentally different types

### HIGH-021: contracts/ module — zero external consumers, zero trait impls (~400 lines)

### HIGH-022: domain/workflow/ module — zero external usage (~3,200 lines)
- Full saga/orchestration pipeline system never used outside the module

### HIGH-023: json/serializers.rs — 8 structs all #[allow(dead_code)] (~540 lines)

### HIGH-024: infrastructure/chaos.rs — chaos testing in production src (~603 lines)

### HIGH-025: Three parallel error mapping systems (~8,000 lines combined)
- error.rs + error_*.rs vs json/error_mapping.rs + json/error_types.rs vs domain/error_conversion*.rs

### HIGH-026: Three parallel design-by-contract systems (~4,000 lines)
- domain/macros.rs vs domain/contracts/mod.rs vs cli_contracts/ (~3300 lines, zero consumers)

### HIGH-027: cli_contracts/ module is an island (~3,300 lines, zero consumers)

### YAGNI findings:
- 1 TODO (output_jsonl/tests.rs:214)
- 5 misused SAFETY comments on non-unsafe code
- 1 todo!() in production (infrastructure/restate/clients.rs:169)
- boundary_test.rs scratch file in source tree
- 2 empty module placeholders (use_cases/mod.rs, validation/validators.rs)

---

## GRADE: D+

**Strengths**:
- Domain layer is I/O-free (clean Functional Core)
- No parameter count violations
- Solid test coverage in core modules (130+ snapshot tests)
- Newtype pattern exists and is used in domain/identifiers/

**Fatal flaws**:
- 3 CRITICAL type-safety violations (validation bypass via From<String> and Deserialize)
- ~15,000 lines of dead/duplicate/speculative code
- Three parallel type systems, three parallel error systems, three parallel contract systems
- Production unwrap() and unreachable!() that will crash under adversarial input

## REMEDIATION PRIORITY

1. **P0 (Immediate)**: Fix CRITICAL-001/002/003 — Remove From<String> on SessionName/AbsolutePath, add serde(try_from). These are silent data corruption vectors.
2. **P0 (Immediate)**: Fix CRITICAL-010 — Replace production unwrap() in workflow executor with proper error handling.
3. **P1 (This Sprint)**: Delete dead code — cli_contracts/cli_contracts/ (12 empty files), contracts/ (zero consumers), boundary_test.rs. ~1,000 lines removed instantly.
4. **P1 (This Sprint)**: Consolidate duplicate types — choose canonical location per type (domain/ wins), delete or re-export from type_*.rs.
5. **P2 (Next Sprint)**: Delete zero-usage modules — domain/workflow/ (3,200 lines), cli_contracts/ (3,300 lines), infrastructure/chaos.rs (603 lines).
6. **P2 (Next Sprint)**: Fix HIGH-012 — Remove I/O from domain aggregates.
7. **P3 (Backlog)**: Split files over 300 lines, fix function length violations, replace boolean parameters with enums.
