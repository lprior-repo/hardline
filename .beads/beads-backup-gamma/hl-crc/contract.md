# Contract Specification

## Context
- **Feature**: SVT Load Test 1 (svt_batch_1)
- **Domain terms**: SVT (Super Velocity Throughput), svt-runner.sh, batch execution, opencode serve, bead dispatch
- **Bead ID**: hl-crc
- **Bead Title**: svt_batch_1
- **Bead Description**: SVT Load Test 1
- **Bead Type**: task
- **Phase**: contract-synthesis

### Assumptions
1. SVT batch 1 uses batch_size=1 (single bead execution for baseline validation)
2. The test targets the repository at `/home/lewis/src/hardline`
3. SVT runner script exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
4. Dependencies: bash, jq, curl, ss, opencode CLI, bd CLI
5. This is a load test running a single bead to validate baseline SVT behavior
6. BASE_PORT defaults to 4500 for this test
7. Server initialization wait time is 5 seconds before dispatch

### Open Questions
- What is the expected success criteria? (Inferred: bead completes without critical errors)
- Are there specific timeout thresholds to validate?
- Should this test establish a baseline for subsequent larger batches?

---

## Preconditions

| ID | Precondition | Enforcement Level | Type/Pattern |
|----|--------------|-------------------|--------------|
| P1 | svt-runner.sh script exists at expected path | Runtime-checked | `std::path::Path::exists()` |
| P2 | svt-runner.sh is executable | Runtime-checked | `std::fs::metadata().permissions()` |
| P3 | jq is installed and available in PATH | Runtime-checked | `which jq` |
| P4 | curl is installed and available | Runtime-checked | `which curl` |
| P5 | ss (socket statistics) is installed | Runtime-checked | `which ss` |
| P6 | opencode CLI is installed and available | Runtime-checked | `which opencode` |
| P7 | bd CLI is installed and available | Runtime-checked | `which bd` |
| P8 | Target directory exists and is readable | Runtime-checked | `Path::exists()` on target_dir |
| P9 | There is at least one ready bead to process | Runtime-checked | `bd ready --json` returns non-empty |
| P10 | At least one port available for opencode serve (BASE_PORT=4500) | Runtime-checked | Port availability check (ss -tuln) |

---

## Postconditions

| ID | Postcondition | Description |
|----|--------------|-------------|
| Q1 | SVT report is generated | Output matrix/report exists with test results |
| Q2 | Single bead in batch completes (success or failure) | Bead shows terminal status |
| Q3 | opencode serve instance is cleaned up | Background process terminated after test |
| Q4 | Execution summary is produced | Summary includes batch size, model, completion status |
| Q5 | Session trace is captured | Message history saved to temp file |

---

## Invariants

| ID | Invariant | Description |
|----|-----------|-------------|
| I1 | No orphan opencode serve processes | After test completion, no stray opencode serve on test port |
| I2 | Consistent bd state | Bead state is not corrupted; bd state remains consistent |
| I3 | Resource cleanup | Temp files and processes cleaned regardless of test outcome |
| I4 | Single server instance | Exactly one opencode serve runs for batch_size=1 |
| I5 | Cleanup trap active | EXIT trap ensures cleanup even on script interruption |

---

## Error Taxonomy

- **Error::DependencyMissing** - When a required dependency (svt-runner.sh, jq, curl, ss, opencode, bd) is not found
  - Fields: `dependency_name: String`
- **Error::PortUnavailable** - When requested port is not available for opencode serve
  - Fields: `requested_port: u16`, `reason: String`
- **Error::NoReadyBeads** - When bd ready returns empty list
- **Error::ServerStartFailed** - When opencode serve fails to start
  - Fields: `port: u16`, `reason: String`
- **Error::SessionCreationFailed** - When session creation for bead fails
  - Fields: `bead_id: String`, `port: u16`, `reason: String`
- **Error::DispatchFailed** - When go-skill dispatch fails
  - Fields: `bead_id: String`, `reason: String`
- **Error::PollTimeout** - When polling exceeds timeout threshold
  - Fields: `bead_id: String`, `timeout_seconds: u64`
- **Error::CompletionCheckFailed** - When session status check fails
  - Fields: `bead_id: String`, `reason: String`
- **Error::ReportGenerationFailed** - When SVT report output fails
  - Fields: `reason: String`
- **Error::CleanupFailed** - When process cleanup fails
  - Fields: `processes: Vec<String>`, `reason: String`
- **Error::InvalidPath** - When target directory is invalid
  - Fields: `path: String`, `reason: String`

---

## Contract Signatures

```rust
/// Run SVT load test batch 1 (single bead baseline test)
/// Returns a report with execution results for the single bead
fn run_svt_batch1(target_dir: &Path) -> Result<SvtBatchReport, Error>;

/// Check all dependencies are available before running SVT
fn check_svt_dependencies() -> Result<Vec<DependencyStatus>, Error>;

/// Verify postconditions after SVT execution
fn verify_batch_completion(report: &SvtBatchReport) -> Result<CompletionStatus, Error>;

/// Poll for single bead completion
fn poll_bead_completion(bead_id: &str, port: u16, session_id: &str) -> Result<BeadStatus, Error>;
```

---

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| P1: script exists | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P2: script executable | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P3-P7: CLI tools available | Runtime-checked | `Result<bool, Error::DependencyMissing>` |
| P8: target directory valid | Runtime-checked | `Result<PathBuf, Error::InvalidPath>` |
| P9: ready beads exist | Runtime-checked | `Result<Vec<Bead>, Error::NoReadyBeads>` |
| P10: port available | Runtime-checked | `Result<u16, Error::PortUnavailable>` |

---

## Violation Examples (REQUIRED)

### Precondition Violations

- **VIOLATES P1**: Running `svt-runner.sh 1` when script is missing from `/home/lewis/.config/opencode/skill/svt/svt-runner.sh` -- should produce `Err(Error::DependencyMissing("svt-runner.sh not found"))`
- **VIOLATES P3**: Running SVT when `jq` is not installed -- should produce `Err(Error::DependencyMissing("jq not found"))`
- **VIOLATES P4**: Running SVT when `curl` is not installed -- should produce `Err(Error::DependencyMissing("curl not found"))`
- **VIOLATES P5**: Running SVT when `ss` is not installed -- should produce `Err(Error::DependencyMissing("ss not found"))`
- **VIOLATES P6**: Running SVT when `opencode` CLI is not available -- should produce `Err(Error::DependencyMissing("opencode not found"))`
- **VIOLATES P7**: Running SVT when `bd` CLI is not available -- should produce `Err(Error::DependencyMissing("bd not found"))`
- **VIOLATES P8**: Running SVT with non-existent target directory -- should produce `Err(Error::InvalidPath("target directory does not exist"))`
- **VIOLATES P9**: Running SVT when no beads are ready -- should produce `Err(Error::NoReadyBeads("no ready beads found"))`
- **VIOLATES P10**: Running SVT when port 4500 is already in use -- should produce `Err(Error::PortUnavailable("port 4500 in use"))`

### Postcondition Violations

- **VIOLATES Q1**: SVT execution completes but no report is generated -- should produce `Err(Error::ReportGenerationFailed("report not found"))`
- **VIOLATES Q2**: Bead remains in non-terminal state after polling loop exits -- should produce `Err(Error::PollTimeout(...))`
- **VIOLATES Q3**: opencode serve process remains running after test -- should produce `Err(Error::CleanupFailed("process still running on port 4500"))`
- **VIOLATES Q4**: Summary is missing from report -- should produce `Err(Error::ReportGenerationFailed("summary missing"))`
- **VIOLATES Q5**: Session trace file is not created -- should produce `Err(Error::ReportGenerationFailed("trace file not found"))`

### Invariant Violations

- **VIOLATES I1**: After test, opencode serve still running on port 4500 -- should produce `Err(Error::CleanupFailed("orphan process"))`
- **VIOLATES I2**: Bead state corrupted in bd after test -- should produce `Err(Error::BeadStateCorrupted(...))`
- **VIOLATES I3**: Temp files remain after test -- should produce `Err(Error::CleanupFailed("temp files not cleaned"))`
- **VIOLATES I4**: Multiple server instances started for batch_size=1 -- should produce `Err(Error::ServerStartFailed("unexpected instance count"))`
- **VIOLATES I5**: Process not cleaned up when script interrupted -- should produce `Err(Error::CleanupFailed("trap not executed"))`

---

## Ownership Contracts (Rust-specific)

- **target_dir**: Shared borrow `&Path` - read-only, no mutation of the path itself
- **SvtBatchReport**: Returned by value - ownership transfers to caller
- **Dependencies**: Checked but not mutated - read-only validation
- **Port allocation**: Acquired and released - must be properly returned to pool

---

## Non-goals

- [ ] Implementing the actual SVT runner (this is a specification only)
- [ ] Modifying opencode serve behavior
- [ ] Testing individual bead execution logic (separate from SVT orchestration)
- [ ] Performance benchmarking beyond basic completion verification
- [ ] Multi-bead concurrent execution (covered by svt_batch_5 and larger)
- [ ] Testing authentication/authorization of opencode serve (out of scope)
