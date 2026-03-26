//! AI-native contract documentation for commands - Part 1A
//! Machine-readable contracts for AI agents to understand scp commands

/// Machine-readable contract for scp add command
pub const fn add() -> &'static str {
    r#"AI CONTRACT for scp add:
{
  "command": "scp add",
  "intent": "Create scpd workspace for manual interactive development",
  "prerequisites": [
    "scp init must have been run",
    "JJ repository must be initialized"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Database session record"],
    "modifies": [],
    "state_transition": "none → active"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "validation": "Must be valid session name (alphanumeric, hyphens, underscores)",
      "examples": ["feature-auth", "bugfix-123", "experiment-alpha"]
    },
    "no_open": {
      "type": "boolean",
      "required": false,
      "flag": "--no-open",
      "description": "Skip opening workspace after creation"
    },
    "no_hooks": {
      "type": "boolean",
      "required": false,
      "flag": "--no-hooks",
      "description": "Skip post-create hooks"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "workspace_path": "string",
      "status": "active"
    },
    "errors": [
      "SessionAlreadyExists",
      "InvalidSessionName",
      "JJInitFailed",
      "DatabaseError"
    ]
  },
  "examples": [
    "scp add feature-auth",
    "scp add bugfix-123 --no-open",
    "scp add experiment -t minimal"
  ],
  "next_commands": [
    "scp focus <name>",
    "scp status <name>",
    "scp work <bead_id>"
  ]
}"#
}

/// Machine-readable contract for scp work command
pub const fn work() -> &'static str {
    r#"AI CONTRACT for scp work:
{
  "command": "scp work",
  "intent": "Create or reuse a named workspace and optionally register an agent",
  "prerequisites": [
    "scp init must have been run",
    "Must run inside a JJ repository"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Database session record"],
    "modifies": ["Session metadata"],
    "state_transition": "none → active"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": true,
      "position": 1,
      "validation": "Must pass session name validation",
      "examples": ["feature-auth", "bug-fix-123"]
    },
    "bead": {
      "type": "string",
      "required": false,
      "flag": "-b|--bead",
      "description": "Optional bead ID to associate"
    },
    "agent_id": {
      "type": "string",
      "required": false,
      "flag": "--agent-id",
      "description": "Optional agent identifier"
    },
    "no_agent": {
      "type": "boolean",
      "required": false,
      "flag": "--no-agent",
      "description": "Skip agent registration"
    },
    "idempotent": {
      "type": "boolean",
      "required": false,
      "flag": "--idempotent",
      "description": "Reuse existing session if present"
    },
    "dry_run": {
      "type": "boolean",
      "required": false,
      "flag": "--dry-run",
      "description": "Preview without creating"
    }
  },
  "outputs": {
    "success": {
      "name": "string",
      "workspace_path": "string",
      "created": "boolean",
      "agent_id": "string|null",
      "bead_id": "string|null",
      "env_vars": "array",
      "enter_command": "string"
    },
    "errors": [
      "InvalidSessionName",
      "SessionAlreadyExists",
      "NotInJjRepository",
      "WorkspaceCreationFailed"
    ]
  },
  "examples": [
    "scp work feature-auth",
    "scp work bug-fix --bead scp-123",
    "scp work feature-auth --agent-id agent-1 --idempotent",
    "scp work feature-auth --dry-run"
  ],
  "next_commands": [
    "scp done",
    "scp checkpoint create",
    "scp status"
  ]
}"#
}

/// Machine-readable contract for scp spawn command
pub const fn spawn() -> &'static str {
    r#"AI CONTRACT for scp spawn:
{
  "command": "scp spawn",
  "intent": "Create workspace and spawn automated agent with isolation",
  "prerequisites": [
    "scp init must have been run",
    "Beads database must be available",
    "Agent system must be configured"
  ],
  "side_effects": {
    "creates": ["JJ workspace", "Agent process", "Database records"],
    "modifies": ["Bead status", "Agent registry"],
    "state_transition": "open → in_progress"
  },
  "inputs": {
    "bead_id": {
      "type": "string",
      "required": true,
      "position": 1,
      "validation": "Must be open bead in database"
    },
    "agent": {
      "type": "string",
      "required": false,
      "flag": "-a|--agent",
      "default": "claude"
    }
  },
  "outputs": {
    "success": {
      "bead_id": "string",
      "session_name": "string",
      "workspace_path": "string",
      "agent": "string",
      "status": "started|running|completed|failed"
    }
  },
  "examples": [
    "scp spawn scp-abc123",
    "scp spawn scp-abc123 --agent claude-opus"
  ]
}"#
}

/// Machine-readable contract for scp done command
pub const fn done() -> &'static str {
    r#"AI CONTRACT for scp done:
{
  "command": "scp done",
  "intent": "Complete work, merge changes to main, and cleanup workspace",
  "prerequisites": [
    "Session must be active",
    "Workspace must have committed changes",
    "No merge conflicts should exist"
  ],
  "side_effects": {
    "creates": ["Merge commit on main"],
    "deletes": ["JJ workspace", "Session record"],
    "modifies": ["Main branch", "Bead status"],
    "state_transition": "active → completed"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "current session"
    },
    "force": {
      "type": "boolean",
      "flag": "--force",
      "description": "Force merge even with conflicts"
    }
  },
  "outputs": {
    "success": {
      "session_name": "string",
      "merged": true,
      "commit_id": "string"
    },
    "errors": [
      "NoActiveSession",
      "MergeConflict",
      "WorkspaceDirty",
      "SessionNotFound"
    ]
  },
  "examples": [
    "scp done",
    "scp done feature-auth",
    "scp done --force"
  ]
}"#
