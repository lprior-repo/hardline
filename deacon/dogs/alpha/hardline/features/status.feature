# Feature: Status Query
#
# As an agent in the SCP control plane
# I want to query my current session status and context
# So that I can understand my work environment and make informed decisions
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Actual CLI commands (from crates/cli/src/cli/args.rs):
#   scp status            -> detailed status (delegates to session::status())
#   scp status --short    -> single line: "<status-char> <branch> <cwd>"
#   scp session status    -> show current session status
#
# Actual error types (from crates/scp-error/src/lib.rs):
#   SessionNotFound -> "Session not found: {0}"  exit 14
#   NotFound       -> "Not found: {0}"          exit 81
#   VcsNotInitialized -> "VCS not initialized in this directory" exit 40
#
# NOTE: The status command does NOT currently output JSON. It outputs human-readable text.

Feature: Status Query

  Background:
    Given a JJ repository is initialized
    And SCP is initialized

  # ==========================================================================
  # Scenario: Status shows current session
  # ==========================================================================
  Scenario: Status shows current session
    Given I have created a session named "feature-status"
    And the session has status "active"
    When I query the status
    Then the output should contain the session name "feature-status"
    And the output should contain the status "active"
    And the output should contain the workspace path

  # ==========================================================================
  # Scenario: Missing session handled gracefully
  # ==========================================================================
  Scenario: Missing session handled gracefully
    Given no session exists
    When I query the status
    Then the output should indicate no active session
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Short status output
  # ==========================================================================
  Scenario: Short status output format
    Given I am in a JJ repository with branch "main"
    When I run "scp status --short"
    Then the output should be a single line
    And the output should contain the branch name "main"
    And the output should contain the current directory path
    And the output should contain a status character indicating clean/dirty/conflicted/detached

  # ==========================================================================
  # Scenario: Status output is read-only
  # ==========================================================================
  Scenario: Status output is read-only
    Given I have created a session named "readonly-test"
    And the session has status "active"
    When I query the status for "readonly-test"
    Then the session status should remain unchanged
    And no files should be modified
    And no state transitions should occur

  # ==========================================================================
  # Scenario: Status for non-existent session
  # ==========================================================================
  @pending
  # NOTE: "scp status" shows current session context, not an arbitrary named session.
  #       There is no "scp status <name>" command. This scenario describes desired
  #       future behavior. The actual error would be SessionNotFound (exit 14).
  Scenario: Status for non-existent session fails gracefully
    Given no session named "nonexistent" exists
    When I attempt to query the status for "nonexistent"
    Then the operation should fail with error "SessionNotFound"
    And the error message should contain "Session not found"
    And the exit code should be 14

  # ==========================================================================
  # Scenario: Multiple sessions status
  # ==========================================================================
  @pending
  # NOTE: There is no "scp status --all" or equivalent. Use "scp session list" to
  #       see all sessions. This scenario describes desired future behavior.
  Scenario: Multiple sessions status
    Given I have created sessions "session-a", "session-b", and "session-c"
    And "session-a" has status "active"
    And "session-b" has status "paused"
    And "session-c" has status "syncing"
    When I query the status for all sessions
    Then the output should contain 3 session entries
    And each session should show its status
    And the summary should show the count of active sessions

  # ==========================================================================
  # Scenario: Status with detailed information
  # ==========================================================================
  @pending
  # NOTE: There is no "scp status <name>" or detailed query for a specific session.
  #       This scenario describes desired future behavior.
  Scenario: Status with detailed information
    Given I have created a session named "detailed-status"
    And the session has 3 modified files
    And the session has 5 open beads
    When I query the status with details for "detailed-status"
    Then the output should show file change statistics
    And the output should show bead statistics

  # ==========================================================================
  # Scenario: JSON output
  # ==========================================================================
  @pending
  # NOTE: The status command does NOT currently output JSON. It outputs human-readable text.
  #       This scenario describes desired future behavior.
  Scenario: JSON output is valid
    Given I have created a session named "json-test"
    And the session has status "active"
    When I query the status with JSON output
    Then the output should be valid JSONL
    And each line should be a valid JSON object
    And the output should contain a "session" type line
    And the output should contain a "summary" type line

  # ==========================================================================
  # Scenario: JSON validity invariant
  # ==========================================================================
  @pending
  # NOTE: The status command does NOT currently output JSON.
  #       This invariant describes desired future behavior.
  Scenario: JSON validity invariant - all status outputs are valid JSON
    Given I have created a session named "invariant-test"
    When I query the status
    Then the output must be valid JSON
    And the output must have a "$schema" field
    And the output must have a "_schema_version" field
    And the output must have a "success" field
