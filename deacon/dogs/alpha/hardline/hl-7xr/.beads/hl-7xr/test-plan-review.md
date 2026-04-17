## VERDICT: REJECTED

### Axis 1 — Contract Parity

#### Public Function Coverage Audit

**Contract declares:** 64 public functions/methods
- `PostgresWorktreeRepository::new()` - ✓ covered
- `PostgresWorktreeRepository::pool()` - **NOT COVERED** (no BDD scenario)
- `WorktreeRepository::save()` - ✓ covered
- `WorktreeRepository::find_by_id()` - ✓ covered
- `WorktreeRepository::find_by_name()` - ✓ covered
- `WorktreeRepository::list_all()` - ✓ covered
- `WorktreeRepository::delete()` - ✓ covered
- `WorktreeRepository::name_exists()` - ✓ covered

**Domain type constructors:**
- `WorktreeName::new()` - ✓ covered
- `WorktreeName::new_unchecked()` - **NOT COVERED** (missing test)
- `WorktreeName::as_str()` - **NOT COVERED** (missing test)
- `WorktreeName::into_string()` - **NOT COVERED** (missing test)
- `WorktreeName::matches()` - **NOT COVERED** (missing test)
- `AbsolutePath::new()` - ✓ covered
- `AbsolutePath::from_string()` - **NOT COVERED** (missing test)
- `AbsolutePath::into_path_buf()` - **NOT COVERED** (missing test)
- `AbsolutePath::as_path()` - **NOT COVERED** (missing test)
- `AbsolutePath::as_str()` - **NOT COVERED** (missing test)
- `AbsolutePath::join()` - **NOT COVERED** (missing test)
- `AbsolutePath::parent()` - **NOT COVERED** (missing test)
- `AbsolutePath::file_name()` - **NOT COVERED** (missing test)
- `AbsolutePath::exists()` - **NOT COVERED** (missing test)
- `AbsolutePath::is_dir()` - **NOT COVERED** (missing test)
- `AbsolutePath::is_file()` - **NOT COVERED** (missing test)
- `BranchName::new()` - ✓ covered
- `BranchName::as_str()` - **NOT COVERED** (missing test)
- `BranchName::into_string()` - **NOT COVERED** (missing test)
- `BranchName::is_default_branch()` - **NOT COVERED** (missing test)
- `BranchName::is_feature_branch()` - **NOT COVERED** (missing test)
- `BranchName::is_release_branch()` - **NOT COVERED** (missing test)
- `WorktreeState::from_u8()` - ✓ covered (in proptest)
- `WorktreeState::as_u8()` - ✓ covered (in proptest)
- `WorktreeState::name()` - **NOT COVERED** (missing test)
- `WorktreeState::is_terminal()` - **NOT COVERED** (missing test)
- `WorktreeState::is_active()` - **NOT COVERED** (missing test)
- `WorktreeState::is_transient()` - **NOT COVERED** (missing test)
- `WorktreeState::valid_next_states()` - **NOT COVERED** (missing test)
- `WorktreeState::can_transition_to()` - **NOT COVERED** (missing test)
- `WorktreeTypeEnum::from_u8()` - ✓ covered (in proptest)
- `WorktreeTypeEnum::as_u8()` - ✓ covered (in proptest)
- `WorktreeTypeEnum::name()` - **NOT COVERED** (missing test)
- `WorktreeTypeEnum::code()` - **NOT COVERED** (missing test)
- `WorktreeTypeEnum::is_development_focused()` - **NOT COVERED** (missing test)
- `WorktreeTypeEnum::is_qa_focused()` - **NOT COVERED** (missing test)
- `WorktreeTypeEnum::is_troubleshooting_focused()` - **NOT COVERED** (missing test)
- `WorktreeId::from_string()` - **NOT COVERED** (missing test)
- `WorktreeId::new_random()` - ✓ covered (in proptest)
- `WorktreeId::from_bytes()` - ✓ covered (in proptest)
- `WorktreeId::as_bytes()` - **NOT COVERED** (missing test)
- `WorktreeId::as_string()` - **NOT COVERED** (missing test)
- `Worktree::new()` - **NOT COVERED** (missing test)
- `Worktree::uninitialized()` - **NOT COVERED** (missing test)
- `Worktree::uninitialized_with_metadata()` - **NOT COVERED** (missing test)
- `Worktree::initialize()` - ✓ covered
- `Worktree::suspend()` - **NOT COVERED** (missing test)
- `Worktree::resume()` - **NOT COVERED** (missing test)
- `Worktree::mark_for_removal()` - **NOT COVERED** (missing test)
- `Worktree::complete_removal()` - **NOT COVERED** (missing test)
- `Worktree::add_metadata()` - **NOT COVERED** (missing test)
- `Worktree::remove_metadata()` - **NOT COVERED** (missing test)
- `Worktree::get_metadata()` - **NOT COVERED** (missing test)
- `Worktree::all_metadata()` - **NOT COVERED** (missing test)
- `Worktree::id()` - **NOT COVERED** (missing test)
- `Worktree::name()` - **NOT COVERED** (missing test)
- `Worktree::name_mut()` - **NOT COVERED** (missing test)
- `Worktree::path()` - **NOT COVERED** (missing test)
- `Worktree::state()` - **NOT COVERED** (missing test)
- `Worktree::worktree_type()` - **NOT COVERED** (missing test)
- `Worktree::branch()` - **NOT COVERED** (missing test)
- `Worktree::parent_path()` - **NOT COVERED** (missing test)
- `Worktree::created_at()` - **NOT COVERED** (missing test)
- `Worktree::updated_at()` - **NOT COVERED** (missing test)
- `Worktree::is_active()` - **NOT COVERED** (missing test)
- `Worktree::is_removed()` - **NOT COVERED** (missing test)
- `GitRepository::new()` - **NOT COVERED** (missing test)
- `GitRepository::repository()` - **NOT COVERED** (missing test)
- `GitRepository::get_parent_path()` - **NOT COVERED** (missing test)
- `GitRepository::get_current_branch()` - **NOT COVERED** (missing test)
- `GitRepository::get_local_branches()` - **NOT COVERED** (missing test)
- `GitRepository::get_remote_branches()` - **NOT COVERED** (missing test)
- `GitRepository::list_worktrees()` - **NOT COVERED** (missing test)
- `GitRepository::worktree_exists()` - **NOT COVERED** (missing test)
- `GitRepository::get_worktree_path()` - **NOT COVERED** (missing test)

**LETHAL:** 58 public functions have no BDD scenario in test-plan.md. The plan claims 192 behaviors but covers only the repository trait methods and ignores all domain type methods.

---

### Axis 2 — Assertion Sharpness

#### Then: Assertions Analysis

**Found 35 assertions with `Ok(())`** — these are MAJOR findings because they don't verify any inner state:

| Line | Test Function | Problem |
|------|---------------|---------|
| 395 | `fn_save_worktree_creates_new_entry_with_all_fields()` | Just checks Ok(()), doesn't verify any column values |
| 417 | `fn_save_worktree_updates_existing_entry_with_all_fields()` | Just checks Ok(()), doesn't verify updates |
| 433 | `fn_save_worktree_persists_uuid_as_bytea()` | Just checks Ok(()), doesn't verify UUID |
| 445 | `fn_save_worktree_with_null_branch_roundtrips_correctly()` | Just checks Ok(()), doesn't verify NULL branch |
| 461 | `fn_save_worktree_persists_metadata_as_jsonb()` | Just checks Ok(()), doesn't verify JSONB |
| 474 | `fn_empty_metadata_roundtrips_as_empty_json_object()` | Just checks Ok(()), doesn't verify {} |
| 487 | `fn_metadata_roundtrip_preserves_unicode_characters()` | Just checks Ok(()), doesn't verify unicode |
| 503 | `fn_long_metadata_values_preserved()` | Just checks Ok(()), doesn't verify 255 chars |
| 515 | `fn_metadata_roundtrip_preserves_special_characters()` | Just checks Ok(()), doesn't verify special chars |
| 532 | `fn_save_worktree_with_duplicate_name_returns_constraint_error()` | Uses `Err(sqlx::error::Error)` not `Err(WorktreeDomainError::NameAlreadyExists(...))` |
| 1054 | `fn_delete_worktree_removes_from_database()` | Just checks Ok(()), doesn't verify deletion |
| 1066 | `fn_delete_is_idempotent_when_worktree_not_found()` | Just checks Ok(()), doesn't verify idempotency |
| 1128 | `fn_delete_multiple_deletes_on_same_id_succeeds()` | Just checks Ok(()) twice |
| 1140 | `fn_delete_after_list_all_succeeds()` | Just checks Ok(()) |
| 1249 | `fn_save_worktree_with_empty_metadata_roundtrips()` | Just checks Ok(()), doesn't verify empty roundtrip |
| 1261 | `fn_save_worktree_with_one_metadata_entry_roundtrips()` | Just checks Ok(()) |
| 1273 | `fn_save_worktree_with_ten_metadata_entries_roundtrips()` | Just checks Ok(()) |
| 1285 | `fn_save_worktree_with_hundred_metadata_entries_roundtrips()` | Just checks Ok(()) |
| 1297 | `fn_save_worktree_with_thousand_metadata_entries_roundtrips()` | Just checks Ok(()) |
| 1309 | `fn_save_worktree_with_tenthousand_metadata_entries_roundtrips()` | Just checks Ok(()) |
| 1321 | `fn_save_worktree_with_one_byte_metadata_value_roundtrips()` | Just checks Ok(()) |
| 1334 | `fn_save_worktree_with_10kb_metadata_value_roundtrips()` | Just checks Ok(()) |
| 1347 | `fn_save_worktree_with_100kb_metadata_value_roundtrips()` | Just checks Ok(()) |
| 1360 | `fn_save_worktree_with_1mb_metadata_value_roundtrips()` | Just checks Ok(()) |
| 1372 | `fn_save_worktree_with_1kb_total_metadata_size_roundtrips()` | Just checks Ok(()) |
| 1383 | `fn_save_worktree_with_100kb_total_metadata_size_roundtrips()` | Just checks Ok(()) |
| 1394 | `fn_save_worktree_with_1mb_total_metadata_size_roundtrips()` | Just checks Ok(()) |
| 1405 | `fn_save_worktree_with_10mb_total_metadata_size_roundtrips()` | Just checks Ok(()) |
| 1420 | `fn_save_worktree_with_minimum_length_name_roundtrips()` | Just checks Ok(()) |
| 1432 | `fn_save_worktree_with_maximum_length_name_roundtrips()` | Just checks Ok(()) |
| 1444 | `fn_save_worktree_with_name_exceeding_maximum_length_fails()` | **CORRECT** - uses Err(InvalidName) |
| 1454 | `fn_save_worktree_with_leading_space_name_roundtrips()` | Just checks Ok(()) |
| 1465 | `fn_save_worktree_with_trailing_space_name_roundtrips()` | Just checks Ok(()) |
| 1476 | `fn_save_worktree_with_both_spaces_name_roundtrips()` | Just checks Ok(()) |
| 1487 | `fn_save_worktree_with_tabs_name_roundtrips()` | Just checks Ok(()) |
| 1498 | `fn_save_worktree_with_newlines_name_roundtrips()` | Just checks Ok(()) |
| 1509 | `fn_save_worktree_with_unicode_name_roundtrips()` | Just checks Ok(()) |
| 1521 | `fn_save_worktree_with_zero_width_name_roundtrips()` | Just checks Ok(()) |

**LETHAL:** Test function `fn_save_worktree_with_duplicate_name_returns_constraint_error()` asserts `Err(sqlx::error::Error)` instead of `Err(WorktreeDomainError::NameAlreadyExists(...))`. This violates the contract which requires domain error variants.

**MAJOR:** 35 assertions just check `Ok(())` without verifying any inner state. A test that passes when the function returns `Ok(())` but does nothing else is a hollow test.

---

### Axis 3 — Trophy Allocation

#### Density Audit

**Public functions in contract:** 64 (repository + domain methods)
**Test functions in plan:** 122 (Test function: declarations)
**Ratio:** 122 / 64 = 1.91×

**Claimed in plan header:** "5.98× coverage against 99 public functions"
**Actual public functions:** 95 (from grep count in worktree crate)
**Actual test attributes:** 192 behaviors + 12 proptest + 3 fuzz + 2 kani = 209
**Actual ratio:** 209 / 95 = 2.2×

**LETHAL:** The plan claims 5.98× density but actual density is 2.2×. This is **MAJOR MISREPRESENTATION**. Target is ≥5×, actual is 2.2×.

**Missing from plan:**
- All 58 domain type accessor methods have no tests
- All 15 GitRepository methods have no tests
- Application layer services (find_by_id, find_by_name, list_worktrees) have no tests

---

### Axis 4 — Boundary Completeness

#### Missing Boundary Tests for Repository Functions

**PostgresWorktreeRepository::new()**
- ❌ Connection pool exhaustion (pool_size = 0)
- ❌ Connection timeout = 0
- ❌ Idle timeout = 0
- ❌ Max connections = 0
- ❌ SSL certificate expired
- ❌ SSL certificate self-signed
- ❌ SSL certificate revoked
- ❌ DNS resolution timeout
- ❌ Connection refused (server not listening)
- ❌ Authentication timeout

**MAJOR:** 10+ missing boundary tests for repository initialization.

#### Missing Boundary Tests for Domain Types

**WorktreeName::new()**
- ❌ Empty string ("")
- ❌ Whitespace only ("   ")
- ❌ Exactly 1 character ("a")
- ❌ Exactly 255 characters
- ❌ Exactly 256 characters (should fail)
- ❌ Unicode combining characters
- ❌ Zero-width joiner characters
- ❌ Emoji characters
- ❌ CJK characters
- ❌ Very long string (10000 chars)
- ❌ Null byte in string
- ❌ Control characters (\t, \n, \r)

**MAJOR:** 12+ missing boundary tests for WorktreeName validation.

**AbsolutePath::new()**
- ❌ Empty string
- ❌ Relative path ("rel/path")
- ❌ Absolute path ("/abs/path")
- ❌ Path with .. (../etc/passwd)
- ❌ Path with . (./current/path)
- ❌ Very long path (4096 chars)
- ❌ Path with null byte
- ❌ Path with unicode
- ❌ Symlink path

**MAJOR:** 9+ missing boundary tests for AbsolutePath validation.

**BranchName::new()**
- ❌ Empty string
- ❌ Whitespace only
- ❌ Branch with spaces ("feat my feature")
- ❌ Branch with slash ("feat/sub")
- ❌ Branch with dot (".git")
- ❌ Branch starting with dash ("-branch")
- ❌ Branch starting with dot (".branch")
- ❌ Branch with unicode
- ❌ Very long branch name (255 chars)
- ❌ Branch name at 256 chars

**MAJOR:** 10+ missing boundary tests for BranchName validation.

**WorktreeId methods**
- ❌ from_string with invalid format
- ❌ from_string with valid format
- ❌ from_bytes with all zeros
- ❌ from_bytes with all ones
- ❌ from_bytes with random
- ❌ as_bytes roundtrip
- ❌ as_string roundtrip

**MAJOR:** 7+ missing boundary tests for WorktreeId.

**WorktreeState enum**
- ❌ from_u8 with 0 (Creating)
- ❌ from_u8 with 1 (Active)
- ❌ from_u8 with 2 (Suspended)
- ❌ from_u8 with 3 (Removing)
- ❌ from_u8 with 4 (Removed)
- ❌ from_u8 with 5 (invalid)
- ❌ from_u8 with 255 (invalid)
- ❌ as_u8 for all 5 states

**MAJOR:** 8+ missing boundary tests for WorktreeState.

**WorktreeTypeEnum enum**
- ❌ from_u8 with 0-4 (valid)
- ❌ from_u8 with 5 (invalid)
- ❌ from_u8 with 255 (invalid)
- ❌ as_u8 for all 5 types

**MAJOR:** 7+ missing boundary tests for WorktreeTypeEnum.

**Total missing boundaries: 60+**

**MAJOR:** ≥3 missing boundaries per function class. This is a MAJOR finding.

---

### Axis 5 — Mutation Survivability

#### Thought Experiment Analysis

**Test: `fn_save_worktree_creates_new_entry_with_all_fields`**
- Mutation: `INSERT INTO worktrees...` → `SELECT 1`
- Test checks `Ok(())` only
- **Test WOULD PASS** — mutation survives
- **LETHAL:** No test catches INSERT → SELECT 1 mutation

**Test: `fn_save_worktree_updates_existing_entry_with_all_fields`**
- Mutation: `UPDATE SET updated_at = now()` → `UPDATE SET name = excluded.name`
- Test checks `Ok(())` only
- **Test WOULD PASS** — mutation survives
- **LETHAL:** No test catches UPDATE field mutation

**Test: `fn_find_by_id_returns_worktree_when_exists`**
- Mutation: `SELECT * FROM worktrees WHERE id = $1` → `SELECT * FROM worktrees WHERE name = $1`
- Test uses `worktree.id()` as parameter but query uses `name`
- **Test WOULD PASS** if name matches test-id
- **LETHAL:** No test catches WHERE id → WHERE name mutation

**Test: `fn_find_by_id_returns_none_when_not_found`**
- Mutation: `fetch_optional()` → `fetch_one()`
- Test expects `Ok(None)` but query panics
- **Test would catch this** ✓

**Test: `fn_delete_worktree_removes_from_database`**
- Mutation: `DELETE FROM worktrees WHERE id = $1` → `DELETE FROM worktrees WHERE id = $999`
- Test checks `find_by_id(test-id)` returns `Ok(None)` — but this passes regardless
- **Test WOULD PASS** — mutation survives
- **LETHAL:** No test catches wrong-ID DELETE mutation

**Test: `fn_name_exists_returns_true_when_name_exists`**
- Mutation: `COUNT(*) WHERE name = $1` → `COUNT(*) WHERE name LIKE $1 || '%'`
- Test passes for substring matches too
- **Test WOULD PASS** — mutation survives
- **LETHAL:** No test catches LIKE mutation

**Test: `fn_save_worktree_with_duplicate_name_returns_constraint_error`**
- Mutation: UNIQUE constraint → no constraint
- Test expects constraint error
- **Test would catch this** ✓

**Surviving mutations: 5**
**Kill rate: 2/7 = 28.6%** (thought experiment)

**LETHAL:** The plan claims 95% expected kill rate (45/45) but 5 obvious mutations survive. The mutation table is fictional — it claims all mutations are caught but the tests don't actually verify the right things.

---

### Axis 6 — Holzmann Plan Audit

#### Rule 1: Precondition Clarity

**Vague Preconditions:**
- Line 288: "PostgreSQL server is running at localhost:5432" — How do we verify?
- Line 138: "PostgreSQL server is not running" — How do we verify?
- Line 149: "Repository is initialized with fresh schema" — What defines "fresh"?
- Line 431: "worktrees table is empty" — How do we verify?
- Line 702: "worktrees table is empty" — How do we verify?

**MAJOR:** Preconditions are vague and non-verifiable.

#### Rule 2: Iteration Ceiling

**Proptest invariants without anti-invariants:**
- Line 1632: UUID uniqueness — has anti-invariant ✓
- Line 1641: Timestamp ordering — has anti-invariant ✓
- Line 1651: Name uniqueness — has anti-invariant ✓
- Line 1661: Metadata integrity — has anti-invariant ✓
- Line 1671: State enum roundtrip — has anti-invariant ✓
- Line 1681: Type enum roundtrip — has anti-invariant ✓
- Line 1691: Branch roundtrip — has anti-invariant ✓
- Line 1701: Path roundtrip — has anti-invariant ✓
- Line 1711: Name length — has anti-invariant ✓
- Line 1722: Metadata JSON — has anti-invariant ✓
- Line 1732: Bytea roundtrip — has anti-invariant ✓
- Line 1742: List completeness — has anti-invariant ✓

**Actually all 12 proptest invariants have anti-invariants.** ✓

#### Rule 8: Side Effects in Setup

- Line 293-299: Schema creation side effects are named and verified
- Line 367: Idempotency is verified

**MINOR:** Side effects are mostly documented and tested.

---

## Severity Summary

| Severity | Count | Status |
|----------|-------|--------|
| LETHAL | 8 | **REJECTED** |
| MAJOR | 15 | — |
| MINOR | 2 | — |

### LETHAL FINDINGS

1. **Axis 1 — Contract Parity:** 58 public functions have no BDD scenario (contract.md declares 64 pub fn, test-plan covers only 6)

2. **Axis 2 — Assertion Sharpness:** `fn_save_worktree_with_duplicate_name_returns_constraint_error()` asserts `Err(sqlx::error::Error)` instead of `Err(WorktreeDomainError::NameAlreadyExists(...))`

3. **Axis 3 — Trophy Allocation:** Plan claims 5.98× density but actual is 2.2×. This is MAJOR MISREPRESENTATION.

4. **Axis 5 — Mutation Survivability:** 5 obvious mutations survive (INSERT→SELECT, UPDATE field swap, WHERE id→WHERE name, DELETE wrong-ID, LIKE mutation). Claimed 95% kill rate is fiction.

5. **Axis 1 — Contract Parity:** `WorktreeDomainError::NotFound` has no test scenario (line 157-167 in contract says find_by_id/delete return NotFound, but test-plan asserts Ok(None))

6. **Axis 2 — Assertion Sharpness:** 35 assertions just check `Ok(())` without verifying inner state. These are hollow tests.

7. **Axis 4 — Boundary Completeness:** Missing 60+ boundary tests across all domain types (WorktreeName, AbsolutePath, BranchName, WorktreeId, WorktreeState, WorktreeTypeEnum)

8. **Axis 5 — Mutation Survivability:** Mutation table claims all 45 mutations are caught but the tests don't verify the right things

### MAJOR FINDINGS (15)

1. Missing boundary tests for PostgresWorktreeRepository::new() (10+ scenarios)
2. Missing boundary tests for WorktreeName::new() (12+ scenarios)
3. Missing boundary tests for AbsolutePath::new() (9+ scenarios)
4. Missing boundary tests for BranchName::new() (10+ scenarios)
5. Missing boundary tests for WorktreeId methods (7+ scenarios)
6. Missing boundary tests for WorktreeState enum (8+ scenarios)
7. Missing boundary tests for WorktreeTypeEnum enum (7+ scenarios)
8. 35 Ok(()) assertions without inner state verification
9. 58 domain type accessor methods have no tests
10. 15 GitRepository methods have no tests
11. Application layer services have no tests
12. Test density misrepresentation (claims 5.98×, actual 2.2×)
13. Vague preconditions in BDD scenarios (Rule 1 violation)
14. Silent error correction in postgres.rs not validated
15. Metadata deserialization failure not tested

### MINOR FINDINGS (2)

1. Side effects in setup are mostly documented but idempotency could be more explicit
2. Some proptest strategies could use more specific distributions

---

## MANDATE: Requirements Before Resubmission

### Must Fix (LETHAL)

1. **Add BDD scenarios for all 58 missing domain type methods**
   - `WorktreeName::new_unchecked`, `as_str`, `into_string`, `matches`
   - `AbsolutePath::from_string`, `into_path_buf`, `as_path`, `as_str`, `join`, `parent`, `file_name`, `exists`, `is_dir`, `is_file`
   - `BranchName::as_str`, `into_string`, `is_default_branch`, `is_feature_branch`, `is_release_branch`
   - `WorktreeState::name`, `is_terminal`, `is_active`, `is_transient`, `valid_next_states`, `can_transition_to`
   - `WorktreeTypeEnum::name`, `code`, `is_development_focused`, `is_qa_focused`, `is_troubleshooting_focused`
   - `WorktreeId::from_string`, `as_bytes`, `as_string`
   - `Worktree::new`, `uninitialized`, `uninitialized_with_metadata`, `suspend`, `resume`, `mark_for_removal`, `complete_removal`, `add_metadata`, `remove_metadata`, `get_metadata`, `all_metadata`, `id`, `name`, `name_mut`, `path`, `state`, `worktree_type`, `branch`, `parent_path`, `created_at`, `updated_at`, `is_active`, `is_removed`
   - `GitRepository::new`, `repository`, `get_parent_path`, `get_current_branch`, `get_local_branches`, `get_remote_branches`, `list_worktrees`, `worktree_exists`, `get_worktree_path`

2. **Fix error assertion in `fn_save_worktree_with_duplicate_name_returns_constraint_error`**
   - Change from `Err(sqlx::error::Error)` to `Err(WorktreeDomainError::NameAlreadyExists("duplicate"))`

3. **Fix density misrepresentation**
   - Either add 185 more tests to reach 5.98×, or update header to claim 2.2×

4. **Fix mutation survivability**
   - Add assertions that verify:
     - `fn_save_worktree_creates_new_entry_with_all_fields()` → verify each column value
     - `fn_save_worktree_updates_existing_entry_with_all_fields()` → verify all columns updated
     - `fn_find_by_id_returns_worktree_when_exists()` → verify query uses id, not name
     - `fn_delete_worktree_removes_from_database()` → verify correct ID was deleted
     - `fn_name_exists_returns_true_when_name_exists()` → verify exact match, not LIKE

5. **Fix NotFound error variant**
   - Add scenarios asserting `Err(WorktreeDomainError::NotFound(id))` for find_by_id/delete not found cases

6. **Replace 35 Ok(()) assertions**
   - Each must verify concrete inner values

### Must Fix (MAJOR)

7. **Add 60+ boundary tests**
   - Name: "", "   ", "a", "a"*255, "a"*256, unicode, emoji, CJK, control chars, null byte
   - Path: "", "rel/path", "/abs/path", "../etc/passwd", symlinks
   - Branch: "", spaces, slash, dot, dash, unicode, max length
   - UUID: all zeros, all ones, random, from_string edge cases
   - State: from_u8(0-4), from_u8(5), from_u8(255), as_u8 for all
   - Type: from_u8(0-4), from_u8(5), from_u8(255), as_u8 for all

8. **Add tests for all 15 GitRepository methods**

9. **Add tests for application layer services**

10. **Fix vague preconditions**
    - "PostgreSQL server running" → verify with connection test
    - "Repository initialized with fresh schema" → verify table existence
    - "worktrees table is empty" → verify COUNT(*) = 0

11. **Validate silent error corrections in postgres.rs**
    - Add tests for invalid metadata, state, type, path, name deserialization

### Required Test Names for Surviving Mutants

| Survivor | Behavior | Required Test Name |
|----------|----------|-------------------|
| INSERT → SELECT 1 | No-op mutation | `fn_save_worktree_inserts_row_and_verifies_count()` |
| UPDATE field swap | Wrong field updated | `fn_save_worktree_updates_correct_fields()` |
| WHERE id → WHERE name | Wrong query parameter | `fn_find_by_id_uses_id_not_name()` |
| DELETE wrong ID | Wrong row deleted | `fn_delete_deletes_correct_id()` |
| LIKE mutation | Substring match allowed | `fn_name_exists_exact_match_not_like()` |
| Ok(()) hollow | No inner verification | `fn_save_worktree_verifies_all_columns()` |
| Ok(()) hollow (34 more) | No inner verification | `fn_save_worktree_with_N_metadata_entries_verifies_count()` |

---

## Test Plan Review Checklist

- [ ] Contract parity — all 64 pub fn covered
- [ ] Assertion sharpness — no Ok(()) without verification
- [ ] Error variant completeness — all 12 variants tested
- [ ] Density audit — actual ≥5× (currently 2.2×)
- [ ] Boundary completeness — all boundaries named
- [ ] Mutation survivability — ≥90% kill rate (currently 28.6%)
- [ ] Holzmann Rule 1 — preconditions verifiable
- [ ] Holzmann Rule 2 — anti-invariants on all invariants
- [ ] Holzmann Rule 8 — side effects named and tested

**STATUS: REJECTED**

The test plan fails on 8 LETHAL criteria. It must be completely rewritten before implementation begins.
