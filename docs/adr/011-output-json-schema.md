# ADR-011: Output JSON Schema - AI-First Response Format

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

Hardline outputs JSON by default for all commands. This enables:

1. **AI consumption** - LLMs can parse responses programmatically
2. **Consistency** - Same format across all commands
3. **Type safety** - Schema validation
4. **Tool use** - Easy integration with external tools

The architecture spec says "JSON-only for all operations to enable AI agent consumption". This ADR formalizes the complete JSON schema.

---

## Decision

### Output Envelope

All responses wrap in a consistent envelope:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: u16,
    pub category: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub fix: Option<FixSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    pub command: String,
    pub description: String,
    pub risk: FixRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMetadata {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub execution_time_ms: u64,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixRisk {
    Safe,
    Moderate,
    Dangerous,
}
```

### Response Variants

```rust
// Success response
Response {
    success: true,
    data: Some(T),  // Actual data
    error: None,
    metadata: ResponseMetadata { ... }
}

// Error response
Response {
    success: false,
    data: None,
    error: Some(ErrorBody { ... }),
    metadata: ResponseMetadata { ... }
}
```

### Workspace Output Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceResponse {
    pub id: WorkspaceId,
    pub name: WorkspaceName,
    pub path: String,
    pub backend: VcsType,
    pub state: WorkspaceState,
    pub agent_id: Option<AgentId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsType {
    Git,
    Jj,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceState {
    Created,
    Active,
    Syncing,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListResponse {
    pub workspaces: Vec<WorkspaceResponse>,
    pub total: usize,
    pub filter: Option<WorkspaceFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFilter {
    pub state: Option<WorkspaceState>,
    pub agent_id: Option<AgentId>,
    pub backend: Option<VcsType>,
}
```

### Queue Output Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryResponse {
    pub id: QueueEntryId,
    pub workspace_id: WorkspaceId,
    pub priority: Priority,
    pub status: QueueStatus,
    pub enqueued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub claimed_by: Option<AgentId>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueListResponse {
    pub entries: Vec<QueueEntryResponse>,
    pub total: usize,
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority(u8);

impl Priority {
    pub const CRITICAL: Priority = Priority(0);
    pub const HIGH: Priority = Priority(1);
    pub const MEDIUM: Priority = Priority(2);
    pub const LOW: Priority = Priority(3);
    pub const BACKLOG: Priority = Priority(4);
}
```

### Agent Output Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<CapabilityResponse>,
    pub status: AgentStatus,
    pub last_heartbeat_at: DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
    pub workspace_id: Option<WorkspaceId>,
    pub current_bead: Option<BeadId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResponse {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
    pub total: usize,
    pub active: usize,
    pub idle: usize,
    pub disconnected: usize,
}
```

### VCS Output Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsStatusResponse {
    pub backend: VcsType,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub status: VcsStatus,
    pub ahead: usize,
    pub behind: usize,
    pub conflicted_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VcsStatus {
    Clean,
    Dirty,
    Conflicted,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchResponse {
    pub name: String,
    pub is_current: bool,
    pub tracking: Option<String>,
}
```

### Error Output Schema

```rust
// Error responses use the ErrorBody in the envelope
Response {
    success: false,
    data: None,
    error: Some(ErrorBody {
        code: 1001,
        category: "workspace",
        message: "Workspace not found: agent-123",
        details: None,
        fix: Some(FixSuggestion {
            command: "hardline workspace list",
            description: "List available workspaces",
            risk: FixRisk::Safe,
        }),
    }),
    metadata: ResponseMetadata { ... }
}
```

### JSON Schema Example

```json
{
  "success": true,
  "data": {
    "workspaces": [
      {
        "id": "ws-abc123",
        "name": "agent-123",
        "path": "/workspaces/agent-123",
        "backend": "jj",
        "state": "active",
        "agentId": "agent-456",
        "createdAt": "2026-03-20T10:30:00Z",
        "updatedAt": "2026-03-20T11:45:00Z"
      }
    ],
    "total": 1
  },
  "error": null,
  "metadata": {
    "version": "1.0.0",
    "timestamp": "2026-03-20T11:50:00Z",
    "command": "workspace list",
    "executionTimeMs": 12,
    "requestId": "req-xyz789"
  }
}
```

---

## Variants

### Variant A: Envelope with Data/Error Discrimination (CHOSEN)

```rust
Response<T> {
    success: bool,
    data: Option<T>,
    error: Option<ErrorBody>,
}
```

**Chosen because:**
- Clear success/error path
- Type-safe data extraction
- Extensible error details

### Variant B: No Envelope, Just Data

```rust
// Just return T or Error
```

**Rejected because:**
- No metadata
- No consistency
- Can't distinguish error types

### Variant C: Status Code in Envelope

```rust
Response<T> {
    status_code: u16,
    data: Option<T>,
}
```

**Rejected because:**
- Overhead of status codes
- Less semantic than success/error

---

## Invariants

### Envelope Invariants

```rust
/// INVARIANT: If success=true, data is Some and error is None
fn assert_success_invariant(response: &Response) {
    assert_eq!(
        response.success,
        response.data.is_some() && response.error.is_none()
    );
}

/// INVARIANT: If success=false, data is None and error is Some
fn assert_error_invariant(response: &Response) {
    assert_eq!(
        !response.success,
        response.data.is_none() && response.error.is_some()
    );
}
```

### Metadata Invariants

```rust
/// INVARIANT: Metadata version matches crate version
fn assert_version_matches(response: &Response, expected: &str) {
    assert_eq!(response.metadata.version, expected);
}

/// INVARIANT: Timestamp is recent (within 1 hour)
fn assert_timestamp_recent(response: &Response) {
    let age = Utc::now() - response.metadata.timestamp;
    assert!(age < Duration::from_secs(3600));
}

/// INVARIANT: Execution time is non-negative
fn assert_execution_time_valid(response: &Response) {
    assert!(response.metadata.execution_time_ms >= 0);
}
```

### Error Body Invariants

```rust
/// INVARIANT: Error code is in valid range (1000-9999)
fn assert_error_code_valid(error: &ErrorBody) {
    assert!(1000 <= error.code && error.code < 10000);
}

/// INVARIANT: Error category matches code range
fn assert_error_category_matches_code(error: &ErrorBody) {
    let expected_category = match error.code / 1000 {
        1 => "workspace",
        2 => "session",
        3 => "bead",
        4 => "queue",
        5 => "vcs",
        6 => "stack",
        7 => "github",
        8 => "snapshot",
        9 => "internal",
        _ => "unknown",
    };
    assert_eq!(error.category, expected_category);
}

/// INVARIANT: Dangerous fix commands include warning
fn assert_dangerous_fix_warning(fix: &FixSuggestion) {
    if fix.risk == FixRisk::Dangerous {
        assert!(fix.command.contains("--force") || fix.command.contains("delete"));
    }
}
```

### Data Structure Invariants

```rust
/// INVARIANT: Workspace path is absolute
fn assert_workspace_path_absolute(workspace: &WorkspaceResponse) {
    assert!(PathBuf::from(&workspace.path).is_absolute());
}

/// INVARIANT: Agent heartbeat is not in future
fn assert_heartbeat_not_future(agent: &AgentResponse) {
    assert!(agent.last_heartbeat_at <= Utc::now());
}

/// INVARIANT: Queue position is non-negative
fn assert_queue_position_valid(entry: &QueueEntryResponse) {
    assert!(entry.position >= 0);
}
```

---

## Consequences

### Positive

1. **AI consumable** - LLMs can parse reliably
2. **Type-safe** - Schema validation
3. **Consistent** - Same structure everywhere
4. **Debuggable** - Request IDs, timestamps

### Negative

1. **Verbose** - More JSON than needed for human output
2. **Schema maintenance** - Must update when types change

### Human-Readable Flag

For CLI users who want human output, use `-ho` flag:

```bash
hardline workspace list --ho
```

This returns formatted text instead of JSON.

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/core/src/api/output/response.rs` | Response<T> envelope |
| `crates/core/src/api/output/workspaces.rs` | Workspace schemas |
| `crates/core/src/api/output/queue.rs` | Queue schemas |
| `crates/core/src/api/output/agents.rs` | Agent schemas |
| `crates/core/src/api/output/vcs.rs` | VCS schemas |
| `crates/core/src/api/output/error.rs` | Error schema |

---

## Related ADRs

- ADR-001: CLI Architecture (-ho flag for human output)
- ADR-007: Error Taxonomy (error codes and categories)
