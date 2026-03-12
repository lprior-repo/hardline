# Contract Specification

## Context
- Feature: Add missing domain types to session crate
- Domain terms:
  - AgentId: Unique identifier for an agent
  - WorkspaceName: Name of a workspace
  - TaskId: Unique identifier for a task
  - AbsolutePath: Absolute file system path
  - Title: Title of a work item
  - Description: Description of a work item
  - Labels: Collection of labels (tags)
  - DependsOn: Dependency reference to another bead
  - Priority: Priority level (0-4)
  - IssueType: Type of issue (bug, feature, task, epic, chore)
- Assumptions:
  - All types follow the same builder/constructor pattern as existing value objects
  - Validation happens at construction time via Result<T, Error>
  - All types implement Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize
- Open questions: None

## Preconditions
- [P1] AgentId: Must not be empty string
- [P2] WorkspaceName: Must not be empty after trim, max length 100
- [P3] TaskId: Must not be empty string
- [P4] AbsolutePath: Must be valid absolute path (starts with /)
- [P5] Title: Must not be empty after trim, max length 200
- [P6] Description: Must not exceed 10000 characters
- [P7] Labels: Must not contain duplicates, max 50 labels
- [P8] DependsOn: Must be valid BeadId format
- [P9] Priority: Must be in range 0-4
- [P10] IssueType: Must be one of (bug, feature, task, epic, chore)

## Postconditions
- [Q1] Each type must provide as_str() returning &str
- [Q2] Each type must provide into_inner() returning owned inner value
- [Q3] Each type must implement Display
- [Q4] Each type must implement TryFrom<String>
- [Q5] All types must serialize/deserialize correctly

## Invariants
- [I1] Value objects are always valid after construction
- [I2] No mutable state in value objects (immutable)

## Error Taxonomy
- Error::InvalidIdentifier - when string validation fails (empty, too long, invalid format)
- Error::InvalidPath - when path validation fails
- Error::InvalidPriority - when priority out of range
- Error::InvalidIssueType - when issue type not recognized

## Contract Signatures

### AgentId
```rust
pub fn new(id: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### WorkspaceName
```rust
pub fn new(name: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### TaskId
```rust
pub fn new(id: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### AbsolutePath
```rust
pub fn new(path: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### Title
```rust
pub fn new(title: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### Description
```rust
pub fn new(desc: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### Labels
```rust
pub fn new(labels: Vec<String>) -> Result<Self, SessionError>
pub fn as_slice(&self) -> &[String]
pub fn into_inner(self) -> Vec<String>
```

### DependsOn
```rust
pub fn new(bead_id: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

### Priority
```rust
pub fn new(priority: u8) -> Result<Self, SessionError>
pub fn as_u8(&self) -> u8
pub fn into_inner(self) -> u8
```

### IssueType
```rust
pub fn new(issue_type: impl Into<String>) -> Result<Self, SessionError>
pub fn as_str(&self) -> &str
pub fn into_inner(self) -> String
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| AgentId not empty | Runtime-checked constructor | `AgentId::new() -> Result` |
| WorkspaceName not empty | Runtime-checked constructor | `WorkspaceName::new() -> Result` |
| TaskId not empty | Runtime-checked constructor | `TaskId::new() -> Result` |
| AbsolutePath absolute | Runtime-checked constructor | `AbsolutePath::new() -> Result` |
| Title not empty | Runtime-checked constructor | `Title::new() -> Result` |
| Description length | Runtime-checked constructor | `Description::new() -> Result` |
| Labels unique | Runtime-checked constructor | `Labels::new() -> Result` |
| DependsOn valid format | Runtime-checked constructor | `DependsOn::new() -> Result` |
| Priority 0-4 | Runtime-checked constructor | `Priority::new() -> Result` |
| IssueType valid | Runtime-checked constructor | `IssueType::new() -> Result` |

## Violation Examples (REQUIRED)
- VIOLATES P1: `AgentId::new("")` -- should produce `Err(SessionError::InvalidIdentifier("AgentId cannot be empty"))`
- VIOLATES P2: `WorkspaceName::new("")` -- should produce `Err(SessionError::InvalidIdentifier("WorkspaceName cannot be empty"))`
- VIOLATES P2: `WorkspaceName::new("   ")` -- should produce `Err(SessionError::InvalidIdentifier("WorkspaceName cannot be empty"))`
- VIOLATES P2: `WorkspaceName::new("a".repeat(101))` -- should produce `Err(SessionError::InvalidIdentifier(...))`
- VIOLATES P3: `TaskId::new("")` -- should produce `Err(SessionError::InvalidIdentifier("TaskId cannot be empty"))`
- VIOLATES P4: `AbsolutePath::new("relative/path")` -- should produce `Err(SessionError::InvalidPath(...))`
- VIOLATES P4: `AbsolutePath::new("")` -- should produce `Err(SessionError::InvalidPath(...))`
- VIOLATES P5: `Title::new("")` -- should produce `Err(SessionError::InvalidIdentifier("Title cannot be empty"))`
- VIOLATES P6: `Description::new("a".repeat(10001))` -- should produce `Err(SessionError::InvalidIdentifier(...))`
- VIOLATES P7: `Labels::new(vec!["a".to_string(), "a".to_string()])` -- should produce `Err(SessionError::InvalidIdentifier(...))`
- VIOLATES P8: `DependsOn::new("")` -- should produce `Err(SessionError::InvalidIdentifier(...))`
- VIOLATES P9: `Priority::new(5)` -- should produce `Err(SessionError::InvalidPriority(...))`
- VIOLATES P9: `Priority::new(255)` -- should produce `Err(SessionError::InvalidPriority(...))`
- VIOLATES P10: `IssueType::new("invalid")` -- should produce `Err(SessionError::InvalidIssueType(...))`

## Ownership Contracts
- All value objects use owned String internally (newtype pattern)
- All constructor methods consume the input and return Result
- All accessors return references or owned copies (into_inner)
- No mutable state - immutable value objects

## Non-goals
- [ ] Validation logic beyond what's specified
- [ ] Complex parsing or transformation
- [ ] Integration with external systems
