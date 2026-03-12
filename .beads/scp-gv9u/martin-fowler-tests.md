# Martin Fowler Test Plan

## Overview

This test plan follows Martin Fowler's Given-When-Then pattern to specify behavior for the SQLite bead schema feature. Tests are organized into categories covering happy paths, error cases, edge cases, contract verification, and end-to-end scenarios.

---

## Happy Path Tests

### test_creates_schema_with_all_tables
Given: A valid SQLite connection pool
When: `ensure_schema` is called
Then:
- Returns `Ok(())`
- `beads` table exists with correct schema
- `bead_dependencies` table exists with correct schema
- `bead_state_history` table exists with correct schema
- All required indexes are created

### test_insert_bead_succeeds_with_valid_data
Given: Database with schema created, valid BeadIssue with all required fields
When: `insert_bead(pool, &bead)` is called
Then:
- Returns `Ok(())`
- Bead is persisted in database
- `get_bead(pool, bead.id)` returns the inserted bead

### test_update_bead_modifies_existing_record
Given: Database with existing bead from previous test
When: `update_bead(pool, id, &updated_bead)` is called with new title
Then:
- Returns `Ok(updated_bead)`
- `get_bead(pool, id)` returns bead with new title
- `updated_at` timestamp is newer than before

### test_delete_bead_removes_record
Given: Database with existing bead
When: `delete_bead(pool, id)` is called
Then:
- Returns `Ok(())`
- `get_bead(pool, id)` returns `Err(BeadsError::NotFound)`

### test_add_dependency_creates_relationship
Given: Two existing beads A and B
When: `add_dependency(pool, "A", "B")` is called
Then:
- Returns `Ok(())`
- `get_dependencies("B")` includes "A" in depends_on

### test_state_history_records_status_change
Given: Database with existing bead in "open" status
When: `update_bead` is called to change status to "in_progress"
Then:
- `get_history(bead_id)` returns at least one entry
- Latest entry shows old_value="open", new_value="in_progress"

### test_query_beads_returns_filtered_results
Given: Database with multiple beads in different statuses
When: `query_beads(pool, Some(BeadFilter { status: Some(IssueStatus::Open) }))` is called
Then:
- Returns only beads with status "open"

---

## Error Path Tests

### test_insert_bead_fails_with_empty_id
Given: Valid BeadIssue with empty ID
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("ID cannot be empty"))`

### test_insert_bead_fails_with_empty_title
Given: Valid BeadIssue with empty title
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("Title cannot be empty"))`

### test_insert_bead_fails_with_duplicate_id
Given: Database with existing bead "test-1"
When: `insert_bead(pool, &BeadIssue { id: "test-1", ... })` is called
Then: Returns `Err(BeadsError::DuplicateId("test-1"))`

### test_insert_bead_fails_closed_without_closed_at
Given: BeadIssue with status=Closed but closed_at=None
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("closed_at must be set when status is 'closed'"))`

### test_insert_bead_fails_invalid_priority
Given: BeadIssue with priority value outside valid range (e.g., 99)
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("Invalid priority value"))`

### test_add_dependency_fails_for_nonexistent_parent
Given: Database with one bead "existing-bead"
When: `add_dependency(pool, "nonexistent", "existing-bead")` is called
Then: Returns `Err(BeadsError::ValidationFailed("Dependency target does not exist: nonexistent"))`

### test_add_dependency_fails_circular_reference
Given: Database with beads A and B, and A depends_on B already exists
When: `add_dependency(pool, "B", "A")` is called (creating cycle A<-B<-A)
Then: Returns `Err(BeadsError::ValidationFailed("Circular dependency detected"))`

### test_get_bead_fails_not_found
Given: Database with no beads
When: `get_bead(pool, "nonexistent-id")` is called
Then: Returns `Err(BeadsError::NotFound("nonexistent-id"))`

### test_update_bead_fails_not_found
Given: Database with no beads
When: `update_bead(pool, "nonexistent-id", &bead)` is called
Then: Returns `Err(BeadsError::NotFound("nonexistent-id"))`

### test_delete_bead_fails_not_found
Given: Database with no beads
When: `delete_bead(pool, "nonexistent-id")` is called
Then: Returns `Err(BeadsError::NotFound("nonexistent-id"))`

### test_remove_dependency_fails_not_found
Given: No dependency exists between A and B
When: `remove_dependency(pool, "A", "B")` is called
Then: Returns `Err(BeadsError::NotFound(...))`

---

## Edge Case Tests

### test_handles_very_long_title
Given: BeadIssue with title of 10,000 characters
When: `insert_bead(pool, &bead)` is called
Then: Returns `Ok(())` and title is stored/retrieved correctly

### test_handles_unicode_in_title
Given: BeadIssue with Unicode title (e.g., "标题", "emoji 🎉")
When: `insert_bead(pool, &bead)` is called
Then: Returns `Ok(())` and title is stored/retrieved correctly

### test_handles_empty_metadata
Given: BeadIssue with metadata = None
When: `insert_bead(pool, &bead)` is called
Then: Returns `Ok(())` and metadata remains NULL in DB

### test_handles_complex_json_metadata
Given: BeadIssue with complex nested JSON metadata
When: `insert_bead(pool, &bead)` is called
Then: Returns `Ok(())` and metadata is stored/retrieved as valid JSON

### test_handles_many_dependencies
Given: Bead A that depends on 100 other beads
When: All dependencies are added and then `get_dependencies("A")` is called
Then: Returns all 100 dependency records

### test_handles_many_history_entries
Given: Bead that has undergone 1000 status changes
When: `get_history(bead_id)` is called
Then: Returns all 1000 history entries

### test_handles_concurrent_inserts
Given: Multiple tasks attempting to insert beads with different IDs simultaneously
When: All inserts execute concurrently
Then: All return `Ok(())`

### test_handles_idempotent_schema_creation
Given: Schema already exists in database
When: `ensure_schema(pool)` is called again
Then: Returns `Ok(())` without error

---

## Contract Verification Tests

### test_precondition_bead_id_nonempty
Given: BeadIssue with empty ID
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed(...))` - NOT a panic

### test_precondition_bead_title_nonempty
Given: BeadIssue with empty title
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed(...))` - NOT a panic

### test_precondition_closed_status_requires_closed_at
Given: BeadIssue with status=Closed but closed_at=None
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed(...))` - NOT a panic

### test_precondition_dependency_exists
Given: BeadIssue with depends_on pointing to nonexistent bead
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed(...))` - NOT a panic

### test_precondition_no_circular_dependency
Given: Existing dependency A->B, attempting to add B->A
When: `add_dependency(pool, "B", "A")` is called
Then: Returns `Err(BeadsError::ValidationFailed("Circular dependency detected"))` - NOT a panic

### test_postcondition_inserted_bead_queryable
Given: Valid bead inserted
When: `get_bead(pool, id)` is called
Then: Returns `Ok(bead)` with all fields matching inserted data

### test_postcondition_updated_bead_reflects_changes
Given: Bead updated with new values
When: `get_bead(pool, id)` is called
Then: Returns `Ok(bead)` with updated field values and new timestamp

### test_postcondition_deleted_bead_not_found
Given: Bead deleted
When: `get_bead(pool, id)` is called
Then: Returns `Err(BeadsError::NotFound(...))`

### test_postcondition_history_recorded_on_status_change
Given: Bead with status=Open
When: Status changed to InProgress via update
Then: `get_history(bead_id)` contains entry with field_name="status", old_value="open", new_value="in_progress"

### test_invariant_bead_id_unique
Given: Database with existing bead ID
When: Attempting to insert bead with same ID
Then: Returns `Err(BeadsError::DuplicateId(...))`

### test_invariant_created_at_immutable
Given: Existing bead
When: Attempting to update created_at field
Then: Field remains unchanged (or update is rejected)

### test_invariant_history_immutable
Given: State history entry exists
When: Attempting to UPDATE or DELETE history entry
Then: Operation fails or is rejected

### test_invariant_foreign_keys_valid
Given: Bead with dependencies
When: Parent bead is deleted
Then: CASCADE delete removes dependency records

---

## Contract Violation Tests

These tests directly verify the violation examples from contract-spec.md.

### test_violation_p2_empty_id_returns_validation_error
Given: BeadIssue with id = ""
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("ID cannot be empty"))`

### test_violation_p3_empty_title_returns_validation_error
Given: BeadIssue with title = ""
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("Title cannot be empty"))`

### test_violation_p6_closed_without_closed_at_returns_validation_error
Given: BeadIssue with status = Closed and closed_at = None
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("closed_at must be set when status is 'closed'"))`

### test_violation_p7_nonexistent_dependency_returns_validation_error
Given: BeadIssue with depends_on = Some(vec!["non-existent"])
When: `insert_bead(pool, &bead)` is called
Then: Returns `Err(BeadsError::ValidationFailed("Dependency target does not exist: non-existent"))`

### test_violation_p8_circular_dependency_returns_validation_error
Given: Beads A and B exist with A depends_on B already
When: `add_dependency(pool, "B", "A")` is called
Then: Returns `Err(BeadsError::ValidationFailed("Circular dependency detected"))`

### test_violation_q2_get_nonexistent_returns_not_found
Given: Empty database
When: `get_bead(pool, "wrong-id")` is called
Then: Returns `Err(BeadsError::NotFound("wrong-id"))`

### test_violation_q6_get_after_delete_returns_not_found
Given: Bead inserted then deleted
When: `get_bead(pool, id)` is called
Then: Returns `Err(BeadsError::NotFound(id))`

---

## Given-When-Then Scenarios

### Scenario 1: Complete bead lifecycle
Given: Fresh database with schema created
When:
1. Insert bead with status=open
2. Update status to in_progress
3. Update status to closed (with closed_at set)
4. Query history
Then:
- All operations return Ok
- History shows 2 entries: open->in_progress, in_progress->closed

### Scenario 2: Dependency chain
Given: Fresh database with schema created
When:
1. Create bead "task-1"
2. Create bead "task-2" depending on "task-1"
3. Create bead "task-3" depending on "task-2"
Then:
- get_dependencies("task-3") returns ["task-2"]
- get_dependencies("task-2") returns ["task-1"]
- Deleting "task-1" cascades to remove its outgoing dependency

### Scenario 3: Metadata updates
Given: Bead with JSON metadata {"version": 1}
When: Update metadata to {"version": 2, "notes": "updated"}
Then:
- get_bead returns updated metadata
- History shows metadata field change

### Scenario 4: Query by multiple filters
Given: Multiple beads with different statuses and priorities
When: Query with filter status=open AND priority=P0
Then: Returns only beads matching both criteria

### Scenario 5: Bulk operations
Given: Need to insert 100 beads
When: Insert all beads in sequence
Then:
- All insertions succeed
- query_beads returns exactly 100 beads

---

## End-to-End Integration Test

### test_end_to_end_full_workflow
This test verifies the complete workflow from schema creation through bead CRUD to dependency tracking:

1. **Given**: Empty database
2. **When**:
   - Create schema
   - Insert 3 beads: "backend", "frontend", "docs"
   - Add dependency: "frontend" blocked_by "backend"
   - Add dependency: "docs" depends_on "frontend"
   - Update "backend" status to in_progress
   - Update "backend" status to closed (with closed_at)
   - Query all beads
   - Get dependencies for "frontend"
   - Get history for "backend"
3. **Then**:
   - All operations succeed
   - Bead count is 3
   - Frontend is blocked by backend
   - Docs depends on frontend
   - History shows 2 status changes for backend
