# Hardline CLI Command Reference

Complete reference for all Hardline CLI commands.

---

## Command Overview

Hardline uses an object-based command structure for organization:

```
hardline task <action>      # Manage tasks/beads
hardline session <action>   # Manage workspaces/sessions  
hardline status <action>    # Query system status
hardline config <action>   # Manage configuration
hardline doctor <action>   # Run diagnostics
```

---

## Object Commands

### Task Management (Beads)

```bash
hardline task list              # List all tasks
hardline task show <id>         # Show task details
hardline task start <id>        # Start work on a task
hardline task done <id>        # Complete a task
```

### Session Management

```bash
hardline session list           # List all sessions
hardline session add <name>     # Create new session
hardline session remove <name>  # Remove a session
hardline session pause <name>   # Pause a session
hardline session resume <name>  # Resume a paused session
hardline session clone <name>   # Clone a session
hardline session rename <name>  # Rename a session
hardline session spawn <bead>   # Spawn session for automated work
hardline session sync           # Sync session with remote
hardline session init           # Initialize hardline in a repo
```

### Status

```bash
hardline status                 # Query system status
hardline status <action>       # Various status queries
```

### Configuration

```bash
hardline config                # Manage hardline configuration
hardline config <action>      # Various config operations
```

### Diagnostics

```bash
hardline doctor                # Run diagnostics
hardline doctor <action>      # Run specific diagnostic
```

---

## Flat Commands

### Initialization

```bash
hardline init                  # Initialize hardline in current JJ repository
hardline init --dry-run       # Preview initialization
hardline init --json          # Output JSON metadata
```

### Session Creation

```bash
hardline add <name>           # Create session for manual work (JJ workspace)
hardline add <name> --bead <id>    # Associate with bead
hardline add <name> --no-open      # Create without opening terminal
hardline add <name> --no-hooks    # Skip post-create hooks
hardline add <name> --idempotent  # Succeed if already exists
hardline work <bead>          # Start work on a task (simpler than add)
hardline work <bead> <name>  # Start work with custom name
hardline work <bead> --idempotent  # Succeed if already exists
hardline spawn <bead>        # Spawn session for automated agent work
hardline spawn <bead> --agent <name>  # Specify agent name
hardline spawn <bead> --idempotent    # Succeed if already exists
```

### Session Navigation

```bash
hardline list                 # List all sessions
hardline list --all          # Include all sessions
hardline context             # Show current context
hardline context --field <path>  # Extract single field (e.g., repository.branch)
hardline context --no-beads   # Skip beads database query (faster)
hardline context --no-health # Skip health checks (faster)
```

### Session Completion

```bash
hardline done [name]         # Complete and merge work
hardline sync                # Sync session with main
hardline abort [name]        # Abort and clean up workspace
hardline abort [name] --force  # Force abort without confirmation
```

### Session Management

```bash
hardline remove <name>       # Remove a session
hardline rename <name>       # Rename a session
hardline clone <name>        # Clone session
hardline pause <name>        # Pause a session
hardline resume <name>       # Resume a paused session
```

### Task Management (Flat Commands)

(Use `hardline task` subcommands instead - see below)

### Task Object Commands

### History & Recovery

```bash
hardline checkpoint [name]    # Create checkpoint
hardline undo                # Undo last operation
hardline revert              # Revert changes
```

### Identity

```bash
hardline whoami              # Show current user/agent
hardline whereami            # Show current location (main or workspace)
```

### Help & Info

```bash
hardline help                # Print help
hardline introspect          # Show all capabilities
hardline introspect <cmd>    # Show command details
hardline introspect --env-vars   # Show environment variables
hardline introspect --workflows  # Show workflow patterns
```

### Completion

```bash
hardline completions <shell> # Generate shell completions
```

### Validation

```bash
hardline validate             # Validate configurations
```

### Other Commands

```bash
hardline diff                # Show changes
hardline clean                # Clean up
hardline prune-invalid        # Remove invalid entries
hardline whatif              # Preview operations
hardline events              # List events
hardline backup              # Create backup
hardline recover             # Recover from errors
hardline retry               # Retry failed operation
hardline rollback            # Rollback operation
hardline wait                # Wait for condition
hardline schema              # Show schema
hardline examples            # Show examples
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
hardline whereami

# Start work on a task
hardline work <bead-id>

# List all sessions
hardline list

# Sync with main
hardline sync

# Complete work
hardline done

# Abort work
hardline abort
```

---

## Object Command Aliases

| Command | Alias |
|---------|-------|
| `hardline done` | `hardline submit` |
| `hardline checkpoint` | `hardline ckpt` |
| `hardline session add` | `hardline session create` |
| `hardline task done` | `hardline task complete` |
| `hardline session sync` | `hardline session rebase` |

---

## Examples

### Start Working on a New Feature

```bash
# Check you're on main
hardline whereami

# Start work on a bead
hardline work feature-abc123

# Do your work...

# Sync with main if needed
hardline sync

# Complete work
hardline done
```

### Continue Existing Work

```bash
# Check where you are
hardline whereami  # Returns "workspace:feature-abc123"

# You're already in the workspace, continue working
```

### Abandon and Start Over

```bash
# Preview abort
hardline abort --dry-run

# Execute abort
hardline abort

# Start fresh
hardline work feature-abc123-v2
```

### Multiple Sessions

```bash
# List all sessions
hardline list --json

# Sync all with main
hardline session sync --all
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
    "suggestion": "Use 'hardline list' to see available sessions"
  }
}
```

---

**Related**: [AI Agent Guide](AI_AGENT_GUIDE.md) | [Index](INDEX.md)
