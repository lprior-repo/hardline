# Contract Specification

## Context
- **Bead ID**: hl-4fz
- **Title**: svt_test
- **Description**: A test bead to run SVT on hardline repo
- **Domain**: Super Velocity Throughput (SVT) testing pipeline
- **Assumptions**:
  - The svt-runner.sh script exists at `/home/lewis/.config/opencode/skill/svt/svt-runner.sh`
  - Required dependencies are available: jq, curl, ss, opencode, bd
  - At least one ready bead exists in the system for SVT to process
  - opencode serve can be started on available ports
- **Open Questions**:
  - What batch size should be used for the test? (Default is 30, but test may use 1)
  - Should the test use actual beads or mock the bead discovery?
  - Should the test clean up server processes after completion?

## Preconditions

| ID | Precondition | Enforcement Level | Type/Pattern |
|----|--------------|-------------------|--------------|
| P1 | svt-runner.sh script exists and is executable | Compile-time | Check file existence before execution |
| P2 | Required dependencies (jq, curl, ss, opencode, bd) are installed | Runtime check | `command -v` for each dependency |
| P3 | Target directory exists and is readable | Compile-time | Path validation |
| P4 | Opencode server password environment variable is set | Runtime check | `OPENCODE_SERVER_PASSWORD` env var |
| P5 | Base port (4500) is available or incrementable | Runtime check | Port availability check via `ss` |

## Postconditions

| ID | Postcondition | Enforcement Level |
|----|---------------|-------------------|
| Q1 | At least one opencode server process is started | Check SERVER_PIDS not empty |
| Q2 | Session is created for each bead | Check BEAD_SESSIONS populated |
| Q3 | Report is generated with execution matrix | Check output contains JSON report |
| Q4 | Server processes are cleaned up (trap handler) | Process cleanup via trap |
| Q5 | Exit code is 0 on successful completion | Check script exit code |

## Invariants

| ID | Invariant |
|----|-----------|
| I1 | BASE_PORT starts at 4500 and increments for each server |
| I2 | Each bead gets a unique port assignment |
| I3 | SVT_PROVIDER defaults to "minimax-coding-plan" if not set |
| I4 | SVT_MODEL defaults to "MiniMax-M2.5-highspeed" if not set |
| I5 | Server cleanup happens regardless of success/failure (trap EXIT) |

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

## Contract Signatures

```rust
// Main SVT test execution
fn run_svt_test(batch_size: u32, target_dir: &Path) -> Result<SvtReport, Error>;

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

// Report generation
fn generate_report(beads: &[BeadResult]) -> Result<SvtReport, Error::ReportGenerationFailed>;
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|--------------|-------------------|----------------|
| batch_size > 0 | Compile-time | `NonZeroU32` |
| target_dir exists | Compile-time | `PathBuf` with existence check |
| dependencies available | Runtime | Result<Vec<String>, Error> with validation |
| port available | Runtime | `ss -tuln` check before bind |
| session valid | Runtime | Non-empty string from API response |

## Violation Examples (REQUIRED)

- VIOLATES P1: Running svt-runner.sh when script is missing -- should produce `Err(Error::DependencyMissing("svt-runner.sh not found"))`
- VIOLATES P2: Running with missing jq dependency -- should produce `Err(Error::DependencyMissing("jq not installed"))`
- VIOLATES P3: Running with non-existent target directory -- should produce `Err(Error::InvalidPath("directory does not exist"))`
- VIOLATES P4: Running without OPENCODE_SERVER_PASSWORD -- should fail at server authentication
- VIOLATES P5: Running when ports 4500-4529 are all occupied -- should produce `Err(Error::PortConflict("no available ports"))`
- VIOLATES Q1: Server startup fails for all beads -- should produce `Err(Error::ServerStartFailed)`
- VIOLATES Q2: Session creation API returns null -- should produce `Err(Error::SessionCreationFailed("null session id"))`
- VIOLATES Q3: Report generation with no session data -- should produce `Err(Error::ReportGenerationFailed("missing required fields"))`

## Ownership Contracts (Rust-specific)

- **Ownership transfer**: N/A for shell script execution
- **Shared borrow**: `&target_dir` - read-only path reference, no mutation
- **Exclusive borrow**: N/A
- **Clone policy**: All path/string types should use clone() where necessary for FFI with shell commands

## Non-goals

- [ ] Testing individual bead execution logic (go-skill handles this)
- [ ] Testing opencode serve internal functionality
- [ ] Testing bd CLI beyond basic ready bead discovery
- [ ] Performance benchmarking of SVT pipeline
- [ ] Testing SVT with multiple concurrent batches

(End of file - total 120 lines)
