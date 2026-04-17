# ADR-012: Stacked PRs - Branch Stack Operations

**Date:** 2026-03-20  
**Status:** Accepted  
**Deciders:** Lewis

---

## Context

AI agents often work on multi-step changes where:

1. **Dependent changes** - PR A must merge before PR B
2. **Stacked PRs** - Series of PRs that must be merged in order
3. **Restack on main advance** - When main moves, all PRs need rebasing
4. **Draf/Published state** - Draft PRs before review, publish when ready

The architecture spec defines `StackBranch` for this. This ADR formalizes the stack operations.

---

## Decision

### Stack Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: StackId,
    pub name: String,
    pub base_branch: BranchName,
    pub branches: Vec<StackBranch>,
    pub state: StackState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackBranch {
    pub branch_name: BranchName,
    pub position: u32,          // 0 = base, 1 = first PR, etc.
    pub pr_info: Option<PrInfo>,
    pub state: BranchState,
    pub last_commit: CommitHash,
    pub parent_branch: Option<BranchName>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackState {
    Draft,      // Stack is draft, PRs not published
    Published,  // All PRs published
    Merging,    // Stack is being merged
    Merged,     // All PRs merged
    Conflict,   // Merge conflict detected
    Failed,     // Merge failed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    Open,       // PR is open
    Draft,      // PR is draft
    Approved,   // PR approved, ready to merge
    Merging,    // Merge in progress
    Merged,     // PR merged
    Closed,     // PR closed
    Conflict,   // Has conflicts
}
```

### PR Info

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub pr_number: u32,
    pub url: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub state: PrState,
    pub draft: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrStatus {
    pub pr_number: u32,
    pub state: PrState,
    pub checks_passed: bool,
    pub reviews_approved: Vec<String>,
    pub mergeable: bool,
    pub conflict_resolution: Option<ConflictResolution>,
}
```

### Stack Operations

```rust
pub struct StackService<R: StackRepository, G: GitHubClient> {
    stack_repo: R,
    github: G,
}

impl<R: StackRepository, G: GitHubClient> StackService<R, G> {
    /// Create a new stack from a branch
    pub fn create_stack(
        &self,
        base_branch: BranchName,
        head_branch: BranchName,
        name: String,
    ) -> Result<Stack, StackError> {
        // 1. Build stack from branch ancestry
        let branches = self.build_stack_tree(base_branch, head_branch)?;
        
        // 2. Create stack entity
        let stack = Stack {
            id: StackId::new(),
            name,
            base_branch,
            branches,
            state: StackState::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // 3. Save to repository
        self.stack_repo.save(&stack)?;
        
        Ok(stack)
    }
    
    /// Push stack to GitHub (creates/updates PRs)
    pub fn publish_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self.stack_repo.find_by_id(stack_id)?
            .ok_or(StackError::StackNotFound(stack_id))?;
        
        for branch in &stack.branches {
            // Create or update PR
            let pr_info = self.github.create_or_update_pr(branch)?;
            branch.pr_info = Some(pr_info);
        }
        
        stack.state = StackState::Published;
        self.stack_repo.save(&stack)?;
        
        Ok(stack)
    }
    
    /// Restack: Rebase all branches onto updated base
    pub fn restack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self.stack_repo.find_by_id(stack_id)?
            .ok_or(StackError::StackNotFound(stack_id))?;
        
        // 1. Fetch latest from remote
        self.github.fetch(&stack.base_branch)?;
        
        // 2. For each branch in order (bottom to top)
        for (i, branch) in stack.branches.iter_mut().enumerate() {
            let parent = if i == 0 {
                stack.base_branch.clone()
            } else {
                stack.branches[i - 1].branch_name.clone()
            };
            
            // Rebase branch onto parent
            self.vcs.rebase(&branch.branch_name, &parent)?;
            
            // Force push to update PR
            self.github.force_push(&branch.branch_name)?;
        }
        
        stack.updated_at = Utc::now();
        self.stack_repo.save(&stack)?;
        
        Ok(stack)
    }
    
    /// Merge stack in order (bottom to top)
    pub fn merge_stack(&self, stack_id: StackId) -> Result<Stack, StackError> {
        let mut stack = self.stack_repo.find_by_id(stack_id)?
            .ok_or(StackError::StackNotFound(stack_id))?;
        
        stack.state = StackState::Merging;
        self.stack_repo.save(&stack)?;
        
        // Merge each branch in order
        for branch in &stack.branches {
            // Wait for CI checks
            self.wait_for_checks(&branch.pr_info)?;
            
            // Merge PR
            self.github.merge(&branch.pr_info)?;
            
            branch.state = BranchState::Merged;
        }
        
        stack.state = StackState::Merged;
        self.stack_repo.save(&stack)?;
        
        Ok(stack)
    }
}
```

### Stack Tree Building

```rust
/// Build stack tree from git branch ancestry
fn build_stack_tree(
    &self,
    base: BranchName,
    head: BranchName,
) -> Result<Vec<StackBranch>, StackError> {
    let mut branches = Vec::new();
    let mut current = head;
    
    // Walk up the ancestry chain
    loop {
        let commit = self.vcs.get_current_commit(&current)?;
        let parent = self.vcs.get_parent_commit(&current)?;
        
        branches.push(StackBranch {
            branch_name: current.clone(),
            position: branches.len() as u32,
            pr_info: None,
            state: BranchState::Open,
            last_commit: commit,
            parent_branch: if branches.is_empty() { None } else { Some(current) },
        });
        
        if current == base {
            break;
        }
        
        current = parent;
    }
    
    // Reverse to get base-first order
    branches.reverse();
    
    // Assign positions
    for (i, branch) in branches.iter_mut().enumerate() {
        branch.position = i as u32;
    }
    
    Ok(branches)
}
```

### Repository Trait

```rust
pub trait StackRepository: Send + Sync {
    fn save(&self, stack: &Stack) -> Result<(), StackRepoError>;
    fn find_by_id(&self, id: &StackId) -> Result<Option<Stack>, StackRepoError>;
    fn find_by_branch(&self, branch: &BranchName) -> Result<Option<Stack>, StackRepoError>;
    fn find_by_pr(&self, pr_number: u32) -> Result<Option<Stack>, StackRepoError>;
    fn list_all(&self) -> Result<Vec<Stack>, StackRepoError>;
    fn list_by_state(&self, state: StackState) -> Result<Vec<Stack>, StackRepoError>;
    fn delete(&self, id: &StackId) -> Result<(), StackRepoError>;
}
```

---

## Variants

### Variant A: GitHub-Only Stacks (CHOSEN)

```rust
struct Stack {
    branches: Vec<GitHubBranch>,
}
```

**Chosen because:**
- Simpler - no local state
- GitHub is source of truth
- Works with existing GitHub API

### Variant B: Local + Remote Stack State

**Rejected because:**
- Dual source of truth problems
- Need to sync local/remote
- Complexity not justified

### Variant C: Dedicated Stack Branch

```rust
// Special "stack" branch that encodes all PRs
```

**Rejected because:**
- Non-standard GitHub pattern
- Confusing for reviewers

---

## Invariants

### Stack Structure Invariants

```rust
/// INVARIANT: Stack branches are ordered base-first
fn assert_branch_order(stack: &Stack) {
    for window in stack.branches.windows(2) {
        let (lower, higher) = (&window[0], &window[1]);
        assert!(lower.position < higher.position);
        assert_eq!(higher.parent_branch, Some(lower.branch_name.clone()));
    }
}

/// INVARIANT: Base branch is not in stack branches
fn assert_base_not_in_stack(stack: &Stack) {
    assert!(!stack.branches.iter().any(|b| b.branch_name == stack.base_branch));
}

/// INVARIANT: Branch names are unique within stack
fn assert_unique_branch_names(stack: &Stack) {
    let names: HashSet<_> = stack.branches.iter().map(|b| &b.branch_name).collect();
    assert_eq!(names.len(), stack.branches.len());
}
```

### Stack State Invariants

```rust
/// INVARIANT: Draft stack has no published PRs
fn assert_draft_stack_no_prs(stack: &Stack) {
    if stack.state == StackState::Draft {
        assert!(stack.branches.iter().all(|b| b.pr_info.is_none()));
    }
}

/// INVARIANT: Published stack has all PRs
fn assert_published_stack_has_prs(stack: &Stack) {
    if stack.state == StackState::Published {
        assert!(stack.branches.iter().all(|b| b.pr_info.is_some()));
    }
}

/// INVARIANT: Merged stack has all branches in Merged state
fn assert_merged_stack_all_merged(stack: &Stack) {
    if stack.state == StackState::Merged {
        assert!(stack.branches.iter().all(|b| b.state == BranchState::Merged));
    }
}
```

### Branch State Invariants

```rust
/// INVARIANT: Branch position is monotonically increasing
fn assert_position_monotonic(branch: &StackBranch) {
    assert!(branch.position >= 0);
}

/// INVARIANT: PR info matches branch state
fn assert_pr_info_matches_state(branch: &StackBranch) {
    match branch.state {
        BranchState::Draft => {
            assert!(branch.pr_info.as_ref().map_or(false, |p| p.draft));
        }
        BranchState::Open | BranchState::Approved => {
            assert!(branch.pr_info.is_some());
        }
        BranchState::Merged => {
            assert!(branch.pr_info.as_ref().map_or(false, |p| p.state == PrState::Merged));
        }
        _ => {}
    }
}
```

### Cycle Detection Invariants

```rust
/// INVARIANT: Stack has no cycles in parent relationships
fn assert_no_parent_cycles(stack: &Stack) -> Result<(), StackError> {
    let mut visited = HashSet::new();
    let mut stack_set = HashSet::new();
    
    for branch in &stack.branches {
        if !stack_set.insert(branch.branch_name.clone()) {
            return Err(StackError::CyclicDependency(branch.branch_name.clone()));
        }
    }
    
    Ok(())
}
```

---

## Consequences

### Positive

1. **Ordered merging** - Dependencies respected
2. **Restack automation** - Rebase all on main advance
3. **Draft publishing** - Easy draft → published transition
4. **GitHub integration** - Uses GitHub API directly

### Negative

1. **GitHub coupling** - Not platform agnostic
2. **CI wait time** - Must wait for checks per PR
3. **Stack depth limits** - GitHub has PR stack limits

### CLI Commands

```bash
hardline stack create <base-branch> <head-branch> <name>
hardline stack list
hardline stack status <stack-id>
hardline stack publish <stack-id>
hardline stack restack <stack-id>
hardline stack merge <stack-id>
hardline stack close <stack-id>
hardline stack add <stack-id> <branch>   # Add branch to stack
hardline stack remove <stack-id> <branch> # Remove from stack
```

### Files to Create/Modify

| File | Change |
|-------|--------|
| `crates/stack/src/domain/stack.rs` | Stack, StackBranch entities |
| `crates/stack/src/domain/state.rs` | StackState, BranchState |
| `crates/stack/src/infrastructure/github.rs` | GitHub API client |
| `crates/stack/src/application/service.rs` | Stack operations |

---

## Related ADRs

- ADR-001: CLI Architecture (stack commands)
- ADR-004: VCS Abstraction (branch operations)
- ADR-008: Queue Processing (stack merge order)
