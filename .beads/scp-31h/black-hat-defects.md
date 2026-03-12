# Black Hat Defects - Bead scp-31h

## Critical Defects Found: 1

---

### DEFECT #1: DependsOn Missing BeadId Format Validation (P8 Violation)

**Severity**: HIGH  
**Phase**: PHASE 1 - Contract Parity  
**Location**: `crates/session/src/domain/value_objects/metadata.rs:98-106`

**Contract Requirement**:
- P8: "DependsOn: Must be valid BeadId format"

**Current Implementation**:
```rust
impl DependsOn {
    pub fn new(bead_id: impl Into<String>) -> Result<Self, SessionError> {
        let bead_id = bead_id.into();
        if bead_id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn cannot be empty".into(),
            ));
        }
        Ok(Self(bead_id))  // <-- Accepts ANY non-empty string!
    }
}
```

**Problem**: Only checks for empty string. Accepts invalid formats like:
- `"not-a-bead-id"` 
- `"random-text"`
- `"12345"`

**Expected**: Should validate using the same `validate_hex_id()` function that `BeadId::parse()` uses in `session.rs` (lines 43-58), which checks for:
1. Not empty
2. Starts with `"bd-"` prefix
3. Hex portion is valid hex characters

**Fix Required**:
```rust
impl DependsOn {
    pub fn new(bead_id: impl Into<String>) -> Result<Self, SessionError> {
        let bead_id = bead_id.into();
        
        // Reuse the validation logic from session.rs
        if bead_id.is_empty() {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn cannot be empty".into(),
            ));
        }
        
        // Validate BeadId format (bd- prefix + hex)
        if !bead_id.starts_with("bd-") {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn must start with 'bd-'".into(),
            ));
        }
        
        let hex_part = &bead_id[3..];
        if hex_part.is_empty() || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SessionError::InvalidIdentifier(
                "DependsOn must be valid hex after 'bd-'".into(),
            ));
        }
        
        Ok(Self(bead_id))
    }
}
```

---

## Contract Checklist

| Precondition | Status | Notes |
|--------------|--------|-------|
| P1: AgentId not empty | ✅ PASS | Lines 11-18 task.rs |
| P2: WorkspaceName validation | ✅ PASS | Lines 13-27 metadata.rs |
| P3: TaskId not empty | ✅ PASS | Lines 50-57 task.rs |
| P4: AbsolutePath validation | ✅ PASS | Lines 11-21 path.rs |
| P5: Title validation | ✅ PASS | Lines 91-105 task.rs |
| P6: Description max length | ✅ PASS | Lines 139-147 task.rs |
| P7: Labels validation | ✅ PASS | Lines 61-74 metadata.rs |
| **P8: DependsOn valid BeadId format** | ❌ **FAIL** | Only checks empty, not format |
| P9: Priority 0-4 | ✅ PASS | Lines 137-144 metadata.rs |
| P10: IssueType validation | ✅ PASS | Lines 176-186 metadata.rs |

---

## Final Verdict

**STATUS**: REJECTED

**Reason**: Contract violation P8 - DependsOn does not validate BeadId format as specified in contract.

**Action Required**: Fix DependsOn::new() to validate BeadId format (bd- prefix + hex) before accepting.
