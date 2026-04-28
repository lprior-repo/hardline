# Isolate CLI Command Reference

Complete reference for all Isolate CLI commands.

---

## Command Overview

Isolate uses an object-based command structure for organization:

```
isolate task <action>      # Manage tasks/beads
isolate session <action>   # Manage workspaces/sessions  
isolate status <action>    # Query system status
isolate config <action>   # Manage configuration
isolate doctor <action>   # Run diagnostics
```

---

## Object Commands

### Task Management (Beads)

```bash
isolate task list              # List all tasks
isolate task show <id>         # Show task details
isolate task start <id>        # Start work on a task
isolate task done <id>        # Complete a task
```

### Session Management

```bash
isolate session list           # List all sessions
isolate session add <name>     # Create new session
isolate session remove <name>  # Remove a session
isolate session pause <name>   # Pause a session
isolate session resume <name>  # Resume a paused session
isolate session clone <name>   # Clone a session
isolate session rename <name>  # Rename a session
isolate session spawn <bead>   # Spawn session for automated work
isolate session sync           # Sync session with remote
isolate session init           # Initialize isolate in a repo
```

### Status

```bash
isolate status                 # Query system status
isolate status <action>       # Various status queries
```

### Configuration

```bash
isolate config                # Manage isolate configuration
isolate config <action>      # Various config operations
```

### Diagnostics

```bash
isolate doctor                # Run diagnostics
isolate doctor <action>      # Run specific diagnostic
```

---

## Flat Commands

### Initialization

```bash
isolate init                  # Initialize isolate in current JJ repository
isolate init --dry-run       # Preview initialization
isolate init --json          # Output JSON metadata
```

### Session Creation

```bash
isolate add <name>           # Create session for manual work (JJ workspace)
isolate add <name> --bead <id>    # Associate with bead
isolate add <name> --no-open      # Create without opening terminal
isolate add <name> --no-hooks    # Skip post-create hooks
isolate add <name> --idempotent  # Succeed if already exists
isolate work <bead>          # Start work on a task (simpler than add)
isolate work <bead> <name>  # Start work with custom name
isolate work <bead> --idempotent  # Succeed if already exists
isolate spawn <bead>        # Spawn session for automated agent work
isolate spawn <bead> --agent <name>  # Specify agent name
isolate spawn <bead> --idempotent    # Succeed if already exists
```

### Session Navigation

```bash
isolate list                 # List all sessions
isolate list --all          # Include all sessions
isolate context             # Show current context
isolate context --field <path>  # Extract single field (e.g., repository.branch)
isolate context --no-beads   # Skip beads database query (faster)
isolate context --no-health # Skip health checks (faster)
```

### Session Completion

```bash
isolate done [name]         # Complete and merge work
isolate sync                # Sync session with main
isolate abort [name]        # Abort and clean up workspace
isolate abort [name] --force  # Force abort without confirmation
```

### Session Management

```bash
isolate remove <name>       # Remove a session
isolate rename <name>       # Rename a session
isolate clone <name>        # Clone session
isolate pause <name>        # Pause a session
isolate resume <name>       # Resume a paused session
```

### Task Management (Flat Commands)

(Use `isolate task` subcommands instead - see below)

### Task Object Commands

### History & Recovery

```bash
isolate checkpoint [name]    # Create checkpoint
isolate undo                # Undo last operation
isolate revert              # Revert changes
```

### Identity

```bash
isolate whoami              # Show current user/agent
isolate whereami            # Show current location (main or workspace)
```

### Help & Info

```bash
isolate help                # Print help
isolate introspect          # Show all capabilities
isolate introspect <cmd>    # Show command details
isolate introspect --env-vars   # Show environment variables
isolate introspect --workflows  # Show workflow patterns
```

### Completion

```bash
isolate completions <shell> # Generate shell completions
```

### Validation

```bash
isolate validate             # Validate configurations
```

### Other Commands

```bash
isolate diff                # Show changes
isolate clean                # Clean up
isolate prune-invalid        # Remove invalid entries
isolate whatif              # Preview operations
isolate events              # List events
isolate backup              # Create backup
isolate recover             # Recover from errors
isolate retry               # Retry failed operation
isolate rollback            # Rollback operation
isolate wait                # Wait for condition
isolate schema              # Show schema
isolate examples            # Show examples
```

---

## Common Flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (machine-parseable) |
| `--verbose`, `-v` | Enable verbose output |
| `--dry-run` | Preview without executing |
| `--idempotent` | Succeed even if already exists |
| `--force`, `-f` | Force operation without confirmation |
| `--contract` | Show machine-readable contract (AI) |
| `--ai-hints` | Show execution hints (AI) |
| `--on-success <CMD>` | Run command after success |
| `--on-failure <CMD>` | Run command after failure |

---

## Quick Reference: 90% of Workflows

```bash
# Check where you are
isolate whereami

# Start work on a task
isolate work <bead-id>

# List all sessions
isolate list

# Sync with main
isolate sync

# Complete work
isolate done

# Abort work
isolate abort
```

---

## Object Command Aliases

| Command | Alias |
|---------|-------|
| `isolate done` | `isolate submit` |
| `isolate checkpoint` | `isolate ckpt` |
| `isolate session add` | `isolate session create` |
| `isolate task done` | `isolate task complete` |
| `isolate session sync` | `isolate session rebase` |

---

## Examples

### Start Working on a New Feature

```bash
# Check you're on main
isolate whereami

# Start work on a bead
isolate work feature-abc123

# Do your work...

# Sync with main if needed
isolate sync

# Complete work
isolate done
```

### Continue Existing Work

```bash
# Check where you are
isolate whereami  # Returns "workspace:feature-abc123"

# You're already in the workspace, continue working
```

### Abandon and Start Over

```bash
# Preview abort
isolate abort --dry-run

# Execute abort
isolate abort

# Start fresh
isolate work feature-abc123-v2
```

### Multiple Sessions

```bash
# List all sessions
isolate list --json

# Sync all with main
isolate session sync --all
```

---

## Error Handling

Exit codes:
- 0: Success
- 1: Validation error (user input)
- 2: Not found
- 3: System error
- 4: External command error
- 5: Lock contention

Errors include suggestions:
```json
{
  "success": false,
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "...",
    "suggestion": "Use 'isolate list' to see available sessions"
  }
}
```

---

**Related**: [AI Agent Guide](AI_AGENT_GUIDE.md) | [Index](INDEX.md)
