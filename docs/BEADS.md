# BEADS: Issue Tracking & Specification

Comprehensive guide to the Beads issue tracking system, covering both day-to-day usage and high-quality specification creation.

---

## Part 1: Core Concept & Quick Start

Beads stores issues in `.beads/beads.jsonl`. Each bead (issue) is a node in a dependency graph. Use `br` for basic operations and `bv` (Beads triage engine) to understand scope, dependencies, and prioritization.

### Quick Commands

```bash
br create "Feature: implement X"           # Create issue
br list                                     # List all open
br update BD-123 --status in_progress      # Start working
br close BD-123                            # Mark done
bv --robot-triage                           # Get AI-powered recommendations
```

---

## Part 2: Creating Issues

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

---

## Part 3: Managing Issues

### List & Filter

```bash
br list                              # All open issues
br list --filter "status:open"       # Only open
br list --filter "assigned:me"       # My issues
br list --filter "label:feature"     # By label
br list --filter "priority:high"    # By priority
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

---

## Part 4: Using `bv` for Triage

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

---

## Part 5: Filtering with Recipes

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

---

## Part 6: Understanding Output

Every `bv` response includes:
- `data_hash` - Fingerprint of beads.jsonl (verify consistency)
- `status` - Metric readiness: `computed|approx|timeout|skipped`
- `as_of_commit` - When using `--as-of`

### Two Phases of Analysis

**Phase 1 (instant)**: degree, topo sort, density
**Phase 2 (async, 500ms timeout)**: PageRank, betweenness, HITS, eigenvector, cycles

For large graphs (>500 nodes), some metrics may be approximated. Always check `status`.

---

## Part 7: jq Cheatsheet

```bash
bv --robot-triage | jq '.quick_ref'                     # Summary
bv --robot-triage | jq '.recommendations[0]'            # Top pick
bv --robot-plan | jq '.plan.summary.highest_impact'     # Best unblock target
bv --robot-insights | jq '.Cycles'                      # Circular deps
bv --robot-insights | jq '.status'                      # Metric status
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

---

## Part 8: Workflow Integration

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

---

## Part 9: Label Standards

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

---

## Part 10: Dependency Management

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

---

## Part 11: Graph Metrics Explained

| Metric | Meaning | Use |
|--------|---------|-----|
| PageRank | Importance in graph | Higher = more critical |
| Betweenness | Bottleneck potential | High = unblock many tasks |
| Critical Path | Min time to done | Shows deadline pressure |
| Cycles | Circular deps | Must eliminate |
| Eigenvector | Authority/hubness | Like PageRank but iterative |
| K-core | Dense subgroups | Tightly coupled work |

---

## Part 12: High-Quality Bead Specifications

> **If GPT-4 or a competent high school senior cannot implement this ticket perfectly on their first attempt, the ticket is incomplete.**

Every bead must be rigorously specified so implementation becomes mechanical. The AI has no choice but to succeed because every edge case, failure mode, and test is explicitly enumerated.

### The 16-Section Bead Template

| Section | Purpose | Must Answer |
|---------|---------|-------------|
| **0. Clarifications** | Anti-assumption gate | All ambiguities resolved? |
| **1. EARS** | What must happen | All 6 patterns covered? |
| **2. KIRK Contracts** | Pre/post/invariants | What's guaranteed? |
| **2.5 Research** | What to investigate | Files/patterns to read? |
| **3. Inversions** | What could go wrong | Every failure mode? |
| **4. ATDD Tests** | Acceptance criteria | Tests before code? |
| **5. E2E Tests** | Full pipeline proof | Real data, no mocks? |
| **5.5 Verification** | Quality gates | All gates defined? |
| **6. Task List** | Implementation steps | Parallel/sequential marked? |
| **7. Failure Modes** | Debugging guide | Where to look? |
| **7.5 Anti-Hallucination** | Ground truth rules | Read-before-write enforced? |
| **7.6 Context Survival** | Long-running support | Progress files defined? |
| **8. Completion** | Definition of done | All boxes checked? |
| **9. Context** | Background info | Related files? |
| **10. AI Hints** | Claude 4.x guidance | Constitution clear? |

### Quick Template

```yaml
# ============================================================================
# BEAD: [ID] - [Title]
# ============================================================================

id: "intent-cli-XXXX"
title: "[Component]: [Action verb] [specific thing]"
type: feature | bug | task | epic | chore
priority: 0 (critical) | 1 (high) | 2 (medium) | 3 (low) | 4 (backlog)
effort_estimate: "15min | 30min | 1hr | 2hr | 4hr"  # Max 4hr per bead
labels: [component, category, methodology]

# SECTION 0: CLARIFICATION MARKERS
clarification_status: "RESOLVED" | "HAS_OPEN_QUESTIONS"
open_clarifications:
  - question: "[NEEDS CLARIFICATION: specific question]"
    context: "[Why this matters]"

# SECTION 1: EARS REQUIREMENTS
ears_requirements:
  event_driven:
    - trigger: "WHEN [specific user action]"
      shall: "THE SYSTEM SHALL [specific response]"

# SECTION 2: KIRK CONTRACTS
contracts:
  preconditions:
    required_inputs:
      - field: "field_name"
        type: "String | Int | Bool"
  postconditions:
    return_guarantees:
      - field: "exit_code"
        guarantee: "0 on success, 3 on invalid input"

# SECTION 3: INVERSION ANALYSIS
inversions:
  security_failures:
    - failure: "[Security vulnerability]"
      prevention: "[How to prevent]"

# SECTION 4: ATDD ACCEPTANCE TESTS
acceptance_tests:
  happy_paths:
    - name: "test_[descriptive_name]"
      given: "[Precondition]"
      when: "[Action]"
      then: "[Expected result]"

# SECTION 5: E2E TESTS
e2e_tests:
  pipeline_test:
    name: "test_full_pipeline"
    setup: ...
    execute: ...
    verify: ...
    cleanup: ...

# SECTION 6: IMPLEMENTATION TASKS
implementation_tasks:
  phase_1_tests_first:
    - task: "Write test: test_[name]"
      done_when: "Test exists and FAILS"
  phase_2_implementation:
    - task: "Implement [function]"
      done_when: "Tests pass"

# SECTION 7: FAILURE MODES
failure_modes:
  - symptom: "[Problem]"
    likely_cause: "[Cause]"
    where_to_look:
      - file: "[path]"
        function: "[name]"

# SECTION 8: COMPLETION CRITERIA
completion_checklist:
  tests:
    - "[ ] All acceptance tests passing"
  code:
    - "[ ] Zero unwrap() calls"
  ci:
    - "[ ] moon run :ci passes"
```

### EARS Patterns Reference

Every requirement MUST use one of these 6 patterns:

1. **UBIQUITOUS** - Always true, no conditions
   - "THE SYSTEM SHALL [behavior that is always true]"

2. **EVENT-DRIVEN** - Trigger-response pairs
   - trigger: "WHEN [specific user action]"
     shall: "THE SYSTEM SHALL [specific response]"

3. **STATE-DRIVEN** - Behavior during specific states
   - state: "WHILE [system is in state X]"
     shall: "THE SYSTEM SHALL [behavior]"

4. **OPTIONAL** - Conditional on configuration/roles
   - condition: "WHERE [feature flag]"
     shall: "THE SYSTEM SHALL [conditional behavior]"

5. **UNWANTED** - Things that must NEVER happen
   - condition: "IF [bad state]"
     shall_not: "THE SYSTEM SHALL NOT [forbidden]"

6. **COMPLEX** - State + Event combinations
   - state: "WHILE [state]"
     trigger: "WHEN [event]"
     shall: "THE SYSTEM SHALL [combined behavior]"

---

## Part 13: Best Practices

1. **Create issues early** - Don't wait until starting work
2. **Link dependencies** - Especially blockers
3. **Use labels** - For grouping and filtering
4. **Run `bv --robot-triage` daily** - Catch issues early
5. **Break cycles immediately** - Never ignore circular deps
6. **Write detailed specifications** - Use the 16-section template for complex beads
7. **Keep beads small** - Max 4hr effort estimate
8. **Test first** - Write acceptance tests before implementation

---

## Part 14: Common Workflows

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

---

## Part 15: Integration with Development

1. **Create issue** → `br create ...`
2. **Claim issue** → `br update BD-123 --status in_progress`
3. **Make branch** → `jj bookmark set feature/...` (implicit in Isolate)
4. **Work** → Edit files, commit with `jj describe`
5. **Push** → `jj git push`
6. **Close** → `br close BD-123`

All connected through Beads dependency graph and tracked by `bv`.

---

## Part 16: The Litmus Tests

Before submitting a bead, verify:

### Specification Quality
1. **GPT-4 Test**: Could GPT-4 implement this without asking clarifying questions?
2. **High School Senior Test**: Could a competent CS student implement this?
3. **Clarification Test**: Are ALL ambiguities explicitly resolved?
4. **EARS Coverage Test**: Are all 6 EARS patterns considered?

### Test Quality
5. **Coverage Test**: Are there tests for the main code paths?
6. **No Mocks Test**: Are tests using real data where possible?
7. **E2E Test**: Is there a test proving the full pipeline works?

### Task Quality
8. **Task Size Test**: Is every task completable in under 4 hours?
9. **Parallelization Test**: Are parallel vs sequential tasks marked?

---

## Creating a Bead from Template

```bash
# 1. Copy template
cp .beads/BEAD_TEMPLATE.md /tmp/new-bead.md

# 2. Fill in all sections
# (This is the hard part - be thorough!)

# 3. Create the bead
bd create "Component: Action description" \
  -t feature \
  -p 2 \
  -d "$(cat /tmp/new-bead.md)"

# 4. Verify with viewer
bv --show <bead-id>
```

---

## Remember

> "Weeks of coding can save you hours of planning."

- **This BEADS.md**: Comprehensive guide merging usage + specification standards
- Ambiguous tickets create ambiguous implementations
- The time spent specifying a bead is always less than debugging a poorly-specified one

---

**Related**: [Version Control with Jujutsu](09_JUJUTSU.md)
