# Daily Workflow: Beads + Git + Moon

Integration of issue tracking, version control, and build system.

## Full Workflow

### 1. Start Work

```bash
# View available issues
br list

# Claim issue
br update BD-123 --status in_progress

# Pull latest
git pull --rebase
```

### 2. Make Changes

```bash
# Create a feature branch
git checkout -b feature/BD-123

# Edit files
vim crates/hardline-core/src/lib.rs

# Check status
git status
git diff

# Test locally
moon run :test
```

### 3. Commit Changes

```bash
# Stage and commit (conventional commits)
git add .
git commit -m "feat: add new feature

- Implementation detail 1
- Implementation detail 2

Closes BD-123"
```

### 4. Push to Remote

```bash
# Push feature branch
git push -u origin feature/BD-123

# Or push to main
git push
```

### 5. Close Issue

```bash
# Mark complete
br close BD-123

# Or mark ready for review
br update BD-123 --status ready
```

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

## Git (Version Control)

### Status & Diff

```bash
git status          # Current state
git diff            # Changes in working copy
git log --oneline   # Commit history
```

### Commits

```bash
# Stage and commit
git add .
git commit -m "feat: description"

# Amend last commit message (before push only)
git commit --amend -m "feat: better description"
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

### Example Commit

```bash
git commit -m "feat: add validation builder

- Implement ValidatorBuilder struct
- Add error types for validation
- Add comprehensive tests

Closes BD-42"
```

### Working with Remotes

```bash
git fetch origin                    # Fetch latest
git push                            # Push changes
git log origin/main..HEAD           # Commits not yet pushed
```

### Branching

```bash
# Create and switch to branch
git checkout -b feature/my-feature

# Switch back to main
git checkout main

# Merge feature branch
git merge feature/my-feature
```

## Moon (Build System)

### Before Committing

```bash
# Quick lint
moon run :quick

# If changes to logic
moon run :test
```

### Before Pushing

```bash
# Full validation
moon run :ci

# If all pass
git push
```

### Common Issues

```bash
# Fix formatting
moon run :fmt-fix

# Re-run tests
moon run :test

# Check lint errors
moon run :quick --log debug
```

## Typical Day

### Morning

```bash
# Check latest
git pull --rebase

# See available work
br list

# Pick an issue
br update BD-123 --status in_progress
```

### During Work

```bash
# Iterate
vim file.rs
moon run :test
# Fix any issues
vim file.rs
moon run :test
```

### Ready to Commit

```bash
# Final validation
moon run :ci

# Commit with message
git add .
git commit -m "feat: implement feature

- Detail 1
- Detail 2"
```

### End of Day

```bash
# Push all changes
git push

# Close completed issues
br close BD-123
br close BD-124

# Review what you're working on
br show BD-125
```

## Multi-Issue Workflow

```bash
# Claim first issue
br update BD-123 --status in_progress

# Make changes, commit
git add .
git commit -m "fix: issue 123"

# Claim second issue
br update BD-124 --status in_progress

# Make changes, commit
git add .
git commit -m "feat: issue 124"

# Push all
git push

# Close both
br close BD-123
br close BD-124
```

## Syncing Workspaces

`hardline sync` rebases your workspace onto main, keeping your work up to date with the latest changes.

### Basic Usage

```bash
# Sync current workspace with main
hardline sync

# Sync specific workspace
hardline sync feature-auth

# Sync all workspaces
hardline sync --all
```

### What It Does

1. Runs: `git pull --rebase`
2. Updates `last_synced` timestamp in session database
3. Shows summary of changes applied

### When to Sync

- **Before starting work**: Ensure you have latest changes
- **During long-running work**: Stay synchronized with team
- **Before creating PRs**: Ensure clean rebase onto main

### Example

```bash
$ hardline sync
Syncing workspace 'feature-auth' with main...
Rebasing workspace commits onto main
Summary: 3 commits rebased, 0 conflicts
Last synced: 2026-02-01 12:00:00
```

### Handling Conflicts During Sync

If conflicts occur during rebase:

```bash
# View conflicts
git diff --name-only --diff-filter=U

# Edit conflicted files
vim conflicted_file.rs

# Stage resolved files
git add conflicted_file.rs

# Continue rebase
git rebase --continue
```

## Handling Conflicts

### Update with Latest

```bash
git pull --rebase
```

### Resolving Conflicts

```bash
# Edit conflicted file
vim conflicted_file.rs

# Stage resolved file
git add conflicted_file.rs

# Continue rebase
git rebase --continue

# Or abort rebase
git rebase --abort
```

## Landing (Finishing Session)

```bash
# 1. Run full pipeline
moon run :ci

# 2. File remaining work
br create "Follow-up: X" --labels chore

# 3. Commit final changes
git add .
git commit -m "chore: final cleanup"

# 4. Update Beads
br close BD-123
br close BD-124

# 5. Push everything
git push

# 6. Verify push
git status
```

## Common Patterns

### Feature Branch

```bash
# Create feature branch
git checkout -b feature/cool-thing

# Make changes and commit
# ... changes ...
git add .
git commit -m "feat: cool thing"

# Push feature branch
git push -u origin feature/cool-thing

# Switch back to main
git checkout main

# Later, merge feature
git merge feature/cool-thing
```

### Stashing

```bash
# Save work in progress
git stash

# Continue elsewhere
git checkout -b other-work

# Come back to stashed work
git stash pop
```

### Squashing Multiple Commits

```bash
# Interactive rebase to squash commits (before push only)
git rebase -i HEAD~3
# Mark commits as 'squash' or 'fixup'
```

## Tips & Tricks

### See what changed since last push

```bash
git log origin/main..HEAD
```

### Abandon unwanted changes

```bash
git reset HEAD~1        # Undo last commit, keep changes
git reset --hard HEAD~1 # Undo last commit, discard changes
```

### Revert a change

```bash
git revert <commit-hash>
```

### Cherry-pick a commit

```bash
git cherry-pick <commit-hash>
```

## Troubleshooting

### "Commit not found"

Use `git log` to find the commit hash.

### "Can't push"

```bash
# Pull first
git pull --rebase

# Then push
git push
```

### "Changes not tracked"

```bash
git status  # Check status
git diff    # Show changes
```

### "Wrong commit message"

Amend before pushing:
```bash
git commit --amend -m "corrected message"
git push
```

## The Flow

1. **Beads**: Organization (what to work on)
2. **Git**: Implementation (tracking changes)
3. **Moon**: Validation (building & testing)
4. **Beads**: Closure (marking done)

Everything flows through these tools. Master them and you master Hardline development.

---

**Next**: [Functional Patterns](04_FUNCTIONAL_PATTERNS.md)
