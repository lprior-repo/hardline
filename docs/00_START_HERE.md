# Isolate Documentation - Start Here

**The Law**: No unwrap, no panic, no unsafe. Period.

## Why JJ Instead of Git?

Isolate uses JJ (Jujutsu) because Git breaks at multi-agent scale (4+ agents). JJ provides:

- **Lock-free concurrency** — agents don't corrupt each other's work
- **Operation log** — undo any operation, always recover
- **Anonymous commits** — no branch pollution at 8-12 agents
- **First-class conflicts** — no blocking on merges

> Running 8-12 agents in parallel? You need JJ. See [09_JUJUTSU.md](09_JUJUTSU.md) for the full comparison.

## 🎯 Go Here For...

| Need | File |
|------|------|
| 5-minute crash course | This page (below) |
| **AI Agent Guide** | **[AI_AGENT_GUIDE.md](AI_AGENT_GUIDE.md)** - Everything agents need |
| Error handling | [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) |
| Build & test commands | [02_MOON_BUILD.md](02_MOON_BUILD.md) |
| Daily workflow | [03_WORKFLOW.md](03_WORKFLOW.md) |
| Functional patterns | [04_FUNCTIONAL_PATTERNS.md](04_FUNCTIONAL_PATTERNS.md) |
| All lint rules | [05_RUST_STANDARDS.md](05_RUST_STANDARDS.md) |
| Iterator combinators | [06_COMBINATORS.md](06_COMBINATORS.md) |
| Testing patterns | [07_TESTING.md](07_TESTING.md) |
| **Complete index** | **[INDEX.md](INDEX.md)** - All docs organized |

## 5-Minute Crash Course

### The Three Laws

```rust
❌ .unwrap()     // FORBIDDEN - Compiler error
❌ .panic!()      // FORBIDDEN - Compiler error
❌ unsafe { }     // FORBIDDEN - Compiler error
```

### How to Handle Errors

```rust
// Use the ? operator (best)
fn operation() -> Result<T> {
    let value = fallible()?;
    Ok(transform(value))
}

// Or match (explicit)
match operation() {
    Ok(v) => use_value(v),
    Err(e) => handle_error(e),
}

// Or combinators (chainable)
operation()
    .map(transform)
    .unwrap_or_default()
```

### Building

```bash
moon run :ci       # Full build + test
moon run :test     # Just tests
moon run :build    # Just build
moon run :quick    # Just lint

# NEVER:
cargo build        # ❌ Wrong
cargo test         # ❌ Wrong
```

### Starting Work

```bash
br update <issue> --status in_progress  # Claim issue
# Make changes
moon run :test     # Test locally
jj describe -m "feat: description"  # Commit
jj git push        # Push
br close <id>      # Close issue
```

## Error Handling at a Glance

| Situation | Code |
|-----------|------|
| Fallible operation | `fn op() -> Result<T>` |
| Early exit | `value?` |
| Transform error | `.map_err(\|e\| new_e)` |
| Transform value | `.map(\|v\| new_v)` |
| Chain operations | `.and_then(\|v\| op(v))` |
| Provide default | `.unwrap_or_default()` (or better: handle error explicitly) |
| Log & continue | `.inspect_err(\|e\| log(e))?` |

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

## Build Profile

```bash
# Release build
moon run :build

# Test build
moon run :test

# Development (manual):
cargo build --dev
```

## Common Commands

```bash
# View issues
br list

# Claim an issue
br update BD-123 --status in_progress

# Make changes (tracked automatically)
# Your changes are tracked automatically

# Review changes
jj diff
jj status

# Commit
jj describe -m "feat: add feature"

# Start next change
jj new

# Push
jj git push

# Close issue
br close BD-123
```

## The Compiler is Your Friend

If your code doesn't compile, it's because:
1. You used unwrap/panic/unsafe (forbidden)
2. You didn't handle an error case
3. You didn't handle an Option (Some/None)
4. A borrow checker issue

All of these are **good things** - the compiler catches bugs before production.

## Next Steps

1. Read [01_ERROR_HANDLING.md](01_ERROR_HANDLING.md) (15 min)
2. Read [02_MOON_BUILD.md](02_MOON_BUILD.md) (10 min)
3. Read [03_WORKFLOW.md](03_WORKFLOW.md) (15 min)
4. Reference [06_COMBINATORS.md](06_COMBINATORS.md) as needed

Then you're ready to code.

## The Mantra

> "No panics. All errors return Results. The compiler enforces this. We write safe, correct Rust."

---

**Next**: [Error Handling](01_ERROR_HANDLING.md)
