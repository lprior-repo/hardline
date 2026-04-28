# Rust Patterns

Comprehensive guide to idiomatic Rust patterns used in isolate.

## Core Philosophy

- **Zero Unwrap, Zero Panic** — All code returns `Result`. No panics. Enforced by compiler.
- **Immutability First** — Values don't change; we create new ones.
- **Composition** — Build from small, pure functions.
- **Lazy Evaluation** — Compute only when needed.
- **Type Safety** — Let the compiler prevent errors.
- **Zero-Cost Abstractions** — Iterator chains compile to equivalent loops.

> "Write code that fails gracefully, not code that crashes."

---

## The Three Laws (Compile Errors)

These patterns will **not compile**:

```rust
❌ .unwrap()        // forbid(clippy::unwrap_used)
❌ .expect()        // forbid(clippy::expect_used)
❌ panic!()         // forbid(clippy::panic)
❌ unsafe { }       // forbid(unsafe_code)
❌ unimplemented!() // forbid(clippy::unimplemented)
❌ todo!()          // forbid(clippy::todo)
```

Every fallible operation must return `Result<T, Error>`:

```rust
// ✅ Correct
fn operation(input: &str) -> Result<Output> {
    validate(input)?;
    Ok(transform(input))
}

// ❌ Wrong - doesn't return Result
fn operation(input: &str) -> Output {
    validate(input).unwrap();  // COMPILE ERROR
    transform(input)
}
```

---

## Required Patterns

### Pattern 1: `?` Operator
```rust
fn operation() -> Result<T> {
    let value = fallible()?;
    Ok(value)
}
```

### Pattern 2: Match
```rust
match operation() {
    Ok(v) => use_it(v),
    Err(e) => handle_error(e),
}
```

### Pattern 3: Combinators
```rust
operation()
    .map(transform)
    .and_then(validate)
    .unwrap_or_default()
```

### Pattern 4: if-let
```rust
if let Ok(value) = operation() {
    use_value(value);
}
```

---

## Option Handling

Never unwrap Options:

```rust
❌ maybe.unwrap()              // COMPILE ERROR
```

Do this:

```rust
✅ if let Some(v) = maybe {
     use_value(v);
   }

✅ maybe.map(use_value).unwrap_or_else(default)

✅ match maybe {
     Some(v) => process(v),
     None => default_action(),
   }
```

---

## Result Combinators

### Transform Value

| Method | Input | Output | Use |
|--------|-------|--------|-----|
| `map` | `Result<T>` | `Result<U>` | Transform success value |
| `map_err` | `Result<E>` | `Result<F>` | Transform error |
| `map_or` | `Result<T>` | `U` | Transform to type U or default |
| `map_or_else` | `Result<T>` | `U` | Transform or compute default |

```rust
Ok(5)
    .map(|x| x * 2)           // Ok(10)
    .map_err(|e| wrap_error(e))
    .map_or(0, |x| x + 1)     // 11
```

### Chain Operations

| Method | Use |
|--------|-----|
| `and_then` | Chain fallible operations |
| `or_else` | Alternative fallible operation |

```rust
Ok(5)
    .and_then(|x| {
        if x > 0 { Ok(x * 2) }
        else { Err(Error::Invalid) }
    })  // Ok(10)
    .or_else(|_| Ok(0))  // Ok(10) - not taken
```

### Extract Value

| Method | Returns | Use |
|--------|---------|-----|
| `unwrap_or` | `T` | Get value or default |
| `unwrap_or_else` | `T` | Get value or compute default |
| `ok` | `Option<T>` | Convert to Option |
| `err` | `Option<E>` | Extract error as Option |

```rust
Ok(5).unwrap_or(0)                    // 5
Err::<i32, _>(Error::X).unwrap_or(0)  // 0

Ok(5).ok()     // Some(5)
Err(Error::X).err()  // Some(Error::X)
```

### Inspect

| Method | Use |
|--------|-----|
| `inspect` | Inspect value, return self |
| `inspect_err` | Inspect error, return self |

```rust
Ok(5)
    .inspect(|x| println!("value: {}", x))
    .map(|x| x * 2)
```

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `is_ok` | `bool` | Check if Ok |
| `is_err` | `bool` | Check if Err |
| `contains` | `bool` | Check if Ok and equals value |
| `contains_err` | `bool` | Check if Err and equals error |

---

## Option Combinators

### Transform

| Method | Use |
|--------|-----|
| `map` | Transform Some value |
| `map_or` | Transform or provide default |
| `map_or_else` | Transform or compute default |

### Chain

| Method | Use |
|--------|-----|
| `and_then` | Chain optional operations |
| `or` | Provide alternative Option |
| `or_else` | Compute alternative Option |

### Extract

| Method | Returns | Use |
|--------|---------|-----|
| `unwrap_or` | `T` | Get value or default |
| `unwrap_or_else` | `T` | Get value or compute |

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `is_some` | `bool` | Check if Some |
| `is_none` | `bool` | Check if None |
| `contains` | `bool` | Check if Some and equals |

---

## Iterator Combinators

### Transform Each

| Method | Use |
|--------|-----|
| `map` | Apply function to each |
| `flat_map` | Map then flatten |
| `filter_map` | Filter + map combined |

```rust
vec![1, 2, 3]
    .iter()
    .map(|x| x * 2)        // [2, 4, 6]
    .flat_map(|x| vec![x, x])  // [2,2,4,4,6,6]
```

### Filter

| Method | Use |
|--------|-----|
| `filter` | Keep matching |
| `take_while` | Take while predicate true |
| `skip_while` | Skip while predicate true |

### Accumulate

| Method | Returns | Use |
|--------|---------|-----|
| `fold` | `T` | Accumulate to single value |
| `try_fold` | `Result<T>` | Fold with error handling |
| `scan` | `Iterator` | Fold while iterating |

```rust
// Simple fold
let sum = (1..=5).fold(0, |acc, x| acc + x);  // 15

// Try-fold (error short-circuits)
let result = (1..=5).try_fold(0, |acc, x| {
    if x > 3 { Err(Error::TooLarge) }
    else { Ok(acc + x) }
});  // Err(Error::TooLarge)

// Scan
vec![1, 2, 3].iter().scan(0, |acc, x| {
    *acc += x;
    Some(*acc)
})  // [1, 3, 6]
```

### Partition & Group

```rust
// Partition: split into two groups
let (evens, odds): (Vec<_>, Vec<_>) = (1..=5).partition(|x| x % 2 == 0);
// evens = [2, 4], odds = [1, 3, 5]

// Group by (with itertools)
use itertools::Itertools;
let grouped = vec!["apple", "apricot", "banana", "blueberry"]
    .into_iter()
    .group_by(|s| s.chars().next().unwrap())
    .map(|(k, g)| (k, g.collect::<Vec<_>>()))
    .collect::<Vec<_>>();
// [('a', ["apple", "apricot"]), ('b', ["banana", "blueberry"])]
```

### Zip & Combine

| Method | Use |
|--------|-----|
| `zip` | Combine two iterators |
| `chain` | Concatenate iterators |
| `cycle` | Repeat infinitely |

### Skip & Take

| Method | Use |
|--------|-----|
| `skip` | Skip n elements |
| `take` | Take first n |
| `skip_while` | Skip while predicate |
| `take_while` | Take while predicate |

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `any` | `bool` | Any element matches |
| `all` | `bool` | All elements match |
| `find` | `Option` | First matching element |
| `position` | `Option` | Index of first match |

### Collect

| Method | Collects into |
|--------|---------------|
| `collect` | `Vec`, `HashMap`, `Result<Vec>`, etc |
| `collect::<Result<T>>` | Error short-circuits |

---

## Higher-Order Functions

Functions that take or return functions:

```rust
// Function as parameter
fn apply<T, U, F: Fn(T) -> U>(value: T, f: F) -> U {
    f(value)
}

// Return a function
fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

// Function composition
fn compose<T, U, V, F, G>(f: F, g: G) -> impl Fn(T) -> V
where
    F: Fn(T) -> U,
    G: Fn(U) -> V,
{
    move |x| g(f(x))
}
```

---

## Lazy Evaluation

```rust
// ❌ Eager - creates intermediate Vec
let result = items
    .iter()
    .filter(|x| x > 5)
    .map(|x| x * 2)
    .collect::<Vec<_>>()  // Materializes here!
    .iter()
    .sum::<i32>();

// ✅ Lazy - never materializes
let result: i32 = items
    .iter()
    .filter(|x| x > 5)
    .map(|x| x * 2)
    .sum();  // Computed lazily
```

---

## Closures

```rust
// Simple
let double = |x| x * 2;

// With type annotations
let add = |x: i32, y: i32| -> i32 { x + y };

// Capturing environment
let multiplier = 2;
let multiply = |x| x * multiplier;

// Move semantics
let text = String::from("hello");
let take_ownership = move || println!("{}", text);
```

---

## Pattern Matching

```rust
// Exhaustive matching enforces all cases
match value {
    Some(x) if x > 0 => process_positive(x),
    Some(x) => process_negative(x),
    None => handle_missing(),
}

// Destructuring
let (a, b, c) = (1, 2, 3);

// Match with guards
match items {
    (a, b) if a > b => ("first larger", a - b),
    (a, b) => ("second larger", b - a),
}
```

---

## Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MyError>;
```

### Error Context Pattern

```rust
fn load_config(path: &str) -> Result<Config> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::Io(format!("reading {}: {}", path, e)))?
        .parse::<Config>()
        .map_err(|e| Error::Parse(format!("parsing config: {}", e)))
}
```

---

## Builder Pattern (Validation on Build)

```rust
pub struct ConfigBuilder {
    name: Option<String>,
}

impl ConfigBuilder {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn build(self) -> Result<Config> {
        let name = self.name
            .ok_or(Error::InvalidConfig("name required".into()))?;

        if name.is_empty() {
            return Err(Error::InvalidConfig("name cannot be empty".into()));
        }

        Ok(Config { name })
    }
}
```

---

## Collecting Results

```rust
// Fail on first error
let values: Result<Vec<i32>> = vec!["1", "2", "3"]
    .into_iter()
    .map(|s| s.parse::<i32>().map_err(Error::ParseError))
    .collect();

// Or accumulate with try_fold
vec!["1", "2", "3"]
    .into_iter()
    .try_fold(Vec::new(), |mut acc, s| {
        acc.push(s.parse::<i32>()?);
        Ok(acc)
    })
```

---

## Filter with Fallible Predicate

```rust
let valid = items
    .into_iter()
    .try_fold(Vec::new(), |mut acc, item| {
        if should_keep(&item)? {  // Fallible predicate
            acc.push(item);
        }
        Ok(acc)
    })?;
```

---

## Functional Error Handling

```rust
// Combine multiple validations
let validators = vec![
    |s: &str| if s.is_empty() { Err("empty") } else { Ok(()) },
    |s: &str| if s.len() > 100 { Err("too long") } else { Ok(()) },
];

fn validate_all(input: &str, validators: &[Box<dyn Fn(&str) -> Result<()>>]) -> Result<()> {
    validators.iter().try_fold((), |_, v| v(input))
}

// Or with Either for branching
use either::{Either, Left, Right};

fn process(value: i32) -> Either<String, i32> {
    if value < 0 {
        Left(format!("Error: {}", value))
    } else {
        Right(value * 2)
    }
}
```

---

## Async Patterns

### Async Runtime

All async code runs on Tokio runtime:

```rust
#[tokio::main]
async fn main() {
    if let Err(err) = run_cli().await {
        eprintln!("Error: {}", format_error(&err));
        process::exit(1);
    }
}
```

### Database Connection Pooling

```rust
// Use connection pool instead of single connection
let pool = SqlitePool::connect(&db_url).await?;
let sessions = sqlx::query_as!("SELECT * FROM sessions")
    .fetch_all(&pool)
    .await?;
```

### Async Command Handlers

```rust
pub async fn run(args: Args, ctx: &CommandContext) -> Result<()> {
    let db = get_session_db().await?;
    let session = db.get(&args.name).await?;
    ctx.output_json(&session)
}
```

### Pattern: Database Query with Error Handling

```rust
pub async fn get_session(db: &SqlitePool, name: &str) -> Result<Option<Session>> {
    sqlx::query_as!(
        "SELECT id, name, status, workspace_path, branch, created_at FROM sessions WHERE name = ?",
        name
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::database(format!("Failed to fetch session: {}", e)))
}
```

### Pattern: Transaction with Multiple Operations

```rust
pub async fn update_session_status(db: &SqlitePool, name: &str, status: Status) -> Result<()> {
    let mut tx = db.begin().await?;

    sqlx::query!("UPDATE sessions SET status = ? WHERE name = ?", status, name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
```

### Pattern: Async File Operations

```rust
use tokio::fs;

pub async fn read_workspace_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path).await?;
    toml::from_str(&content)
        .map_err(|e| Error::validation(format!("Invalid config: {}", e)))
}
```

### Pattern: Spawning Background Tasks

```rust
tokio::spawn(async move {
    while let Ok(msg) = rx.recv().await {
        process_message(msg).await?;
    }
    Ok::<(), Error>(())
});
```

### Pattern: Blocking Calls in Async Context

When calling blocking code from async context, use `tokio::task::spawn_blocking`:

```rust
let result = tokio::task::spawn_blocking(move || {
    heavy_computation(data)
})
.await
.context("Failed to join blocking task")??;
```

**Real-World Example:**

```rust
async fn backup_workspace(
    workspace_path: &Path,
    checkpoint_id: &str,
) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        // Heavy blocking I/O: tarball creation, compression
        let backup_file = File::create(&backup_path)?;
        let gz_encoder = flate2::write::GzEncoder::new(backup_file, Compression::default());
        let mut tar_builder = Builder::new(gz_encoder);
        tar_builder.append_dir_all(".", &workspace_path)?;
        Ok(backup_path.to_string_lossy().to_string())
    })
    .await
    .context("Failed to join backup task")?
}
```

### Railway-Oriented Programming

```rust
pub async fn create_session(db: &SqlitePool, name: &str) -> Result<Session> {
    let workspace_path = resolve_workspace_path(name).await?;
    validate_name(name).map_err(|e| Error::validation(e))?;
    let session = db_insert_session(db, name, &workspace_path).await?;
    Ok(session)
}
```

---

## Common Async Pitfalls

### Pitfall 1: Blocking Event Loop

```rust
// BAD: Blocks entire async runtime
let files = std::fs::read_dir(".").unwrap();

// GOOD: Offloads to blocking thread pool
let files = tokio::fs::read_dir(".").await?;
```

### Pitfall 2: Forgetting `.await`

```rust
// BAD: Function returns Future, never executes
let sessions = db.list();

// GOOD: Actually awaits result
let sessions = db.list().await?;
```

### Pitfall 3: Holding Lock Across Await Points

```rust
// BAD: Lock held across await, causes deadlock
let guard = mutex.lock().unwrap();
let data = fetch_remote(guard).await?;

// GOOD: Drop guard before await
{
    let guard = mutex.lock().unwrap();
    let local_data = guard.clone();
}
let data = fetch_remote(local_data).await?;
```

---

## Async with Combinators

```rust
use futures::stream::StreamExt;

async fn process() -> Result<Vec<String>> {
    futures::stream::iter(vec![1, 2, 3, 4, 5])
        .then(|x| async move {
            fetch_data(x).await
        })
        .filter(|result| futures::future::ready(result.is_ok()))
        .map(|result| result.map(|data| data.to_uppercase()))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()
}
```

---

## Immutable Data Structures

```rust
use im::HashMap;

// Create immutable map
let map1 = HashMap::new();

// "Mutate" by creating new map
let map2 = map1.update("key1", "value1");
let map3 = map2.update("key2", "value2");

// All three exist independently (with shared structural sharing)
assert_eq!(map1.len(), 0);
assert_eq!(map2.len(), 1);
assert_eq!(map3.len(), 2);
```

---

## Real-World Example

```rust
use itertools::Itertools;

fn analyze_logs(lines: Vec<String>) -> Result<LogAnalysis> {
    lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(|line| parse_log_entry(&line))
        .collect::<Result<Vec<_>>>()?  // Error short-circuits
        .into_iter()
        .group_by(|entry| entry.level.clone())
        .map(|(level, entries)| (level, entries.count()))
        .collect::<HashMap<_, _>>()
        .into_iter()
        .try_fold(LogAnalysis::default(), |mut analysis, (level, count)| {
            analysis.add_level(level, count)?;
            Ok(analysis)
        })
}
```

---

## Command Implementation Patterns

isolate follows a consistent pattern for command implementation:

### 1. Command Structure

**Args + Options Pattern** (for commands with flags):
```rust
// CLI arguments (from clap::ArgMatches)
pub struct Args {
    pub bead_id: String,
    pub format: String,  // Raw string from clap
}

// Internal options (for business logic)
pub struct Options {
    pub bead_id: String,
    pub format: OutputFormat,  // Enum for code clarity
}

// Conversion: Args → Options
impl Args {
    pub fn to_options(&self) -> Options {
        Options {
            bead_id: self.bead_id.clone(),
            format: if self.format == "json" {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
        }
    }
}
```

**Options-Only Pattern** (for commands without conversion):
```rust
pub struct Options {
    pub query_type: String,
}
```

### 2. Error Handling Pattern

**Business Logic Errors**: Use `isolate_core::Error` at command boundaries
```rust
pub fn run(options: &Options) -> Result<()> {
    let result = execute_business_logic(options)
        .map_err(|e| anyhow::Error::new(e))?;

    output_result(&result, options.format)
}
```

**System Errors**: Use `anyhow::Error` with `.context()`
```rust
pub fn run(options: &Options) -> Result<()> {
    let file = read_file(path)
        .context("Failed to read configuration")?;
    Ok(())
}
```

### 3. JSON Output Pattern

All commands use `SchemaEnvelope` for JSON output:
```rust
use isolate_core::json::SchemaEnvelope;

pub struct Output {
    pub name: String,
    pub status: String,
}

fn output_json(data: &Output) -> Result<()> {
    let envelope = SchemaEnvelope::new("command-response", "single", data);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
}
```

### 4. Module Boundaries

- **isolate-core**: Pure library (no CLI, database, or side effects)
- **crates/isolate**: CLI + commands + database + Zellij integration
- **isolate_core::Error**: Defined in core for semantic exit codes

---

## Documentation Requirements

All public items must be documented:

```rust
/// Brief description.
///
/// Longer description if needed.
///
/// # Errors
///
/// Returns an error if [condition].
///
/// # Examples
///
/// ```ignore
/// let result = my_function(input)?;
/// ```
pub fn my_function(input: &str) -> Result<Output> {
    // implementation
}
```

---

## Testing Without Panics

```rust
#[test]
fn test_success() {
    let result = operation("valid");
    assert!(result.is_ok());
}

#[test]
fn test_error() {
    let result = operation("invalid");
    assert!(result.is_err());
}

#[test]
fn test_error_type() {
    match operation("invalid") {
        Err(Error::Validation(_)) => {}, // ✓ Expected
        other => panic!("unexpected: {:?}", other),
    }
}
```

### Async Tests

```rust
#[tokio::test]
async fn test_create_session() {
    let pool = create_test_pool().await;
    let result = create_session(&pool, "test-session").await;
    assert!(result.is_ok());
}
```

---

## Clippy Rules (Auto-Enforced)

### Forbidden (Compile Errors)
- `unsafe_code` - No unsafe blocks
- `unwrap_used` - No unwrap()
- `expect_used` - No expect()
- `panic` - No panic!()
- `unimplemented` - No unimplemented!()
- `todo` - No todo!()

### Enforced (Warnings = Errors)
- `clippy::all` - All pedantic warnings
- `clippy::pedantic` - Best practices
- `clippy::correctness` - Likely bugs
- `clippy::suspicious` - Suspicious code

---

## Code Review Checklist

Before any PR:

- [ ] No `unwrap()` calls (compiler checks)
- [ ] No `expect()` calls (compiler checks)
- [ ] No `panic!()` calls (compiler checks)
- [ ] No `unsafe { }` (compiler checks)
- [ ] All `Err` paths handled
- [ ] All `None` paths handled
- [ ] All public items documented
- [ ] `Result` types for fallible operations
- [ ] Error types are descriptive
- [ ] Tests don't panic
- [ ] `moon run :ci` passes
- [ ] No clippy warnings

---

## Libraries

- **itertools** - `Itertools` trait with 40+ combinator methods
- **futures** - Future and stream combinators
- **either** - Left/Right sum types
- **im** - Immutable collections
- **tokio** - Async runtime with functional patterns
- **thiserror** - Derive for error types

---

## Performance Notes

- **Zero-cost abstractions** — Iterator chains compile to equivalent loops
- **Lazy evaluation** — Avoid materializing intermediates
- **Move semantics** — Rust's ownership prevents unnecessary copies
- **Inline** — Closures often inlined by compiler
- **`collect()` is the only materialization point**

---

> "Functional code is easier to reason about, test, parallelize, and optimize."

Rust's type system and zero-cost abstractions make functional programming both ergonomic AND performant.
