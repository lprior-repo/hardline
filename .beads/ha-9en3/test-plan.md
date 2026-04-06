---
bead_id: ha-9en3
bead_title: Test Commit — construction, SHA validation, parent tracking
phase: p4-test-plan
updated_at: 2026-04-06T03:00:00Z
---

# Test Plan: Commit Type Exhaustive Tests

## Testing Trophy Allocation

| Tier | Target | Tests | Rationale |
|------|--------|-------|-----------|
| Unit | Construction, field access, validation | 35+ | Commit is a pure data struct — unit tests dominate |
| Property | Serde roundtrip, clone identity, SHA patterns | 8+ | Proptest catches edge cases in serialization |
| Integration | Cross-module Commit↔CommitId usage | 3+ | Verify interop between the two types |

## BDD Scenarios

### Feature: Commit Construction

#### Scenario: Root commit with no parents
- **Given** an id "abc123", message "init", author "Alice", timestamp T, empty parents
- **When** `Commit::new()` is called
- **Then** all fields match inputs exactly
- **And** `parents` is empty

#### Scenario: Normal commit with one parent
- **Given** a 40-char hex SHA, one parent SHA
- **When** `Commit::new()` is called
- **Then** `parents.len() == 1`
- **And** `parents[0]` matches the input

#### Scenario: Merge commit with two parents
- **Given** two parent SHAs
- **When** `Commit::new()` is called
- **Then** `parents.len() == 2`
- **And** order is preserved

#### Scenario: Octopus merge with 3+ parents
- **Given** three or more parent SHAs
- **When** `Commit::new()` is called
- **Then** all parents are stored in order

#### Scenario: Commit with duplicate parents
- **Given** the same parent SHA listed twice
- **When** `Commit::new()` is called
- **Then** duplicates are preserved (no dedup)

#### Scenario: Commit with empty id
- **Given** an empty string for id
- **When** `Commit::new()` is called
- **Then** `id` is empty (struct accepts it)

#### Scenario: Commit with empty message
- **Given** an empty string for message
- **When** `Commit::new()` is called
- **Then** `message` is empty

#### Scenario: Commit with empty author
- **Given** an empty string for author
- **When** `Commit::new()` is called
- **Then** `author` is empty

### Feature: SHA Format Validation (as stored, not enforced)

#### Scenario: 40-char lowercase hex SHA
- **Given** a valid 40-char lowercase hex string
- **When** stored as `id`
- **Then** `id.len() == 40` and all chars are ascii_hexdigit

#### Scenario: 40-char uppercase hex SHA
- **Given** a valid 40-char uppercase hex string
- **When** stored as `id`
- **Then** preserved exactly (case-sensitive)

#### Scenario: Short 7-char SHA
- **Given** a 7-char hex string
- **When** stored as `id`
- **Then** `id.len() == 7`

#### Scenario: Mixed case SHA
- **Given** a SHA with mixed case hex
- **When** stored as `id`
- **Then** preserved exactly

#### Scenario: Non-hex id (tag name, branch name)
- **Given** "v1.0.0" as id
- **When** stored as `id`
- **Then** preserved exactly (no format enforcement)

### Feature: Parent Tracking

#### Scenario: Empty parents vector
- **Given** an empty `Vec<String>`
- **When** stored as `parents`
- **Then** `parents.is_empty()` is true

#### Scenario: Single parent
- **Given** one parent SHA
- **When** stored
- **Then** `parents.len() == 1`

#### Scenario: Parent order preservation
- **Given** parents ["aaa", "bbb", "ccc"]
- **When** stored
- **Then** `parents[0] == "aaa"`, `parents[1] == "bbb"`, `parents[2] == "ccc"`

#### Scenario: Duplicate parents preserved
- **Given** parents ["same", "same"]
- **When** stored
- **Then** `parents[0] == parents[1]`

#### Scenario: Many parents (10+)
- **Given** 10+ parent SHAs
- **When** stored
- **Then** all preserved in order

### Feature: Serialization Round-trip

#### Scenario: Root commit JSON round-trip
- **Given** a root Commit (no parents)
- **When** serialized to JSON and deserialized
- **Then** all fields match original

#### Scenario: Merge commit JSON round-trip
- **Given** a merge Commit with 2 parents
- **When** serialized and deserialized
- **Then** all fields match, including parents order

#### Scenario: Empty fields preserved in round-trip
- **Given** a Commit with empty message, empty author, no parents
- **When** serialized and deserialized
- **Then** all empty fields remain empty

#### Scenario: Deserialize from known JSON
- **Given** a JSON string `{"id":"abc","message":"test","author":"A","timestamp":"...","parents":["p1"]}`
- **When** deserialized
- **Then** fields match expected values

#### Scenario: Unicode round-trip
- **Given** a Commit with unicode in message and author
- **When** serialized and deserialized
- **Then** unicode is preserved exactly

### Feature: Clone Independence

#### Scenario: Clone then mutate original
- **Given** a Commit
- **When** cloned, then original fields are mutated
- **Then** clone retains original values

#### Scenario: Clone preserves all fields
- **Given** a Commit with all fields populated
- **When** cloned
- **Then** every field matches

### Feature: Debug Format

#### Scenario: Debug output contains type name
- **Given** a Commit
- **When** `format!("{commit:?}")` is called
- **Then** output contains "Commit"

#### Scenario: Debug output contains field values
- **Given** a Commit with known id
- **When** debug formatted
- **Then** output contains the id string

### Feature: CommitId Validation (Result-based)

#### Scenario: Valid commit ID
- **Given** a non-empty string
- **When** `CommitId::new()` is called
- **Then** returns `Ok(CommitId)`

#### Scenario: Empty string rejected
- **Given** empty string
- **When** `CommitId::new()` is called
- **Then** returns `Err(VcsError::InvalidCommitId)`

#### Scenario: Whitespace-only rejected
- **Given** "   " or "\t" or "\n"
- **When** `CommitId::new()` is called
- **Then** returns `Err(VcsError::InvalidCommitId)`

#### Scenario: Invisible chars only rejected
- **Given** zero-width space, BOM, etc.
- **When** `CommitId::new()` is called
- **Then** returns `Err(VcsError::InvalidCommitId)`

#### Scenario: Mixed visible and invisible accepted
- **Given** "abc\u{200B}" (visible chars + invisible)
- **When** `CommitId::new()` is called
- **Then** returns `Ok` (has visible chars)

### Feature: Timestamp Handling

#### Scenario: Epoch zero
- **Given** timestamp at Unix epoch
- **When** stored in Commit
- **Then** preserved exactly

#### Scenario: Far future date
- **Given** timestamp in 2099
- **When** stored
- **Then** preserved

#### Scenario: Millisecond precision
- **Given** timestamp with milliseconds
- **When** stored
- **Then** sub-second precision preserved

### Feature: Author and Message Edge Cases

#### Scenario: Author with email
- **Given** "Alice <alice@example.com>"
- **When** stored
- **Then** preserved exactly

#### Scenario: Author with unicode
- **Given** unicode name
- **When** stored
- **Then** preserved

#### Scenario: Multiline message
- **Given** message with newlines
- **When** stored
- **Then** newlines preserved

#### Scenario: Very long message
- **Given** a 10KB message
- **When** stored
- **Then** preserved exactly

## Proptest Invariants

1. **Serde round-trip**: For any string id, message, author, and parent list, `deserialize(serialize(commit)) == commit` (field-by-field)
2. **Clone identity**: `cloned.id == original.id && cloned.message == original.message && ...`
3. **Parent count**: `commit.parents.len() == input_parents.len()`
4. **Parent order**: For all i, `commit.parents[i] == input_parents[i]`
5. **SHA length preservation**: `commit.id.len() == input_id.len()`
6. **CommitId rejection**: All-whitespace/empty strings are rejected
7. **CommitId acceptance**: Any string with visible chars is accepted

## Mutation Testing Checkpoints

Mutations to verify test suite catches:
- Change `parents` field order → parent order tests catch
- Remove `Serialize` derive → serde tests catch
- Return wrong error type from `CommitId::new()` → validation tests catch
- `as_str()` returns modified string → round-trip tests catch
- Clone returns shallow copy → clone independence tests catch

## Test Location

All tests added to the existing `#[cfg(test)] mod tests` blocks:
- `crates/vcs/src/domain/entities/mod.rs` — Commit exhaustive tests
- `crates/vcs/src/vcs/types/commit.rs` — CommitId edge case additions
