# Development Workflow

Isolate is designed for the development phase of multi-agent workflows.

## The Workflow

```
┌─────────────────────────────────────────────────────────┐
│  1. SPAWN                                              │
│  isolate spawn <bead-id>                               │
│  - Agent claims a bead                                  │
│  - JJ workspace created                                │
│  - Agent begins work                                   │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│  2. DEVELOP                                            │
│  - Agent makes changes                                 │
│  - Runs tests locally                                  │
│  - Commits with jj describe                            │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│  3. SYNC (as needed)                                  │
│  isolate sync                                           │
│  - Fetches latest from main                            │
│  - Auto-rebases onto new main                          │
│  - Handles conflicts if any                            │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│  4. DONE                                               │
│  isolate done                                           │
│  - Feature complete                                     │
│  - Ready to hand off to queue/stacking tool            │
└─────────────────────────────────────────────────────────┘
```

---

## Agent Workflow Example

### Start Work

```bash
# Check you're on main
isolate whereami
# Output: "main"

# Start work on a feature
isolate work feature-abc123
# Output: "workspace:feature-abc123 created"
```

### While Working

```bash
# Make changes to code...

# Sync with main if it has advanced
isolate sync

# Check status
isolate context
```

### Complete Work

```bash
# Feature is done and validated
isolate done

# Hand off to queue/stacking tool
# (external process)
```

---

## When to Sync

Run `isolate sync` when:

1. **Main has advanced** — other agents have merged features
2. **Before completing** — ensure your work is rebased onto latest
3. **On conflicts** — resolve and continue

---

## Handling Conflicts

If `isolate sync` produces conflicts:

```bash
# Check what conflicts exist
jj status
jj diff

# Resolve conflicts manually
vim <conflicted-file>

# Commit the resolution
jj describe -m "resolve: merge conflicts"

# Continue working
```

---

## Aborting

If something goes wrong:

```bash
# Preview what will happen
isolate abort --dry-run

# Abort and cleanup
isolate abort
```

---

## Architecture: Dev vs Queue

Isolate handles **development phase only**:

| Phase | Tool | What Happens |
|-------|------|--------------|
| **Development** | Isolate | Agent spawns workspace, works on feature, syncs as needed |
| **Queue/Stacking** | External Tool | Feature queued, stacked, rebased, merged to main |

The handoff happens after `isolate done`.

---

## Why This Architecture

**Isolate's job:** Workspace isolation during development
- Spawns isolated workspaces per agent
- Handles sync + auto-rebase while working
- Provides orientation (whereami, context)

**External tool's job:** Queue management and final integration
- Receives completed features
- Handles stacking, final rebase
- Merges to main

This separation keeps Isolate focused and simple.
