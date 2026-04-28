# Error Troubleshooting Guide

Complete guide to understanding and resolving isolate errors.

## Quick Reference: Error Codes by Exit Code

| Exit Code | Meaning | Error Codes |
|-----------|---------|-------------|
| 1 | Validation Error | `INVALID_CONFIG`, `VALIDATION_ERROR`, `PARSE_ERROR` |
| 2 | Not Found | `NOT_FOUND` |
| 3 | System Error | `IO_ERROR`, `DATABASE_ERROR` |
| 4 | External Command | `COMMAND_ERROR`, `JJ_COMMAND_ERROR`, `HOOK_FAILED`, `HOOK_EXECUTION_FAILED` |
| 5 | Lock Contention | `SESSION_LOCKED`, `NOT_LOCK_HOLDER` |
| 130 | Cancelled | `OPERATION_CANCELLED` |

## Common Errors and Solutions

### VALIDATION_ERROR (Exit Code 1)

**What it means**: Input validation failed (invalid session name, bad parameter, etc.)

**Error message examples**:
```
Validation error: Session name must start with letter and contain only alphanumeric, dash, underscore
Validation error: value cannot be empty
```

**What to check**:
- Session names must match pattern: `^[a-zA-Z][a-zA-Z0-9_-]*$`
- Required fields cannot be empty
- File paths must be valid

**How to fix**:
```bash
# Example: Create a valid session name
isolate add feature-auth          # Valid (starts with letter)
isolate add feature_auth          # Valid (underscores allowed)
isolate add 123-bad               # Invalid (starts with number)
```

**Expected vs Received**:
When validation fails, the error includes:
- **field**: Which parameter failed
- **expected**: What format was required
- **received**: What you actually provided
- **example**: A valid example
- **pattern**: Regex pattern (if applicable)

---

### INVALID_CONFIG (Exit Code 1)

**What it means**: Configuration file is malformed or contains invalid values

**Error message examples**:
```
Invalid configuration: Unknown key 'workspace_dir'
Invalid configuration: Failed to parse config: TOML parse error
```

**What to check**:
- TOML syntax is correct
- Required keys are present
- Values match expected types

**How to fix**:
```bash
# View current configuration
isolate config list

# Reset to defaults
isolate config reset

# Edit configuration
isolate config edit
```

**Common issues**:
- Missing `[isolate]` section header
- Invalid key names
- Wrong value types (string vs number)
- Unquoted strings with special characters

---

### PARSE_ERROR (Exit Code 1)

**What it means**: Failed to parse JSON or TOML data

**Error message examples**:
```
Parse error: Expected comma at line 5
Parse error: Failed to parse config: invalid TOML syntax
```

**What to check**:
- JSON syntax (if parsing JSON output)
- TOML syntax (if parsing config)
- File encoding (must be UTF-8)

**How to fix**:
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

**What it means**: Requested resource doesn't exist

**Error message examples**:
```
Not found: session 'my-feature' not found
Not found: workspace 'fix-isolate-abc' not found
```

**What to check**:
- Typo in session/workspace name
- Session was removed
- Working in wrong directory

**How to fix**:
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

**What it means**: Filesystem operation failed

**Error message examples**:
```
IO error: Permission denied
IO error: No such file or directory
IO error: Disk quota exceeded
```

**What to check**:
- File permissions
- Disk space
- File/directory existence
- Network mount status

**How to fix**:
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

**What it means**: SQLite database operation failed

**Error message examples**:
```
Database error: database is locked
Database error: disk I/O error
Database error: database disk image is malformed
```

**What to check**:
- Multiple processes accessing database
- Disk corruption
- Filesystem issues

**How to fix**:
```bash
# Run database diagnostics
isolate doctor

# Attempt automatic repair
isolate doctor --fix

# Manual repair (last resort)
rm ~/.isolate/state.db
br sync
```

**Prevention**:
- Avoid running multiple isolate instances simultaneously
- Use proper shutdown procedures
- Backup database regularly

---

### COMMAND_ERROR (Exit Code 4)

**What it means**: External command execution failed

**Error message examples**:
```
Command error: jj: command failed with exit code 1
```

**What to check**:
- Command is installed
- Command is in PATH
- Command syntax is correct

**How to fix**:
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

**What it means**: JJ (Jujutsu) command failed

**Error message examples**:
```
Failed to create workspace: JJ is not installed or not in PATH.

Install JJ:

  cargo install jj-cli

or:

  brew install jj

or visit: https://github.com/martinvonz/jj#installation

Error: No such file or directory (os error 2)
```

**What to check**:
- JJ is installed
- JJ is in PATH
- Current directory is a JJ repo
- JJ is working correctly

**How to fix**:
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

**What it means**: Hook script execution failed

**Error message examples**:
```
Hook 'post_create' failed: npm install
Exit code: 1
Stderr: Package not found
```

**What to check**:
- Hook script exists and is executable
- Hook script has correct shebang
- Hook dependencies are installed
- Hook script returns exit code 0 on success

**How to fix**:
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

**Hook exit codes**:
- 0: Success
- 1-255: Failure (hook reports this exit code)

---

### HOOK_EXECUTION_FAILED (Exit Code 4)

**What it means**: Failed to execute hook script

**Error message examples**:
```
Failed to execute hook '/path/to/hook': No such file or directory
Failed to execute hook 'invalid-shell': Permission denied
```

**What to check**:
- Hook file exists
- Hook file is executable
- Hook shebang is valid
- Shell interpreter exists

**How to fix**:
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

**What it means**: Session is locked by another agent

**Error message examples**:
```
Session 'feature-auth' is locked by agent 'agent-123'
```

**What to check**:
- Another agent is working on this session
- Previous agent crashed without releasing lock

**How to fix**:
```bash
# Check agent status
isolate agent status feature-auth

# Force release (only if agent is dead)
isolate agent kill agent-123
```

**Lock timeout**: Locks auto-release after 1 hour of inactivity

---

### NOT_LOCK_HOLDER (Exit Code 5)

**What it means**: You don't hold the lock for this session

**Error message examples**:
```
Agent 'agent-456' does not hold the lock for session 'feature-auth'
```

**What to check**:
- Which agent holds the lock
- Your agent ID

**How to fix**:
```bash
# Check lock holder
isolate agent status feature-auth
```

---

### OPERATION_CANCELLED (Exit Code 130)

**What it means**: Operation was cancelled by user (SIGINT)

**Error message examples**:
```
Operation cancelled: User interrupted
Operation cancelled: Timeout exceeded
```

**What to check**:
- User pressed Ctrl+C
- Operation timeout
- Manual cancellation

**How to fix**:
- No fix needed - this is expected behavior
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

4. **Run diagnostics**
   ```bash
   isolate doctor
   ```

5. **Check the logs**
   ```bash
   # View recent logs
   isolate logs

   # Enable debug logging
   export Isolate_LOG=debug
   isolate <command>
   ```

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

## Error Messages Best Practices

When reporting errors, include:

1. **The exact error message**
   ```bash
   # Good
   Error: NOT_FOUND: session 'my-feature' not found

   # Bad
   "It said not found"
   ```

2. **The command you ran**
   ```bash
   # Good
   $ isolate session list

   # Bad
   "I tried to list sessions"
   ```

3. **Exit code**
   ```bash
   # Check exit code
   echo $?  # Should be 0-5 or 130
   ```

4. **Your environment**
   ```bash
    isolate --version
    jj --version
    uname -a
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

**For AI agents**: Use `--json` output for programmatic error handling.

---

## Prevention

**Best practices to avoid errors**:

1. **Always validate input** before running commands
2. **Use `isolate doctor`** to check system health
3. **Keep dependencies updated** (JJ)
4. **Backup regularly**: `br sync`
5. **Use `--dry-run`** to preview changes
6. **Read error messages** carefully before acting
7. **Check file permissions** before operations
8. **Avoid concurrent access** to same session

---

**Next**: [Building with Moon](02_MOON_BUILD.md)
