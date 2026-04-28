# CLI Commands

Complete reference for Isolate CLI commands.

---

## Core Commands

These are the commands you need 90% of the time:

| Command | Description |
|---------|-------------|
| `isolate spawn <bead>` | Spawn isolated workspace for a task |
| `isolate work [bead] [name]` | Start work on a task |
| `isolate sync` | Sync workspace with main (auto-rebase) |
| `isolate done [name]` | Complete and merge work |
| `isolate abort [name]` | Abort and cleanup workspace |
| `isolate whereami` | Show current location |
| `isolate context` | Show full context |

---

## Session Management

| Command | Description |
|---------|-------------|
| `isolate add <name>` | Create session for manual work |
| `isolate list` | List all sessions |
| `isolate remove <name>` | Remove a session |
| `isolate clone <name>` | Clone a session |
| `isolate rename <old> <new>` | Rename a session |
| `isolate pause [name]` | Pause a session |
| `isolate resume [name]` | Resume a paused session |

---

## Object Commands

### Task (Beads)

```
isolate task list              # List all tasks
isolate task show <id>         # Show task details
isolate task claim <id>        # Claim a task
isolate task yield <id>        # Yield a task
isolate task start <id>        # Start work on a task
isolate task done <id>         # Complete a task
```

### Session

```
isolate session list           # List all sessions
isolate session add <name>     # Create new session
isolate session remove <name>  # Remove a session
isolate session spawn <bead>   # Spawn session for agent work
isolate session sync           # Sync with remote
```

### Status

```
isolate status show            # Show current status
isolate status whereami       # Show current location
isolate status whoami         # Show current identity
isolate status context        # Show context information
```

### Config

```
isolate config list            # List configuration values
isolate config get <key>      # Get a config value
isolate config set <key> <value>  # Set a config value
isolate config schema         # Show configuration schema
```

### Doctor

```
isolate doctor check           # Run diagnostics
isolate doctor fix            # Fix detected issues
isolate doctor integrity      # Check system integrity
isolate doctor clean          # Clean up invalid sessions
```

---

## Additional Commands

| Command | Description |
|---------|-------------|
| `isolate init` | Initialize isolate in a JJ repository |
| `isolate checkpoint [name]` | Create checkpoint |
| `isolate undo` | Undo last operation |
| `isolate revert` | Revert changes |
| `isolate claim <resource>` | Claim a resource |
| `isolate yield <resource>` | Yield a resource |
| `isolate lock <name>` | Acquire lock |
| `isolate unlock <name>` | Release lock |
| `isolate whoami` | Show current user/agent |
| `isolate diff` | Show changes |
| `isolate clean` | Clean up |
| `isolate validate` | Validate configurations |

---

## Common Flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON |
| `--verbose`, `-v` | Enable verbose output |
| `--dry-run` | Preview without executing |
| `--idempotent` | Succeed if already exists |
| `--force`, `-f` | Force operation |

---

## Aliases

| Command | Alias |
|---------|-------|
| `isolate done` | `isolate submit` |
| `isolate checkpoint` | `isolate ckpt` |
| `isolate task claim` | `isolate task take` |
| `isolate task yield` | `isolate task release` |
| `isolate session sync` | `isolate session rebase` |

---

## Quick Reference

### Start Working

```bash
isolate whereami           # Check you're on main
isolate work feature-123   # Start work
```

### While Working

```bash
isolate sync              # Sync with main
isolate context           # Check status
```

### Finish Work

```bash
isolate done              # Complete and merge
isolate abort             # Abort and cleanup
```

---

## Exit Codes

- 0: Success
- 1: Validation error (user input)
- 2: Not found
- 3: System error
- 4: External command error
- 5: Lock contention
