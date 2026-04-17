# Bead ha-9en3: Commit Type Test Infrastructure Map

## 1. Directory Tree (3 levels)

```
.
├── adversarial_test.rs
├── AGENTS.md
├── architecture-spec.md
├── BEAD-001.json
├── .beads
│   ├── ha-9en3/
│   │   └── STATE.md
│   └── [other beads...]
├── crates/
│   ├── beads/
│   │   ├── src/
│   │   │   ├── domain/
│   │   │   │   ├── entities/
│   │   │   │   │   └── mod.rs        # Commit struct definition
│   │   │   │   └── mod.rs
│   │   │   └── types.rs              # Commit struct (duplicate)
│   │   └── ...
│   ├── cli/
│   │   ├── src/
│   │   │   └── commands/
│   │   └── tests/
│   ├── core/
│   │   ├── src/
│   │   │   ├── vcs_types.rs         # Commit struct (main implementation)
│   │   │   └── ...
│   │   └── tests/
│   ├── vcs/
│   │   ├── src/
│   │   │   ├── vcs/
│   │   │   │   ├── types/
│   │   │   │   │   ├── commit.rs      # CommitId newtype with validation
│   │   │   │   │   └── mod.rs
│   │   │   │   └── ...
│   │   │   └── domain/
│   │   │       └── entities/
│   │   │           └── mod.rs        # Commit struct definition
│   │   └── tests/
│   └── [other crates...]
├── tests/
│   ├── architecture_test.rs
│   ├── compile_fail/
│   └── cucumber.rs
└── [other files...]
```

## 2. Public Types and Traits Relevant to Commit Type

### Commit Struct (Multiple Locations)

#### A. `crates/beads/src/domain/entities/mod.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub parents: Vec<String>,
}

impl Commit {
    pub fn new(
        id: String,
        message: String,
        author: String,
        timestamp: DateTime<Utc>,
        parents: Vec<String>,
    ) -> Self {
        Self {
            id,
            message,
            author,
            timestamp,
            parents,
        }
    }
}
```

#### B. `crates/core/src/vcs_types.rs` (Primary Implementation)
Same struct definition as above, but with comprehensive tests

#### C. `crates/vcs/src/domain/entities/mod.rs`
Same struct definition as A

### CommitId Newtype (Validation-focused)

#### `crates/vcs/src/vcs/types/commit.rs`
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitId(String);

impl CommitId {
    /// Create a new commit ID with validation
    pub fn new(id: impl Into<String>) -> Result<Self, VcsError> {
        let id = id.into();
        if is_effectively_empty(&id) {
            return Err(VcsError::InvalidCommitId(id));
        }
        Ok(Self(id))
    }

    /// Get the commit ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### SHA-related Types

#### From `crates/core/src/vcs_types.rs`:
- `CommitId(String)` - Basic validation (non-empty)
- `BranchName(String)` - Complex validation (git-specific rules)
- `ChangeId(String)` - Basic validation (non-empty)

### Serialization Support

All Commit-related types derive:
- `serde::Serialize`
- `serde::Deserialize`
- `Debug`
- `Clone`
- `PartialEq`/`Eq` (where applicable)

## 3. Existing Test Files and Coverage Areas

### Current Test Coverage:

#### A. `crates/vcs/src/vcs/types/commit.rs` (CommitId)
- Basic validation (empty, whitespace-only, invisible chars)
- Valid commit IDs (simple, full SHA, numeric, special chars)
- Clone, equality, and hash behavior
- Serde roundtrip tests
- **Property-based tests with proptest**

#### B. `crates/core/src/vcs_types.rs` (Comprehensive Commit Tests)
- **Exhaustive construction tests**:
  - Root commits (no parents)
  - Normal commits (1 parent)
  - Merge commits (2 parents)
  - Octopus merges (3+ parents)
  - Many parents (10+)

- **SHA pattern tests**:
  - 40-char lowercase hex
  - 40-char uppercase hex
  - Short SHAs (7-8 chars)

- **Parent tracking tests**:
  - Order preservation
  - Duplicate parents allowed

- **Timestamp handling**:
  - Epoch zero
  - Specific dates
  - Far future dates
  - Millisecond precision

- **Author field tests**:
  - With email
  - Name only
  - Empty string
  - Unicode characters

- **Message tests**:
  - Single line
  - Multiline
  - Empty
  - Unicode

- **Comparison tests**:
  - Same SHA comparison
  - Different SHA comparison
  - Field-level equality

- **Serde tests**:
  - Roundtrip for various commit types
  - JSON deserialization
  - Empty field preservation

- **Property-based tests with proptest**:
  - Serde roundtrip for any string inputs
  - Clone identity
  - SHA validation
  - Parent count validation

### Missing Test Areas:

1. **Error handling tests**:
   - Invalid timestamp formats
   - Malformed parent references
   - Serialization edge cases

2. **Integration tests**:
   - With real git repositories
   - Cross-system compatibility

3. **Performance tests**:
   - Large commit histories
   - Serialization performance

## 4. Contract and Plan Artifacts in `.beads/ha-9en3/`

Current state:
- `STATE.md` contains "STATE 2" (exploration phase)

No existing contract or plan artifacts found for this bead. The bead appears to be in the initial exploration state.

## 5. Cargo.toml Configuration

### Test Dependencies (from workspace root):

```toml
[workspace.dependencies]
# Property-based testing
proptest = "1.5"

# Traditional testing
tokio-test = "0.4"
trybuild = "1"
loom = "0.7"          # For concurrent testing
pretty_assertions = "1.4"
tempfile = "3.14"
serial_test = "3.0"   # For async tests

# Mutation testing
cargo-mutants = "0.8"

# Snapshot testing
insta = { version = "1.40", features = ["yaml", "serde"] }

# Benchmarking
criterion = "0.5"

# Verification tools
cargo-deny = "0.16"
```

### Key Test Crates:

1. **proptest**: Used extensively in existing commit tests for property-based testing
2. **tempfile**: Used for creating temporary test repositories
3. **insta**: For snapshot testing of serialized outputs
4. **loom**: For concurrent/async testing scenarios
5. **cargo-mutants**: For mutation testing coverage

## 6. Testing Recommendations for ha-9en3

### Priority Areas:

1. **Error Handling Tests**:
   - Test invalid commit ID formats
   - Test malformed timestamps
   - Test serialization/deserialization failures

2. **Edge Case Tests**:
   - Maximum length commit messages
   - Unicode edge cases
   - Large numbers of parents
   - Empty and whitespace-only fields

3. **Integration Tests**:
   - Git repository integration
   - Cross-platform compatibility
   - Large dataset performance

4. **Mutation Testing**:
   - Use cargo-mutants to verify test suite effectiveness
   - Focus on mutation hotspots in commit-related code

### Test Organization:

Consider creating a dedicated test module:
```
crates/core/tests/commit_comprehensive_tests.rs
crates/vcs/tests/commit_validation_tests.rs
crates/beads/tests/domain_commit_tests.rs
```

### Property-Based Testing Strategy:

Extend existing proptest coverage to include:
- Random commit generation with constraints
- Property testing for invariants (e.g., parent SHA validity)
- Fuzz testing for serialization robustness

### Testing Dependencies to Add:

For comprehensive commit testing, consider adding:
```toml
[dev-dependencies]
# For git integration testing
git2 = "0.20"

# For more complex test scenarios
rand = "0.8"
quickcheck = "1.0"  # Alternative to proptest
```

---

*Generated for Bead ha-9en3 - Commit Type Test Infrastructure*
*Date: 2026-04-05*
