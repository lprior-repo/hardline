# Contract Specification: Agent Registration

## Context

- **Feature**: Agent Registration (features/agent.feature lines 30-68)
- **Domain terms**: Agent ID, auto-generated ID, reserved keywords, update semantics
- **Assumptions**:
  - Agent registry stores agents with ID, timestamps, and metadata
  - Environment variable ISOLATE_AGENT_ID must be set on successful registration
  - Auto-generated ID pattern: `agent-XXXXXXXX-XXXX` (8 hex chars + 4 hex chars)
- **Open questions**:
  - What is the full list of reserved keywords? (Feature mentions "null" as example)

## Preconditions

- [P1] Agent ID must be provided (either explicitly or auto-generated)
- [P2] Agent ID must not be empty (after trimming whitespace)
- [P3] Agent ID must not be whitespace-only
- [P4] Agent ID must not be a reserved keyword (e.g., "null", "undefined", "none")

## Postconditions

- [Q1] On success, an agent with the specified/generated ID must exist in the registry
- [Q2] On success, the agent must have a valid registration timestamp
- [Q3] On success, environment variable ISOLATE_AGENT_ID must be set to the agent ID
- [Q4] On success, agent details must be returned as JSON
- [Q5] On duplicate ID (update semantics), the agent's last_seen timestamp must be updated (not recreated)

## Invariants

- [I1] Agent IDs are unique in the registry
- [I2] Every registered agent has a valid created_at timestamp

## Error Taxonomy

- **Error::ValidationError** - Invalid input (empty, whitespace, reserved keyword)
  - Error code: "VALIDATION_ERROR"
  - Message: "Agent ID cannot be empty" or "Agent ID cannot be empty or whitespace-only" or "Agent ID cannot be a reserved keyword"
  
- **Error::DuplicateId** - Agent with this ID already exists (handled as update)
  - Note: Feature specifies "succeeds with update semantics", not an error

- **Error::StorageError** - Underlying storage failure

## Contract Signatures

```rust
/// Register a new agent, optionally with a specified ID
///
/// # Errors
///
/// Returns `Error` variants as defined in error taxonomy
pub fn register_agent(id: Option<String>) -> Result<Agent, Error>;

/// Generate a unique agent ID with pattern agent-XXXXXXXX-XXXX
pub fn generate_agent_id() -> String;

/// Check if an ID is a reserved keyword
pub fn is_reserved_keyword(id: &str) -> bool;

/// Validate agent ID meets all requirements
pub fn validate_agent_id(id: &str) -> Result<(), ValidationError>;

pub enum Error {
    ValidationError { message: String },
    StorageError { message: String },
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| id provided | Compile-time | `Option<String>` (None = auto-generate) |
| id non-empty | Runtime-checked constructor | `validate_agent_id()` returning Result |
| id not whitespace | Runtime-checked | `validate_agent_id()` |
| id not reserved | Runtime-checked | `is_reserved_keyword()` in validation |
| auto-gen pattern | Compile-time | `format!("agent-{:08x}-{:04x}", random_u32(), random_u16())` |

## Violation Examples (REQUIRED)

- **VIOLATES P2**: `register_agent(Some("".to_string()))` → `Err(Error::ValidationError { message: "Agent ID cannot be empty" })`
- **VIOLATES P3**: `register_agent(Some("   ".to_string()))` → `Err(Error::ValidationError { message: "Agent ID cannot be empty or whitespace-only" })`
- **VIOLATES P4**: `register_agent(Some("null".to_string()))` → `Err(Error::ValidationError { message: "Agent ID cannot be a reserved keyword" })`
- **VIOLATES Q1**: After successful call, `get_agent(id)` returns `Some(agent)`
- **VIOLATES Q3**: After successful call, `std::env::var("ISOLATE_AGENT_ID")` returns `Ok(id)`

## Ownership Contracts

- `register_agent` takes ownership of the Option<String> for ID
- Agent struct is returned by value (ownership transferred to caller)
- Environment variable set is a side effect (documented in postconditions)

## Non-goals

- [Agent deletion/cleanup]
- [Agent authentication/authorization]
- [Agent heartbeat/timeout handling]

---

# Contract Specification: Session Focus

## Context
- **Feature**: Session Focus (features/session.feature lines 103-121)
- **Domain terms**:
  - Session: A isolated workspace with a name and status
  - Focus: Get details about a session
  - Active session: A session with status "active" that can be focused
- **Assumptions**:
  - Session exists in the database
  - Session has a valid status

## Preconditions
- [P1] Session name must be provided and non-empty
- [P2] Session with the given name must exist in the database
- [P3] Session must have status "active" (not completed, failed, paused, or creating)

## Postconditions
- [Q1] If caller is inside Zellij: Zellij tab "isolate:{name}" becomes active
- [Q2] If caller is outside Zellij: Zellij attaches to the session
- [Q3] Session details are returned as JSON to stdout
- [Q4] On success, an action with verb "focus" and status "completed" is emitted

## Invariants
- [I1] Session in database remains unchanged by focus operation (read-only query)
- [I2] Output always includes exactly one Session line and one Result line on success

## Error Taxonomy
- `Error::SessionNotFound` - when session name does not exist in database
- `Error::InvalidSessionStatus` - when session exists but status is not "active"
- `Error::SessionNameRequired` - when session name is empty or not provided
- `Error::ZellijCommandFailed` - when Zellij switch/attach command fails
- `Error::InsideZellijDetectionFailed` - when unable to detect whether caller is inside Zellij

## Contract Signatures
```rust
pub async fn focus_session(name: &str) -> Result<SessionOutput, Error>
pub async fn run_with_options(name: Option<&str>, options: &FocusOptions) -> Result<()>
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| name is non-empty | Runtime-checked constructor | `name.filter(\|n\| !n.trim().is_empty())` returns Some |
| session exists | Runtime query + Result | `db.get(name).await?` then `.ok_or(Error::SessionNotFound)` |
| session status is Active | Runtime check + Result | `if session.status == SessionStatus::Active` then Ok, else Err |
| inside Zellij detection | Runtime check + fallback | `std::env::var("ZELLIJ_SESSION_NAME").is_ok()` |

## Violation Examples (REQUIRED)
- VIOLATES P1: `focus_session("")` -- should produce `Err(Error::SessionNameRequired)`
- VIOLATES P1: `focus_session("   ")` -- should produce `Err(Error::SessionNameRequired)`
- VIOLATES P2: `focus_session("nonexistent")` -- should produce `Err(Error::SessionNotFound)`
- VIOLATES P3: `focus_session("completed-session")` where status=Completed -- should produce `Err(Error::InvalidSessionStatus)`
- VIOLATES Q1: Focus from inside Zellij when tab switch fails -- should produce `Err(Error::ZellijCommandFailed)`
- VIOLATES Q2: Focus from outside Zellij when attach fails -- should produce `Err(Error::ZellijCommandFailed)`

## Ownership Contracts
- `db.get(name)` - shared borrow from database, no mutation
- No `&mut` parameters in focus operation - read-only query
- Session data is cloned into output types for JSON emission

## Non-goals
- Creating or destroying sessions
- Pausing or resuming sessions
- Modifying session state
- Handling session synchronization

## Context

- **Feature**: Session List (features/session.feature lines 165-182)
- **Domain terms**:
  - Session: A workspace session with name, status, and workspace path
  - Status: Enum variant (active, paused, completed)
  - JSON lines: Each session as separate JSON object on its own line
  - JSON array: Valid JSON array format
- **Assumptions**:
  - Session has at minimum: name (String), status (SessionStatus), workspace_path (PathBuf)
  - Status filtering is exact match (not fuzzy/partial)
  - Empty list returns empty array `[]`, not empty JSON lines
- **Open questions**:
  - What is the max number of sessions supported?
  - Is there pagination?
  - What happens if status filter is invalid (unknown status)?

## Preconditions

- [P1] No preconditions for listing all sessions
- [P2] Status filter, if provided, must be a valid SessionStatus variant

## Postconditions

- [Q1] All sessions are returned when no filter applied
- [Q2] Only sessions matching status filter are returned when filter provided
- [Q3] Each session output includes name, status, and workspace_path
- [Q4] Output format is valid JSON lines (newline-delimited JSON) for non-empty results
- [Q5] Output format is valid JSON array `[]` for empty results

## Invariants

- [I1] Session count in output matches actual count in storage
- [I2] Output always contains valid JSON (never partial/invalid)

## Error Taxonomy

- **Error::InvalidStatusFilter** - when status filter string is not a valid SessionStatus variant
  - Exit code: non-zero
  - Output: JSON with error code "INVALID_STATUS_FILTER"
  
- **Error::SessionStorageError** - when underlying storage fails (IO error, corruption)
  - Exit code: non-zero
  - Output: JSON with error code "STORAGE_ERROR"

## Contract Signatures
```rust
/// List all sessions, optionally filtered by status
///
/// # Errors
///
/// Returns `Error` variants as defined in error taxonomy
pub fn list_sessions(status_filter: Option<SessionStatus>) -> Result<Vec<Session>, Error>;

/// List all sessions as JSON (JSON lines for non-empty, array for empty)
pub fn list_sessions_json(status_filter: Option<SessionStatus>) -> Result<String, Error>;
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| status_filter is valid | Runtime-checked constructor | `SessionStatus::from_str() -> Result<SessionStatus, Error::InvalidStatusFilter>` |
| output is valid JSON | Runtime-checked | `serde_json::to_string()` Result |

## Violation Examples (REQUIRED)

- **VIOLATES P2**: `list_sessions(Some(SessionStatus::from_str("invalid")))` -- should produce `Err(Error::InvalidStatusFilter)`
- **VIOLATES Q1**: Storage corruption causes partial session read -- should produce `Err(Error::SessionStorageError)`
- **VIOLATES Q4**: Session with invalid UTF-8 in path -- produces invalid JSON, should return `Err(Error::SessionStorageError)`

## Ownership Contracts

- `list_sessions` takes no ownership, operates on internal storage reference
- No `&mut` parameters - read-only operation
- Returned `Vec<Session>` transfers ownership to caller

## Non-goals

- [Session creation/deletion via list command]
- [Sorting/ordering of sessions]
- [Pagination of results]

---

# Contract Specification: Agent Heartbeat

## Context

- **Feature**: Agent heartbeat (features/agent.feature lines 73-100)
- **Domain terms**:
  - `Agent` - Autonomous worker with ID, last_seen timestamp, current_command, actions_count
  - `Heartbeat` - Signal from agent to indicate liveness
  - `last_seen` - RFC3339 timestamp of last heartbeat
  - `actions_count` - Counter incremented on each heartbeat
  - `current_command` - Optional command string describing current work
- **Assumptions**:
  - Agent must be registered before sending heartbeat
  - Heartbeat is triggered via CLI with optional `--command` flag
  - ISOLATE_AGENT_ID env var provides agent ID when not explicitly passed
- **Open questions**: None

## Preconditions

- [P1] Agent ID must be provided (via argument or ISOLATE_AGENT_ID env var)
- [P2] Agent with given ID must exist in the database

## Postconditions

- [Q1] `last_seen` timestamp is updated to current time (RFC3339 format)
- [Q2] `actions_count` is incremented by 1
- [Q3] If command is provided, `current_command` is updated to the provided command
- [Q4] If command is None, `current_command` remains unchanged
- [Q5] HeartbeatOutput is returned with agent_id, timestamp, and message

## Invariants

- [I1] Agent ID is unique in the database
- [I2] actions_count is never decremented
- [I3] last_seen is always >= registered_at

## Error Taxonomy

- **Error::NotFound** - when agent_id does not exist in database (AGENT_NOT_FOUND)
  - Error code: "AGENT_NOT_FOUND"
  - Message: "Agent not found: {agent_id}"

- **Error::ValidationError** - when agent_id is empty or missing (NO_AGENT_REGISTERED)
  - Error code: "NO_AGENT_REGISTERED"
  - Message: "No agent registered in environment"

## Contract Signatures

```rust
/// Heartbeat request with optional command
pub struct HeartbeatRequest {
    pub agent_id: String,
    pub command: Option<String>,
}

/// Heartbeat response
pub struct HeartbeatOutput {
    pub agent_id: String,
    pub timestamp: String,  // RFC3339
    pub message: String,
}

/// Update agent heartbeat - returns error if agent not found
fn heartbeat(&self, request: HeartbeatRequest) -> Result<HeartbeatOutput, Error>;
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| agent_id not empty | Runtime-checked constructor | `NonEmptyString::new() -> Result<Self, Error>` |
| agent exists in DB | Error variant | `Result<T, Error::NotFound>` |
| command is valid UTF-8 | Compile-time | `String` (guaranteed valid) |

## Violation Examples (REQUIRED)

- **VIOLATES P2**: `heartbeat(HeartbeatRequest { agent_id: "nonexistent", command: None })` -- should produce `Err(Error::NotFound("Agent not found: nonexistent"))`
- **VIOLATES Q2**: After heartbeat, `actions_count` must increment -- check via `get_agent().actions_count == initial + 1`
- **VIOLATES Q3**: `heartbeat(HeartbeatRequest { agent_id: "agent-1", command: Some("test") })` -- agent.current_command should equal "test"

## Ownership Contracts

- `HeartbeatRequest` - borrowed by function, no ownership transfer
- `HeartbeatOutput` - owned by caller, returned by value
- No cloning required; types are Copy where appropriate

## Non-goals

---

# Contract Specification: Doctor Command

## Context
- **Feature**: Doctor Command (features/doctor.feature)
- **Domain terms**:
  - `DoctorCheck` - Struct with name, status (Pass/Warn/Fail), message, suggestion, auto_fixable flag, details
  - `CheckStatus` - Enum: Pass, Warn, Fail
  - `FixResult` - Result of fix operation: issue name, action description, success boolean
  - `UnfixableIssue` - Issue that cannot be auto-fixed: issue name, reason, suggestion
  - `Issue` / `IssueId` / `IssueTitle` / `IssueKind` / `IssueSeverity` - Output types for JSON emission
- **Assumptions**:
  - JJ (Jujutsu) must be installed for workspace operations
  - State database stored at `.isolate/state.db`
  - Recovery log at `.isolate/recovery.log`
  - 11 diagnostic checks run in fixed order
  - Auto-fix operations are idempotent
- **Open questions**:
  - What is the maximum age for stale session detection? (Currently 5 minutes)

## Preconditions
- [P1] No preconditions for running doctor -- check-only mode
- [P2] For fix mode: no preconditions (all checks run first, fixes applied to results)
- [P3] For dry-run mode: must be combined with --fix flag

## Postconditions
- [Q1] Output is always valid JSON (JSON Lines format: one JSON object per line)
- [Q2] Output contains `$schema` field (or is valid JSONL with schema)
- [Q3] Output contains `_schema_version` field
- [Q4] Output contains `success` field
- [Q5] Exit code 0 when all checks pass OR when --fix succeeds
- [Q6] Exit code 1 when errors (CheckStatus::Fail) detected
- [Q7] No changes made to system when running without --fix flag (check-only is read-only)
- [Q8] Fix operations are idempotent (running twice produces same result)

## Invariants
- [I1] JSON output is always valid - never partial or malformed JSON
- [I2] Fix operations are idempotent - running twice produces same result
- [I3] Check-only mode (without --fix) is always read-only - no files modified, no DB records deleted

## Error Taxonomy
- **Error::FixFailed** - when auto-fix operation fails (critical issues remain)
  - Exit code: 1
  - Output: JSON with failed fix details

- **Error::HealthCheckFailed** - when one or more checks have CheckStatus::Fail
  - Exit code: 1
  - Output: JSON with check results and summary

- **Error::DryRunRequiresFix** - when --dry-run used without --fix
  - Exit code: 1
  - Output: JSON error envelope

- **Error::VerboseRequiresFix** - when --verbose used without --fix
  - Exit code: 1
  - Output: JSON error envelope

## Contract Signatures
```rust
/// Run doctor command
///
/// # Errors
///
/// Returns `Error` variants as defined in error taxonomy
pub async fn run(format: bool, fix: bool, dry_run: bool, verbose: bool) -> Result<()>;

/// Run all diagnostic checks
async fn run_all_checks() -> Vec<DoctorCheck>;

/// Run auto-fixes based on check results
async fn run_fixes(checks: &[DoctorCheck], dry_run: bool, verbose: bool) -> Result<()>;

/// Show health report (check-only mode)
fn show_health_report(checks: &[DoctorCheck]) -> Result<()>;

/// Emit a check result as an Issue output line
fn emit_check_as_issue(check: &DoctorCheck) -> Result<()>;
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| --dry-run requires --fix | Runtime validation | Error::DryRunRequiresFix if dry_run && !fix |
| --verbose requires --fix | Runtime validation | Error::VerboseRequiresFix if verbose && !fix |
| Output is valid JSON | Runtime-checked | `serde_json::to_string()` Result |
| Exit code matches state | Runtime validation | Exit 0 if no Fail, Exit 1 otherwise |

## Violation Examples (REQUIRED)
- **VIOLATES Q1**: Output with invalid JSON (e.g., truncated) -- should always emit valid JSON
- **VIOLATES Q5**: System with 1 error check but exit code 0 -- should exit with code 1
- **VIOLATES Q7**: Running doctor without --fix modifies files/DB -- should be read-only
- **VIOLATES Q8**: Running doctor --fix twice produces different results -- should be idempotent
- **VIOLATES P3**: `doctor --dry-run` without --fix -- should return Error::DryRunRequiresFix
- **VIOLATES P3**: `doctor --verbose` without --fix -- should return Error::VerboseRequiresFix

## Ownership Contracts
- `run_all_checks()` returns `Vec<DoctorCheck>` by value (owned)
- `run_fixes` takes `&[DoctorCheck]` (borrowed reference)
- No `&mut` parameters - no mutation of input data
- JSON emission clones data as needed for output

## Non-goals
- [Automatic recovery from all corruption types]
- [Session status transitions other than cleanup]
- [Network connectivity checks]

## Context

- **Feature**: Submit session - pushes bookmark to remote (features/session.feature lines 126-160)
- **Domain terms**:
  - Session: isolated workspace with name, status, bookmark, workspace path
  - Bookmark: named reference to push to remote
  - Status: session state (synced, active, etc.)
  - Dedup key: response identifier for the push operation
- **Assumptions**:
  - Session must exist in local state
  - VCS operations (git/jj) available
  - Remote repository configured
- **Open questions**:
  - What is the exact format of the dedupe key?
  - What VCS is used (git or jj)?

## Preconditions

- [P1] Session named "X" must exist in local state
- [P2] Session status must be "synced" OR auto-commit must be enabled
- [P3] Session must have a bookmark defined
- [P4] If status is "active" and auto-commit is false, fails with DIRTY_WORKSPACE

## Postconditions

- [Q1] Bookmark successfully pushed to remote
- [Q2] Response includes dedupe key (non-empty string)
- [Q3] If auto-commit enabled, changes committed before push
- [Q4] If dry-run enabled, no bookmark pushed and response indicates dry_run: true

## Invariants

- [I1] Session name remains unchanged after submit
- [I2] Workspace path remains unchanged after submit

## Error Taxonomy

- **Error::SessionNotFound** - when session name does not exist (EXIT_CODE: 3)
- **Error::DirtyWorkspace** - when session has uncommitted changes and auto-commit not enabled (EXIT_CODE: 3)
- **Error::NoBookmark** - when session has no bookmark to push (EXIT_CODE: 3)
- **Error::PushFailed** - when VCS push to remote fails
- **Error::CommitFailed** - when auto-commit fails
- **Error::NetworkError** - when remote is unreachable

## Contract Signatures

```rust
fn submit_session(name: &str, options: SubmitOptions) -> Result<SubmitResponse, Error>;

struct SubmitOptions {
    auto_commit: bool,
    dry_run: bool,
}

struct SubmitResponse {
    dedupe_key: String,
    dry_run: bool,
}
```

## Type Encoding

| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| Session exists | Runtime-checked | `Result<Session, Error::SessionNotFound>` |
| Status "synced" or auto-commit | Runtime-checked | Error::DirtyWorkspace if neither |
| Bookmark exists | Runtime-checked | `Error::NoBookmark` if missing |
| auto_commit is bool | Compile-time | `bool` primitive |
| dry_run is bool | Compile-time | `bool` primitive |

## Violation Examples (REQUIRED)

- **VIOLATES P1**: `submit_session("nonexistent", SubmitOptions { auto_commit: false, dry_run: false })` → `Err(Error::SessionNotFound)`
- **VIOLATES P3**: `submit_session("session-no-bookmark", SubmitOptions { auto_commit: false, dry_run: false })` → `Err(Error::NoBookmark)`
- **VIOLATES P4**: `submit_session("dirty-session", SubmitOptions { auto_commit: false, dry_run: false })` → `Err(Error::DirtyWorkspace)`

## Ownership Contracts

- `name: &str` - shared borrow, read-only, lifetime tied to call
- `options: SubmitOptions` - value copy, no ownership transfer
- Response owns `dedupe_key: String` - heap allocation, caller decides clone/ownership

## Non-goals

- [Session migration or renaming]
- [Remote repository creation]
- [Concurrent submit handling]
