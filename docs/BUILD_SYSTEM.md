# Build System

Comprehensive guide for building, testing, and deploying Hardline using Moon, Git, and Beads.

## Philosophy

The Hardline build system is built on three core principles:

1. **Cache** - Skip unchanged work
2. **Parallelize** - Run independent tasks simultaneously
3. **Consistency** - Same commands work locally and in CI

Everything flows through these tools:
- **Beads**: Organization (what to work on)
- **Git**: Implementation (tracking changes)
- **Moon**: Validation (building & testing)

---

## Moon Build System

Moon is the build orchestration tool. It caches tasks, parallelizes execution, and ensures consistency.

### The Rule

**ALWAYS use Moon. NEVER use cargo directly.**

```bash
✅ moon run :ci      # Correct
✅ moon run :test    # Correct
❌ cargo build       # Wrong
❌ cargo test        # Wrong
```

### Common Commands

```bash
# Full CI pipeline
moon run :ci

# Fast lint only
moon run :quick

# Tests only
moon run :test

# Build release binaries
moon run :build

# Deploy (CI + Windmill push)
moon run :deploy
```

### What Each Command Does

#### `moon run :ci` (Full Pipeline)

```
fmt check       ← Code formatting check
clippy          ← Lints (-D warnings)
validate        ← YAML validation
test            ← Tests (parallel with nextest)
build           ← Release build
copy            ← Copy binaries to bin/
```

**Duration**: ~60-120 seconds (first run slower due to compilation)

#### `moon run :quick` (Fast Check)

```
fmt check       ← Code formatting
clippy          ← Lints
```

**Duration**: ~10-15 seconds

**Use**: Before pushing or committing

#### `moon run :test`

```
test            ← Run all tests (parallel)
```

**Duration**: ~30-45 seconds

#### `moon run :build`

```
build           ← Release build
copy            ← Copy binaries
```

**Duration**: ~45-90 seconds

### Caching & Speed

Moon caches based on **input fingerprints**:
- First build: **slow** (full compilation)
- After change to a file: **fast** (only recompile affected)
- No changes: **instant** (cached)

**Speedups via**:
- mold (fast linker)
- sccache (compiler cache)
- nextest (parallel test runner)
- incremental builds

### Performance Metrics

| Command | First Run | Cached |
|---------|-----------|--------|
| `:quick` | 15s | 5s |
| `:test` | 45s | 25s |
| `:build` | 90s | 45s |
| `:ci` | 120s | 60s |

**Optimization Tips**:
- Change minimal files → faster rebuild
- Use `:quick` for frequent checks
- Run `:test` before `:build`
- Let caching work (reuse outputs)

### Project Structure

```
.moon/
├── workspace.yml       # Workspace config
├── toolchain.yml       # Rust version (nightly)
└── bin/                # Moon binaries

moon.yml                 # Task definitions
Cargo.toml               # Workspace root
rust-toolchain.toml      # Rust version
```

### Build Profiles

#### Release (moon run :build)

- Optimization level: 3 (maximum)
- Debug info: stripped
- Panic: abort (smaller binary)
- LTO: enabled
- Code gen units: 1

#### Debug (development)

- Optimization level: 0
- Debug info: included
- Panic: unwind
- Incremental: enabled

#### Test

- Optimization level: 1
- Debug assertions: enabled
- Panic: unwind

### Binaries

All binaries in `crates/*/src/bin/` are built to `target/release/`.

### Exit Codes

- `0` - All tasks passed
- `1` - At least one task failed

---

## Daily Workflow

Integration of issue tracking (Beads), version control (Git), and build system (Moon).

### Full Workflow

#### 1. Start Work

```bash
# View available issues
br list

# Claim issue
br update BD-123 --status in_progress

# Pull latest
git fetch --all
```

#### 2. Make Changes

```bash
# Edit files (automatically tracked by git)
vim crates/hardline-core/src/lib.rs

# Check status
git status
git diff

# Test locally
moon run :test
```

#### 3. Commit Changes

```bash
# Describe change (conventional commits)
git add .
# git commit -m"feat: add new feature

- Implementation detail 1
- Implementation detail 2

Closes BD-123"

# Start next change
# git checkout -b new-feature
```

#### 4. Push to Remote

```bash
# Fetch latest
git fetch --all

# Push
git push

# Verify
git log -r @
```

#### 5. Close Issue

```bash
# Mark complete
br close BD-123

# Or mark ready for review
br update BD-123 --status ready
```

### Typical Day

#### Morning

```bash
# Check latest
git fetch --all

# See available work
br list

# Pick an issue
br update BD-123 --status in_progress
```

#### During Work

```bash
# Iterate
vim file.rs
moon run :test
# Fix any issues
vim file.rs
moon run :test
```

#### Ready to Commit

```bash
# Final validation
moon run :ci

# Commit with message
git add .
# git commit -m"feat: implement feature

- Detail 1
- Detail 2"

# Start next
# git checkout -b new-feature
```

#### End of Day

```bash
# Push all changes
git push

# Close completed issues
br close BD-123
br close BD-124

# Review what you're working on
br show BD-125
```

### Before Committing

```bash
# Quick lint
moon run :quick

# If changes to logic
moon run :test

# If satisfied, commit
git add .
# git commit -m"feat: description"
```

### Before Pushing

```bash
# Full validation
moon run :ci

# If all pass
git push

# If any fail, fix and retry
moon run :ci
```

---

## Beads (Issue Tracking)

### Creating Issues

```bash
# Feature
br create "Feature: X" --priority high --labels feature

# Bug
br create "Bug: X fails on Y" --priority high --labels bug \
  --description "Steps: 1. Do X 2. See Y"

# Chore
br create "Chore: refactor X" --labels chore
```

### Managing Issues

```bash
br list                           # Show all open
br list --filter "assigned:me"    # My issues
br update BD-123 --status in_progress  # Start working
br update BD-123 --status ready    # Mark ready for review
br close BD-123                   # Mark done
br update BD-123 --status open    # Reopen
```

### Labels

```
epic       - Large feature
feature    - New functionality
bug        - Something broken
chore      - Maintenance, refactoring
p0, p1, p2 - Priority (0=highest)
```

---

## Git (Version Control)

### Status & Diff

```bash
git status           # Current state
git diff             # Changes in working copy
git log              # Commit history
git log -r @         # Current change
```

### Commits

```bash
# Set commit message
git add .
# git commit -m"feat: description"

# View full message
git log -1

# Edit message
git commit --amend

# Start new change
# git checkout -b new-feature
```

### Conventional Commits

```
feat: New feature
fix: Bug fix
refactor: Code refactoring
chore: Build, dependencies, tooling
docs: Documentation changes
test: Test additions/modifications
perf: Performance improvements
```

### Working with Remotes

```bash
git fetch --all        # Fetch latest
git push                        # Push changes
git log -r origin/main..@           # Commits not yet pushed
```

### Syncing Workspaces

`hardline sync` rebases your workspace onto main, keeping your work up to date with the latest changes.

```bash
# Sync current workspace with main
hardline sync

# Sync specific workspace
hardline sync feature-auth

# Sync all workspaces
hardline sync --all
```

### Handling Conflicts

```bash
# Fetch latest
git fetch --all

# View conflicts
git diff

# Edit conflicted file
vim conflicted_file.rs

# Verify resolution
git diff  # Should show no conflicts

# Commit resolution
git add .
# git commit -m"merge: resolve conflicts"
git push
```

### Common Patterns

#### Feature Branch

```bash
# Create feature bookmark
git checkout -b feature/cool-thing

# Make changes on current commit
git add .
# git commit -m"feat: cool thing"
# git checkout -b new-feature

# Switch back to main
git checkout -b main
```

#### Stashing (Temporal Commits)

```bash
# Save work in progress
git add .
# git commit -m"wip: work in progress"

# Continue elsewhere
# git checkout -b new-feature

# Come back to WIP later
git log
git checkout <wip-commit>
```

#### Squashing Multiple Commits

```bash
# Make several commits
git add .
# git commit -m"feat: part 1"
# git checkout -b new-feature
git add .
# git commit -m"feat: part 2"
# git checkout -b new-feature

# Squash into parent (now just one commit)
git rebase -i
```

### Tips & Tricks

```bash
# See what changed since last push
git log -r origin/main..@

# Abandon unwanted changes
git reset <revision>

# Revert a change
git revert <revision>

# Move changes between commits
git cherry-pick <source> <destination>
```

### Landing (Finishing Session)

```bash
# 1. Run full pipeline
moon run :ci

# 2. File remaining work
br create "Follow-up: X" --labels chore

# 3. Commit final changes
git add .
# git commit -m"chore: final cleanup"
# git checkout -b new-feature

# 4. Update Beads
br close BD-123
br close BD-124

# 5. Push everything
git fetch --all
git push

# 6. Verify push
git log -r @
```

---

## Rollout and Rollback

Safe rollout plan for Hardline changes with deterministic rollback paths.

### Goals

- Ship changes in small, reversible increments.
- Detect regressions early with explicit gates.
- Provide a deterministic rollback path at every step.

### Pre-Rollout Checklist

Run and record these checks before any rollout step:

```bash
moon run :quick
moon run :test
moon run :ci
hardline --version
```

Confirm:
- No local panic/unwrap regressions.
- CI green on the release commit.
- Release owner and rollback owner assigned.

### Phased Rollout Plan

#### Phase 0 - Dry Run in Local/CI

**Actions**:
- Run the exact release candidate commit in CI and one local operator environment.
- Verify critical commands with JSON output paths used by automation.

**Success criteria**:
- CI green (`:ci`).
- No command contract drift in manual smoke commands.

**Rollback**:
- Stop promotion.
- Fix on current branch.
- Re-run Phase 0.

#### Phase 1 - Limited Operator Rollout

**Actions**:
- Deploy to a small set of operators/agents.
- Monitor command failures and system health.

**Suggested checks**:
```bash
swarm status
swarm monitor --view failures
```

**Success criteria**:
- Error rate stays at baseline.

**Rollback**:
- Revert to previous tagged commit.
- Restart affected operator sessions.
- Validate state health after downgrade.

#### Phase 2 - Broad Rollout

**Actions**:
- Promote to all operators after stable Phase 1 window.
- Continue monitoring at fixed intervals during the first hour.

**Success criteria**:
- No P0/P1 incidents.
- Normal throughput and agent completion behavior.

**Rollback**:
- Perform version rollback to last known-good build.
- Freeze new deployments until root cause is documented.

### Rollback Procedure (Operational)

1. Identify last known-good commit/tag.
2. Deploy that artifact to all affected runners.
3. Verify health with:
```bash
hardline --version
swarm status
```
4. Confirm critical command paths used by automation.
5. Open remediation bead with incident notes and reproduction details.

### Failure Triggers That Require Immediate Rollback

- Command contract breaks (`--json` structure drift, non-deterministic exits).
- Lock contention spikes.
- Swarm agent failure cascade.
- Data corruption indicators in workspace/session state.

### Communications Template

Use this format in team updates:

```
Change: <commit/tag>
Phase: <0|1|2>
Start: <timestamp>
Health: <green|yellow|red>
Decision: <continue|pause|rollback>
Owner: <name>
```

### Post-Rollout Closeout

- Record metrics and incidents.
- Update `docs/ERROR_TROUBLESHOOTING.md` and failure taxonomy if new failure classes were observed.
- Close rollout bead only after a stable monitoring window.

---

## Troubleshooting

### Moon

#### "moon not found"

```bash
# Check if in PATH
which moon

# Or use brew
brew install moonrepo/tools/moon
```

#### "sccache not found"

```bash
# Install via mise
mise install

# Or manually
cargo install sccache
```

#### "Task failed"

```bash
# Run with debug logging
moon run :ci --log debug

# Run single task
moon run :test

# Check task definition
cat moon.yml
```

#### "Cache not working"

```bash
# View task definition
moon dump :ci

# Check last run
ls -la ~/.moon/cache
```

### Git

#### "Can't push"

```bash
# Fetch first
git fetch --all

# Then push
git push
```

#### "Wrong commit message"

```bash
git commit --amend  # Opens editor
git push     # Push corrected
```

#### "Commit not found"

Use `git log` to find commit hash, then use hash instead of shorthand.

### All Tools

```bash
# Fix formatting
cargo fmt

# Re-run tests
moon run :test

# Check lint errors
moon run :quick --log debug
```

---

## Configuration

### `.moon/workspace.yml`

```yaml
workspace:
  version: "1.20"
  generator:
    templates:
      - .moon/templates
```

### `.moon/toolchain.yml`

```yaml
rustup:
  version: "nightly"
```

### `moon.yml`

Task definitions. Never edit directly unless you know what you're doing.

---

## Advanced

### Run specific task

```bash
moon run :test --scope hardline-core
```

### Watch mode (experimental)

```bash
moon watch :test
```

### Dry run

```bash
moon run :test --dry-run
```

### CI Integration

Moon runs in CI with:

```bash
moon ci :build
```

This uses cached outputs when available and checks all dependencies.
