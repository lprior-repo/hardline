# Test Plan: CLI Config Command (hl-0g4)

## Summary

- Behaviors: 62 | Trophy: 22 unit / 30 integration / 10 e2e
- Proptests: 6 | Fuzz targets: 5 | Kani harnesses: 3
- Density: 62 / 12 public functions = 5.2x

**IMPLEMENTATION RULE (Holzmann Rule 2)**: All multi-input scenarios in this
plan MUST be implemented as separate `#[test]` functions or `#[rstest]`
parameterized tests. NO loops (`for`, `while`, `iter`) in test bodies. Each
row in a multi-input table below becomes its own `#[rstest]` case.

---

## 1. Behavior Inventory

**ConfigKey::try_from()** (12): accepts valid 2+ segment dot paths / rejects empty / rejects single-segment / rejects non-ASCII / rejects chars outside `[a-zA-Z0-9_]` / rejects path-traversal (`/`,`\`,`..`,null,newline) / rejects >256 chars / rejects leading/trailing/consecutive dots / rejects keys not in Config struct schema / accepts underscored segments / accepts at max length (256) / accepts minimal segments (1-char each, "a.b")

**parse_cli_value()** (14): infers `"true"`/`"false"` as TOML bool (case-sensitive) / infers i64-parseable strings as TOML integer / infers `["a","b"]` as string array / falls back to TOML string for all else / rejects arrays with non-string elements / overflow i64 falls back to string (not error) / empty array `[]` accepted / empty string falls to string / i64::MAX as integer / i64::MIN as integer / single-element array / array with empty string element / malformed array rejected / whitespace-padded bool is string

**ConfigScope precedence** (4): env > project > global > defaults / env overrides project+global / project overrides global / defaults with exact value when no config

**ConfigReadPort trait** (6): load_merged returns merged Config / load_merged with missing global file / load_merged with invalid TOML in project file / load_global_only returns Config without project/env / global_config_path returns valid PathBuf / project_config_path returns Err when no project

**File locking** (4): exclusive lock before read-modify-write / lock timeout after 5s / lock released on failure (RAII Drop) / lock verified held during write via concurrent read

**TOML round-trip** (2): valid TOML after every set with exact key-value / types preserved with exact type assertions

**Error taxonomy** (8): ConfigKeyNotFound(40) / ConfigParseError(41) / ConfigWriteError(42) / ConfigScopeError(43) / ConfigLockError(44) / NotFound(40) / Invalid(41) / Permission(42)

**Env scope read-only** (2): config_set rejects Env scope / config_get returns empty source_path for env

**Nested value ops** (5): get_nested_value retrieves leaf via dot notation / returns ConfigKeyNotFound for missing segments / set_nested_value creates intermediate tables / rejects traversal through non-table value / single-segment parts rejected

**config_get direct** (3): returns all ConfigGetResult fields / key stability (invariant #8) / source_path populated for file scopes

**config_list** (4): list all keys sorted alpha with exact expected set / global_only=true returns only global keys / empty config returns empty list / single key returns one entry

**Legacy error variants** (3): NotFound variant with exit code 40 / Invalid variant with exit code 41 / Permission variant with exit code 42

**Command dispatch** (5): list when no args / get when key only / set when key+value / reject value without key / exit codes match contract

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Static | -- | clippy pedantic, cargo-deny, `#![forbid(unsafe_code)]` |
| Unit | 22 | ConfigKey validation, parse_cli_value inference, error construction, nested value ops, legacy error variants |
| Integration | 30 | ConfigManager with real tmpfiles, save+reload, file locking, scope precedence, env vars, ConfigReadPort, config_get direct, config_list |
| E2E | 10 | CLI process-boundary: get/set/list, exit codes, legacy error exits |

Deviation: integration at 48% vs trophy ideal 60% because this contract introduces many pure Calc functions (ConfigKey, parse_cli_value) that demand exhaustive unit boundary coverage.

---

## 3. BDD Scenarios

### 3.1 ConfigKey::try_from()

| Test Function | Given | When | Then |
|---|---|---|---|
| `config_key_accepts_two_segment_key` | `"watch.enabled"` | `try_from` | Ok, segments==["watch","enabled"], raw=="watch.enabled" |
| `config_key_accepts_multi_segment_key` | `"conflict_resolution.mode"` | `try_from` | Ok, segments==["conflict_resolution","mode"] |
| `config_key_accepts_minimal_segments` | `"a.b"` | `try_from` | Ok, segments==["a","b"], raw=="a.b" |
| `config_key_rejects_empty_string` | `""` | `try_from` | Err(ConfigParseError) msg contains "empty" |
| `config_key_rejects_single_segment` | `"nosection"` | `try_from` | Err(ConfigParseError) msg contains "dot" or "segment" |
| `config_key_rejects_non_ascii` | `"s\xe9.key"` | `try_from` | Err(ConfigParseError) msg contains "ASCII" or "non-ascii" |
| `config_key_rejects_hyphen` | `"my-key.val"` | `try_from` | Err(ConfigParseError) msg contains "invalid" or "character" |
| `config_key_rejects_space` | `"my key.val"` | `try_from` | Err(ConfigParseError) msg contains "invalid" or "character" |
| `config_key_rejects_path_traversal_dotdot` | `"a..b"` | `try_from` | Err(ConfigParseError) msg contains "consecutive" or "dot" |
| `config_key_rejects_path_traversal_slash` | `"../etc"` | `try_from` | Err(ConfigParseError) msg contains "invalid" or "traversal" |
| `config_key_rejects_backslash` | `"a\\b"` | `try_from` | Err(ConfigParseError) msg contains "invalid" or "character" |
| `config_key_rejects_null_byte` | `"k\x00.s"` | `try_from` | Err(ConfigParseError) msg contains "null" or "invalid" |
| `config_key_rejects_leading_dot` | `".k"` | `try_from` | Err(ConfigParseError) msg contains "leading" or "empty segment" |
| `config_key_rejects_trailing_dot` | `"k."` | `try_from` | Err(ConfigParseError) msg contains "trailing" or "empty segment" |
| `config_key_rejects_overlength` | 257-char valid key | `try_from` | Err(ConfigParseError) msg contains "256" or "length" |
| `config_key_accepts_at_max_length` | 256-char valid key | `try_from` | Ok, raw.len()==256 |
| `config_key_rejects_unknown_schema` | `"zzz.yyy"` | `try_from` | Err(ConfigParseError) msg contains "schema" or "unknown" or "not found" |

**NOTE**: Rows with specific single inputs (non_ascii, hyphen, space, dotdot, slash, backslash, null, leading_dot, trailing_dot) are each separate `#[rstest]` cases. NO loop iterating over a vec of inputs.

### 3.2 parse_cli_value()

| Test Function | Given | When | Then |
|---|---|---|---|
| `parse_cli_infers_bool_true` | `"true"` | `parse_cli_value` | Ok, item==bool(true) |
| `parse_cli_infers_bool_false` | `"false"` | `parse_cli_value` | Ok, item==bool(false) |
| `parse_cli_true_case_sensitive` | `"True"` | `parse_cli_value` | Ok, item==string("True") (not bool) |
| `parse_cli_false_case_sensitive` | `"FALSE"` | `parse_cli_value` | Ok, item==string("FALSE") (not bool) |
| `parse_cli_whitespace_bool_is_string` | `" true"` | `parse_cli_value` | Ok, item==string(" true") (leading space) |
| `parse_cli_infers_positive_int` | `"42"` | `parse_cli_value` | Ok, item==integer(42) |
| `parse_cli_infers_negative_int` | `"-100"` | `parse_cli_value` | Ok, item==integer(-100) |
| `parse_cli_i64_max` | `"9223372036854775807"` | `parse_cli_value` | Ok, item==integer(9223372036854775807) |
| `parse_cli_i64_min` | `"-9223372036854775808"` | `parse_cli_value` | Ok, item==integer(-9223372036854775808) |
| `parse_cli_overflow_falls_to_string` | `"99999999999999999999"` | `parse_cli_value` | Ok, item==string("99999999999999999999") |
| `parse_cli_infers_string_array` | `"[\"a\",\"b\"]"` | `parse_cli_value` | Ok, item==array["a","b"] |
| `parse_cli_single_element_array` | `"[\"only\"]"` | `parse_cli_value` | Ok, item==array["only"] |
| `parse_cli_array_with_empty_string` | `"[\"\"]"` | `parse_cli_value` | Ok, item==array[""] |
| `parse_cli_accepts_empty_array` | `"[]"` | `parse_cli_value` | Ok, item==empty array |
| `parse_cli_rejects_non_string_array` | `"[1,2]"` | `parse_cli_value` | Err(ConfigParseError) msg contains "string" or "non-string" |
| `parse_cli_rejects_malformed_array` | `"[\"a\","` | `parse_cli_value` | Err(ConfigParseError) msg contains "parse" or "malformed" or "TOML" |
| `parse_cli_falls_back_to_string` | `"hello world"` | `parse_cli_value` | Ok, item==string("hello world") |
| `parse_cli_empty_string_falls_to_string` | `""` | `parse_cli_value` | Ok, item==string("") |

### 3.3 ConfigReadPort Trait Methods

| Test Function | Given | When | Then | Layer |
|---|---|---|---|---|
| `port_load_merged_all_layers` | global.toml has watch.enabled=false, project.toml has watch.enabled=true, env SCP_WATCH_ENABLED=true | `load_merged()` | Ok(Config) where watch.enabled=="true" (env wins) | integ |
| `port_load_merged_missing_global` | no global file, project.toml exists with conflict.mode="Auto" | `load_merged()` | Ok(Config) where conflict.mode=="Auto" | integ |
| `port_load_merged_invalid_toml` | project.toml contains "bad [[toml{" | `load_merged()` | Err(ConfigParseError) msg contains "parse" or "TOML" or "invalid" | integ |
| `port_load_merged_env_only` | no files exist, env SCP_WATCH_ENABLED=true | `load_merged()` | Ok(Config) where watch.enabled=="true" from env, scope resolution shows Env | integ |
| `port_load_global_only_returns_no_project` | global.toml has watch.enabled=false, project.toml has watch.enabled=true | `load_global_only()` | Ok(Config) where watch.enabled=="false" (project ignored) | integ |
| `port_global_config_path_returns_valid` | HOME set to temp dir | `global_config_path()` | Ok(PathBuf) == tempdir/.config/scp/config.toml | integ |
| `port_project_config_path_returns_valid` | inside git repo with .scp/ | `project_config_path()` | Ok(PathBuf) == repo_root/.scp/config.toml | integ |
| `port_project_config_path_err_no_project` | outside any git repo, no project context | `project_config_path()` | Err(ConfigScopeError) msg contains "project" or "no project" | integ |

### 3.4 Scope Precedence, Locking, TOML, Errors, Nested, Direct Ops

| Test Function | Given | When | Then | Layer |
|---|---|---|---|---|
| `precedence_env_overrides_all` | global=false, project=true, env SCP_WATCH_ENABLED=true | `config_get("watch.enabled")` | Ok{value:"true",scope:Env,source_path:PathBuf::new()} | integ |
| `precedence_project_overrides_global` | global=false, project=true, no env | `config_get("watch.enabled")` | Ok{value:"true",scope:Project,source_path:project_path} | integ |
| `precedence_global_only` | global=false, no project, no env | `config_get("watch.enabled")` | Ok{value:"false",scope:Global,source_path:global_path} | integ |
| `precedence_defaults_when_no_config` | no files, no env | `config_get("watch.enabled")` | Ok{value:"false",scope:Global,source_path:default_path} | integ |
| `lock_acquired_on_write` | writable tmpdir + config.toml containing `# header\nwatch.enabled = false` | `config_set("watch.enabled","true",Global)` | Ok, file re-read => watch.enabled==true, header line `# header` preserved verbatim | integ |
| `lock_timeout_returns_error` | helper `hold_lock_for(path, 10s)` spawns thread holding exclusive lock | `config_set("watch.enabled","true",Global)` after 5s | Err(ConfigLockError) msg contains "timeout" or "5" or "lock" | integ |
| `lock_released_on_failure` | writable file with content "CORRUPT [[[toml" | `config_set` fails, then 2nd process acquires lock within 1s | 2nd lock acquisition succeeds within 1s | integ |
| `lock_verified_held_during_write` | writable tmpdir, concurrent reader thread | `config_set` in progress | reader thread cannot acquire shared lock until set completes | integ |
| `lock_retry_behavior` | helper `hold_lock_for(path, 500ms)` holds lock for 500ms | `config_set` with 5s timeout | Ok (succeeds after lock released), elapsed >= 400ms | integ |
| `toml_valid_after_set` | TOML file: `# top comment\n[watch]\nenabled = false\ninterval = 5` | `config_set("watch.enabled","true",Global)` | file parses as valid TOML; re-read => watch.enabled==true, watch.interval==5, `# top comment` line preserved | integ |
| `toml_types_preserved` | file: `[watch]\nenabled = true\ninterval = 5\nname = "test"\ntags = ["a","b"]` | `config_set("watch.name","updated",Global)` then reload | watch.enabled==bool(true), watch.interval==integer(5), watch.name==string("updated"), watch.tags==array["a","b"] | integ |
| `error_key_not_found` | valid syntax, absent key | `config_get("no.key")` | Err(ConfigKeyNotFound) msg contains "no.key", exit_code()==40 | integ |
| `error_parse_error` | invalid key syntax `"no dot"` | `config_get("no dot")` | Err(ConfigParseError) msg contains "dot" or "segment", exit_code()==41 | unit |
| `error_write_error` | read-only dir (chmod 444 on parent) | `config_set("watch.enabled","true",Global)` | Err(ConfigWriteError) msg contains read-only path, exit_code()==42 | integ |
| `error_scope_env_write` | scope==Env | `config_set(...,Env)` | Err(ConfigScopeError("Cannot save to environment scope")), exit_code()==43 | unit |
| `error_scope_no_project` | outside git repo, no project_path | `config_set(...,Project)` | Err(ConfigScopeError) msg contains "project" or "no project", exit_code()==43 | integ |
| `error_lock_timeout` | lock held >5s via `hold_lock_for(path, 10s)` | `config_set` | Err(ConfigLockError) msg contains "timeout" or "5s", exit_code()==44 | integ |
| `error_not_found_variant` | config dir cannot be determined (unset HOME, no XDG) | construct `NotFound("path")` | variant matches NotFound, display contains "path", exit_code()==40 | unit |
| `error_invalid_variant` | generic validation failure | construct `Invalid("bad config")` | variant matches Invalid, display contains "bad config", exit_code()==41 | unit |
| `error_permission_variant` | permission denied scenario | construct `Permission("/etc/config")` | variant matches Permission, display contains "/etc/config", exit_code()==42 | unit |
| `exit_codes_match_contract` | each variant | `exit_code()` | ConfigKeyNotFound=40, ConfigParseError=41, ConfigWriteError=42, ConfigScopeError=43, ConfigLockError=44, NotFound=40, Invalid=41, Permission=42 | unit |
| `env_scope_rejects_set` | ConfigScope::Env | `config_set` | Err(ConfigScopeError("Cannot save to environment scope")) | unit |
| `env_scope_empty_source_path` | SCP_WATCH_ENABLED=true, no file | `config_get("watch.enabled")` | Ok{scope:Env,source_path:PathBuf::new()} | integ |
| `config_get_direct_full_result` | global.toml: watch.enabled=true, no env | `config_get("watch.enabled",Global)` | Ok, result.key.raw=="watch.enabled" (invariant #8), result.value=="true", result.scope==Global, result.source_path==global_toml_path | integ |
| `config_get_key_stability` | any config state | `config_get("conflict_resolution.mode")` | result.key.raw == "conflict_resolution.mode" (exact input, no normalization) | integ |
| `config_list_all_sorted` | global.toml: watch.enabled=true, conflict.mode="Auto" | `config_list(global_only=false)` | Ok, list has exactly [ConfigGetResult{key:"conflict_resolution.mode",value:"Auto",...}, ConfigGetResult{key:"watch.enabled",value:"true",...}] (sorted alpha by key) | integ |
| `config_list_global_only` | global.toml: watch.enabled=true, project.toml: conflict.mode="Auto" | `config_list(global_only=true)` | Ok, list has exactly [ConfigGetResult{key:"watch.enabled",value:"true",scope:Global,...}] (no project keys) | integ |
| `config_list_empty` | no config files, no env | `config_list(global_only=false)` | Ok, list is empty Vec | integ |
| `config_list_single_key` | global.toml: only watch.enabled=true | `config_list(global_only=false)` | Ok, list.len()==1, list[0].key.raw=="watch.enabled", list[0].value=="true" | integ |
| `get_nested_returns_leaf` | Config with conflict.mode="Auto" | `get_nested_value(config,"conflict.mode")` | Ok("Auto") | unit |
| `get_nested_rejects_unknown` | Config with no "nonexistent" section | `get_nested_value(config,"nonexistent.key")` | Err(ConfigKeyNotFound) msg contains "nonexistent" or "not found" | unit |
| `get_nested_deep_path` | Config with a.b.c.d="deep_val" | `get_nested_value(config,"a.b.c.d")` | Ok("deep_val") | unit |
| `set_nested_creates_tables` | empty DocumentMut | `set_nested_value(doc,["new_sec","key"],"42")` | Ok, doc["new_sec"]["key"]==integer(42) | unit |
| `set_nested_rejects_non_table` | TOML where "watch" is string `"watch = \"hello\"` | `set_nested_value(doc,["watch","enabled"],"true")` | Err(ConfigParseError) msg contains "table" or "not a table" | unit |
| `set_nested_single_segment_rejected` | DocumentMut | `set_nested_value(doc,["key"],"val")` | Err(ConfigParseError) msg contains "empty" or "segment" or "non-empty" | unit |
| `run_lists_all` | config files: watch.enabled=true, conflict.mode="Auto" | `run{key:None,value:None}` | stdout contains exact lines "conflict_resolution.mode = Auto" and "watch.enabled = true" in alpha order | e2e |
| `run_gets_value` | watch.enabled=true | `run{key:Some("watch.enabled"),value:None}` | stdout contains "watch.enabled = true" | e2e |
| `run_sets_value` | writable config with watch.enabled=true | `run{key:Some("watch.enabled"),value:Some("false")}` | stdout contains "watch.enabled = false", file re-read => watch.enabled==false, other keys unchanged | e2e |
| `run_rejects_value_no_key` | `ConfigOptions{key:None,value:Some("v")}` | `run` | Err(ConfigParseError) msg contains "key" or "required", exit_code==41 | e2e |
| `cli_exit_codes` | various errors | CLI exits | codes: 40,41,42,43,44 | e2e |

---

## 4. Proptest Invariants

| Function | Invariant | Strategy | Anti-invariant |
|---|---|---|---|
| `ConfigKey::try_from` | Valid S => `try_from(S).unwrap().as_str() == S` | `[a-zA-Z0-9_]{1,50}` segments joined by `.`, len<=256, >=2 segments | Any char outside `[a-zA-Z0-9_.]` must fail |
| `parse_cli_value` bool | Only exact `"true"`/`"false"` produce TOML bool; all others do not | `proptest::string(".*")` | `"TRUE"`,`"False"`,`"true "` must not be bool |
| `parse_cli_value` int | i64-parseable S => integer value == S.parse::<i64>() | `proptest::num::<i64>().prop_map(\|n\| n.to_string())` | Overflow strings produce string, not integer |
| `get_nested_value` round-trip | For Config C, key K in schema: get_nested_value(C, K) never panics and returns Ok or Err(ConfigKeyNotFound) | random Config structs via serde_json + valid dot-notation keys from schema | Invalid keys always produce Err, never panic |
| `set_nested_value` round-trip | For valid TOML D, after set then `toml::from_str(D.to_string())` => valid TOML, K==V, other keys unchanged | random TOML docs + key/value pairs | Round-trip must never produce invalid TOML |
| Scope precedence | For any key with values in N scopes, resolved value == highest-precedence source | random value assignments across scope layers | Removing higher-precedence source changes resolved value |

---

## 5. Fuzz Targets

| Target | Input | Risk | Corpus Seeds |
|---|---|---|---|
| `ConfigKey::try_from` | &str (arbitrary UTF-8) | panic on null, 257-char, boundary | `""`,`"."`,`".."`,`"a.b"`,`"a..b"`,`"k\x00s"`,`"../t"`,257-char |
| `parse_cli_value` | &str | panic in array parse, overflow | `"true"`,`"0"`,`"[\"a\"]"`,`"[1,2]"`,`"[]"`,`"["`,`"[\""`,`""` |
| `get_nested_value` | (JSON bytes for Config, &str key) | panic on deep nesting, arrays, mixed types | `{"watch":{"enabled":true}}`+`"watch.enabled"`, `{}`+`"a.b.c"`, `{"a":[1,2]}`+`"a.0"` |
| `set_nested_value` | (Vec<String>, &str) | panic on empty segments, non-table | `([],)`, `(["a"],)`, `(["a","b"],"v")`, `([""],)` |
| TOML file parsing | &str raw contents | panic on malformed/binary | valid TOML, binary garbage, long lines, null bytes |

---

## 6. Kani Harnesses

| Harness | Property | Bound | Rationale |
|---|---|---|---|
| `config_key_no_panic` | `try_from` returns Ok or Err(ConfigParseError), never panics | all &str 0..=256 chars | DoS vector -- gateway to all config ops |
| `parse_cli_value_no_panic` | Returns Ok or Err, no unwrap/OOB/overflow | all &str 0..=512 chars | Untrusted CLI input must be panic-free |
| `scope_write_exhaustive` | `config_set` with Env always returns Err(ConfigScopeError) | 3 ConfigScope variants | Security boundary -- missed arm allows env writes |

---

## 7. Mutation Checkpoints

**Threshold: 95% kill rate minimum**

| Mutation Site | Caught By |
|---|---|
| ConfigKey empty-string early return | `config_key_rejects_empty_string` (msg check) |
| ConfigKey segment char validation | `config_key_rejects_hyphen`, `config_key_rejects_space`, `config_key_rejects_non_ascii` (msg checks distinguish each) |
| ConfigKey min-segment check (len<2) | `config_key_rejects_single_segment` |
| ConfigKey max-length boundary `>` vs `>=` | `config_key_accepts_at_max_length` + `config_key_rejects_overlength` |
| parse_cli "true"/"false" exact match | `parse_cli_infers_bool_true`, `parse_cli_true_case_sensitive` |
| parse_cli integer overflow check | `parse_cli_overflow_falls_to_string` (exact value "99999999999999999999") |
| parse_cli array bracket detection | `parse_cli_infers_string_array`, `parse_cli_rejects_non_string_array` (msg check) |
| ConfigScope::Env match arm | `env_scope_rejects_set` (exact msg) |
| Lock timeout comparison (>=5s) | `lock_timeout_returns_error` (msg check) |
| Lock acquisition skipped entirely | `lock_verified_held_during_write` |
| Lock retry loop removed | `lock_retry_behavior` (elapsed time >= 400ms) |
| TOML validation step (skip it) | `toml_valid_after_set` |
| Precedence ordering in load | `precedence_env_overrides_all`, `precedence_project_overrides_global` |
| Default value replaced with Default::default() | `precedence_defaults_when_no_config` (exact value "false") |
| ConfigGetResult.key normalization | `config_get_key_stability` (invariant #8) |
| set_nested intermediate-table check | `set_nested_rejects_non_table` (msg check) |
| exit_code match arms | `exit_codes_match_contract` (8 variants) |
| config_list sort removed/reversed | `config_list_all_sorted` (exact order assertion) |

---

## 8. Open Questions

1. **Schema registry**: Is known-keys a const array (`ConfigContracts::known_keys()`) or derived via serde? Tests need a concrete list.
2. **Multi-level env vars**: `SCP_SECTION_SUB_KEY` -> `section.sub.key` (3-segment). Supported? Tests assume yes.
3. **i64 overflow**: Contract says overflow stored as string (not error). Tests assert string fallback with exact value. Confirm.
4. **Kani in CI**: Advisory harnesses (not CI-gated)? Assumed yes.
5. **ConfigReadPort fake**: Integration tests use in-memory fake (hashmap of TOML by scope), not mockall. Confirm.
6. **Legacy error variants**: NotFound/Invalid/Permission are "preserved for backward compatibility". Tests assert construction + exit codes. If these are unreachable in new code, document rationale for keeping variants.
