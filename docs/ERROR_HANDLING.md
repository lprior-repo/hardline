# Error Handling Guide

Comprehensive guide to idiomatic, zero-panic error handling in isolate — both for writing error-aware code and troubleshooting errors when they occur.

---

## Table of Contents

1. [Core Pattern: Result<T, Error>](#core-pattern-resultt-error)
2. [Error Handling Patterns](#error-handling-patterns)
3. [Custom Error Types](#custom-error-types)
4. [Error Suggestions and Fix Commands](#error-suggestions-and-fix-commands)
5. [Common Errors and Solutions](#common-errors-and-solutions)
6. [Debugging Workflow](#debugging-workflow)
7. [Testing Error Paths](#testing-error-paths)
8. [JSON Output Format](#json-output-format)
9. [Prevention Best Practices](#prevention-best-practices)

---

## Core Pattern: Result<T, Error>

All fallible operations return `Result<T, Error>`:

```rust
pub fn operation(input: &str) -> Result<Output> {
    // implementation
}
```

**Never:**
- Return bare `T` for fallible operations
- Use `bool` for success/failure
- Throw exceptions (Rust doesn't have them)

> **Principle:** "Every error is recoverable information. Capture it, propagate it, handle it."
>
> Never throw away error information with `unwrap()`. Never panic. Always return `Result`.

---

## Error Handling Patterns

### Pattern 1: The `?` Operator (Recommended)

Early return on error, continue on success:

```rust
fn process_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)?;     // Returns if error
    let parsed = parse(&content)?;                    // Returns if error
    let validated = validate(&parsed)?;              // Returns on error
    Ok(validated)                                     // Success
}
```

**Why:** Concise, readable, idiomatic Rust. The `?` operator unwraps on `Ok` and returns on `Err`.

---

### Pattern 2: Match Expressions (Explicit)

When you need to handle both cases explicitly:

```rust
match operation() {
    Ok(value) => {
        println!("Success: {}", value);
        process(value)
    }
    Err(e) => {
        eprintln!("Error: {}", e);
        handle_error(e)
    }
}
```

**Why:** Explicit, clear intent, handles all branches.

---

### Pattern 3: if-let (When Ok Matters)

When you only care about success:

```rust
if let Ok(value) = operation() {
    process(value);
} else {
    // Implicitly ignore error
}
```

**Why:** Concise when error path is unimportant.

---

### Pattern 4: Combinators (Functional)

Chain operations with combinators:

```rust
operation()
    .map(|v| v * 2)                    // Transform on success
    .and_then(validate)                // Chain fallible ops
    .unwrap_or_else(|e| {              // Fallback on error
        eprintln!("Error: {}", e);
        default_value()
    })
```

| Combinator | Use | Returns |
|------------|-----|---------|
| `map` | Transform value | `Result<U, E>` |
| `and_then` | Chain operations | `Result<U, E>` |
| `or` | Provide alt Result | `Result<T, E>` |
| `or_else` | Compute alt | `Result<T, E>` |
| `unwrap_or` | Provide default | `T` |
| `unwrap_or_else` | Compute default | `T` |
| `map_err` | Transform error | `Result<T, F>` |

---

### Pattern 5: Error Context

Add context to errors:

```rust
fn load_config(path: &str) -> Result<Config> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::Io(format!("reading {}: {}", path, e)))?
        .parse::<Config>()
        .map_err(|e| Error::InvalidJson(format!("parsing config: {}", e)))
}
```

**Why:** Users understand which step failed and why.

---

### Pattern 6: Early Return

Return immediately on error:

```rust
fn validate_input(input: &str) -> Result<ValidInput> {
    if input.is_empty() {
        return Err(Error::Empty);
    }

    if input.len() > 1000 {
        return Err(Error::TooLong);
    }

    Ok(ValidInput { data: input.to_string() })
}
```

**Why:** Clear validation logic, obvious error paths.

---

### Pattern 7: Collect Errors (try_collect)

Collect results or fail on first error:

```rust
// Fail on first error
let values: Result<Vec<i32>> = vec!["1", "2", "3"]
    .into_iter()
    .map(|s| s.parse::<i32>().map_err(Error::ParseError))
    .collect();

// Or use try_fold to accumulate
vec!["1", "2", "3"]
    .into_iter()
    .try_fold(Vec::new(), |mut acc, s| {
        acc.push(s.parse::<i32>()?);
        Ok(acc)
    })
```

**Why:** Collect multiple results with error short-circuiting.

---

### Pattern 8: Option to Result

Convert Option to Result:

```rust
let required = maybe_value
    .ok_or(Error::NotFound("value required".into()))?;

let or_default = maybe_value
    .ok_or_else(|| Error::NotFound("using default".into()))
    .unwrap_or(default);
```

**Why:** Integrate Option-returning APIs with Result-based code.

---

### Pattern 9: Filter with Error

```rust
let result = values
    .into_iter()
    .try_fold(Vec::new(), |mut acc, v| {
        if v > 0 {
            acc.push(v);
        }
        Ok::<_, Error>(acc)
    })?;
```

**Why:** Filter with fallible predicate.

---

## Custom Error Types

### Keep Errors Simple and Typed

```rust
// ✅ Good - specific, typed errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),
}

// ❌ Bad - stringly-typed
type Result<T> = std::result::Result<T, String>;
```

### Provide Context

```rust
// ✅ Good - error with context
.map_err(|e| Error::Database(format!("finding user {}: {}", user_id, e)))?

// ❌ Bad - no context
.map_err(|_| Error::Database)?
```

### Use From Implementations

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

// Now these work automatically:
std::fs::read_to_string("file")?;  // io::Error -> Error
serde_json::from_str(json)?;       // serde_json::Error -> Error
```

### Example: Real-World Error Type

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("missing required field: {0}")]
    MissingField(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

pub fn load_config(path: &str) -> ConfigResult<Config> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;

    let name = value["name"]
        .as_str()
        .ok_or(ConfigError::MissingField("name".into()))?;

    let port = value["port"]
        .as_u64()
        .ok_or(ConfigError::MissingField("port".into()))? as u16;

    Ok(Config {
        name: name.to_string(),
        port,
    })
}
```

---

## Error Suggestions and Fix Commands

All errors should provide actionable guidance to users. The `Error` enum includes three methods for this:

```rust
impl Error {
    /// Returns a human-readable suggestion for fixing the error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Error::NotFound(_) => Some("Try 'isolate list' to see available sessions"),
            Error::IoError(msg) if msg.contains("Permission") => {
                Some("Check file permissions: 'ls -la' or run with appropriate access rights")
            }
            Error::SessionLocked { session, holder } => Some(
                format!("Session '{session}' is locked by '{holder}'. Use 'isolate agent kill {holder}' to force release")
            )
            // ... more cases
        }
    }

    /// Returns copy-pastable shell commands to resolve the error
    pub fn fix_commands(&self) -> Vec<String> {
        match self {
            Error::NotFound(_) => vec!["isolate list".to_string(), "isolate add <session-name>".to_string()],
            Error::IoError(_) => vec!["ls -la".to_string(), "isolate doctor".to_string()],
            // ... more cases
        }
    }

    /// Returns structured validation hints
    pub fn validation_hints(&self) -> Vec<ValidationHint> {
        // Explains what was expected vs received
    }
}
```

**Why:** Users don't just need to know what went wrong—they need to know how to fix it.

### Guidelines for Error Suggestions

1. **Be specific:** Don't just say "check config" — say "run 'isolate config list' to review configuration"
2. **Provide commands:** Include exact shell commands users can copy-paste
3. **Account for variations:** Handle different error types with context-specific suggestions
4. **Include fallbacks:** For unknown errors, suggest "isolate doctor" as a catch-all

### Example: Session Locked Error

```rust
Error::SessionLocked {
    session: "my-session".to_string(),
    holder: "agent-123".to_string(),
}

// Output:
// Error: Session 'my-session' is locked by agent 'agent-123'
// Suggestion: Session 'my-session' is locked by 'agent-123'. Use 'isolate agent kill agent-123' to force release or check status with 'isolate agent status'
// Fix commands:
//   - isolate agent status my-session
//   - isolate agent kill agent-123
```

This turns a confusing error into clear next steps.

---

## Common Errors and Solutions

### Quick Reference: Error Codes by Exit Code

| Exit Code | Meaning | Error Codes |
|-----------|---------|-------------|
| 1 | Validation Error | `INVALID_CONFIG`, `VALIDATION_ERROR`, `PARSE_ERROR` |
| 2 | Not Found | `NOT_FOUND` |
| 3 | System Error | `IO_ERROR`, `DATABASE_ERROR` |
| 4 | External Command | `COMMAND_ERROR`, `JJ_COMMAND_ERROR`, `HOOK_FAILED`, `HOOK_EXECUTION_FAILED` |
| 5 | Lock Contention | `SESSION_LOCKED`, `NOT_LOCK_HOLDER` |
| 130 | Cancelled | `OPERATION_CANCELLED` |

---

### VALIDATION_ERROR (Exit Code 1)

**What it means:** Input validation failed (invalid session name, bad parameter, etc.)

**Error message examples:**
```
Validation error: Session name must start with letter and contain only alphanumeric, dash, underscore
Validation error: value cannot be empty
```

**What to check:**
- Session names must match pattern: `^[a-zA-Z][a-zA-Z0-9_-]*$`
- Required fields cannot be empty
- File paths must be valid

**How to fix:**
```bash
# Example: Create a valid session name
isolate add feature-auth          # Valid (starts with letter)
isolate add feature_auth          # Valid (underscores allowed)
isolate add 123-bad               # Invalid (starts with number)
```

**Expected vs Received:**
When validation fails, the error includes:
- **field:** Which parameter failed
- **expected:** What format was required
- **received:** What you actually provided
- **example:** A valid example
- **pattern:** Regex pattern (if applicable)

---

### INVALID_CONFIG (Exit Code 1)

**What it means:** Configuration file is malformed or contains invalid values

**Error message examples:**
```
Invalid configuration: Unknown key 'workspace_dir'
Invalid configuration: Failed to parse config: TOML parse error
```

**What to check:**
- TOML syntax is correct
- Required keys are present
- Values match expected types

**How to fix:**
```bash
# View current configuration
isolate config list

# Reset to defaults
isolate config reset

# Edit configuration
isolate config edit
```

**Common issues:**
- Missing `[isolate]` section header
- Invalid key names
- Wrong value types (string vs number)
- Unquoted strings with special characters

---

### PARSE_ERROR (Exit Code 1)

**What it means:** Failed to parse JSON or TOML data

**Error message examples:**
```
Parse error: Expected comma at line 5
Parse error: Failed to parse config: invalid TOML syntax
```

**What to check:**
- JSON syntax (if parsing JSON output)
- TOML syntax (if parsing config)
- File encoding (must be UTF-8)

**How to fix:**
```bash
# Validate JSON syntax
echo '{...}' | jq .

# Validate TOML syntax
isolate config list

# Check file encoding
file ~/.isolate/config.toml
```

---

### NOT_FOUND (Exit Code 2)

**What it means:** Requested resource doesn't exist

**Error message examples:**
```
Not found: session 'my-feature' not found
Not found: workspace 'fix-isolate-abc' not found
```

**What to check:**
- Typo in session/workspace name
- Session was removed
- Working in wrong directory

**How to fix:**
```bash
# List available sessions
isolate list

# Show current context
isolate context

# Create missing session
isolate add my-feature
```

---

### IO_ERROR (Exit Code 3)

**What it means:** Filesystem operation failed

**Error message examples:**
```
IO error: Permission denied
IO error: No such file or directory
IO error: Disk quota exceeded
```

**What to check:**
- File permissions
- Disk space
- File/directory existence
- Network mount status

**How to fix:**
```bash
# Check permissions
ls -la ~/.isolate

# Check disk space
df -h

# Fix permissions
chmod 755 ~/.isolate

# Check for locked files
lsof ~/.isolate/state.db
```

---

### DATABASE_ERROR (Exit Code 3)

**What it means:** SQLite database operation failed

**Error message examples:**
```
Database error: database is locked
Database error: disk I/O error
Database error: database disk image is malformed
```

**What to check:**
- Multiple processes accessing database
- Disk corruption
- Filesystem issues

**How to fix:**
```bash
# Run database diagnostics
isolate doctor

# Attempt automatic repair
isolate doctor --fix

# Manual repair (last resort)
rm ~/.isolate/state.db
br sync
```

**Prevention:**
- Avoid running multiple isolate instances simultaneously
- Use proper shutdown procedures
- Backup database regularly

---

### COMMAND_ERROR (Exit Code 4)

**What it means:** External command execution failed

**Error message examples:**
```
Command error: jj: command failed with exit code 1
```

**What to check:**
- Command is installed
- Command is in PATH
- Command syntax is correct

**How to fix:**
```bash
# Check if command exists
which jj

# Verify PATH
echo $PATH

# Test command directly
jj status
```

---

### JJ_COMMAND_ERROR (Exit Code 4)

**What it means:** JJ (Jujutsu) command failed

**Error message examples:**
```
Failed to create workspace: JJ is not installed or not in PATH.

Install JJ:

  cargo install jj-cli

or:

  brew install jj

or visit: https://github.com/martinvonz/jj#installation

Error: No such file or directory (os error 2)
```

**What to check:**
- JJ is installed
- JJ is in PATH
- Current directory is a JJ repo
- JJ is working correctly

**How to fix:**
```bash
# Install JJ (choose one)
cargo install jj-cli
brew install jj
# Visit: https://martinvonz.github.io/jj/latest/install-and-setup/

# Verify installation
jj --version
jj status

# Initialize JJ repo (if needed)
cd /path/to/project
jj init

# Check JJ is working
jj log
jj diff
```

---

### HOOK_FAILED (Exit Code 4)

**What it means:** Hook script execution failed

**Error message examples:**
```
Hook 'post_create' failed: npm install
Exit code: 1
Stderr: Package not found
```

**What to check:**
- Hook script exists and is executable
- Hook script has correct shebang
- Hook dependencies are installed
- Hook script returns exit code 0 on success

**How to fix:**
```bash
# Check hook configuration
isolate config get hooks.post_create
isolate config list hooks

# Test hook manually
~/.isolate/hooks/post_create

# Skip hooks for debugging
isolate add test-session --no-hooks

# Fix hook script
chmod +x ~/.isolate/hooks/post_create
```

**Hook exit codes:**
- 0: Success
- 1-255: Failure (hook reports this exit code)

---

### HOOK_EXECUTION_FAILED (Exit Code 4)

**What it means:** Failed to execute hook script

**Error message examples:**
```
Failed to execute hook '/path/to/hook': No such file or directory
Failed to execute hook 'invalid-shell': Permission denied
```

**What to check:**
- Hook file exists
- Hook file is executable
- Hook shebang is valid
- Shell interpreter exists

**How to fix:**
```bash
# Check hook file
ls -la ~/.isolate/hooks/

# Add executable permission
chmod +x ~/.isolate/hooks/*

# Verify shebang line
head -1 ~/.isolate/hooks/post_create

# Test hook directly
~/.isolate/hooks/post_create
```

---

### SESSION_LOCKED (Exit Code 5)

**What it means:** Session is locked by another agent

**Error message examples:**
```
Session 'feature-auth' is locked by agent 'agent-123'
```

**What to check:**
- Another agent is working on this session
- Previous agent crashed without releasing lock

**How to fix:**
```bash
# Check agent status
isolate agent status feature-auth

# Force release (only if agent is dead)
isolate agent kill agent-123
```

**Lock timeout:** Locks auto-release after 1 hour of inactivity.

---

### NOT_LOCK_HOLDER (Exit Code 5)

**What it means:** You don't hold the lock for this session

**Error message examples:**
```
Agent 'agent-456' does not hold the lock for session 'feature-auth'
```

**What to check:**
- Which agent holds the lock
- Your agent ID

**How to fix:**
```bash
# Check lock holder
isolate agent status feature-auth
```

---

### OPERATION_CANCELLED (Exit Code 130)

**What it means:** Operation was cancelled by user (SIGINT)

**Error message examples:**
```
Operation cancelled: User interrupted
Operation cancelled: Timeout exceeded
```

**What to check:**
- User pressed Ctrl+C
- Operation timeout
- Manual cancellation

**How to fix:**
- No fix needed — this is expected behavior
- Resume operation if needed
- Check for partial state changes

---

## Debugging Workflow

When you encounter an error:

1. **Read the error message carefully**
   - What operation failed?
   - What's the error code?
   - What's the exit code?

2. **Check the error context** (in JSON output)
   - `operation`: What was being attempted
   - `error`: Specific error details
   - `resource_type`/`resource_id`: What resource was affected

3. **Follow the suggestion**
   - Most errors include actionable suggestions
   - Copy-paste suggested commands

4. **Run diagnostics:**
   ```bash
   isolate doctor
   ```

5. **Check the logs:**
   ```bash
   # View recent logs
   isolate logs

   # Enable debug logging
   export Isolate_LOG=debug
   isolate <command>
   ```

---

## Avoiding Common Mistakes

### ❌ Wrong: Using panic!

```rust
let value = maybe_value.unwrap();  // COMPILE ERROR
if maybe_value.is_some() {
    let v = maybe_value.unwrap();  // ❌ COMPILE ERROR (even though safe!)
}
```

### ✅ Right: Using pattern matching

```rust
let value = match maybe_value {
    Some(v) => v,
    None => return Err(Error::NotFound),
};

if let Some(v) = maybe_value {
    use_value(v);
}
```

### ❌ Wrong: Ignoring errors

```rust
operation()?;  // ❌ Value unused warning
```

### ✅ Right: Handling results

```rust
operation()?;  // ✅ If operation() returns Result<T, E>

let _ = operation();  // ✅ Explicit ignore
operation().ok();    // ✅ Convert to Option, ignore
```

---

## Testing Error Paths

Always test error paths:

```rust
#[test]
fn test_operation_success() {
    let result = operation("valid");
    assert!(result.is_ok());
}

#[test]
fn test_operation_error() {
    let result = operation("invalid");
    assert!(result.is_err());

    match result {
        Err(Error::Validation(_)) => {}, // ✓ Expected error
        other => panic!("unexpected: {:?}", other),
    }
}
```

---

## JSON Output Format

For machine-readable error output:

```bash
isolate <command> --json
```

JSON error structure:
```json
{
  "schema": "error-response",
  "version": "1.0",
  "type": "single",
  "data": {
    "error": {
      "code": "NOT_FOUND",
      "message": "session 'my-feature' not found",
      "exit_code": 2,
      "details": {
        "resource_type": "session",
        "resource_id": "my-feature",
        "searched_in": "database"
      },
      "suggestion": "Use 'isolate list' to see available sessions"
    },
    "validation_hints": [],
    "fix_commands": [
      "isolate list",
      "isolate add my-feature"
    ]
  }
}
```

**For AI agents:** Use `--json` output for programmatic error handling.

---

## Getting Help

If you can't resolve the error:

1. **Collect diagnostic information**
   ```bash
   isolate doctor > doctor-output.txt
   isolate context > context.txt
   isolate logs --last 100 > logs.txt
   ```

2. **Search existing issues**
   - GitHub Issues: https://github.com/your-org/isolate/issues
   - Search for your error code

3. **Create a bug report**
   - Include error code and message
   - Include diagnostic output
   - Include steps to reproduce
   - Include `isolate --version`

---

## Prevention Best Practices

1. **Always validate input** before running commands
2. **Use `isolate doctor`** to check system health
3. **Keep dependencies updated** (JJ)
4. **Backup regularly:** `br sync`
5. **Use `--dry-run`** to preview changes
6. **Read error messages** carefully before acting
7. **Check file permissions** before operations
8. **Avoid concurrent access** to same session

---

**Next:** [Building with Moon](02_MOON_BUILD.md)
