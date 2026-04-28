# Beads: Issue Tracking & Triage

Issue tracking and intelligent triage using Beads (graph-aware dependency system).

## Core Concept

Beads stores issues in `.beads/beads.jsonl`. Each bead (issue) is a node in a dependency graph. Use `bv` (Beads triage engine) to understand scope, dependencies, and prioritization.

## Creating Issues

### Basic Issue

```bash
br create "Feature: implement X"

# With description
br create "Bug: Y fails on Z" \
  --description "Steps to reproduce:
1. Do X
2. Observe Y
3. Should see Z instead"

# With priority and labels
br create "Feature: add validation" \
  --priority high \
  --labels feature,p0
```

### Issue Templates

**Feature**:
```bash
br create "Feature: description" --labels feature --priority high
```

**Bug**:
```bash
br create "Bug: what fails" --labels bug --priority high \
  --description "Steps: 1. ... 2. ... Expected: ... Actual: ..."
```

**Chore**:
```bash
br create "Chore: refactor X" --labels chore --priority medium
```

## Managing Issues

### List & Filter

```bash
br list                              # All open issues
br list --filter "status:open"       # Only open
br list --filter "assigned:me"       # My issues
br list --filter "label:feature"     # By label
br list --filter "priority:high"     # By priority
```

### Claiming Issues

```bash
br update BD-123 --status in_progress  # Start working
br update BD-123 --claim               # Or use --claim flag (atomic claim)
br show BD-123                         # See details
```

### Updating Status

```bash
br update BD-123 --status ready   # Mark ready for review
br close BD-123                  # Mark done
br update BD-123 --status open   # Reopen
```

### Adding Dependencies

```bash
br dep add BD-123 BD-124    # BD-123 blocks BD-124
br dep remove BD-123 BD-124 # Remove dependency
```

## Using `bv` for Triage

**`bv` is your triage engine.** It computes graph metrics (PageRank, critical path, cycles) and provides intelligent recommendations.

### Start Here: `bv --robot-triage`

```bash
bv --robot-triage
```

Returns in one call:
- `quick_ref` - At-a-glance summary (counts, top 3 picks)
- `recommendations` - Ranked actionable items with reasons
- `quick_wins` - Low-effort, high-impact tasks
- `blockers_to_clear` - Tasks that unblock most work
- `project_health` - Status/type distributions, graph metrics
- `commands` - Copy-paste commands for next steps

### Quick Next Steps

```bash
bv --robot-next    # Just the single top pick + claim command
```

### Planning & Parallel Work

```bash
bv --robot-plan              # Parallel execution tracks with unblock dependencies
bv --robot-plan --label core # Scope to "core" label subgraph
```

### Graph Analysis

```bash
bv --robot-insights  # Full metrics:
                     # - PageRank (importance)
                     # - Betweenness (bottleneck)
                     # - Critical path (minimum time to completion)
                     # - Cycles (circular dependencies - must fix!)
                     # - K-core (dense subgroups)
                     # - Eigenvector (authority)

bv --robot-insights | jq '.Cycles'  # Find cycles
```

### Label & Flow Analysis

```bash
bv --robot-label-health           # Health by label: velocity, staleness, block count
bv --robot-label-flow             # Cross-label dependencies and bottlenecks
bv --robot-label-attention        # What needs attention most (PageRank × staleness)
```

### History & Change Tracking

```bash
bv --robot-history                        # Bead-to-commit correlations
bv --robot-diff --diff-since HEAD~10     # What changed (new/closed/modified)
```

### Forecasting & Burndown

```bash
bv --robot-forecast BD-123  # ETA prediction with deps
bv --robot-burndown sprint  # Sprint burndown tracking
```

### Alerts & Hygiene

```bash
bv --robot-alerts   # Stale issues, blocking cascades, priority mismatches
bv --robot-suggest  # Hygiene: duplicates, missing deps, cycle breaks
```

### Export & Visualization

```bash
bv --robot-graph --graph-format json   # JSON dependency graph
bv --robot-graph --graph-format mermaid # Mermaid diagram
bv --export-graph graph.html           # Interactive visualization
```

## Filtering with Recipes

```bash
bv --recipe actionable --robot-plan     # Only ready-to-work (no blockers)
bv --recipe high-impact --robot-triage  # Only high PageRank
```

## Scoping by Dimension

```bash
bv --robot-plan --label backend         # Just backend work
bv --robot-insights --as-of HEAD~30     # Historical snapshot
bv --robot-triage --robot-triage-by-label  # Grouped by domain
bv --robot-triage --robot-triage-by-track  # Grouped by parallel tracks
```

## Understanding Output

Every `bv` response includes:
- `data_hash` - Fingerprint of beads.jsonl (verify consistency)
- `status` - Metric readiness: `computed|approx|timeout|skipped`
- `as_of_commit` - When using `--as-of`

### Two Phases of Analysis

**Phase 1 (instant)**: degree, topo sort, density
**Phase 2 (async, 500ms timeout)**: PageRank, betweenness, HITS, eigenvector, cycles

For large graphs (>500 nodes), some metrics may be approximated. Always check `status`.

## jq Cheatsheet

```bash
bv --robot-triage | jq '.quick_ref'                     # Summary
bv --robot-triage | jq '.recommendations[0]'            # Top pick
bv --robot-plan | jq '.plan.summary.highest_impact'     # Best unblock target
bv --robot-insights | jq '.Cycles'                      # Circular deps
bv --robot-insights | jq '.status'                      # Metric status
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

## Workflow Integration

### Morning: Triage

```bash
# Get recommendations
bv --robot-triage

# Pick top item
bv --robot-next

# Claim it
br update BD-123 --status in_progress
```

### During Work: Track Progress

```bash
# See where we are
bv --robot-plan --label core

# Update as you finish
br close BD-123
```

### End of Day: Health Check

```bash
# Any blockers?
bv --robot-alerts

# Cycles introduced?
bv --robot-insights | jq '.Cycles'

# Health by area?
bv --robot-label-health
```

## Label Standards

Use consistent labels:

```
epic        - Large feature (multiple issues)
feature     - New functionality
bug         - Something broken
chore       - Maintenance, refactoring, tooling
p0, p1, p2  - Priority (0=highest)
core        - Core functionality
testing     - Test-related
docs        - Documentation
```

## Dependency Management

### Creating Dependencies

Good reasons to link issues:
- "BD-124 can't start until BD-123 is done"
- "BD-124 is a subtask of BD-123"
- "BD-124 requires output from BD-123"

```bash
br dep add BD-123 BD-124  # BD-123 blocks BD-124
```

### Checking for Cycles

```bash
bv --robot-insights | jq '.Cycles | length'

# If > 0, you have circular dependencies
# Break them before continuing
```

### Critical Path

```bash
bv --robot-insights | jq '.CriticalPath'  # Longest dependency chain
```

## Graph Metrics Explained

| Metric | Meaning | Use |
|--------|---------|-----|
| PageRank | Importance in graph | Higher = more critical |
| Betweenness | Bottleneck potential | High = unblock many tasks |
| Critical Path | Min time to done | Shows deadline pressure |
| Cycles | Circular deps | Must eliminate |
| Eigenvector | Authority/hubness | Like PageRank but iterative |
| K-core | Dense subgroups | Tightly coupled work |

## Best Practices

1. **Create issues early** - Don't wait until starting work
2. **Link dependencies** - Especially blockers
3. **Use labels** - For grouping and filtering
4. **Run `bv --robot-triage` daily** - Catch issues early
5. **Break cycles immediately** - Never ignore circular deps
6. **Estimate (optional)** - For forecast accuracy

## Common Workflows

### Feature Development

```bash
# Create epic
br create "Epic: feature X" --labels epic --priority high

# Break into tasks
br create "Feature: part 1" --labels feature --priority high
br create "Feature: part 2" --labels feature --priority high
br create "Tests: feature X" --labels testing

# Link to epic
br dep add BD-epic BD-part1
br dep add BD-epic BD-part2
br dep add BD-epic BD-tests

# Triage
bv --robot-plan

# Work
br update BD-part1 --status in_progress
# ... implement ...
br close BD-part1
```

### Bug Triage

```bash
# Report bug
br create "Bug: X fails" --labels bug --priority high

# Find impact
bv --robot-insights | jq '.PageRank[] | select(.id == "BD-123")'

# Estimate effort
br update BD-123 --status in_progress
# ... investigate ...
br update BD-123 --status ready  # Ready for review
```

## Integration with Development

1. **Create issue** → `br create ...`
2. **Claim issue** → `br update BD-123 --status in_progress`
3. **Make branch** → `jj bookmark set feature/...` (implicit in Isolate)
4. **Work** → Edit files, commit with `jj describe`
5. **Push** → `jj git push`
6. **Close** → `br close BD-123`

All connected through Beads dependency graph and tracked by `bv`.

---

**Next**: [Version Control with Jujutsu](09_JUJUTSU.md)
