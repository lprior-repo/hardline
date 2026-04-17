# Test Plan Review: CLI Config Command (hl-0g4)

**Reviewer**: test-reviewer (Mode 1 — Plan Inquisition)
**Date**: 2026-03-31
**Inputs**: `contract.md`, `test-plan.md`

---

## VERDICT: REJECTED

---

## Axis 1 — Contract Parity

### Public Functions Inventory

| # | Function | Scenario Coverage | Verdict |
|---|----------|-------------------|---------|
| 1 | `ConfigKey::try_from()` | 10 BDD scenarios in S3.1 | PASS |
| 2 | `parse_cli_value()` | 10 BDD scenarios in S3.2 | PASS |
| 3 | `run()` | 5 BDD scenarios (rows 103-107) | PASS |
| 4 | `config_get()` | Covered indirectly via `run_gets_value` and precedence scenarios | **MINOR** — no direct unit scenario |
| 5 | `config_set()` | Covered indirectly via `run_sets_value` and lock scenarios | **MINOR** — no direct unit scenario |
| 6 | `config_list()` | 1 scenario (`run_lists_all`) | **MINOR** — no `global_only=true` path tested |
| 7 | `get_nested_value()` | 2 scenarios (rows 99-100) | PASS |
| 8 | `set_nested_value()` | 2 scenarios (rows 101-102) | PASS |
| 9 | `load_merged()` (ConfigReadPort) | 0 scenarios | **LETHAL** |
| 10 | `load_global_only()` (ConfigReadPort) | 0 scenarios | **LETHAL** |
| 11 | `global_config_path()` (ConfigReadPort) | 0 scenarios | **LETHAL** |
| 12 | `project_config_path()` (ConfigReadPort) | 0 scenarios | **LETHAL** |

### LETHAL FINDINGS

**L-1.1**: `ConfigReadPort::load_merged()` has zero BDD scenarios. This is the core data-loading function that all precedence behavior depends on. The contract defines it as returning `Pin<Box<dyn Future<Output = Result<Config>> + Send>>`. No scenario validates: what happens when global file is missing? What happens when project file has invalid TOML? What happens when env vars are set but no files exist?

**L-1.2**: `ConfigReadPort::load_global_only()` has zero BDD scenarios. The `config_list` function accepts a `global_only: bool` parameter but no test scenario exercises the `global_only=true` code path. The contract specifies this function exists; no test proves it works.

**L-1.3**: `ConfigReadPort::global_config_path()` has zero BDD scenarios. The contract specifies this returns `Result<PathBuf>`. Under what conditions does it return `Err`? No test tells us.

**L-1.4**: `ConfigReadPort::project_config_path()` has zero BDD scenarios. Same problem. The `error_scope_no_project` scenario (row 94) tests that `config_set(..., Project)` fails when no project path exists, but this is testing the consumer, not the port method itself. The contract defines `project_config_path()` as a separate function with its own error conditions.

### Error Variant Parity

| Variant | Scenario Asserting Exact Variant | Verdict |
|---------|----------------------------------|---------|
| `ConfigKeyNotFound` | `error_key_not_found` (row 90), `get_nested_rejects_unknown` (row 100) | PASS |
| `ConfigParseError` | `config_key_rejects_*` (rows 53-60), `parse_cli_rejects_non_string_array` (row 74), `error_parse_error` (row 91), `set_nested_rejects_non_table` (row 102) | PASS |
| `ConfigWriteError` | `error_write_error` (row 92) | PASS |
| `ConfigScopeError` | `error_scope_env_write` (row 93), `env_scope_rejects_set` (row 97), `error_scope_no_project` (row 94) | PASS |
| `ConfigLockError` | `lock_timeout_returns_error` (row 86), `error_lock_timeout` (row 95) | PASS |
| `NotFound` | 0 scenarios | **LETHAL** |
| `Invalid` | 0 scenarios | **LETHAL** |
| `Permission` | 0 scenarios | **LETHAL** |

**L-1.5**: `ConfigErrorKind::NotFound(String)` has zero test scenarios. The contract defines it with exit code 40. No test asserts this variant is ever produced.

**L-1.6**: `ConfigErrorKind::Invalid(String)` has zero test scenarios. The contract defines it with exit code 41. No test asserts this variant is ever produced.

**L-1.7**: `ConfigErrorKind::Permission(String)` has zero test scenarios. The contract defines it with exit code 42. No test asserts this variant is ever produced.

### MAJOR FINDINGS

**M-1.1**: `config_list(global_only: bool)` — The `global_only` parameter has two code paths. Only `global_only=false` (list all) is tested via `run_lists_all`. The `global_only=true` path is untested. Missing branch.

**M-1.2**: `config_get()` is only tested through `run()` dispatch and precedence scenarios. No test calls `config_get()` directly and asserts the exact `ConfigGetResult` field values (key stability invariant #8, scope reporting, source_path population). The precedence scenarios test scope resolution but do not verify `ConfigGetResult.key == requested_key_string` (invariant #8).

**Axis 1 LETHAL count: 7 | MAJOR count: 2 | MINOR count: 3**

---

## Axis 2 — Assertion Sharpness

### Full "Then" Column Audit

| Row | Test | Then Clause | Verdict |
|-----|------|-------------|---------|
| 51 | `config_key_accepts_two_segment_key` | `Ok, segments==["watch","enabled"], raw=="watch.enabled"` | PASS — exact values |
| 52 | `config_key_accepts_multi_segment_key` | `Ok, segments==["conflict_resolution","mode"]` | PASS |
| 53 | `config_key_rejects_empty_string` | `Err(ConfigParseError) msg contains "empty"` | PASS |
| 54 | `config_key_rejects_single_segment` | `Err(ConfigParseError) msg contains "dot"` | PASS |
| 55 | `config_key_rejects_non_ascii` | `Err(ConfigParseError)` | **LETHAL** |
| 56 | `config_key_rejects_invalid_chars` | `Err(ConfigParseError) each` | **LETHAL** |
| 57 | `config_key_rejects_path_traversal` | `Err(ConfigParseError) each` | **LETHAL** |
| 58 | `config_key_rejects_overlength` | `Err(ConfigParseError)` | **LETHAL** |
| 66 | `parse_cli_infers_bool_true` | `Ok, item==bool(true)` | PASS |
| 67 | `parse_cli_infers_bool_false` | `Ok, item==bool(false)` | PASS |
| 68 | `parse_cli_case_sensitive_bool` | `Ok, item==string (not bool)` | PASS |
| 69 | `parse_cli_infers_positive_int` | `Ok, item==integer(42)` | PASS |
| 70 | `parse_cli_infers_negative_int` | `Ok, item==integer(-100)` | PASS |
| 71 | `parse_cli_overflow_falls_to_string` | `Ok, item==string (fallback)` | **MAJOR** |
| 72 | `parse_cli_infers_string_array` | `Ok, item==array["a","b"]` | PASS |
| 73 | `parse_cli_accepts_empty_array` | `Ok, item==empty array` | PASS |
| 74 | `parse_cli_rejects_non_string_array` | `Err(ConfigParseError)` | **LETHAL** |
| 75 | `parse_cli_falls_back_to_string` | `Ok, item==string("hello world")` | PASS |
| 81 | `precedence_env_overrides_all` | `Ok{value:"true",scope:Env,source_path:empty}` | PASS |
| 82 | `precedence_project_overrides_global` | `Ok{value:"true",scope:Project}` | **MINOR** — source_path not asserted |
| 83 | `precedence_global_only` | `Ok{value:"false",scope:Global}` | **MINOR** — source_path not asserted |
| 84 | `precedence_defaults_when_no_config` | `Ok with default value` | **LETHAL** |
| 85 | `lock_acquired_on_write` | `Ok, file valid TOML` | **MAJOR** — "valid TOML" is vague; should assert exact key-value |
| 86 | `lock_timeout_returns_error` | `Err(ConfigLockError)` | **LETHAL** |
| 87 | `lock_released_on_failure` | `lock released (2nd process acquires)` | PASS — behavioral |
| 88 | `toml_valid_after_set` | `file valid TOML, comments preserved, other keys unchanged` | PASS |
| 89 | `toml_types_preserved` | `original types retained` | **MAJOR** — "types retained" is vague |
| 90 | `error_key_not_found` | `Err(ConfigKeyNotFound), exit=40` | PASS |
| 91 | `error_parse_error` | `Err(ConfigParseError), exit=41` | **LETHAL** |
| 92 | `error_write_error` | `Err(ConfigWriteError), exit=42` | **LETHAL** |
| 93 | `error_scope_env_write` | `Err(ConfigScopeError), exit=43` | **LETHAL** |
| 94 | `error_scope_no_project` | `Err(ConfigScopeError)` | **LETHAL** |
| 95 | `error_lock_timeout` | `Err(ConfigLockError), exit=44` | **LETHAL** |
| 96 | `exit_codes_match_contract` | `NotFound=40,Parse=41,Write=42,Scope=43,Lock=44` | PASS |
| 97 | `env_scope_rejects_set` | `Err(ConfigScopeError("Cannot save to environment scope"))` | PASS — exact message |
| 98 | `env_scope_empty_source_path` | `Ok{scope:Env,source_path:PathBuf::new()}` | PASS |
| 99 | `get_nested_returns_leaf` | `Ok("Auto")` | PASS |
| 100 | `get_nested_rejects_unknown` | `Err(ConfigKeyNotFound)` | **LETHAL** |
| 101 | `set_nested_creates_tables` | `Ok, doc["new_sec"]["key"]==42` | PASS |
| 102 | `set_nested_rejects_non_table` | `Err(ConfigParseError)` | **LETHAL** |
| 103 | `run_lists_all` | `all key=value, sorted alpha` | **MAJOR** — "sorted alpha" is behavioral but no exact expected output |
| 104 | `run_gets_value` | `output has "watch.enabled = true"` | PASS — substring match |
| 105 | `run_sets_value` | `output confirms, file has false` | **MAJOR** — "confirms" is vague; exact output? |
| 106 | `run_rejects_value_no_key` | `Err(ConfigParseError)` | **LETHAL** |
| 107 | `cli_exit_codes` | `codes: 40,41,42,43,44` | PASS |

### LETHAL FINDINGS

**L-2.1**: Rows 55, 56, 57, 58 (reject tests for ConfigKey) — All assert only `Err(ConfigParseError)` without any message content check. These are four separate error conditions (non-ASCII, invalid chars, path traversal, overlength) that should produce distinguishable error messages. Without asserting message content, a single `Err(ConfigParseError("unknown"))` satisfies all four. This is `is_err()` in disguise.

**L-2.2**: Row 74 (`parse_cli_rejects_non_string_array`) — Asserts only `Err(ConfigParseError)` with no message check. Same problem as L-2.1.

**L-2.3**: Rows 86, 91, 92, 93, 94, 95 (error taxonomy tests) — These assert `Err(Variant)` without asserting the string payload. For example, `error_write_error` (row 92) should assert the message contains the read-only path that caused the failure. Without message content, a `ConfigWriteError("")` passes the test.

**L-2.4**: Row 84 (`precedence_defaults_when_no_config`) — "Ok with default value" is a bare `is_ok()` with no concrete expected value. Which default? What value? This assertion passes if the function returns `Ok("anything_at_all")`.

**L-2.5**: Rows 100, 102, 106 — `Err(ConfigKeyNotFound)` and `Err(ConfigParseError)` without message content assertions. Same category as L-2.1.

### MAJOR FINDINGS

**M-2.1**: Row 71 (`parse_cli_overflow_falls_to_string`) — "item==string (fallback)" does not specify the exact string value. Should be `item==string("99999999999999999999")` (the original input preserved verbatim).

**M-2.2**: Row 85 (`lock_acquired_on_write`) — "file valid TOML" is a structural assertion without value verification. Must assert the written key-value is exactly what was set.

**M-2.3**: Row 89 (`toml_types_preserved`) — "original types retained" is hopelessly vague. Must specify: `file["watch"]["enabled"] == bool(true)`, `file["watch"]["interval"] == integer(5)`, etc.

**M-2.4**: Row 103 (`run_lists_all`) — "all key=value, sorted alpha" does not specify the expected set of keys or their values. A test that passes with any subset of keys in any alphabetical order is not a test.

**M-2.5**: Row 105 (`run_sets_value`) — "output confirms" is not an assertion. What exact string or JSON field? "file has false" — which key? What about other keys?

### MINOR FINDINGS

**m-2.1**: Rows 82, 83 — `source_path` not asserted in precedence scenarios. Invariant #8 requires `ConfigGetResult.key` to exactly match the input, but this is never checked for these scenarios.

**Axis 2 LETHAL count: 5 (clusters) | MAJOR count: 5 | MINOR count: 2**

---

## Axis 3 — Trophy Allocation

### Density Audit

- Public functions: 12 (8 direct + 4 ConfigReadPort trait methods)
- Planned tests: 41 (16 unit + 20 integration + 5 e2e)
- Ratio: 41 / 12 = **3.4x**

**LETHAL**: Target is >= 5x. 3.4x is below threshold. The test plan has 41 tests for 12 public functions. It needs at least 60 tests to meet the 5x density requirement.

### Proptest Coverage

| Pure Function | Proptest Planned? | Verdict |
|---------------|-------------------|---------|
| `ConfigKey::try_from()` | Yes (S4, row 115) | PASS |
| `parse_cli_value()` | Yes (S4, rows 116-118) | PASS |
| `get_nested_value()` | No | **LETHAL** |
| `set_nested_value()` | Yes (S4, row 118) | PASS |

**L-3.1**: `get_nested_value()` is a pure function with non-trivial input space (arbitrary Config struct, arbitrary dot-notation path). No proptest invariant planned. This function traverses a JSON tree; edge cases around arrays, nested tables, and mixed types are easily missed by hand-written tests.

### Fuzz Coverage

| Parser/Deserializer | Fuzz Target Planned? | Verdict |
|---------------------|----------------------|---------|
| `ConfigKey::try_from()` | Yes (S5, row 127) | PASS |
| `parse_cli_value()` | Yes (S5, row 128) | PASS |
| `set_nested_value()` | Yes (S5, row 129) | PASS |
| TOML file parsing | Yes (S5, row 130) | PASS |
| `get_nested_value()` | No | **LETHAL** |

**L-3.2**: `get_nested_value()` takes a `&Config` (arbitrarily complex struct) and a `&str` key. It traverses a serde_json::Value tree internally. This is a parser-like function with rich input space and no fuzz target.

### Kani Coverage

Three Kani harnesses planned. All target panic-freedom for untrusted input. Reasonable scope.

### Integration/Unit Ratio

- Unit: 16 (39%)
- Integration: 20 (49%)
- E2E: 5 (12%)

The deviation justification in the plan ("many pure Calc functions demand exhaustive unit boundary coverage") is reasonable, but the unit count of 16 for 8+ pure functions is thin. `ConfigReadPort` integration tests are missing entirely.

**Axis 3 LETHAL count: 3 | MAJOR count: 0 | MINOR count: 0**

---

## Axis 4 — Boundary Completeness

### `ConfigKey::try_from()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| Minimum valid (2-char: "a.b") | No explicit test for minimal 1-char segments | **MINOR** |
| Maximum valid (256 chars) | `config_key_accepts_at_max_length` | PASS |
| One-below-max (255 chars) | Not tested explicitly (max passes, so 255 passes by extension) | PASS (implicit) |
| One-above-max (257 chars) | `config_key_rejects_overlength` | PASS |
| Empty string | `config_key_rejects_empty_string` | PASS |
| Single segment (no dot) | `config_key_rejects_single_segment` | PASS |
| Exactly 2 segments | `config_key_accepts_two_segment_key` | PASS |
| 3+ segments | `config_key_accepts_multi_segment_key` | PASS |
| Null bytes | Yes (in path_traversal) | PASS |
| Unicode/non-ASCII | `config_key_rejects_non_ascii` | PASS |

### `parse_cli_value()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| Empty string | No test | **MINOR** |
| i64::MIN | No test | **MINOR** |
| i64::MAX | No test | **MINOR** |
| i64::MAX + 1 (overflow) | `parse_cli_overflow_falls_to_string` (uses a large number, not i64::MAX+1) | **MINOR** |
| i64::MIN - 1 (underflow) | No test | **MINOR** |
| Empty array `[]` | `parse_cli_accepts_empty_array` | PASS |
| Single-element array | No test | **MINOR** |
| Array with escaped quotes | No test | **MINOR** |
| Array with empty string element `[""]` | No test | **MINOR** |
| Malformed array `["a",` | No test | **MINOR** |
| Bool with whitespace `" true"` | No test | **MINOR** |

**M-4.1**: `parse_cli_value()` has **8 missing boundaries** on a single function. This exceeds the 3-missing threshold for MAJOR.

### `config_set()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| Lock acquired successfully | `lock_acquired_on_write` | PASS |
| Lock timeout (5s exceeded) | `lock_timeout_returns_error` | PASS |
| Lock at exactly 4.9s (succeeds) | No test | **MINOR** |
| Lock at exactly 5.0s (fails) | No test | PASS (covered by timeout test) |
| Lock released on failure | `lock_released_on_failure` | PASS |
| Parent dir creation | No explicit test | **MINOR** |
| File does not exist (create) | Implicitly tested | PASS |
| File exists with content | Implicitly tested | PASS |
| Env scope rejected | `env_scope_rejects_set` | PASS |

### `get_nested_value()` / `set_nested_value()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| Empty parts (set) | No test — contract says `parts is non-empty` but no test for empty | **MINOR** |
| Single-segment parts | No test | **MINOR** |
| Very deep nesting (10+ levels) | No test | **MINOR** |
| Value at root (no nesting needed) | No test | **MINOR** |

### `config_list()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| Empty config (no keys) | No test | **MINOR** |
| One key | No test | **MINOR** |
| global_only=true | No test | **MINOR** |

**M-4.2**: `config_list()` has 3 missing boundaries.

### `run()`

| Boundary | Explicit in Plan? | Verdict |
|----------|-------------------|---------|
| (None, None) -> list | `run_lists_all` | PASS |
| (Some, None) -> get | `run_gets_value` | PASS |
| (Some, Some) -> set | `run_sets_value` | PASS |
| (None, Some) -> error | `run_rejects_value_no_key` | PASS |
| Key validation fails before dispatch | No test | **MINOR** |

**Axis 4 LETHAL count: 0 | MAJOR count: 2 | MINOR count: 17**

---

## Axis 5 — Mutation Survivability

### Mutation Site Analysis

**Mutation 1**: Change `>` to `>=` in ConfigKey max-length check (256 boundary).

- Plan has `config_key_accepts_at_max_length` (256 chars => Ok) and `config_key_rejects_overlength` (257 chars => Err).
- If implementation uses `len > 256` and mutation changes to `len >= 256`, the 256-char test flips from Ok to Err and catches it.
- **CAUGHT**.

**Mutation 2**: Delete the `ConfigScope::Env` match arm in `config_set`, so Env writes fall through to the default write path.

- `env_scope_rejects_set` (row 97) asserts `Err(ConfigScopeError("Cannot save to environment scope"))`. If the arm is deleted, this would return Ok or a different error. **CAUGHT**.

**Mutation 3**: Return `Ok(Default::default())` from `config_get` instead of the real resolved value.

- `precedence_env_overrides_all` asserts `value:"true"`, not default. **CAUGHT**.
- But `precedence_defaults_when_no_config` (row 84) asserts "Ok with default value" — which is the default! This test would **PASS** with `Ok(Default::default())` even though it should be testing that the default comes from the actual Config struct defaults, not from `Default::default()` on the result type. **SURVIVES**.

**Mutation 4**: Swap precedence order — load project before env.

- `precedence_env_overrides_all` would fail because env would no longer override project. **CAUGHT**.

**Mutation 5**: Remove TOML validation step (step 7 in file locking protocol).

- `toml_valid_after_set` (row 88) asserts "file valid TOML, comments preserved". If validation is removed, invalid TOML could be written but this test only checks valid writes. **PARTIALLY CAUGHT** — need a test that writes a value that would produce invalid TOML without the validation step.

**Mutation 6**: Return `Ok(ConfigGetResult { key: normalized_key, ... })` instead of exact input key (violate invariant #8).

- No test asserts `result.key == input_key_string` directly. The `precedence_env_overrides_all` test asserts value and scope but not key equality. `run_gets_value` checks output contains "watch.enabled = true" but does not programmatically assert `result.key.raw == "watch.enabled"`. **SURVIVES**.

**Mutation 7**: Remove lock acquisition entirely (skip step 3).

- `lock_timeout_returns_error` would never trigger since no lock is held. **CAUGHT** (assuming another process holds a real lock).
- But `lock_acquired_on_write` (row 85) — if lock is skipped, the write still succeeds. This test only checks "file valid TOML", not that a lock was actually acquired. **PARTIALLY SURVIVES**.

**Mutation 8**: Remove the retry loop in lock acquisition (timeout = immediate fail on first try).

- `lock_timeout_returns_error` tests 5s timeout. If implementation retries only once with 100ms interval, the test may pass by accident if lock is held long enough. The test does not verify the retry behavior itself — only the timeout outcome. **SURVIVES** for retry-count mutation.

**Mutation 9**: `parse_cli_value` — remove the array branch, all arrays fall to string.

- `parse_cli_infers_string_array` asserts `item==array["a","b"]`. If arrays are parsed as strings, this fails. **CAUGHT**.

**Mutation 10**: `parse_cli_value` — return integer for overflow strings instead of string fallback.

- `parse_cli_overflow_falls_to_string` asserts `item==string`. If mutation returns integer, this catches it. But the assertion is "item==string (fallback)" without the exact value — what if mutation returns `integer(0)`? The test checks type, not value. **PARTIALLY CAUGHT** — type check works for this mutation, but the vague assertion is still a problem per Axis 2.

**Mutation 11**: `set_nested_value` — skip intermediate table creation, return error.

- `set_nested_creates_tables` asserts `doc["new_sec"]["key"]==42`. If no table created, assertion fails. **CAUGHT**.

**Mutation 12**: `config_list` — return unsorted keys.

- `run_lists_all` asserts "sorted alpha". If unsorted, caught. **CAUGHT** — but the assertion is behavioral (e2e output), not structural. If the sort is buggy (e.g., reverse alpha), caught. If sort is case-insensitive but should be case-sensitive, may survive.

### LETHAL FINDINGS

None — mutation analysis at plan level is advisory.

### MAJOR FINDINGS

**M-5.1**: Mutation 3 survives — `precedence_defaults_when_no_config` does not distinguish between "correct default from Config struct" and `Ok(Default::default())`. Must assert exact expected default value.

**M-5.2**: Mutation 6 survives — no test programmatically asserts `ConfigGetResult.key.raw == input_key_string` (invariant #8). Key stability is untested.

**M-5.3**: Mutation 7 partially survives — no test verifies that a lock was actually acquired during a successful write. The test only checks the TOML output. A `config_set` that skips locking entirely would pass `lock_acquired_on_write`.

**M-5.4**: Mutation 8 survives — no test verifies the retry behavior (100ms intervals, up to 5s). Only the timeout outcome is tested.

**Axis 5 LETHAL count: 0 | MAJOR count: 4 | MINOR count: 0**

---

## Axis 6 — Holzmann Plan Audit

### Rule 2 — Bound Every Loop

The plan itself has no loops in test bodies (it is a plan, not code). However, several scenarios test multiple inputs in a single row:

- Row 56: `"my-key.val"`, `"my key.val"` — `try_from` each
- Row 57: 6 inputs tested in one scenario
- Row 68: `"True"`, `"FALSE"` — `parse_cli_value` each

These will require loops in the test implementation to iterate over inputs, unless each input is expanded into its own test function. The plan must explicitly state that each input in these rows becomes its own separate test function (no loops), or use `rstest` cartesian product.

**L-6.1**: Rows 56, 57, 68, and the proptest/anti-invariant tables describe multi-input scenarios that implicitly require iteration. The plan must mandate individual test functions or `rstest` parameterization. As written, a developer could implement these as `for input in inputs { assert!(...) }` loops, violating Holzmann Rule 2. The plan does not explicitly prohibit this. **LETHAL**.

### Rule 5 — State Your Assumptions (Explicit Preconditions)

| Scenario | Explicit Given? | Verdict |
|----------|-----------------|---------|
| `precedence_env_overrides_all` (row 81) | "global=false, project=true, env SCP_WATCH_ENABLED=true" | PASS |
| `lock_timeout_returns_error` (row 86) | "another process holds lock" | **MINOR** — how is this set up? What process? What lock? |
| `lock_released_on_failure` (row 87) | "writable but invalid content" | **MINOR** — invalid how? Corrupted TOML? Permission issue? |
| `toml_valid_after_set` (row 88) | "TOML file with comments" | **MINOR** — which comments? Where? |
| `error_write_error` (row 92) | "read-only dir" | PASS |
| `error_scope_no_project` (row 94) | "no project_path" | **MINOR** — how is "no project_path" established? Outside a git repo? Env var unset? |

### Rule 8 — Surface Your Side Effects

- `lock_timeout_returns_error` requires spawning a background process or thread to hold a lock. This side effect is not named in the plan. A test helper like `hold_lock_for_duration(path, 10s)` should be specified. **MINOR**.
- Integration tests create temp directories and config files. The plan says "real tmpfiles" but does not name the helper functions or their side effects. **MINOR**.
- `lock_released_on_failure` requires a second process/thread. Side effect not named. **MINOR**.

### Rule 1 — Keep it Linear

No nested conditionals visible in plan. PASS.

### Rule 4 — One Function, One Job

Several scenarios test multiple inputs in a single row (rows 56, 57, 68). Each should be a separate test function. **MINOR**.

### Rule 3, 6, 7, 9 — Not Applicable at Plan Level

These rules apply to test code, not plans. Deferred to Mode 2.

**Axis 6 LETHAL count: 1 | MAJOR count: 0 | MINOR count: 7**

---

## Aggregated Findings

### LETHAL (16)

| ID | Axis | Description |
|----|------|-------------|
| L-1.1 | 1 | `ConfigReadPort::load_merged()` — zero BDD scenarios |
| L-1.2 | 1 | `ConfigReadPort::load_global_only()` — zero BDD scenarios |
| L-1.3 | 1 | `ConfigReadPort::global_config_path()` — zero BDD scenarios |
| L-1.4 | 1 | `ConfigReadPort::project_config_path()` — zero BDD scenarios |
| L-1.5 | 1 | `ConfigErrorKind::NotFound` — zero scenarios asserting exact variant |
| L-1.6 | 1 | `ConfigErrorKind::Invalid` — zero scenarios asserting exact variant |
| L-1.7 | 1 | `ConfigErrorKind::Permission` — zero scenarios asserting exact variant |
| L-2.1 | 2 | Rows 55-58: `Err(ConfigParseError)` without message content — `is_err()` in disguise |
| L-2.2 | 2 | Row 74: `Err(ConfigParseError)` without message content |
| L-2.3 | 2 | Rows 86, 91-95: Error taxonomy tests assert variant but not string payload |
| L-2.4 | 2 | Row 84: "Ok with default value" — bare `is_ok()` with no concrete expected value |
| L-2.5 | 2 | Rows 100, 102, 106: `Err(Variant)` without message content assertions |
| L-3.1 | 3 | `get_nested_value()` — no proptest invariant for pure function with non-trivial input space |
| L-3.2 | 3 | `get_nested_value()` — no fuzz target for parser-like function |
| L-3.3 | 3 | Test density 3.4x — below 5x threshold (41 tests / 12 public functions) |
| L-6.1 | 6 | Multi-input scenarios (rows 56, 57, 68) implicitly require loops; plan does not mandate loop-free implementation |

### MAJOR (13)

| ID | Axis | Description |
|----|------|-------------|
| M-1.1 | 1 | `config_list(global_only=true)` path untested |
| M-1.2 | 1 | `config_get()` never tested directly; invariant #8 (key stability) unverified |
| M-2.1 | 2 | Row 71: overflow string value not exact |
| M-2.2 | 2 | Row 85: "file valid TOML" without exact key-value assertion |
| M-2.3 | 2 | Row 89: "types retained" is vague |
| M-2.4 | 2 | Row 103: "all key=value, sorted alpha" — no expected set |
| M-2.5 | 2 | Row 105: "output confirms" — not an assertion |
| M-4.1 | 4 | `parse_cli_value()` has 8 missing boundaries |
| M-4.2 | 4 | `config_list()` has 3 missing boundaries |
| M-5.1 | 5 | Mutation 3: defaults test does not distinguish from `Default::default()` |
| M-5.2 | 5 | Mutation 6: `ConfigGetResult.key` stability invariant untested |
| M-5.3 | 5 | Mutation 7: no test verifies lock was actually acquired on successful write |
| M-5.4 | 5 | Mutation 8: no test verifies retry behavior, only timeout outcome |

### MINOR (29)

- m-1.1 through m-1.3: Axis 1 minor function coverage gaps
- m-2.1, m-2.2: Axis 2 source_path not asserted in precedence scenarios
- m-4.x (17 items): Axis 4 boundary gaps across multiple functions
- m-6.x (7 items): Axis 6 Holzmann preconditions and side-effect naming

---

## Severity Threshold Check

- LETHAL: 16 (any single = REJECTED)
- MAJOR: 13 (threshold: 3)
- MINOR: 29 (threshold: 5)

**Result: REJECTED on all three severity counts.**

---

## MANDATE

Before resubmission, the test plan MUST address ALL of the following:

### LETHAL — Must Fix

1. **Add ConfigReadPort scenarios**: At minimum 4 BDD scenarios, one per trait method. Cover: missing global file, missing project file, both files present, invalid TOML in file, env-only config. Each must assert exact return type and error variant.

2. **Add scenarios for `NotFound`, `Invalid`, `Permission` error variants**: Either add BDD rows that trigger these exact variants, or remove them from the contract (if they are legacy-only and cannot be produced by the new code). If removing, document why.

3. **Harden all `Err(Variant)` assertions**: Every error scenario must include a message content check. Change `Err(ConfigParseError)` to `Err(ConfigParseError) where message contains "specific substring"`. This applies to rows 55-58, 74, 86, 91-95, 100, 102, 106.

4. **Fix row 84 (`precedence_defaults_when_no_config`)**: Replace "Ok with default value" with exact expected value, e.g., `Ok{value:"false",scope:Global,source_path:...}`.

5. **Add proptest invariant for `get_nested_value()`**: Strategy: random Config struct + valid dot-notation key => value retrieved must match serde_json traversal.

6. **Add fuzz target for `get_nested_value()`**: Input: arbitrary Config struct serialized to JSON + arbitrary &str key. Must never panic.

7. **Increase test density to >= 5x**: Add at least 19 more tests (need 60 total for 12 public functions). Suggested additions:
   - 4 ConfigReadPort method tests
   - 3 legacy error variant tests (or document removal)
   - 4 `config_get` direct tests (key stability, source_path, scope)
   - 2 `config_list` tests (empty config, global_only=true)
   - 3 boundary tests for `parse_cli_value` (empty string, i64::MAX, i64::MIN)
   - 2 lock verification tests (verify lock held, verify retry count)
   - 1 `set_nested_value` single-segment test

8. **Mandate loop-free test implementation**: Add explicit note to plan: "All multi-input scenarios (rows 56, 57, 68) MUST be implemented as separate `#[test]` functions or `#[rstest]` parameterized tests. NO loops in test bodies."

### MAJOR — Must Fix

9. **Specify exact overflow string value** in row 71: `item==string("99999999999999999999")`.
10. **Add exact TOML key-value assertion** to row 85.
11. **Specify exact type checks** in row 89.
12. **Specify expected key set** in row 103 `run_lists_all`.
13. **Specify exact output** in row 105 `run_sets_value`.
14. **Add `config_list(global_only=true)` test**.
15. **Add `config_get()` direct test** asserting all `ConfigGetResult` fields including key stability (invariant #8).
16. **Add boundary tests for `parse_cli_value()`**: empty string, i64::MAX, i64::MIN, single-element array, array with empty string, malformed array, whitespace-padded bool.
17. **Add lock acquisition verification test**: Verify lock is held during write by attempting concurrent read.
18. **Add retry behavior test**: Verify 5s timeout with controlled lock hold duration.
19. **Harden defaults test**: Assert exact default value, not just "some default".

---

STATUS: **REJECTED**
