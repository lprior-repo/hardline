//! JSON OUTPUT documentation for command help
//! These strings document the `SchemaEnvelope` structure used in JSON output

pub const fn add() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://add-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "message": "Created session '<name>'"
  }"#
}

pub const fn list() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelopeArray:
  {
    "$schema": "hardline://list-response/v1",
    "_schema_version": "1.0",
    "schema_type": "array",
    "success": true,
    "data": [
      {
        "display_branch": "<branch_name or null>",
        "changes": "<modified_count>",
        "beads": "<open/in_progress/blocked>",
        "id": <db_id>,
        "name": "<session_name>",
        "status": "<creating|active|paused|completed|failed>",
        "state": "<created|working|ready|merged|abandoned|conflict>",
        "workspace_path": "<absolute_path>",
        "branch": "<branch_name or null>",
        "created_at": <unix_timestamp>,
        "updated_at": <unix_timestamp>,
        "last_synced": <unix_timestamp or null>,
        "metadata": { ... } or null
      }
    ]
  }
  
  NOTE: display_branch is a convenience field for display (null shown as "-").
  Session fields are included via serde(flatten) - no duplicate keys (RFC 8259 compliant)."#
}

pub const fn remove() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://remove-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "message": "Removed session '<name>' | Session '<name>' already removed"
  }"#
}

pub const fn focus() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://focus-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "message": "Switched to session '<name>'"
  }"#
}

pub const fn status() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps sessions in a SchemaEnvelope:
  {
    "$schema": "hardline://status-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "sessions": [
      {
        "name": "<session_name>",
        "status": "<active|paused|completed|failed>",
        "workspace_path": "<absolute_path>",
        "branch": "<branch_name>",
        "changes": {
          "modified": <count>,
          "added": <count>,
          "deleted": <count>,
          "renamed": <count>
        },
        "diff_stats": {
          "insertions": <count>,
          "deletions": <count>
        },
        "beads": {
          "open": <count>,
          "in_progress": <count>,
          "blocked": <count>,
          "closed": <count>
        },
        ...
      }
    ]
  }"#
}

pub const fn sync() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://sync-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name_or_null>",
    "synced_count": <count>,
    "failed_count": <count>,
    "errors": []
  }"#
}

pub const fn init() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://init-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "message": "<message>",
    "hardline_dir": "<absolute_path>",
    "config_file": "<absolute_path>",
    "state_db": "<absolute_path>",
    "layouts_dir": "<absolute_path>"
  }"#
}

pub const fn spawn() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://spawn-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "bead_id": "<bead_id>",
    "session_name": "<session_name>",
    "workspace_path": "<absolute_path>",
    "agent": "<agent_command>",
    "status": "<started|running|completed|failed>",
    "message": "<status_message>"
  }"#
}

pub const fn done() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://done-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "session_name": "<session_name>",
    "merged": true,
    "commit_id": "<commit_hash>",
    "message": "Merged and cleaned up '<name>'"
  }"#
}

pub const fn diff() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://diff-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "name": "<session_name>",
    "base": "<base_commit>",
    "head": "<head_commit>",
    "diff_stat": {
      "files_changed": <count>,
      "insertions": <count>,
      "deletions": <count>,
      "files": [...]
    },
    "diff_content": "<full_diff_or_null>"
  }"#
}

pub const fn config() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://config-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "key": "<config_key_or_null>",
    "value": "<config_value_or_null>",
    "config": {...}
  }"#
}

pub const fn clean() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://clean-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "removed_count": <count>,
    "sessions": ["<session_name>", ...]
  }"#
}

pub const fn introspect() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://introspect-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "commands": [...],
    "dependencies": {...},
    "system_state": {...}
  }"#
}

pub const fn doctor() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://doctor-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "checks": [
      {
        "name": "<check_name>",
        "status": "<pass|warn|fail>",
        "message": "<message>",
        "suggestion": "<suggestion_or_null>"
      },
      ...
    ],
    "summary": {
      "passed": <count>,
      "warnings": <count>,
      "failed": <count>
    }
  }"#
}

pub const fn query() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used (default), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://query-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "query_type": "<query_type>",
    "result": <query_specific_result>
  }"#
}

pub const fn context() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used (default when not TTY), output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://context-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "repository": {...},
    "sessions": [...],
    "beads": {...},
    "health": {...},
    "environment": {...}
  }"#
}

pub const fn checkpoint() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://checkpoint-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "action": "<create|restore|list>",
    "checkpoint_id": "<id_or_null>",
    "checkpoints": [...]
  }"#
}

pub const fn export() -> &'static str {
    r#"JSON OUTPUT:
  When --json is used, output wraps the response in a SchemaEnvelope:
  {
    "$schema": "hardline://export-response/v1",
    "_schema_version": "1.0",
    "schema_type": "single",
    "success": true,
    "version": "<format_version>",
    "exported_at": "<RFC3339_timestamp>",
    "count": <session_count>,
    "sessions": [...]
  }"#
}

/// AI-Native contract documentation for commands
pub mod ai_contracts {
    /// Machine-readable contract for hardline add command
    pub const fn add() -> &'static str {
        r#"AI CONTRACT for hardline add:
{
  "command": "hardline add",
  "intent": "Create isolated workspace for manual interactive development",
  "prerequisites": [
    "hardline init must have been run",
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
    "hardline add feature-auth",
    "hardline add bugfix-123 --no-open",
    "hardline add experiment -t minimal"
  ],
  "next_commands": [
    "hardline focus <name>",
    "hardline status <name>",
    "hardline work <bead_id>"
  ]
}"#
    }

    /// Machine-readable contract for hardline work command
    const WORK_CONTRACT: &str = r#"AI CONTRACT for hardline work:
{
  "command": "hardline work",
  "intent": "Create or reuse a named workspace and optionally register an agent",
  "prerequisites": [
    "hardline init must have been run",
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
    "hardline work feature-auth",
    "hardline work bug-fix --bead hardline-123",
    "hardline work feature-auth --agent-id agent-1 --idempotent",
    "hardline work feature-auth --dry-run"
  ],
  "next_commands": [
    "hardline done",
    "hardline checkpoint create",
    "hardline status"
  ]
}"#;

    pub const fn work() -> &'static str {
        WORK_CONTRACT
    }

    /// Machine-readable contract for hardline spawn command
    pub const fn spawn() -> &'static str {
        r#"AI CONTRACT for hardline spawn:
{
  "command": "hardline spawn",
  "intent": "Create workspace and spawn automated agent with isolation",
  "prerequisites": [
    "hardline init must have been run",
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
    "hardline spawn hardline-abc123",
    "hardline spawn hardline-abc123 --agent claude-opus"
  ]
}"#
    }

    /// Machine-readable contract for hardline done command
    pub const fn done() -> &'static str {
        r#"AI CONTRACT for hardline done:
{
  "command": "hardline done",
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
    "hardline done",
    "hardline done feature-auth",
    "hardline done --force"
  ]
}"#
    }

    /// Machine-readable contract for hardline sync command
    const SYNC_CONTRACT: &str = r#"AI CONTRACT for hardline sync:
{
  "command": "hardline sync",
  "intent": "Sync session workspace with main branch by rebasing onto latest main",
  "prerequisites": [
    "Session must exist in database",
    "Workspace directory must exist",
    "JJ repository must be accessible",
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
      "JjCommandError"
    ]
  },
  "examples": [
    "hardline sync",
    "hardline sync feature-auth",
    "hardline sync --all",
    "hardline sync --dry-run",
    "hardline sync --json"
  ],
  "next_commands": [
    "hardline done",
    "hardline diff",
    "hardline status"
  ]
}"#;

    pub const fn sync() -> &'static str {
        SYNC_CONTRACT
    }

    /// Machine-readable contract for hardline abort command
    const ABORT_CONTRACT: &str = r#"AI CONTRACT for hardline abort:
{
  "command": "hardline abort",
  "intent": "Abandon workspace without merging, discarding all changes",
  "prerequisites": [
    "Must be in a workspace or specify --workspace",
    "Workspace should exist in session database"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Workspace files (unless --keep-workspace)"],
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
      "description": "Keep workspace files, just remove from hardline tracking"
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
    "hardline abort",
    "hardline abort --workspace feature-x",
    "hardline abort --keep-workspace",
    "hardline abort --dry-run"
  ]
}"#;

    pub const fn abort() -> &'static str {
        ABORT_CONTRACT
    }

    /// Machine-readable contract for hardline remove command
    const REMOVE_CONTRACT: &str = r#"AI CONTRACT for hardline remove:
{
  "command": "hardline remove",
  "intent": "Remove a session and its workspace, optionally merging changes first",
  "prerequisites": [
    "hardline init must have been run",
    "Session must exist in database (unless --idempotent)"
  ],
  "side_effects": {
    "creates": [],
    "deletes": ["JJ workspace", "Session record", "Workspace directory"],
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
      "message": "string (e.g., 'Removed session <name>' or 'Session <name>' already removed)"
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
    "hardline remove old-feature",
    "hardline remove test-session -f",
    "hardline remove feature-x --merge",
    "hardline remove stale-session --idempotent",
    "hardline remove experiment --json"
  ],
  "next_commands": [
    "hardline list",
    "hardline add <name>",
    "hardline clean"
  ]
}"#;

    pub const fn remove() -> &'static str {
        REMOVE_CONTRACT
    }

    /// Machine-readable contract for hardline status command
    pub const fn status() -> &'static str {
        r#"AI CONTRACT for hardline status:
{
  "command": "hardline status",
  "intent": "Query current state of sessions and workspaces",
  "prerequisites": [
    "hardline init must have been run"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "description": "Specific session name, or all if omitted"
    }
  },
  "outputs": {
    "success": {
      "sessions": [
        {
          "name": "string",
          "status": "active|paused|completed|failed",
          "workspace_path": "string",
          "branch": "string",
          "changes": {
            "modified": "number",
            "added": "number",
            "deleted": "number"
          },
          "beads": {
            "open": "number",
            "in_progress": "number",
            "blocked": "number"
          }
        }
      ]
    }
  },
  "examples": [
    "hardline status",
    "hardline status feature-auth"
  ]
}"#
    }

    /// Machine-readable contract for hardline ai command
    const AI_CONTRACT: &str = r#"AI CONTRACT for hardline ai:
{
  "command": "hardline ai",
  "intent": "AI-first entry point providing status, workflows, and guidance for AI agents",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "subcommand": {
      "type": "string",
      "required": false,
      "default": "default overview",
      "options": ["status", "workflow", "quick-start", "next"],
      "description": "AI subcommand to execute"
    }
  },
  "outputs": {
    "status": {
      "location": "string",
      "workspace": "string|null",
      "agent_id": "string|null",
      "initialized": "boolean",
      "active_sessions": "number",
      "ready": "boolean",
      "suggestion": "string",
      "next_command": "string"
    },
    "workflow": {
      "name": "string",
      "steps": [
        {
          "step": "number",
          "command": "string",
          "description": "string"
        }
      ]
    },
    "quick-start": {
      "essential_commands": "array",
      "orientation": "array",
      "workflow": "array"
    },
    "next": {
      "action": "string",
      "command": "string",
      "reason": "string",
      "priority": "high|medium|low"
    }
  },
  "examples": [
    "hardline ai",
    "hardline ai status",
    "hardline ai workflow",
    "hardline ai quick-start",
    "hardline ai next",
    "hardline ai --json"
  ]
}"#;

    pub const fn ai() -> &'static str {
        AI_CONTRACT
    }

    /// Machine-readable contract for hardline contract command
    pub const fn contract() -> &'static str {
        r#"AI CONTRACT for hardline contract:
{
  "command": "hardline contract",
  "intent": "Query machine-readable contracts for hardline commands to understand inputs, outputs, and side effects",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": false,
      "position": 1,
      "description": "Specific command to show contract for (shows all if omitted)",
      "examples": ["add", "done", "spawn", "work"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON format"
    }
  },
  "outputs": {
    "success": {
      "commands": [
        {
          "name": "string",
          "description": "string",
          "required_args": "array",
          "optional_args": "array",
          "flags": "array",
          "output_schema": "string",
          "side_effects": "array",
          "examples": "array",
          "reversible": "boolean",
          "undo_command": "string|null",
          "prerequisites": "array"
        }
      ],
      "global_flags": "array",
      "version": "string"
    },
    "errors": [
      "UnknownCommand"
    ]
  },
  "examples": [
    "hardline contract                    Show all command contracts",
    "hardline contract add                Show contract for 'add' command",
    "hardline contract --json             Output all contracts as JSON",
    "hardline contract done --json        Show 'done' contract as JSON"
  ]
}"#
    }

    /// Machine-readable contract for hardline can-i command
    pub const fn can_i() -> &'static str {
        r#"AI CONTRACT for hardline can-i:
{
  "command": "hardline can-i",
  "intent": "Check if an action is permitted in the current context",
  "prerequisites": [
    "hardline must be initialized"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "action": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Action to check permission for",
      "examples": ["add", "done", "merge", "abort"]
    },
    "resource": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "Resource to check permission on",
      "examples": ["session-name", "workspace-name"]
    }
  },
  "outputs": {
    "success": {
      "action": "string",
      "resource": "string|null",
      "permitted": "boolean",
      "reason": "string"
    },
    "errors": [
      "InvalidAction",
      "ResourceNotFound"
    ]
  },
  "examples": [
    "hardline can-i add",
    "hardline can-i done feature-x",
    "hardline can-i merge"
  ]
}"#
    }

    /// AI hints for command sequencing
    pub const fn command_flow() -> &'static str {
        r#"AI COMMAND FLOW:
{
  "typical_workflows": {
    "manual_feature_development": [
      "hardline init",
      "hardline add feature-name",
      "hardline focus feature-name",
      "... work ...",
      "hardline checkpoint create",
      "hardline done"
    ],
    "automated_agent_task": [
      "hardline init",
      "hardline work feature-name --agent-id agent-1",
      "hardline focus session-name",
      "... agent works ...",
      "hardline done"
    ],
    "parallel_agent_tasks": [
      "hardline init",
      "hardline spawn bead-1",
      "hardline spawn bead-2",
      "hardline spawn bead-3",
      "... agents work in parallel ...",
      "hardline sync --all",
      "hardline done --all"
    ]
  },
  "command_preconditions": {
    "hardline add": ["hardline init"],
    "hardline work": ["hardline init"],
    "hardline spawn": ["hardline init"],
    "hardline done": ["active session"],
    "hardline focus": ["active session"],
    "hardline sync": ["active session"]
  },
  "error_recovery": {
    "MergeConflict": ["hardline resolve", "hardline done --force"],
    "WorkspaceDirty": ["hardline checkpoint create", "jj commit"],
    "SessionNotFound": ["hardline list", "hardline add"],
    "AgentCrash": ["hardline attach", "hardline status"]
  }
}"#
    }

    /// Machine-readable contract for hardline diff command
    const DIFF_CONTRACT: &str = r#"AI CONTRACT for hardline diff:
{
  "command": "hardline diff",
  "intent": "Show changes between session workspace and main branch",
  "prerequisites": [
    "Session must exist in database",
    "Workspace directory must exist",
    "JJ repository must be accessible"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "default": "auto-detected from current workspace",
      "description": "Session name to show diff for",
      "examples": ["feature-auth", "bugfix-123"]
    },
    "stat": {
      "type": "boolean",
      "flag": "--stat",
      "required": false,
      "description": "Show diffstat summary instead of full diff"
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
      "session": "string",
      "diff_type": "full|stat",
      "content": "string (diff output)",
      "stats": {
        "files_changed": "number",
        "insertions": "number",
        "deletions": "number"
      }
    },
    "errors": [
      "SessionNotFound",
      "WorkspaceNotFound",
      "JjCommandError"
    ]
  },
  "examples": [
    "hardline diff",
    "hardline diff feature-auth",
    "hardline diff --stat",
    "hardline diff feature-auth --json"
  ],
  "next_commands": [
    "hardline done",
    "hardline status",
    "hardline sync"
  ]
}"#;

    pub const fn diff() -> &'static str {
        DIFF_CONTRACT
    }

    /// Machine-readable contract for hardline list command
    const LIST_CONTRACT: &str = r#"AI CONTRACT for hardline list:
{
  "command": "hardline list",
  "intent": "Query all sessions in the repository to see status and metadata",
  "prerequisites": [
    "hardline init must have been run"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "all": {
      "type": "boolean",
      "flag": "--all",
      "required": false,
      "description": "Include completed and failed sessions (default: active only)"
    },
    "verbose": {
      "type": "boolean",
      "flag": "-v, --verbose",
      "required": false,
      "description": "Show workspace paths and bead titles"
    },
    "bead": {
      "type": "string",
      "flag": "--bead",
      "required": false,
      "description": "Filter sessions by bead ID",
      "examples": ["hardline-abc123", "feat-456"]
    },
    "agent": {
      "type": "string",
      "flag": "--agent",
      "required": false,
      "description": "Filter sessions by agent owner"
    },
    "state": {
      "type": "string",
      "flag": "--state",
      "required": false,
      "description": "Filter by workspace state (created, working, ready, merged, abandoned, conflict, active, complete, terminal, non-terminal)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelopeArray"
    }
  },
  "outputs": {
    "success": {
      "schema_type": "array",
      "data": [
        {
          "name": "string",
          "status": "active|paused|completed|failed",
          "branch": "string",
          "changes": "string (count)",
          "beads": "string (open/in_progress/blocked)",
          "workspace_path": "string",
          "metadata": "object|null"
        }
      ]
    },
    "errors": [
      "DatabaseError"
    ]
  },
  "examples": [
    "hardline list",
    "hardline list --all",
    "hardline list --verbose",
    "hardline list --bead hardline-abc123",
    "hardline list --agent agent-001",
    "hardline list --state active",
    "hardline list --json"
  ],
  "next_commands": [
    "hardline status <name>",
    "hardline focus <name>",
    "hardline add <name>",
    "hardline work <bead_id>"
  ]
}"#;

    pub const fn list() -> &'static str {
        LIST_CONTRACT
    }

    /// Machine-readable contract for hardline focus command
    pub const fn focus() -> &'static str {
        r#"AI CONTRACT for hardline focus:
{
  "command": "hardline focus",
  "intent": "Switch to a session to work on it",
  "prerequisites": [
    "Session must exist in database"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "name": {
      "type": "string",
      "required": false,
      "position": 1,
      "default": "interactive selection",
      "description": "Name of the session to focus",
      "examples": ["feature-auth", "bugfix-123"]
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
      "message": "string"
    },
    "errors": [
      "SessionNotFound",
      "NoSessionsAvailable"
    ]
  },
  "examples": [
    "hardline focus feature-auth",
    "hardline focus",
    "hardline focus bugfix-123 --json"
  ],
  "next_commands": [
    "hardline status",
    "hardline done",
    "hardline diff"
  ]
}"#
    }

    /// Machine-readable contract for hardline context command
    const CONTEXT_CONTRACT: &str = r#"AI CONTRACT for hardline context:
{
  "command": "hardline context",
  "intent": "Show complete environment context for AI agents and programmatic access",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "field": {
      "type": "string",
      "flag": "--field",
      "required": false,
      "description": "Extract single field using JSON pointer path",
      "examples": ["repository.branch", "session.name", "location.path"]
    },
    "no_beads": {
      "type": "boolean",
      "flag": "--no-beads",
      "required": false,
      "description": "Skip beads database query (faster)"
    },
    "no_health": {
      "type": "boolean",
      "flag": "--no-health",
      "required": false,
      "description": "Skip health checks (faster)"
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "default": "true when not TTY",
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "location": {
        "type": "string (main) or object (workspace)",
        "description": "Current location in repository"
      },
      "session": {
        "type": "object|null",
        "description": "Session context if in workspace",
        "fields": ["name", "status", "bead_id", "agent", "created_at", "last_synced"]
      },
      "repository": {
        "type": "object",
        "description": "Repository state information",
        "fields": ["root", "branch", "uncommitted_files", "commits_ahead", "has_conflicts"]
      },
      "beads": {
        "type": "object|null",
        "description": "Beads tracking information",
        "fields": ["active", "blocked_by", "ready_count", "in_progress_count"]
      },
      "health": {
        "type": "object",
        "description": "Health status of the system",
        "status_values": ["good", "warn", "error"]
      },
      "suggestions": {
        "type": "array of strings",
        "description": "Actionable suggestions based on context"
      }
    },
    "errors": [
      "NotInJjRepo",
      "SessionNotFound",
      "BeadsDatabaseError"
    ]
  },
  "examples": [
    "hardline context",
    "hardline context --json",
    "hardline context --field=repository.branch",
    "hardline context --no-beads --no-health",
    "hardline context --field=location.path"
  ],
  "next_commands": [
    "hardline whereami",
    "hardline status",
    "hardline work"
  ]
}"#;

    pub const fn context() -> &'static str {
        CONTEXT_CONTRACT
    }

    /// Machine-readable contract for hardline introspect command
    const INTROSPECT_CONTRACT: &str = r#"AI CONTRACT for hardline introspect:
{
  "command": "hardline introspect",
  "intent": "Discover hardline capabilities, command details, and system state for AI agent understanding",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": false,
      "position": 1,
      "description": "Specific command to introspect (shows all if omitted)",
      "examples": ["add", "done", "focus", "sync"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    },
    "ai": {
      "type": "boolean",
      "flag": "--ai",
      "required": false,
      "description": "AI-optimized output: combines capabilities, state, and recommendations"
    },
    "env-vars": {
      "type": "boolean",
      "flag": "--env-vars",
      "required": false,
      "description": "Show environment variables hardline reads and sets"
    },
    "workflows": {
      "type": "boolean",
      "flag": "--workflows",
      "required": false,
      "description": "Show common workflow patterns for AI agents"
    },
    "session-states": {
      "type": "boolean",
      "flag": "--session-states",
      "required": false,
      "description": "Show valid session state transitions"
    }
  },
  "outputs": {
    "success": {
      "version": "string",
      "commands": [
        {
          "name": "string",
          "description": "string",
          "arguments": "array",
          "flags": "array",
          "examples": "array",
          "prerequisites": "object",
          "side_effects": "array",
          "error_conditions": "array"
        }
      ],
      "dependencies": {
        "jj": "object|null"
      },
      "system_state": {
        "initialized": "boolean",
        "jj_repo": "boolean",
        "active_sessions": "number"
      }
    },
    "env_vars_mode": {
      "env_vars": [
        {
          "name": "string",
          "description": "string",
          "direction": "read|write|both",
          "default": "string|null",
          "example": "string"
        }
      ]
    },
    "workflows_mode": {
      "workflows": [
        {
          "name": "string",
          "description": "string",
          "steps": [
            {
              "step": "number",
              "command": "string",
              "description": "string"
            }
          ]
        }
      ]
    },
    "session_states_mode": {
      "states": ["creating", "active", "syncing", "merging", "completed", "failed"],
      "transitions": [
        {
          "from": "string",
          "to": "string",
          "trigger": "string"
        }
      ]
    },
    "errors": [
      "UnknownCommand"
    ]
  },
  "examples": [
    "hardline introspect",
    "hardline introspect add",
    "hardline introspect --json",
    "hardline introspect --env-vars",
    "hardline introspect --workflows",
    "hardline introspect --session-states",
    "hardline introspect --ai"
  ],
  "next_commands": [
    "hardline contract",
    "hardline context",
    "hardline ai"
  ]
}"#;

    pub const fn introspect() -> &'static str {
        INTROSPECT_CONTRACT
    }

    /// Machine-readable contract for hardline examples command
    const EXAMPLES_CONTRACT: &str = r#"AI CONTRACT for hardline examples:
{
  "command": "hardline examples",
  "intent": "Show copy-pastable usage examples for commands, useful for AI agents and users",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "position": 1,
      "required": false,
      "description": "Filter examples for a specific command",
      "examples": ["add", "done", "work", "spawn"]
    },
    "use_case": {
      "type": "string",
      "flag": "--use-case",
      "required": false,
      "description": "Filter by use case category",
      "options": ["workflow", "single-command", "error-handling", "maintenance", "automation", "ai-agent", "multi-agent", "safety"]
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
      "examples": [
        {
          "name": "string",
          "description": "string",
          "commands": ["array of command strings"],
          "expected_output": "string or null",
          "use_case": "string",
          "prerequisites": ["array of strings"],
          "notes": "string or null"
        }
      ],
      "use_cases": ["array of available use case categories"]
    }
  },
  "examples": [
    "hardline examples",
    "hardline examples add",
    "hardline examples --use-case workflow",
    "hardline examples --json",
    "hardline examples done --json"
  ],
  "next_commands": [
    "hardline contract",
    "hardline ai quick-start",
    "hardline context"
  ]
}"#;

    pub const fn examples() -> &'static str {
        EXAMPLES_CONTRACT
    }

    /// Machine-readable contract for hardline validate command
    pub const fn validate() -> &'static str {
        r#"AI CONTRACT for hardline validate:
{
  "command": "hardline validate",
  "intent": "Validate command arguments before execution",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Command to validate arguments for"
    },
    "args": {
      "type": "array of strings",
      "required": false,
      "description": "Arguments to validate"
    },
    "dry_run": {
      "type": "boolean",
      "flag": "--dry-run",
      "required": false,
      "description": "Preview validation without executing"
    }
  },
  "outputs": {
    "success": {
      "valid": "boolean",
      "command": "string",
      "errors": "array of strings"
    }
  },
  "examples": [
    "hardline validate add feature-auth",
    "hardline validate remove old-session"
  ]
}"#
    }

    /// Machine-readable contract for hardline whatif command
    const WHATIF_CONTRACT: &str = r#"AI CONTRACT for hardline whatif:
{
  "command": "hardline whatif",
  "intent": "Preview what a command would do without executing it, showing steps, resources, and reversibility",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none (preview only)"
  },
  "inputs": {
    "command": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Command to preview",
      "examples": ["add", "done", "remove", "spawn", "sync"]
    },
    "args": {
      "type": "array of strings",
      "required": false,
      "position": "2..",
      "description": "Arguments for the command being previewed",
      "examples": ["feature-auth", "--workspace", "my-session"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output preview as JSON with SchemaEnvelope"
    },
    "on_success": {
      "type": "string",
      "flag": "--on-success",
      "required": false,
      "description": "Command to run after successful execution"
    },
    "on_failure": {
      "type": "string",
      "flag": "--on-failure",
      "required": false,
      "description": "Command to run after failed execution"
    }
  },
  "outputs": {
    "success": {
      "command": "string",
      "args": "array of strings",
      "steps": [
        {
          "order": "number",
          "description": "string",
          "action": "string",
          "can_fail": "boolean",
          "on_failure": "string or null"
        }
      ],
      "creates": [
        {
          "resource_type": "string",
          "resource": "string",
          "description": "string"
        }
      ],
      "modifies": "array of resource changes",
      "deletes": "array of resource changes",
      "side_effects": "array of strings",
      "reversible": "boolean",
      "undo_command": "string or null",
      "warnings": "array of strings",
      "prerequisites": [
        {
          "check": "string",
          "status": "met|notmet|unknown",
          "description": "string"
        }
      ]
    },
    "errors": [
      "InvalidSessionName"
    ]
  },
  "examples": [
    "hardline whatif add feature-x",
    "hardline whatif done --workspace feature-x",
    "hardline whatif remove old-session",
    "hardline whatif spawn hardline-abc123",
    "hardline whatif sync --all --json"
  ],
  "next_commands": [
    "hardline add",
    "hardline done",
    "hardline remove",
    "hardline spawn"
  ]
}"#;

    pub const fn whatif() -> &'static str {
        WHATIF_CONTRACT
    }

    /// Machine-readable contract for hardline whereami command
    const WHEREAMI_CONTRACT: &str = r#"AI CONTRACT for hardline whereami:
{
  "command": "hardline whereami",
  "intent": "Quick location query returning simple location identifier for AI agent orientation",
  "prerequisites": [
    "Must be in a JJ repository"
  ],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    },
    "contract": {
      "type": "boolean",
      "flag": "--contract",
      "required": false,
      "description": "Show machine-readable contract for AI integration"
    }
  },
  "outputs": {
    "success": {
      "location_type": "string (main or workspace)",
      "workspace_name": "string or null",
      "workspace_path": "string or null",
      "simple": "string (main or workspace:<name>)"
    },
    "main_location": {
      "location_type": "main",
      "workspace_name": null,
      "workspace_path": null,
      "simple": "main"
    },
    "workspace_location": {
      "location_type": "workspace",
      "workspace_name": "<workspace_name>",
      "workspace_path": "<absolute_path>",
      "simple": "workspace:<workspace_name>"
    },
    "errors": [
      "NotInJjRepo"
    ]
  },
  "examples": [
    "hardline whereami                    Returns 'main' or 'workspace:<name>'",
    "hardline whereami --json             Full JSON output with SchemaEnvelope",
    "hardline whereami --contract         Show this contract"
  ],
  "next_commands": [
    "hardline context",
    "hardline status",
    "hardline work"
  ]
}"#;

    pub const fn whereami() -> &'static str {
        WHEREAMI_CONTRACT
    }

    /// Machine-readable contract for hardline query command
    const QUERY_CONTRACT: &str = r#"AI CONTRACT for hardline query:
{
  "command": "hardline query",
  "intent": "Query system state programmatically for AI agents and automation",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "query_type": {
      "type": "string",
      "required": true,
      "position": 1,
      "description": "Type of query to execute",
      "options": [
        "session-exists",
        "session-count",
        "can-run",
        "suggest-name",
        "lock-status",
        "can-spawn",
        "pending-merges",
        "location"
      ],
      "examples": ["session-exists", "can-run", "location"]
    },
    "args": {
      "type": "string",
      "required": false,
      "position": 2,
      "description": "Query-specific arguments",
      "examples": ["my-session", "add", "feat{n}"]
    },
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "default": true,
      "description": "Output as JSON (default for query command)"
    }
  },
  "outputs": {
    "session-exists": {
      "exists": "boolean",
      "session": {
        "name": "string",
        "status": "string"
      },
      "error": "object or null"
    },
    "session-count": {
      "count": "number",
      "filter": "object or null"
    },
    "can-run": {
      "can_run": "boolean",
      "command": "string",
      "blockers": ["array of blocker objects"],
      "prerequisites_met": "number",
      "prerequisites_total": "number"
    },
    "suggest-name": {
      "pattern": "string",
      "suggested": "string",
      "next_available_n": "number",
      "existing_matches": ["array of strings"]
    },
    "lock-status": {
      "session": "string",
      "locked": "boolean",
      "holder": "string or null",
      "expires_at": "string or null",
      "error": "object or null"
    },
    "can-spawn": {
      "can_spawn": "boolean",
      "bead_id": "string or null",
      "reason": "string or null",
      "blockers": ["array of strings"]
    },
    "pending-merges": {
      "sessions": ["array of session objects"],
      "count": "number",
      "error": "object or null"
    },
    "location": {
      "type": "string (main or workspace)",
      "name": "string or null",
      "path": "string or null",
      "simple": "string",
      "error": "object or null"
    },
    "errors": [
      "UnknownQueryType",
      "MissingRequiredArgument",
      "DatabaseError",
      "InvalidPattern"
    ]
  },
  "examples": [
    "hardline query session-exists my-feature",
    "hardline query session-count",
    "hardline query session-count --status=active",
    "hardline query can-run add",
    "hardline query suggest-name 'feature-{n}'",
    "hardline query lock-status my-session",
    "hardline query can-spawn",
    "hardline query pending-merges",
    "hardline query location"
  ],
  "next_commands": [
    "hardline context",
    "hardline status",
    "hardline introspect"
  ]
}"#;

    pub const fn query() -> &'static str {
        QUERY_CONTRACT
    }

    /// Machine-readable contract for hardline whoami command
    pub const fn whoami() -> &'static str {
        r#"AI CONTRACT for hardline whoami:
{
  "command": "hardline whoami",
  "intent": "Query the current agent identity - returns agent ID or 'unregistered'",
  "prerequisites": [],
  "side_effects": {
    "creates": [],
    "modifies": [],
    "state_transition": "none"
  },
  "inputs": {
    "json": {
      "type": "boolean",
      "flag": "--json",
      "required": false,
      "description": "Output as JSON with SchemaEnvelope"
    }
  },
  "outputs": {
    "success": {
      "registered": "boolean",
      "agent_id": "string|null",
      "current_session": "string|null",
      "current_bead": "string|null",
      "simple": "string (agent_id or 'unregistered')"
    },
    "environment_sources": {
      "Hardline_AGENT_ID": "Agent identifier",
      "Hardline_BEAD_ID": "Current bead being worked on",
      "Hardline_WORKSPACE": "Current workspace path",
      "Hardline_SESSION": "Current session name"
    }
  },
  "examples": [
    "hardline whoami",
    "hardline whoami --json"
  ],
  "next_commands": [
    "hardline context",
    "hardline status",
    "hardline whereami"
  ]
}"#
    }
}

#[cfg(test)]
mod tests {
    use super::ai_contracts;

    mod martin_fowler_work_contract_behavior {
        use super::*;

        /// GIVEN: The AI contract for `hardline work`
        /// WHEN: We inspect supported agent-related flags
        /// THEN: It should document `--agent-id` and reject stale `--agent`
        #[test]
        fn given_work_contract_when_inspecting_flags_then_documents_real_agent_flag() {
            let contract = ai_contracts::work();

            assert!(contract.contains("--agent-id"));
            assert!(!contract.contains("\"flag\": \"--agent\""));
            assert!(contract.contains("claude-opus"));
        }
    }
}
