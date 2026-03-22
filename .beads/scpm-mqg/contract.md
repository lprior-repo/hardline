# Contract Specification: TaskId Value Object

## Context
- **Feature:** session: implement TaskId type
- **Bead ID:** scpm-mqg
- **Domain terms:** TaskId, Bead ID, Dolt backing store
- **Assumptions:** TaskId follows the "bd-" prefix convention used by bead CLI tools
- **Open questions:** None

## Preconditions
- P1: Input string must be non-empty
- P2: Input string must start with "bd-" prefix
- P3: The suffix after "bd-" must contain only hexadecimal characters [0-9a-fA-F]
- P4: The suffix after "bd-" must not be empty (minimum length after prefix: 1 character)

## Postconditions
- Q1: Returned TaskId object is guaranteed to be valid "bd-" prefixed hex string
- Q2: TaskId::to_string() always returns a string starting with "bd-"
- Q3: TaskId::as_str() returns the validated string slice

## Invariants
- I1: TaskId.to_string() always starts with "bd-"
- I2: The suffix after "bd-" consists entirely of characters in [0-9a-fA-F]
- I3: A valid TaskId is always constructible from its own string representation

## Error Taxonomy
- Error::InvalidPrefix - when input does not start with "bd-"
- Error::InvalidHex - when suffix contains non-hexadecimal characters
- Error::EmptySuffix - when suffix after "bd-" is empty
- Error::InvalidInput(String) - for other invalid input cases

## Contract Signatures
```rust
impl TaskId {
    pub fn parse(input: &str) -> Result<TaskId, Error>;
    pub fn to_string(&self) -> String;
    pub fn as_str(&self) -> &str;
}

impl TryFrom<&str> for TaskId {
    type Error = Error;
}
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| P1: non-empty | Runtime-checked constructor | Result<T, Error::InvalidInput> |
| P2: "bd-" prefix | Runtime-checked constructor | Result<T, Error::InvalidPrefix> |
| P3: valid hex | Runtime-checked constructor | Result<T, Error::InvalidHex> |
| P4: non-empty suffix | Runtime-checked constructor | Result<T, Error::EmptySuffix> |

## Violation Examples (REQUIRED)
- VIOLATES P1: `TaskId::parse("")` -- should produce `Err(Error::InvalidInput("empty string"))`
- VIOLATES P2: `TaskId::parse("abc-123")` -- should produce `Err(Error::InvalidPrefix)`
- VIOLATES P3: `TaskId::parse("bd-xyz")` -- should produce `Err(Error::InvalidHex("xyz contains non-hex chars"))`
- VIOLATES P4: `TaskId::parse("bd-")` -- should produce `Err(Error::EmptySuffix)`

## Ownership Contracts
- TaskId is a newtype wrapping String, owning the validated identifier
- Clone is provided for convenience; equality is by value
- No interior mutability; TaskId is immutable once constructed

## Non-goals
- Uniqueness validation (delegated to backing store)
- Persistence of TaskId objects
