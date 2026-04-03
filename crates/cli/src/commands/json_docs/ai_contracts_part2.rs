//! AI-native contract documentation for commands - Part 2
//! Machine-readable contracts for AI agents to understand scp commands

/// Machine-readable contract for scp sync command
pub const fn sync() -> &'static str {
    r#"AI CONTRACT for scp sync:
{
  "command": "scp sync",
  "intent": "Sync session workspace with main branch by rebasing onto latest main",
  "prerequisites": [
    "Session must exist in database",
    "Workspace directory must exist",
    "Git repository must be accessible",
    "No uncommitted changes with conflicts"
  ],
  "side_effects": {
    "creates": [],
    "modifies": ["Session workspace (rebases onto main)", "last_synced timestamp"],
    "state_transition": "workspace -> workspace (updated)"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "current workspace (detected from context)",
      "description": "Session name to sync",
      "examples": ["feature-auth", "bugfix-123"]
    },
    "all": {
      "type": "boolean",
      "flag": "--all",
      "required": false,
      "description": "Sync all active sessions"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview sync without executing"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "name": "string|null",
      "synced_count": "number",
      "failed_count": "number",
      "errors": "array of error objects"
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceNotFound",
      "RebaseConflict",
      "GitCommandError"
    ]
  },
  "examples": [
    "scp sync",
    "scp sync feature-auth",
    "scp sync --all",
    "scp sync --dry-run",
    "scp sync --json"
  ],
  "next_commands": [
    "scp done",
    "scp diff",
    "scp status"
  ]
}"#
}

/// Machine-readable contract for scp abort command
pub const fn abort() -> &'static str {
    r#"AI CONTRACT for scp abort:
{
  "command": "scp abort",
  "intent": "Abandon workspace without merging, discarding all changes",
  "prerequisites": [
    "Must be in a workspace or specify --workspace",
    "Workspace should exist in session database"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["Git workspace", "Session record", "Workspace files (unless --keep-workspace)"],
    "modifies": ["Bead status (set back to ready unless --no-bead-update)"],
    "state_transition": "active → abandoned"
  },
  "inputs": {
    "workspace": {
      "type": "string",
      "flag": "-w|--workspace",
      "required": false,
      "default": "current workspace",
      "description": "Workspace/session to abort"
    },
    "keep_workspace": {
      "type": "boolean",
      "flag": "--keep-workspace",
      "required": false,
      "description": "Keep workspace files, just remove from scp tracking"
    },
    "no_bead_update": {
      "type": "boolean",
      "flag": "--no-bead-update",
      "required": false,
      "description": "Don't update bead status back to ready"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview abort without executing"
    }
  },
  "outputs": {
    "success": {
      "session_name": "string",
      "workspace_removed": "boolean",
      "bead_updated": "boolean",
      "message": "string"
    },
    "errors": [
      "NotInWorkspace",
      "SessionNotFound",
      "WorkspaceRemovalFailed"
    ]
  },
  "examples": [
    "scp abort",
    "scp abort --workspace feature-x",
    "scp abort --keep-workspace",
    "scp abort --dry-run"
  ]
}"#
}

/// Machine-readable contract for scp remove command
pub const fn remove() -> &'static str {
    r#"AI CONTRACT for scp remove:
{
  "command": "scp remove",
  "intent": "Remove a session and its workspace, optionally merging changes first",
  "prerequisites": [
    "scp init must have been run",
    "Session must exist in database (unless --idempotent)"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["Git workspace", "Session record", "Workspace directory"],
    "modifies": ["Session database", "Main branch (if --merge)"],
    "state_transition": "active → removed"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Name of the session to remove",
      "examples": ["old-feature", "test-session", "experiment"]
    },
    "force": {
      "type": "boolean",
      "flag": "-f, --force",
      "required": false,
      "description": "Skip pre_remove hooks (non-interactive, no confirmation)"
    },
    "merge": {
      "type": "boolean",
      "flag": "-m, --merge",
      "required": false,
      "description": "Squash-merge changes to main before removal"
    },
    "keep_branch": {
      "type": "boolean",
      "flag": "-k, --keep-branch",
      "required": false,
      "description": "Preserve branch after removal"
    },
    "idempotent": {
      "type": "boolean",
      "flag": "--idempotent",
      "required": false,
      "description": "Succeed if session doesn't exist (safe for retries)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "message": "string (e.g., 'Removed session <name>' or 'Session <name> already removed')"
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceRemovalFailed",
      "MergeFailed",
      "DatabaseError"
    ]
  },
  "exit_codes": {
    "0": "Success",
    "1": "Validation error",
    "2": "Not found error",
    "3": "IO error"
  },
  "examples": [
    "scp remove old-feature",
    "scp remove test-session -f",
    "scp remove feature-x --merge",
    "scp remove stale-session --idempotent",
    "scp remove experiment --json"
  ],
  "next_commands": [
    "scp list",
    "scp add <name>",
    "scp clean"
  ]
}"#
}
