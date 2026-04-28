# Troubleshooting

Common issues and how to resolve them.

---

## Workspace Issues

### "Session not found"

```bash
# List available sessions
isolate list

# Check where you are
isolate whereami
```

### "Workspace already exists"

```bash
# Use --idempotent to succeed if already exists
isolate work feature-123 --idempotent
```

### "Detached HEAD"

This shouldn't happen with JJ, but if it does:

```bash
# Check current state
jj status

# See the current commit
jj log -r @

# Create a new change if needed
jj new
```

---

## Sync Issues

### "Sync failed"

```bash
# Try again with verbose output
isolate sync --verbose

# Check for conflicts
jj status
jj diff
```

### Conflicts during sync

```bash
# See what conflicts exist
jj status

# Resolve manually
vim <conflicted-file>

# Commit the resolution
jj describe -m "resolve: merge conflicts"
```

---

## JJ Issues

### "jj: command not found"

Install JJ:

```bash
# Via cargo
cargo install jj-cli

# Via Homebrew
brew install jj
```

### "Cannot lock"

```bash
# Check what's locking
jj log

# Force unlock if needed (rare)
# JJ handles this automatically
```

---

## Exit Codes

| Code | Meaning | What to Do |
|------|---------|-------------|
| 0 | Success | Done |
| 1 | Validation error | Check input syntax |
| 2 | Not found | Check session/task name |
| 3 | System error | Check system resources |
| 4 | External command error | Check JJ installation |
| 5 | Lock contention | Try again later |

---

## Getting Help

```bash
# Check isolate version
isolate --version

# Get help for a command
isolate <command> --help

# Check context
isolate context
```

---

## Common Patterns

### Start Fresh

```bash
isolate whereami                    # Should return "main"
isolate work feature-auth --idempotent
```

### Continue Existing Work

```bash
isolate whereami                    # Returns "workspace:feature-auth"
# Already in workspace, continue working
```

### Abandon and Start Over

```bash
isolate abort --dry-run             # Preview
isolate abort                       # Execute
isolate work feature-auth-v2        # Start fresh
```

---

## Prevention

1. **Run `isolate sync` regularly** — Don't let main get too far ahead
2. **Use `--idempotent` when retrying** — Prevents "already exists" errors
3. **Check `isolate whereami` before operations** — Know where you are
4. **Use `isolate context` for full status** — See everything at once
