# Contract Specification: Init Command (hl-98v)

## Context

- **Bead ID**: hl-98v
- **Feature**: Port CLI: init command
- **Source**: `~/src/hardline/crates/cli/src/commands/handlers/`
- **Target**: `~/src/hardline/crates/cli/src/commands/handlers/`
- **Domain**: Project initialization for Hardline workspaces

### Domain Terms

| Term | Meaning |
|------|---------|
| **Workspace** | A Git repository managed by hardline |
| **Init Lock** | Exclusive file lock preventing concurrent initialization |
| **Stale Lock** | Lock file with age > 60 seconds (strictly greater than), considered abandoned |
| **Git Repository** | Version control system using Git |
| **Hardline Directory** | `.hardline/` directory containing configuration and state |
| **Moon Pipeline** | Build system configuration (`.moon/`) |
| **Git Hooks** | Pre-commit enforcement preventing non-hardline commits |
| **Bead** | Issue/feature tracked via bd (beads) |

**Note on Stale Lock**: A lock is considered stale only when `age > 60` (strictly greater than).
- age = 59: NOT stale
- age = 60: NOT stale  
- age = 61: STALE

### Assumptions

1. The target project uses Moon as the build system
2. Git must be installed and available in PATH
3. The init command runs within an existing Git repository
4. All file operations use async Tokio runtime
5. Lock files prevent TOCTOU (time-of-check-time-of-use) race conditions
6. Symlink attacks on lock files must be prevented

### Open Questions

1. What is the exact config schema for `config.toml`? (currently CUE-based in source)
2. Should state.db use SQLite or a different format?
3. What is the expected error type hierarchy for the CLI crate?
4. Should the lock file be kept on disk or removed after release?

---

## Preconditions

### Global Preconditions

- [ ] **P1**: Current working directory must be a Git repository root
- [ ] **P2**: User must have write permissions to current directory
- [ ] **P3**: Required dependencies (git) must be installed and discoverable in PATH
- [ ] **P4**: If `.hardline/` directory exists, it must not be a symlink
- [ ] **P5**: Lock file path must not be a symlink (symlink attack prevention)

### Function-Specific Preconditions

#### `check_dependencies()`
- [ ] **P6**: None (always safe to call)

#### `ensure_git_repo_with_cwd(cwd)`
- [ ] **P7**: `cwd` must be a valid path that can be accessed
- [ ] **P8**: User must have read/write permissions in `cwd`

#### `git_root_with_cwd(cwd)`
- [ ] **P9**: `cwd` must be inside a Git repository
- [ ] **P10**: `git` command must be executable

#### `InitLock::acquire(lock_path)`
- [ ] **P11**: `lock_path` parent directory must exist and be writable
- [ ] **P12**: `lock_path` must not be a symlink

#### `create_gitignore(repo_root)`
- [ ] **P13**: `repo_root` must be a valid Git repository root

#### `create_git_hooks(repo_root)`
- [ ] **P14**: `repo_root` must be a valid Git repository root
- [ ] **P15**: User must have permissions to create `.git/hooks/` directory

#### `create_repo_ai_instructions(repo_root)`
- [ ] **P16**: `repo_root` must be a valid, writable directory

#### `create_moon_pipeline(repo_root)`
- [ ] **P17**: `repo_root` must be a valid, writable directory

#### `create_agents_md(repo_root)`
- [ ] **P18**: `repo_root` must be a valid, writable directory

#### `create_docs(repo_root)`
- [ ] **P19**: `repo_root` must be a valid, writable directory

#### `SessionDb::create_or_open(db_path)`
- [ ] **P20**: `db_path` parent directory must exist and be writable

---

## Postconditions

### Global Postconditions

- [ ] **POST1**: If initialization succeeds, `.hardline/` directory exists
- [ ] **POST2**: If initialization succeeds, `.hardline/config.toml` exists with valid TOML
- [ ] **POST3**: If initialization succeeds, `.hardline/layouts/` directory exists
- [ ] **POST4**: If initialization succeeds, `.hardline/state.db` database exists
- [ ] **POST5**: If initialization succeeds, `.gitignore` contains `.hardline/` pattern
- [ ] **POST6**: If initialization succeeds, `.git/hooks/pre-commit` exists and is executable
- [ ] **POST7**: If initialization succeeds, `.ai-instructions.md` exists
- [ ] **POST8**: If initialization succeeds, `.moon/workspace.yml` exists
- [ ] **POST9**: If initialization succeeds, `.moon/toolchain.yml` exists
- [ ] **POST10**: If initialization succeeds, `.moon/tasks.yml` exists
- [ ] **POST11**: If initialization succeeds, `docs/01_ERROR_HANDLING.md` exists
- [ ] **POST12**: If initialization succeeds, `docs/02_MOON_BUILD.md` exists
- [ ] **POST13**: If initialization succeeds, `docs/03_WORKFLOW.md` exists
- [ ] **POST14**: If initialization succeeds, `docs/05_RUST_STANDARDS.md` exists
- [ ] **POST15**: If initialization succeeds, `docs/08_BEADS.md` exists
- [ ] **POST16**: If initialization succeeds, `docs/09_GIT.md` exists
- [ ] **POST17**: If already initialized, no files are modified
- [ ] **POST18**: If dry_run mode, no files are created or modified

### Function-Specific Postconditions

#### `check_dependencies()`
- [ ] **POST19**: Returns `Ok(())` if all dependencies present
- [ ] **POST20**: Returns `Err(MissingDependencies)` if any dependency missing

#### `ensure_git_repo_with_cwd(cwd)`
- [ ] **POST21**: Returns `Ok(())` if Git repo exists or was created
- [ ] **POST22**: Returns `Err(GitInitFailed)` if Git initialization fails

#### `git_root_with_cwd(cwd)`
- [ ] **POST23**: Returns `Ok(PathBuf)` containing the Git repository root path

#### `InitLock::acquire(lock_path)`
- [ ] **POST24**: Returns `Ok(InitLock)` with exclusive lock held
- [ ] **POST25**: Returns `Err(SymlinkAttackDetected)` if lock_path is symlink
- [ ] **POST26**: Returns `Err(InitInProgress)` if lock cannot be acquired

#### `InitLock::release()`
- [ ] **POST27**: Releases exclusive lock but keeps file on disk
- [ ] **POST28**: Idempotent - safe to call multiple times

#### `create_gitignore(repo_root)`
- [ ] **POST29**: `.gitignore` contains `.hardline/` pattern
- [ ] **POST30**: Idempotent - safe to call multiple times

#### `create_git_hooks(repo_root)`
- [ ] **POST31**: `.git/hooks/pre-commit` contains valid shell script
- [ ] **POST32**: Pre-commit hook is executable (mode 0755)
- [ ] **POST33**: Pre-commit hook references `Isolate_ACTIVE` environment variable

#### `create_repo_ai_instructions(repo_root)`
- [ ] **POST34**: `.ai-instructions.md` exists with valid content

#### `create_agents_md(repo_root)`
- [ ] **POST35**: `AGENTS.md` exists with valid content

#### `create_claude_md(repo_root)`
- [ ] **POST36**: `CLAUDE.md` exists with valid content

#### `create_moon_pipeline(repo_root)`
- [ ] **POST37**: `.moon/workspace.yml` contains valid Moon schema
- [ ] **POST38**: `.moon/toolchain.yml` contains valid Moon schema
- [ ] **POST39**: `.moon/tasks.yml` contains valid Moon schema

#### `create_docs(repo_root)`
- [ ] **POST40**: All 6 documentation files exist in `docs/` directory

#### `SessionDb::create_or_open(db_path)`
- [ ] **POST41**: Database file exists and is accessible
- [ ] **POST42**: WAL mode enabled if supported

---

## Invariants

### Type Invariants

#### `InitLock`
- [ ] **INV1**: `released` flag accurately reflects lock state
- [ ] **INV2**: If `released == true`, lock is not held
- [ ] **INV3**: If `released == false`, lock is held (unless dropped)
- [ ] **INV4**: `Drop` implementation releases lock if not already released

#### `InitOptions`
- [ ] **INV5**: `format` is either `Json` or `Human`
- [ ] **INV6**: `dry_run` is boolean (true/false)

#### `InitResponse`
- [ ] **INV7**: `root` matches actual repository root
- [ ] **INV8**: `paths.data_directory` always equals `.hardline/`
- [ ] **INV9**: `paths.config` always equals `.hardline/config.toml`
- [ ] **INV10**: `paths.state_db` always equals `.hardline/state.db`
- [ ] **INV11**: `paths.layouts` always equals `.hardline/layouts/`
- [ ] **INV12**: `git_initialized` is always `true` (command ensures this)
- [ ] **INV13**: `already_initialized` accurately reflects initialization state

#### `InitPaths`
- [ ] **INV14**: All paths are relative to repository root
- [ ] **INV15**: All paths start with `.hardline/`

### Operational Invariants

#### Initialization Process
- [ ] **INV16**: Only one init process can run at a time (lock enforced)
- [ ] **INV17**: Stale locks (age > 60s, strictly greater than) are automatically removed
  - Locks with age == 60s are NOT removed (60 > 60 is false)
  - Locks with age == 61s ARE removed (61 > 60 is true)
- [ ] **INV18**: Lock file is never deleted after creation (inode-based locking)
- [ ] **INV19**: `.hardline/` directory is created before lock acquisition
- [ ] **INV20**: All file creation is idempotent (safe to retry)
- [ ] **INV21**: Git repository is initialized before `.hardline/` setup
- [ ] **INV22**: Config, layouts, and database are created in single transaction
- [ ] **INV23**: Hooks are only created if pre-commit doesn't exist

#### Error Handling
- [ ] **INV24**: All fallible operations return `Result<T, Error>`
- [ ] **INV25**: Errors are propagated with `?` operator (no unwraps in source)
- [ ] **INV26**: Symlink attacks are detected before lock acquisition
- [ ] **INV27**: All file operations use appropriate error context

#### Concurrency
- [ ] **INV28**: No TOCTOU vulnerabilities in file creation
- [ ] **INV29**: File locks prevent concurrent `.gitignore` modifications
- [ ] **INV30**: File locks prevent concurrent hook creation

---

## Error Taxonomy

```rust
/// Error types for the init command
#[derive(Debug)]
pub enum InitError {
    // Dependency Errors
    MissingDependencies {
        missing: Vec<String>,
    },
    
    // Git Repository Errors
    GitNotInstalled,
    GitCommandFailed {
        command: String,
        stderr: String,
    },
    GitRepoNotFound,
    GitInitFailed {
        stderr: String,
    },
    
    // File System Errors
    Io {
        source: std::io::Error,
        context: String,
    },
    PermissionDenied {
        path: std::path::PathBuf,
        operation: String,
    },
    
    // Lock Errors
    SymlinkAttackDetected {
        path: std::path::PathBuf,
    },
    LockNotAcquirable {
        path: std::path::PathBuf,
        message: String,
    },
    LockReleaseFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    LockTOCTOU {
        path: std::path::PathBuf,
        operation: String,
    },
    
    // File Creation Errors
    ConfigWriteFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    LayoutsCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    HooksCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    HooksPermissionsFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    GitIgnoreUpdateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    
    // Document Creation Errors
    AgentsMdCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    ClaudeMdCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    DocsCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    MoonPipelineCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    AiInstructionsCreateFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    
    // Database Errors
    DatabaseCreateFailed {
        path: std::path::PathBuf,
        source: anyhow::Error,
    },
    
    // Command Errors
    CurrentDirFailed,
    OutputFormatInvalid,
    JsonSerializationFailed {
        source: serde_json::Error,
    },
    
    // Contract Violation Errors
    PreconditionViolation {
        expected: String,
        actual: String,
        context: String,
    },
    InvariantViolated {
        invariant: String,
        context: String,
    },
    
    // General Errors
    Unknown {
        message: String,
    },
}
```

### Error Construction Guidelines

#### Dependency Errors
```rust
// Multiple missing dependencies
Err(InitError::MissingDependencies {
    missing: vec!["git".to_string()],
})

// With installation instructions
Err(InitError::MissingDependencies {
    missing: vec!["git".to_string()],
})
```

#### Git Repository Errors
```rust
// Git command failed
Err(InitError::GitCommandFailed {
    command: "git init".to_string(),
    stderr: "error: ...".to_string(),
})

// Git not in PATH
Err(InitError::GitNotInstalled)
```

#### Lock Errors
```rust
// Symlink attack detected
Err(InitError::SymlinkAttackDetected {
    path: PathBuf::from("/path/to/.init.lock"),
})

// Another init in progress
Err(InitError::LockNotAcquirable {
    path: PathBuf::from("/path/to/.init.lock"),
    message: "Another hardline init is in progress".to_string(),
})
```

#### File System Errors
```rust
// Generic I/O with context
Err(InitError::Io {
    source: std::io::Error::from_raw_os_error(13),
    context: "Failed to create config.toml".to_string(),
})

// Permission denied
Err(InitError::PermissionDenied {
    path: PathBuf::from("/path/to/file"),
    operation: "write".to_string(),
})
```

---

## Contract Signatures

### Main Entry Points

```rust
/// Options for the init command
#[derive(Debug, Clone, Copy, Default)]
pub struct InitOptions {
    /// Output format: JSON or human-readable
    pub format: OutputFormat,
    
    /// Dry run mode - don't create any files
    pub dry_run: bool,
}

/// Run init command with default options
pub async fn run() -> Result<(), InitError>;

/// Run init command with custom options
pub async fn run_with_options(options: InitOptions) -> Result<(), InitError>;

/// Run init command with custom working directory
pub async fn run_with_cwd_and_options(
    cwd: Option<&Path>,
    options: InitOptions,
) -> Result<(), InitError>;
```

### Dependency Checking

```rust
/// Check that required dependencies are installed
pub async fn check_dependencies() -> Result<(), InitError>;

/// Check if Git is installed
pub async fn is_git_installed() -> bool;
```

### Git Repository Management

```rust
/// Ensure Git repository exists, initialize if needed
pub async fn ensure_git_repo_with_cwd(
    cwd: &Path,
    json_mode: bool,
) -> Result<(), InitError>;

/// Get the Git repository root path
pub async fn git_root_with_cwd(
    cwd: &Path,
) -> Result<PathBuf, InitError>;

/// Check if current directory is a Git repository
pub async fn is_git_repo_with_cwd(
    cwd: &Path,
) -> Result<bool, InitError>;
```

### Lock Management

```rust
/// RAII lock guard for init process
pub struct InitLock {
    // Private fields
}

impl InitLock {
    /// Acquire exclusive lock, handling stale locks
    pub fn acquire(lock_path: PathBuf) -> Result<Self, InitError>;
    
    /// Explicitly release the lock
    pub fn release(&mut self) -> Result<(), InitError>;
}

impl Drop for InitLock {
    fn drop(&mut self);
}
```

### File Creation Helpers

```rust
/// Create .hardline/layouts/ directory
pub async fn create_layouts(repo_root: &Path) -> Result<(), InitError>;

/// Create .gitignore file
pub async fn create_gitignore(repo_root: &Path) -> Result<(), InitError>;

/// Create Git hooks
pub async fn create_git_hooks(repo_root: &Path) -> Result<(), InitError>;

/// Create repo-level AI instructions
pub async fn create_repo_ai_instructions(repo_root: &Path) -> Result<(), InitError>;

/// Create AGENTS.md file
pub async fn create_agents_md(repo_root: &Path) -> Result<(), InitError>;

/// Create CLAUDE.md file
pub async fn create_claude_md(repo_root: &Path) -> Result<(), InitError>;

/// Create Moon pipeline configuration
pub async fn create_moon_pipeline(repo_root: &Path) -> Result<(), InitError>;

/// Create documentation files
pub async fn create_docs(repo_root: &Path) -> Result<(), InitError>;
```

### Response Types

```rust
/// Response from init command
#[derive(Debug, Clone, Serialize)]
pub struct InitResponse {
    /// Status message
    pub message: String,
    
    /// Repository root path
    pub root: String,
    
    /// Paths to created resources
    pub paths: InitPaths,
    
    /// Whether Git repository was initialized
    pub git_initialized: bool,
    
    /// Whether hardline was already initialized
    pub already_initialized: bool,
}

/// Paths to hardline resources
#[derive(Debug, Clone, Serialize)]
pub struct InitPaths {
    /// Data directory
    pub data_directory: String,
    
    /// Configuration file
    pub config: String,
    
    /// State database
    pub state_db: String,
    
    /// Layouts directory
    pub layouts: String,
}

/// Build init response
pub fn build_init_response(
    root: &Path,
    already_initialized: bool,
) -> InitResponse;
```

### Database Operations

```rust
/// Session database for init command
pub struct SessionDb {
    // Private fields
}

impl SessionDb {
    /// Create database if it doesn't exist, or open existing
    pub async fn create_or_open(db_path: &Path) -> Result<Self, InitError>;
}
```

---

## Non-Goals

- [ ] **NG1**: No migration support for existing `.hardline/` directories
- [ ] **NG2**: No config validation beyond TOML parsing
- [ ] **NG3**: No interactive prompts or user input
- [ ] **NG4**: No template customization options
- [ ] **NG5**: No rollback mechanism if init partially fails
- [ ] **NG6**: No parallel initialization support
- [ ] **NG7**: No configuration upgrade/migration
- [ ] **NG8**: No dependency version checking (only presence)
- [ ] **NG9**: No network operations (no fetching from remote)
- [ ] **NG10**: No integration beyond Git repository detection

---

## Configuration Constants

```rust
/// Lock file timeout - stale locks older than this are removed
/// 
/// **Important**: Locks are removed only when age > STALE_LOCK_TIMEOUT_SECS
/// (strictly greater than, not greater-than-or-equal).
/// 
/// - age = 59: lock NOT removed (59 > 60 is false)
/// - age = 60: lock NOT removed (60 > 60 is false)  
/// - age = 61: lock IS removed (61 > 60 is true)
const STALE_LOCK_TIMEOUT_SECS: u64 = 60;

/// Default config content
pub const DEFAULT_CONFIG: &str = r#"# hardline Configuration File
# This file was generated by 'hardline init'

workspace_dir = "../{repo}__workspaces"
main_branch = ""  # auto-detect
state_db = ".hardline/state.db"

[watch]
enabled = true
debounce_ms = 100
paths = [".beads/beads.db"]

[hooks]
post_create = []
pre_remove = []
post_merge = []

[agent]
command = "claude"

[agent.env]

[session]
auto_commit = false
commit_prefix = "wip:"

[recovery]
policy = "warn"
log_recovered = true
auto_recover_corrupted_wal = true
delete_corrupted_database = false
"#;
```

---

## Implementation Notes

### Railway-Oriented Programming

All functions must:
1. Use `Result<T, InitError>` for fallible operations
2. Propagate errors with `?` operator (no `.unwrap()` in source)
3. Use combinators (`map`, `and_then`) for transformations
4. Early return on errors

### Type Safety

1. Use newtype pattern for paths where appropriate
2. Use sealed traits for extensible types
3. Make illegal states unrepresentable with types

### Immutability

1. Prefer `let` over `let mut`
2. Clone explicitly when mutation is needed
3. Avoid shared mutable state

### Concurrency Safety

1. Use file locks for TOCTOU prevention
2. Use `tokio::task::spawn_blocking` for fs2 compatibility
3. Ensure atomic operations where needed

---

## Verification Checklist

Before marking implementation complete:

- [ ] All preconditions documented and enforced
- [ ] All postconditions verifiable
- [ ] All invariants enforced by types or runtime checks
- [ ] All error cases have corresponding variants
- [ ] No `.unwrap()` or `.expect()` in source code
- [ ] All fallible operations return `Result`
- [ ] Lock acquisition handles stale locks
- [ ] Symlink attacks are detected
- [ ] Idempotency verified for all file creation
- [ ] Dry-run mode doesn't create files
- [ ] Already-initialized case handled correctly
- [ ] JSON and human output formats work
- [ ] All documentation files created

---

## References

- Source: `~/src/hardline/crates/cli/src/commands/handlers/`
- Target: `~/src/hardline/crates/cli/src/commands/handlers/`
- Design Pattern: Railway-Oriented Programming
- Testing Strategy: Design-by-contract + Martin Fowler tests
- Build System: Moon (never cargo)
- Version Control: Git
