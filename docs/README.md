# Isolate Documentation

The Isolate project documentation — everything you need to start working.

---

## The Law

**No unwrap, no panic, no unsafe. Period.**

All fallible operations return `Result<T, Error>`. The compiler enforces this. We write safe, correct Rust.

---

## Quick Start

### The Three Laws (Compiler-Enforced)

```rust
❌ .unwrap()     // FORBIDDEN - Compiler error
❌ .panic!()      // FORBIDDEN - Compiler error
❌ unsafe { }     // FORBIDDEN - Compiler error
```

### How to Handle Errors

```rust
// Use the ? operator (best for early exit)
fn operation() -> Result<T> {
    let value = fallible()?;
    Ok(transform(value))
}

// Or match (explicit control)
match operation() {
    Ok(v) => use_value(v),
    Err(e) => handle_error(e),
}

// Or combinators (chainable)
operation()
    .map(transform)
    .unwrap_or_default()
```

### Error Handling Patterns

| Situation | Code |
|-----------|------|
| Fallible operation | `fn op() -> Result<T>` |
| Early exit | `value?` |
| Transform error | `.map_err(|e| new_e)` |
| Transform value | `.map(|v| new_v)` |
| Chain operations | `.and_then(|v| op(v))` |
| Provide default | `.unwrap_or_default()` |
| Log & continue | `.inspect_err(|e| log(e))?` |

---

## Common Commands

### Build & Test (Use Moon, not Cargo)

```bash
moon run :ci       # Full build + test
moon run :test     # Just tests
moon run :build    # Just build
moon run :quick    # Just lint
moon run :check    # Type check
moon run :fmt-fix  # Auto-fix formatting

# NEVER use cargo directly - it's wrong for this project
cargo build        # ❌ Wrong
cargo test         # ❌ Wrong
```

### Isolate (Workspace Management)

```bash
# Start work
isolate work <bead-id>     # Start on a task
isolate add <name>        # Create session manually

# Navigation
isolate list              # List all sessions
isolate whereami          # Check current location

# Complete work
isolate sync              # Sync with main
isolate done              # Complete and merge

# Abandon work
isolate abort             # Abort and cleanup
```

### Jujutsu (Version Control)

```bash
jj describe -m "feat: description"  # Commit
jj git push                         # Push
jj new                              # Start new change
jj log                              # View history
jj diff                             # Show changes
jj status                           # Show working state
```

### Beads (Issue Tracking)

```bash
br list                     # View issues
br update BD-123 --status in_progress  # Claim issue
br close BD-123             # Close issue
br sync --flush-only        # Sync bead state
```

### Codanna (Code Search)

```bash
codanna mcp find_symbol <name>                    # Exact symbol
codanna mcp search_symbols query:<pattern>        # Fuzzy search
codanna mcp semantic_search_docs query:"<query>"  # Semantic search
codanna index src lib                             # Reindex code
```

---

## Project Structure

```
isolate/
├── Cargo.toml              # Workspace (strict lints)
├── rust-toolchain.toml     # Nightly Rust
├── docs/                   # This documentation
└── crates/
    └── isolate-core/
        ├── Cargo.toml
        └── src/
            ├── lib.rs      # Library root
            ├── error.rs    # Error types
            ├── result.rs   # Result extensions
            └── functional.rs
```

---

## Why JJ Instead of Git?

Isolate uses JJ (Jujutsu) because Git breaks at multi-agent scale (4+ agents). JJ provides:

- **Lock-free concurrency** — agents don't corrupt each other's work
- **Operation log** — undo any operation, always recover
- **Anonymous commits** — no branch pollution at 8-12 agents
- **First-class conflicts** — no blocking on merges

> Running 8-12 agents in parallel? You need JJ. See [09_JUJUTSU.md](09_JUJUTSU.md) for the full comparison.

---

## Navigation

### I'm New Here

| Document | Purpose | Read Time |
|----------|---------|-----------|
| [00_START_HERE.md](00_START_HERE.md) | This file (5-min crash course) | 5 min |
| [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) | Everything AI agents need | 20 min |
| [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) | Fallible operations, Result patterns | 20 min |
| [02_MOON_BUILD.md](02_MOON_BUILD.md) | Building, testing, caching | 15 min |

### I Need a Command

See **Common Commands** above, or [COMMANDS.md](COMMANDS.md) for the complete reference.

### I'm an AI Agent

1. Read [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) — **everything you need**
2. Reference [08_BEADS.md](08_BEADS.md) for bead commands

### I Need...

| Need | Go To |
|------|-------|
| Error handling patterns | [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) |
| Build & test commands | [02_MOON_BUILD.md](02_MOON_BUILD.md) |
| Daily workflow | [03_WORKFLOW.md](03_WORKFLOW.md) |
| Functional patterns | [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) |
| All lint rules | [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) |
| Iterator combinators | [06_COMBINATORS.md](06_COMBINATORS.md) |
| Testing patterns | [07_TESTING.md](07_TESTING.md) |
| Issue tracking | [08_BEADS.md](08_BEADS.md) |
| Version control (JJ) | [09_JUJUTSU.md](09_JUJUTSU.md) |
| Complete index | [INDEX.md](INDEX.md) |

---

## Learning Paths

### Quick Start (1 hour)
1. This file (5 min)
2. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) (20 min)
3. [02_MOON_BUILD.md](02_MOON_BUILD.md) (15 min)
4. [03_WORKFLOW.md](03_WORKFLOW.md) (20 min)

### AI Agent Onboarding (30 minutes)
1. [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) (20 min) — everything you need!

### Deep Dive (2 hours)
1. This file (5 min)
2. [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) (20 min)
3. [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) (25 min)
4. [06_COMBINATORS.md](06_COMBINATORS.md) (20 min)
5. [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) (20 min)
6. [07_TESTING.md](07_TESTING.md) (15 min)

---

## The Mantra

> "No panics. All errors return Results. The compiler enforces this. We write safe, correct Rust."

---

## Related Documentation

- [INDEX.md](INDEX.md) — Complete documentation index
- [COMMANDS.md](COMMANDS.md) — Full CLI command reference
- [AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md) — AI agent rules and workflow
