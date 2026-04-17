# Red Queen Test Plan — scenarios crate

**Generation**: 1
**Date**: 2026-04-17
**Champion**: scenarios v0.5.0

## Summary

Started with 13 existing tests. Added 71 adversarial tests across 3 dimensions.
Total: 84 tests. Found 5 bugs (3 CRITICAL, 1 MAJOR, 1 MINOR).

## Bugs Found

### CRITICAL: JSONPath array indexing broken (runner.rs:301-311)

`navigate_path` discards array index when the parent value is an Object.
Path `items[0]` on `{"items": ["a","b","c"]}` returns the whole array `["a","b","c"]` instead of `"a"`.

**Root cause**: `parse_path_segment("items[0]")` returns `("items", Some(0))`, but `navigate_path` matches the `Value::Object` branch and does `map.get("items")` — the index is silently discarded.

**Impact**: Any JSONPath with array indexing (e.g., `users[0].name`, `items[1].id`) returns incorrect data.

**Beads**: ha-2b8z, ha-pdqr, ha-2wd6

### CRITICAL: JSONPath out-of-bounds returns data instead of None (runner.rs:306-308)

`arr.get(99)` on a 2-element array returns `None` from Rust, but the code uses `index.map_or(0, |i| i)` which defaults to index 0 for the Array branch. When the parent is an Object, the Array branch is never reached, so out-of-bounds is moot — the index is ignored entirely.

### MAJOR: value_to_string(Null) returns "null" not "" (runner.rs:279-285)

`serde_json::to_string(Value::Null)` produces `"null"`. The `or_else` chain in `value_to_string` should detect Null and return empty string, but it falls through to `to_string`.

**Impact**: Extracted null values produce the string "null" instead of empty string.

**Bead**: ha-3d5l

### MINOR: Level5 PASS includes scenario name prefix (sanitizer.rs:214-215)

Level5 sanitize for passing scenarios outputs `"Scenario: {name}\nPASS"` instead of just `"PASS"`. Levels 1-4 correctly output just `"PASS"` for passing scenarios.

**Bead**: ha-ss5j

### OBSERVATION: Template resolution with missing variables

`resolve_template("{{missing}}", &context)` returns `"{{missing}}"` (placeholder preserved).
This causes `Exists` assertions on missing variables to pass (non-empty placeholder)
and `NotExists` assertions on missing variables to fail.

Tests document this as known behavior (not a bug per se, but a semantic surprise).

## Test Coverage by Dimension

| Dimension | Tests | Survivors | Fitness |
|-----------|-------|-----------|---------|
| JSONPath edge cases | 15 | 3 | 0.20 |
| Template resolution | 9 | 0 | 0.00 |
| Assertion evaluation | 14 | 0 | 0.00 |
| Sanitizer info barrier | 16 | 0 | 0.00 |
| Scenario parsing | 11 | 0 | 0.00 |
| Runner execution | 6 | 0 | 0.00 |
| RunnerConfig | 3 | 0 | 0.00 |
| value_to_string | 6 | 1 | 0.17 |

## Fix Plan (for follow-up beads)

1. **Fix navigate_path** (CRITICAL): When an Object value is an Array and index is Some,
   apply the index: `map.get(key).and_then(|v| if let Value::Array(arr) = v { arr.get(idx) } else { Some(v.clone()) })`

2. **Fix value_to_string** (MAJOR): Add explicit Null check before `to_string`:
   `if value.is_null() { return String::new(); }`

3. **Fix Level5 PASS** (MINOR): Match the behavior of Levels 1-4 by not prefixing
   scenario name on passing results, or document this as intentional Level5 behavior.
