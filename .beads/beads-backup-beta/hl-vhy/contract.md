# Contract Specification

## Context
- **Bead ID**: hl-vhy
- **Title**: svt_batch_3
- **Description**: SVT Load Test 3 - Batch load testing iteration for the Super Velocity Throughput testing pipeline
- **Domain**: Super Velocity Throughput (SVT) batch load testing
- **Assumptions**:
  - The svt-runner.sh script exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
  - Required dependencies are available: jq, curl, ss, opencode, bd
  - At least one ready bead exists in the system for SVT to process
  - opencode serve can be started on available ports
  - This is the third iteration of batch load testing (batch 3)
- **Open Questions**:
  - What specific load parameters differ from previous batches (hl-4fz)?
  - Is this testing a specific batch size or concurrent load scenario?
  - Should performance metrics be captured and compared to previous batches?

## Preconditions

| ID | Precondition | Enforcement Level | Type/Pattern |
|----|--------------|-------------------|--------------|
| P1 | svt-runner.sh script exists and is executable | Compile-time | Check file existence before execution |
| P2 | Required dependencies (jq, curl, ss, opencode, bd) are installed | Runtime check | `command -v` for each dependency |
| P3 | Target directory exists and is readable | Compile-time | Path validation |
| P4 | OPENCODE_SERVER_PASSWORD environment variable is set | Runtime check | `OPENCODE_SERVER_PASSWORD` env var |
| P5 | Base port (4500) is available or incrementable | Runtime check | Port availability check via `ss` |
| P6 | At least one ready bead exists for batch processing | Runtime check | `bd ready` returns non-empty list |
| P7 | Batch configuration is valid (batch_id = 3, valid batch params) | Compile-time | Enum or struct validation |

## Postconditions

| ID | Postcondition | Enforcement Level |
|----|---------------|-------------------|
| Q1 | At least one opencode server process is started | Check SERVER_PIDS not empty |
| Q2 | Session is created for each bead in the batch | Check BEAD_SESSIONS populated |
| Q3 | Report is generated with execution matrix including batch_id | Check output contains JSON report |
| Q4 | Server processes are cleaned up (trap handler) | Process cleanup via trap |
| Q5 | Exit code is 0 on successful completion | Check script exit code |
| Q6 | Batch execution metadata includes batch iteration (3) | Report includes batch_number field |

## Invariants

| ID | Invariant |
|----|-----------|
| I1 | BASE_PORT starts at 4500 and increments for each server |
| I2 | Each bead gets a unique port assignment |
| I3 | SVT_PROVIDER defaults to "minimax-coding-plan" if not set |
| I4 | SVT_MODEL defaults to "MiniMax-M2.5-highspeed" if not set |
| I5 | Server cleanup happens regardless of success/failure (trap EXIT) |
| I6 | Batch iteration number is preserved (batch 3) |
| I7 | Concurrent sessions do not interfere with each other |

## Error Taxonomy

| Error Variant | Description | Trigger Condition |
|---------------|-------------|-------------------|
| Error::DependencyMissing | Required CLI tool not found | Any required dependency (jq, curl, ss, opencode, bd) not installed |
| Error::NoReadyBeads | No beads available for processing | `bd ready` returns empty list |
| Error::PortConflict | Cannot find available port | All ports 4500-4529 in use |
| Error::ServerStartFailed | opencode serve failed to start | Server process exits prematurely |
| Error::SessionCreationFailed | Cannot create opencode session | POST to /session returns invalid response |
| Error::DispatchFailed | Cannot dispatch agent to session | POST to /prompt_async fails |
| Error::PollTimeout | Bead processing never completes | Excessive polling without completion |
| Error::ReportGenerationFailed | Cannot generate execution report | Missing session/port data for report |
| Error::BatchConfigInvalid | Invalid batch configuration | Invalid batch_id or parameters |
| Error::ConcurrentSessionConflict | Concurrent sessions conflict | Race condition in session management |

## Contract Signatures

```rust
// Main SVT batch test execution
fn run_svt_batch(batch_id: u32, batch_size: u32, target_dir: &Path) -> Result<SvtBatchReport, Error>;

// Dependency validation
fn validate_dependencies() -> Result<Vec<String>, Error::DependencyMissing>;

// Port management
fn find_available_port(base: u16, used_ports: &[u16]) -> Result<u16, Error::PortConflict>;

// Server lifecycle
fn start_opencode_server(port: u16) -> Result<u32, Error::ServerStartFailed>;
fn stop_server(pid: u32) -> Result<(), Error>;

// Session management
fn create_session(port: u16, title: &str) -> Result<String, Error::SessionCreationFailed>;
fn dispatch_agent(port: u16, session_id: &str, bead_id: &str) -> Result<(), Error::DispatchFailed>;
fn poll_completion(port: u16, session_id: &str, timeout_secs: u64) -> Result<SessionStatus, Error::PollTimeout>;

// Batch-specific operations
fn execute_batch_load(batch_id: u32, beads: &[BeadId]) -> Result<BatchResult, Error>;
fn generate_batch_report(batch_id: u32, results: &[BeadResult]) -> Result<SvtBatchReport, Error::ReportGenerationFailed>;
fn validate_batch_config(batch_id: u32, params: &BatchParams) -> Result<(), Error::BatchConfigInvalid>;
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| batch_id = 3 | Compile-time | `BatchId::Three` enum variant |
| batch_size > 0 | Compile-time | `NonZeroU32` |
| target_dir exists | Compile-time | `PathBuf` with existence check |
| dependencies available | Runtime | Result<Vec<String>, Error> with validation |
| port available | Runtime | `ss -tuln` check before bind |
| session valid | Runtime | Non-empty string from API response |
| batch params valid | Compile-time | `BatchParams` struct with validation |

## Violation Examples (REQUIRED)

- VIOLATES P1: Running svt-runner.sh when script is missing -- should produce `Err(Error::DependencyMissing("svt-runner.sh not found"))`
- VIOLATES P2: Running with missing jq dependency -- should produce `Err(Error::DependencyMissing("jq not installed"))`
- VIOLATES P3: Running with non-existent target directory -- should produce `Err(Error::InvalidPath("directory does not exist"))`
- VIOLATES P4: Running without OPENCODE_SERVER_PASSWORD -- should fail at server authentication
- VIOLATES P5: Running when ports 4500-4529 are all occupied -- should produce `Err(Error::PortConflict("no available ports"))`
- VIOLATES P6: Running with no ready beads -- should produce `Err(Error::NoReadyBeads)`
- VIOLATES P7: Running with invalid batch configuration (wrong batch_id) -- should produce `Err(Error::BatchConfigInvalid("batch_id must be 3"))`
- VIOLATES Q1: Server startup fails for all beads -- should produce `Err(Error::ServerStartFailed)`
- VIOLATES Q2: Session creation API returns null -- should produce `Err(Error::SessionCreationFailed("null session id"))`
- VIOLATES Q3: Report generation with no session data -- should produce `Err(Error::ReportGenerationFailed("missing required fields"))`
- VIOLATES Q6: Report missing batch iteration number -- should produce `Err(Error::ReportGenerationFailed("batch_id not in report"))`

## Ownership Contracts (Rust-specific)

- **Ownership transfer**: N/A for shell script execution
- **Shared borrow**: `&target_dir` - read-only path reference, no mutation
- **Exclusive borrow**: N/A
- **Clone policy**: All path/string types should use clone() where necessary for FFI with shell commands

## Non-goals

- [ ] Testing individual bead execution logic (go-skill handles this)
- [ ] Testing opencode serve internal functionality
- [ ] Testing bd CLI beyond basic ready bead discovery
- [ ] Performance benchmarking comparison with previous batches
- [ ] Testing SVT with more than 30 concurrent sessions
