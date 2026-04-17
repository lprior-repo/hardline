# BLACK HAT REVIEW: scp-error crate

**Reviewer**: polecat-eta (adversarial audit)
**Date**: 2026-04-17
**Scope**: `crates/scp-error/src/lib.rs` (1 file, ~350 LOC production + ~1100 LOC tests)
**Verdict**: **REJECT** — 2 CRITICAL, 2 HIGH, 3 MEDIUM, 2 LOW

---

## PHASE 1: Contract & Bead Parity

### CRITICAL-1: God Enum Anti-Pattern — 50+ Variants in Single Flat Enum

**File**: `src/lib.rs:18-256`

The entire crate is one flat enum with 50+ variants spanning 9+ bounded contexts (Workspace, Session, Bead, Queue, VCS, Config, Agent, Validation, IO, Orchestration, Scenario, Internal). This is the opposite of DDD. Every downstream crate that depends on `scp-error` must handle (or explicitly ignore) variants from domains it has no business knowing about.

```rust
pub enum Error {
    // Workspace domain
    WorkspaceNotFound(String),
    // Session domain
    SessionNotFound(String),
    // Bead domain
    BeadNotFound(String),
    // Queue domain
    QueueEmpty,
    // VCS domain
    VcsNotInitialized,
    // Config domain
    ConfigNotFound(String),
    // Agent domain
    AgentNotFound(String),
    // Validation domain
    ValidationError(String),
    // IO domain
    IoError(String),
    // ... 40+ more
}
```

Each domain should have its own error enum in its own crate, composed at application boundaries — not dumped into a shared "kitchen sink" error type.

### CRITICAL-2: Exit Code Mapping Is Exhaustive — Adding a Variant Requires Updating 3 Places

**File**: `src/lib.rs:281-348`

The `exit_code()` method is a massive match with 50+ arms. Adding any new error variant requires:
1. Adding the variant to the enum
2. Adding the display message
3. Adding the exit code arm

If you forget step 3, the compiler won't catch it because the match is on `&self` and there's no exhaustiveness guarantee (the `#[non_exhaustive]` attribute actually makes this worse — the compiler won't warn about missing match arms).

The exit code scheme itself is fragile — `101` is skipped (goes from `Database: 104` to `Serialization: 105`), and bead errors start at `133` far from the `19-20` range of the original bead variants.

---

## PHASE 2: Farley Engineering Rigor

### HIGH-1: `exit_code()` Function Is 68 Lines — Nearly 3x the 25-Line Limit

**File**: `src/lib.rs:281-348`

This is a pure dispatch function that maps every variant to a number. It should be a constant lookup or derived from variant metadata, not a hand-maintained match statement that will drift.

### HIGH-2: `suggestion()` Couples Error Types to CLI Command Strings

**File**: `src/lib.rs:259-278`

```rust
pub fn suggestion(&self) -> Option<String> {
    match self {
        Self::WorkspaceNotFound(_) => {
            Some("Try 'scp workspace list' to see available workspaces".into())
        }
        // ...
    }
}
```

This embeds CLI command knowledge (`scp workspace list`, `scp agent kill`, `scp init`) inside an error type crate. The error crate should not know about CLI commands. This is an application-level concern, not a domain-level concern.

---

## PHASE 3: NASA-Level Functional Rust (The Big 6)

### MEDIUM-1: All Error Data Is `String` — No Domain Types

Every variant wraps raw `String` for identifiers:

```rust
WorkspaceNotFound(String),    // ← should be WorkspaceId
SessionNotFound(String),     // ← should be SessionId
BeadNotFound(String),        // ← should be BeadId
AgentNotFound(String),       // ← should be AgentId
BranchNotFound(String),      // ← should be BranchName
```

No parsing at the boundary. No type safety. Any string is accepted. `Error::WorkspaceNotFound("")` is valid.

### MEDIUM-2: `QueueInvalidPosition(usize)` — Raw Primitive in Domain Model

**File**: `src/lib.rs:94`

`usize` for position means negative positions are impossible at the type level, but `usize::MAX` is valid — which is almost certainly not a real queue position. This should be a `NonZeroUsize` or a `QueuePosition` newtype.

### LOW-1: `ValidationFieldError` Has `value: Option<String>` — Inconsistent Typing

**File**: `src/lib.rs:178-182`

Sometimes values are `String`, sometimes `Option<String>`, sometimes `usize`. No consistent approach to error data.

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

### MEDIUM-3: `#[non_exhaustive]` on Enum Makes External Matching Fragile

**File**: `src/lib.rs:19`

`#[non_exhaustive]` prevents downstream crates from exhaustively matching. This means adding a new variant silently breaks downstream pattern matches that don't have a `_ =>` wildcard. The test at line 1259-1266 even brags about this:

```rust
fn non_exhaustive_allows_wildcard() {
    match err {
        Error::Internal(_) => {}  // known arm
        _ => {}                   // wildcard required
    }
}
```

This test proves the anti-pattern: downstream code is forced to use wildcards, meaning new error variants silently become unhandled.

---

## PHASE 5: The Bitter Truth (Velocity & Legibility)

### LOW-2: `Result<T>` Type Alias Shadows `std::result::Result`

**File**: `src/lib.rs:16`

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

This is a common Rust pattern, but in a workspace with multiple error types, it causes confusion. Crates that use their own error types (like `orchestrator::PhaseError`) can't use this alias.

### LOW-3: Comments Claim Exit Code Ranges But Ranges Are Inconsistent

**File**: `src/lib.rs:22-255`

Comments say "Bead Errors (1xxx - extended)" but bead exit codes are `133-138`. Queue errors are commented as "2xxx" but use `30-35`. The numbering scheme is a lie.

---

## Summary Table

| ID | Severity | Phase | Finding | File:Line |
|----|----------|-------|---------|-----------|
| C-1 | CRITICAL | 1 | God enum: 50+ variants across 9 bounded contexts | lib.rs:18-256 |
| C-2 | CRITICAL | 1 | Exit code mapping is hand-maintained, fragile, incomplete range | lib.rs:281-348 |
| H-1 | HIGH | 2 | `exit_code()` is 68 lines (limit: 25) | lib.rs:281 |
| H-2 | HIGH | 2 | `suggestion()` couples error types to CLI commands | lib.rs:259-278 |
| M-1 | MEDIUM | 3 | All identifiers are raw String, no domain types | lib.rs:18-256 |
| M-2 | MEDIUM | 3 | `QueueInvalidPosition(usize)` — raw primitive | lib.rs:94 |
| M-3 | MEDIUM | 4 | `#[non_exhaustive]` forces wildcards, silently breaks downstream | lib.rs:19 |
| L-1 | LOW | 3 | Inconsistent typing: String vs Option<String> vs usize | lib.rs:178 |
| L-2 | LOW | 5 | `Result<T>` alias shadows std::result::Result in multi-error workspace | lib.rs:16 |
| L-3 | LOW | 5 | Comments claim exit code ranges that don't match reality | lib.rs:22-255 |

---

## VERDICT: **REJECT**

2 CRITICAL findings. The crate is a monolithic error enum that violates DDD bounded contexts, couples error types to CLI commands, and maintains a fragile hand-mapped exit code scheme. The `#[non_exhaustive]` attribute makes it worse by ensuring downstream breakage is silent.

**Required actions before resubmission:**

1. **Split the god enum** into per-domain error types (WorkspaceError, SessionError, BeadError, etc.) each in their respective domain crates.
2. **Remove CLI suggestions** from the error type — move to application layer.
3. **Replace raw String identifiers** with domain newtypes (WorkspaceId, SessionId, BeadId).
4. **Replace hand-maintained exit_code()** with a derive macro or const mapping that can't drift.
5. **Remove `#[non_exhaustive]`** or replace with a proper versioning strategy.
