# Contract Specification

## Context
- **Feature**: SVT Load Test 2 (svt_batch_2)
- **Domain terms**: SVT (Super Velocity Throughput), svt-runner.sh, batch execution, opencode serve
- **Bead ID**: hl-vtr
- **Bead Title**: svt_batch_2
- **Phase**: contract-synthesis

### Assumptions
1. SVT batch 2 uses a specific batch configuration (likely batch_size=2 or 2 concurrent beads)
2. The test targets the repository at `/home/lewis/src/hardline`
3. SVT runner script exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
4. Dependencies: bash, jq, opencode CLI, bd CLI
5. This is a load test running 2 beads in parallel to test throughput at smaller scale

### Open Questions
- What specific batch_size does "batch 2" use? (Inferred: likely 2 concurrent beads)
- What is the expected success criteria? (Inferred: all beads complete without errors)
- How does this differ from other batches in svt_batch_2? (Inferred: smaller scale than batch 5)

---

## Preconditions

| ID | Precondition | Enforcement Level | Type/Pattern |
|----|--------------|-------------------|--------------|
| P1 | svt-runner.sh script exists at expected path | Runtime-checked | `std::path::Path::exists()` |
| P2 | svt-runner.sh is executable | Runtime-checked | `std::fs::metadata().permissions()` |
| P3 | jq is installed and available in PATH | Runtime-checked | `which jq` |
| P4 | opencode CLI is installed and available | Runtime-checked | `which opencode` |
| P5 | bd CLI is installed and available | Runtime-checked | `which bd` |
| P6 | Target directory exists and is readable | Runtime-checked | `Path::exists()` on target_dir |
| P7 | There are at least 2 ready beads to process | Runtime-checked | `bd ready --json` returns at least 2 |
| P8 | At least 2 available ports for opencode serve instances | Runtime-checked | Port availability check (ports >= 2) |

---

## Postconditions

| ID | Postcondition | Description |
|----|--------------|-------------|
| Q1 | SVT report is generated | Output matrix/report exists with test results |
| Q2 | All beads in batch complete (success or failure) | Each bead either completes or fails with documented error |
| Q3 | opencode serve instances are cleaned up | Background processes terminated after test |
| Q4 | Execution summary is produced | Summary includes batch size (2), model, completion status |
| Q5 | Exactly 2 beads are processed | Batch size is honored, no more/less |

---

## Invariants

| ID | Invariant | Description |
|----|-----------|-------------|
| I1 | No orphan opencode serve processes | After test completion, no stray opencode serve on test ports |
| I2 | Consistent state | Beads are not corrupted; bd state remains consistent |
| I3 | Resource cleanup | Temp files and processes are cleaned regardless of test outcome |
| I4 | Batch size is respected | Exactly 2 beads are processed in parallel |

---

## Error Taxonomy

- **Error::DependencyMissing** - When a required dependency (svt-runner.sh, jq, opencode, bd) is not found
  - Fields: `dependency_name: String`
- **Error::PortUnavailable** - When fewer than 2 ports are available for opencode serve instances
  - Fields: `requested_ports: u32`, `available_ports: u32`
- **Error::InsufficientReadyBeads** - When fewer than 2 beads are ready
  - Fields: `requested: u32`, `available: u32`
- **Error::ServerStartFailed** - When opencode serve fails to start
  - Fields: `port: u16`, `reason: String`
- **Error::SessionCreationFailed** - When session creation for bead fails
  - Fields: `bead_id: String`, `reason: String`
- **Error::DispatchFailed** - When go-skill dispatch fails
  - Fields: `bead_id: String`, `reason: String`
- **Error::PollTimeout** - When polling exceeds timeout threshold
  - Fields: `bead_id: String`, `timeout_seconds: u64`
- **Error::ReportGenerationFailed** - When SVT report output fails
  - Fields: `reason: String`
- **Error::CleanupFailed** - When process cleanup fails
  - Fields: `processes: Vec<String>`, `reason: String`

---

## Contract Signatures

```rust
/// Run SVT load test batch 2
/// Returns a report with execution results for all beads in the batch
fn run_svt_batch2(target_dir: &Path) -> Result<SvtBatchReport, Error>;

/// Check all dependencies are available before running SVT
fn check_svt_dependencies() -> Result<Vec<DependencyStatus>, Error>;

/// Verify postconditions after SVT execution
fn verify_batch_completion(report: &SvtBatchReport) -> Result<CompletionStatus, Error>;
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| P1: script exists | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P2: script executable | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P3-P5: CLI tools available | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P6: target directory valid | Runtime-checked | `Result<PathBuf, Error::InvalidPath>` |
| P7: ready beads exist | Runtime-checked | `Result<Vec<Bead>, Error::InsufficientReadyBeads>` (need >= 2) |
| P8: ports available | Runtime-checked | `Result<PortPool, Error::PortUnavailable>` (need >= 2) |

---

## Violation Examples (REQUIRED)

### Precondition Violations

- **VIOLATES P1**: Running `svt-runner.sh` when script is missing from `/home/lewis/.config/opencode/skill/svt/svt-runner.sh` -- should produce `Err(Error::DependencyMissing("svt-runner.sh not found"))`
- **VIOLATES P3**: Running SVT when `jq` is not installed -- should produce `Err(Error::DependencyMissing("jq not found"))`
- **VIOLATES P4**: Running SVT when `opencode` CLI is not available -- should produce `Err(Error::DependencyMissing("opencode not found"))`
- **VIOLATES P5**: Running SVT when `bd` CLI is not available -- should produce `Err(Error::DependencyMissing("bd not found"))`
- **VIOLATES P6**: Running SVT with non-existent target directory -- should produce `Err(Error::InvalidPath("target directory does not exist"))`
- **VIOLATES P7**: Running SVT when fewer than 2 beads are ready -- should produce `Err(Error::InsufficientReadyBeads { requested: 2, available: N })`
- **VIOLATES P8**: Running SVT when fewer than 2 ports are available -- should produce `Err(Error::PortUnavailable { requested: 2, available: N })`

### Postcondition Violations

- **VIOLATES Q1**: SVT execution completes but no report is generated -- should produce `Err(Error::ReportGenerationFailed("report not found"))`
- **VIOLATES Q2**: Some beads remain in incomplete state after timeout -- should produce `Err(Error::PollTimeout(...))`
- **VIOLATES Q3**: opencode serve processes remain running after test -- should produce `Err(Error::CleanupFailed("processes still running"))`
- **VIOLATES Q5**: Batch processes more than 2 beads -- should produce `Err(Error::BatchSizeViolation("expected 2, processed N"))`

---

## Ownership Contracts (Rust-specific)

- **target_dir**: Shared borrow `&Path` - read-only, no mutation of the path itself
- **SvtBatchReport**: Returned by value - ownership transfers to caller
- **Dependencies**: Checked but not mutated - read-only validation

---

## Non-goals

- [ ] Implementing the actual SVT runner (this is a specification only)
- [ ] Modifying opencode serve behavior
- [ ] Testing individual bead execution logic (separate from SVT orchestration)
- [ ] Performance benchmarking beyond basic completion verification
- [ ] Testing larger batch sizes (batch 3, 4, 5)
