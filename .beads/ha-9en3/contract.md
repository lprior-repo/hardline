---
bead_id: ha-9en3
bead_title: Test Commit — construction, SHA validation, parent tracking
phase: p3-contract
updated_at: 2026-04-06T03:00:00Z
---

# Commit Type Test Contract

## Types Under Test

### 1. `Commit` struct (`crates/vcs/src/domain/entities/mod.rs`)

```rust
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}
```

Constructor: `Commit::new(id, message, author, timestamp, parents) -> Self`

Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`

### 2. `CommitId` newtype (`crates/vcs/src/vcs/types/commit.rs`)

```rust
pub struct CommitId(String);
```

Constructor: `CommitId::new(id: impl Into<String>) -> Result<Self, VcsError>`

Derives: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`

## Invariants

### Commit (plain struct)
- INV-C1: `Commit::new()` stores all fields exactly as provided (no validation, no transformation)
- INV-C2: `id` field accepts any `String` (no SHA format enforcement at struct level)
- INV-C3: `parents` preserves insertion order and allows duplicates
- INV-C4: `Clone` produces an independent deep copy
- INV-C5: `Serialize` + `Deserialize` round-trips all fields identically
- INV-C6: `Debug` output contains the type name "Commit" and all field values

### CommitId (validated newtype)
- INV-CI1: `CommitId::new()` rejects effectively-empty strings (empty, whitespace-only, invisible-only)
- INV-CI2: `CommitId::new()` accepts any string containing at least one visible character
- INV-CI3: `as_str()` returns the original input string exactly
- INV-CI4: Equality is string-based: same string => equal
- INV-CI5: Hash consistency: equal CommitIds produce equal hashes
- INV-CI6: Serde round-trip preserves the inner string exactly

## Preconditions

- PRE-C1: `Commit::new()` has no preconditions (accepts all inputs)
- PRE-CI1: `CommitId::new()` requires a string with at least one non-whitespace, non-invisible character

## Postconditions

- POST-C1: `Commit::new()` returns a `Commit` with all fields matching inputs exactly
- POST-CI1: `CommitId::new()` returns `Ok(CommitId)` for valid inputs, `Err(VcsError::InvalidCommitId)` for invalid
- POST-CI2: `CommitId::as_str()` returns a `&str` identical to the constructor input

## Error Taxonomy

- `VcsError::InvalidCommitId(String)` — the provided string is effectively empty

## Test Scope

Primary target: `crates/vcs/src/domain/entities/mod.rs` (Commit struct with `new()` constructor)
Secondary target: `crates/vcs/src/vcs/types/commit.rs` (CommitId newtype — already has extensive tests, add edge case gaps)
