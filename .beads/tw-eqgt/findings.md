# Findings: tw-eqgt - hardline: Replace assert! with Result in ReservedPermitBudget and fix production panics

## Summary
Security-focused bug: production panic paths that can crash the process on invalid input or corrupt state.

## Issues Found

### 1. cli/src/commands/task_store.rs:119-124 — LazyLock+expect() CRITICAL
**Severity**: P0 - Production panic

```rust
static TASK_STORE: LazyLock<Arc<TaskStore>> =
    LazyLock::new(|| {
        Arc::new(TaskStore::load().expect(
            "Fatal: failed to initialize task store — check file permissions and disk state",
        ))
    });
```

**Problem**: `TaskStore::load()` returns `CoreResult<Self>`. If loading fails (corrupt JSON, permissions issue, etc.), `.expect()` panics the entire process.

**Root Cause**: `LazyLock` doesn't support fallible initialization. The code uses `.expect()` to flatten the Result.

**Fix Required**: Replace `LazyLock` with `OnceLock<RwLock<Option<Arc<TaskStore>>>>` pattern, similar to `PORT_REGISTRY` in `command_types.rs:237`. Initialization happens lazily on first access, and errors are handled gracefully.

### 2. core/src/config/config_core.rs:551-556 — expect() in Default impl
**Severity**: P0 - Production panic on startup

```rust
#[allow(clippy::expect_used)]
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create config manager")
    }
}
```

**Problem**: `ConfigManager::new()` returns `Result<Self>`. It can fail if `directories::ProjectDirs::from("com", "scp", "scp")` returns `None` (which happens on unsupported platforms). Calling `.expect()` panics on startup.

**Fix Required**: Change `Default` impl to handle the error case. Since `Default` must return `Self` (not `Result`), either:
- Use `unwrap_or_else` with a fallback that uses a default config path, OR
- Change the `Default` impl to be fallible and change all callers

### 3. core/src/config/config_core.rs:559-562 — global_config() CRITICAL
**Severity**: P0 - Production panic on startup

```rust
#[allow(clippy::expect_used)]
pub fn global_config() -> ConfigManager {
    ConfigManager::new().expect("Failed to create config manager")
}
```

**Problem**: Same issue as #2 - can panic on startup if config directory cannot be determined.

**Fix Required**: Change return type to `Result<ConfigManager>` and propagate errors properly.

### 4. core/src/events.rs:183-190 — .unwrap() in uuid_simple()
**Severity**: P2 - Test-only code

```rust
#[allow(clippy::unwrap_used)]
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}
```

**Problem**: Uses `.unwrap()` on SystemTime calculation. However, this function is:
- Marked with `#[allow(clippy::unwrap_used)]`
- Only used in test code (`uuid_simple()` is called from tests, not production)
- `.duration_since(UNIX_EPOCH)` can only fail if now < UNIX_EPOCH, which is essentially impossible on modern systems

**Recommendation**: Low priority. Could use `expect()` with better message or `unwrap_or_else()` with fallback, but the risk is minimal for test-only code.

### 5. ReservedPermitBudget (NOT FOUND)
**Severity**: N/A - Code no longer exists

The bead mentions `core/src/workload_class/budget.rs:39` and `ReservedPermitBudget::new()` but this file/struct does not exist in the current codebase. Either:
- Code was refactored and the struct was removed, OR
- The bead was created for a different version of the code

**Action**: No fix needed for non-existent code. The bead may need to be updated or closed.

## Recommended Fixes

### task_store.rs
Replace `LazyLock` with `OnceLock` pattern:

```rust
use std::sync::{Arc, OnceLock, RwLock};

static TASK_STORE: OnceLock<RwLock<Option<Arc<TaskStore>>>> = OnceLock::new();

fn task_store_init() -> &'static RwLock<Option<Arc<TaskStore>>> {
    TASK_STORE.get_or_init(|| {
        RwLock::new(
            TaskStore::load()
                .ok()
                .map(Arc::new)
        )
    })
}

pub fn get_task_store() -> Arc<TaskStore> {
    task_store_init()
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().cloned())
        .unwrap_or_else(|| {
            // Fallback: return a new empty store
            Arc::new(TaskStore {
                tasks: RwLock::new(HashMap::new()),
                tasks_file: PathBuf::new(),
            })
        })
}
```

### config_core.rs
1. Change `Default` impl to use a fallback:
```rust
impl Default for ConfigManager {
    fn default() -> Self {
        Self::with_paths(
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".config/scp/config.toml"),
            None,
        )
    }
}
```

2. Change `global_config()` return type to `Result<ConfigManager>`:
```rust
pub fn global_config() -> Result<ConfigManager> {
    Self::new()
}
```

## Files Requiring Changes

1. `/home/lewis/src/hardline/crates/cli/src/commands/task_store.rs` - Replace LazyLock panic with error handling
2. `/home/lewis/src/hardline/crates/core/src/config/config_core.rs` - Fix Default impl and global_config()

## Verification

After fixes:
1. `cargo build --release` should succeed
2. `cargo test` should pass
3. Code should handle corrupt task_store.json gracefully (return empty store or error, not panic)
4. Code should handle missing config directory gracefully (return error, not panic)
