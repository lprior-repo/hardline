# scp-cli

Source Control Plane — unified CLI for workspace isolation, queue management, and Git operations.

## Installation

```bash
cargo install --path crates/cli
```

## Quick Start

```bash
# Initialize SCP in a Git repository
scp init

# Create a workspace for a task
scp workspace spawn feature-auth

# Do your work, then commit
scp workspace commit "Add OAuth2 flow"

# Merge the workspace back to main
scp workspace done -m "Implement OAuth2 authentication"
```

## Global Flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable debug output |
| `-q, --quiet` | Suppress normal output |
| `-f, --format <fmt>` | Output format: `human`, `json`, `yaml` |
| `--database <path>` | Custom database path |

## Command Reference

### Workspace Management

Workspace is the core primitive — isolated git worktrees for concurrent work.

```bash
scp workspace spawn <name> [--sync]    # Create a new workspace
scp workspace switch <name>            # Switch to a workspace
scp workspace list                     # List all workspaces
scp workspace status                   # Show current workspace status
scp workspace done [name] [-m msg]     # Complete and merge workspace
scp workspace abort [name]             # Delete a workspace
scp workspace sync [name] [--all]      # Rebase workspace onto main
scp workspace fork <name> [from]       # Fork from current or another workspace
scp workspace merge <name>             # Merge workspace into main
scp workspace rename <old> <new>       # Rename a workspace
scp workspace log [limit]              # Show workspace commit log
scp workspace diff [path]              # Show uncommitted changes
scp workspace uncommitted              # List uncommitted files
scp workspace commit <message>         # Commit all changes
scp workspace branches                 # List branches
scp workspace branch <name>            # Create a branch
scp workspace branch-delete <name>     # Delete a branch
scp workspace branch-current           # Show current branch
scp workspace branch-rename <old> <new>  # Rename a branch
scp workspace add <path>               # Register existing path as workspace
```

### Integrity & Recovery

```bash
scp workspace integrity-validate <workspace>    # Validate workspace state
scp workspace integrity-repair <workspace> [--force]  # Repair corrupted workspace
scp workspace integrity-backup-list             # List available backups
scp workspace integrity-backup-restore <id> [--force]  # Restore from backup
scp workspace recover [target] [--diagnose] [--dry-run]  # Recover broken state
scp workspace rollback <session> <commit> [--dry-run]  # Rollback to a commit
scp workspace revert <name> [--dry-run]         # Revert a session merge
```

### Query & Inspection

```bash
scp workspace query <type> [arg] [--status] [--agent]  # Query session state
scp workspace can-i <action> [resource]                  # Check if operation is permitted
scp workspace events [--session] [--type] [--follow]     # Show event history
scp workspace clean [--dry-run] [--force] [--verbose]    # Remove stale data
scp workspace introspect [target]                         # Introspect command metadata
scp workspace whoami [--json]                             # Show agent identity
scp workspace wait <condition> [--timeout] [--poll]       # Wait for a condition
scp workspace undo [--dry-run] [--list]                   # Undo last operation
scp workspace checkpoint create [-m msg]                  # Create a checkpoint
scp workspace checkpoint restore <id>                     # Restore a checkpoint
scp workspace checkpoint list                             # List checkpoints
scp workspace contract [command]                          # Show command contracts
scp workspace validate <cmd> [args] [--dry-run]           # Validate command inputs
scp workspace export [session] [-o file]                  # Export sessions
scp workspace import <file> [--force] [--dry-run]         # Import sessions
scp workspace work [name] [--bead] [--agent] [--dry-run]  # Show work context
scp workspace completions <shell>                         # Generate shell completions
scp workspace prune [--yes] [--dry-run]                   # Prune invalid data
scp workspace schema [--list] [--all] [name]              # Show JSON schemas
```

### Bookmarks

```bash
scp workspace bookmark create <name>   # Create a bookmark (Git branch)
scp workspace bookmark list            # List bookmarks
scp workspace bookmark delete <name>   # Delete a bookmark
scp workspace bookmark track <name>    # Set upstream for a bookmark
```

### Lock Management

Distributed locking for multi-agent coordination.

```bash
scp lock acquire <session> <agent> [--ttl]  # Acquire a session lock
scp lock release <session> <agent>          # Release a session lock
scp lock heartbeat <session> <agent>        # Send heartbeat for a lock
scp lock status [session]                   # Show lock status
scp lock list                              # List all locks
```

### Queue Management

```bash
scp queue list                              # List merge queue
scp queue enqueue <branch> [--priority]     # Add branch to queue
scp queue dequeue                           # Pop next from queue
scp queue process [--checks]                # Process queue head
scp queue insert <position> <branch>        # Insert at position
scp queue remove <branch>                   # Remove from queue
scp queue status                            # Show queue status
```

### Agent Management

```bash
scp agent create <name>       # Register a new agent
scp agent list                # List known agents
scp agent kill <id>           # Terminate an agent
scp agent status [id]         # Show agent status
scp agent register [session]  # Register agent to a session
scp agent heartbeat [session] # Send agent heartbeat
```

### Session Management

```bash
scp session list                    # List sessions
scp session status                  # Show session status
scp session focus <name>            # Set active session
scp session submit [name] [-m msg]  # Submit session for review
scp session remove <name> [--force] [--merge]  # Remove a session
scp session pause <name>            # Pause a session
scp session resume <name>           # Resume a paused session
scp session clone <source> <target> [--dry-run]  # Clone a session
```

### Task Management

```bash
scp task list                # List tasks
scp task show <id>           # Show task details
scp task claim <id> <user>   # Claim a task
scp task yield <id> <user>   # Yield a claimed task
scp task start <id> <user>   # Start working on a task
scp task done [id] <user>    # Mark task complete
```

### Configuration

```bash
scp config get <key>         # Get a config value
scp config set <key> <value> # Set a config value
scp config list              # List all config
scp config ports [--json]    # Show port configuration
```

### Git Stash

```bash
scp stash save [-m msg] [--include-untracked]  # Stash changes
scp stash pop [stash@{n}]                      # Apply and remove stash
scp stash list                                  # List stashes
scp stash drop <stash@{n}>                     # Drop a stash
scp stash show [stash@{n}]                     # Show stash contents
```

### Git Tags

```bash
scp tag list [pattern]                  # List tags
scp tag create <name> [-m msg] [--force]  # Create a tag
scp tag delete <name> [--remote]         # Delete a tag
scp tag push <tag> [-r remote]           # Push a tag to remote
```

### Batch

```bash
scp batch run <workspace> <cmd>...  # Execute commands atomically in a workspace
```

### Sync & Push

```bash
scp fetch [remote] [-p] [-t] [-a]  # Fetch from remotes
scp pull                           # Pull from remote
scp push [-r remote] [-b branch] [--force] [-t]  # Push to remote
```

### Utilities

```bash
scp status [-s]       # Show short or detailed status
scp doctor [--full]   # Run health diagnostics
scp context           # Show current context (workspace, branch, VCS)
scp whereami          # Alias for context
scp whatif <cmd> [args]     # Preview command without executing
scp examples [cmd] [--use-case]  # Show usage examples
scp init [--vcs git]  # Initialize SCP in current directory
scp switch <name>     # Top-level alias for workspace switch
```

## Shell Completions

Generate shell completions for bash, zsh, fish, or powershell:

```bash
scp workspace completions zsh > ~/.zsh/completions/_scp
```

## Configuration

SCP stores configuration in the platform-appropriate config directory. Key settings:

- **Database path**: Override with `--database` flag or `SCP_DATABASE_PATH` env var
- **Log level**: Controlled by `-v`/`-q` flags or `RUST_LOG` env var

## Architecture

`scp-cli` is the CLI frontend for the Source Control Plane. It delegates to:

- **scp-core**: Error types, output formatting, shared domain types
- **scp-vcs**: Git operations via gitoxide (gix)
- **scp-stack**: Queue and merge management

The CLI follows a command-dispatch pattern: `main.rs` parses arguments via clap and delegates to handler functions in `commands/`.
