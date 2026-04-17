# BLACK HAT REVIEW: crates/scenarios

**Bead**: ha-6onx
**Reviewer**: polecat delta (hardline rig)
**Date**: 2026-04-17
**Grade**: F — REJECT, REWRITE REQUIRED

## Critical Findings (8)

| # | Finding | Location | Phase |
|---|---------|----------|-------|
| C1 | `unwrap_or(Value::Null)` silently swallows JSON parse errors | runner.rs:207 | 3 |
| C2 | Duplicate `ScenarioResult` across crates with different shapes | runner.rs:41 vs metrics.rs:11 | 1 |
| C3 | Crate excluded from workspace — CI is blind | Cargo.toml:3 | 1 |
| C4 | Hardcoded `step_index: 0` in HTTP step results | runner.rs:164 | 2 |
| C5 | Silent serialization failure sends body-less request | runner.rs:192 | 3 |
| C6 | `ScenarioParseResult` defined but never constructed or returned | scenario.rs:113 | 1 |
| C7 | 3 of 4 `RunnerError` variants never returned | runner.rs:416-429 | 1 |
| C8 | 5 of 6 `ScenarioError` variants never returned | scenario.rs:140-158 | 1 |

## Major Findings (8)

| # | Finding | Location | Phase |
|---|---------|----------|-------|
| M1 | `blocks_scenario_access()` ignores level, always returns true | sanitizer.rs:277 | 4 |
| M2 | `sanitize_value` leaks paths and special characters | sanitizer.rs:258 | 4 |
| M3 | No newtypes — all domain strings are raw `String` | scenario.rs, runner.rs | 3 |
| M4 | Boolean parameter `follow_redirects` | runner.rs:63 | 3 |
| M5 | `run_with_sanitized_feedback` mutates runner state unnecessarily | runner.rs:404 | 4 |
| M6 | 7 unused dependencies in Cargo.toml | Cargo.toml:11-25 | 4 |
| M7 | Step ordering has no type-level guarantees | scenario.rs:31 | 4 |
| M8 | Template resolution silently passes unresolvable variables | runner.rs:390-394 | 3 |

## Minor Findings (5)

| # | Finding | Location | Phase |
|---|---------|----------|-------|
| m1 | `evaluate_assertion` has copy-pasted arms | runner.rs:344-381 | 2 |
| m2 | Tests create unused runner instances | runner.rs:437,449,458 | 2 |
| m3 | `#[allow(dead_code)]` on config field | runner.rs:81 | 5 |
| m4 | `config.twin_url` stored but never read after construction | runner.rs:58-65 | 5 |
| m5 | Tests assert implementation details not behavior | scenario.rs:194-229 | 2 |

## Summary

This crate is a prototype promoted to a crate without maturation. Dead types, dead error variants, dead dependencies, a duplicate type across crates, a critical error-swallowing bug, security theater sanitization, zero integration tests, and excluded from CI. The 5-level information barrier concept is sound but implementation is deeply flawed. Requires ground-up rewrite.
