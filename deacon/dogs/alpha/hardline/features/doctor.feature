# Feature: Doctor Command
#
# As an agent in the SCP control plane
# I want to diagnose system health and optionally fix issues
# So that I can maintain a healthy development environment
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Actual CLI command (from crates/cli/src/cli/args.rs):
#   scp doctor            -> basic diagnostics (5 checks)
#   scp doctor --full     -> full diagnostics (includes disk, locks, working copy)
#
# Actual doctor checks (from crates/cli/src/commands/doctor.rs):
#   [1/5] Checking VCS...     (looks for .jj or .git directory)
#   [2/5] Checking dependencies... (checks jj and git are available)
#   [3/5] Checking configuration... (checks config.toml exists)
#   [4/5] Checking workspaces... (counts workspaces via VCS backend)
#   [5/5] Full diagnostics (only with --full): disk space, lock files, working copy status
#
# Actual error types (from crates/scp-error/src/lib.rs):
#   When checks fail, doctor returns Err(Error::internal("Diagnostics failed"))
#   Internal -> "Internal error: {0}"  exit 130
#
# NOTE: Doctor does NOT currently output JSON. It outputs human-readable text.
# NOTE: Doctor does NOT have --fix, --dry-run, or --verbose flags.

Feature: Doctor Command

  Background:
    Given a JJ repository is initialized
    And SCP is initialized

  # ==========================================================================
  # Scenario: Basic health check runs all diagnostics
  # ==========================================================================
  Scenario: Basic health check runs all diagnostics
    Given the system is in a healthy state
    When I run the doctor command without --full flag
    Then all 5 diagnostic checks should run
    And the output should contain check results
    And the exit code should be 0
    And the output should contain "All checks passed"

  # ==========================================================================
  # Scenario: Doctor detects missing VCS
  # ==========================================================================
  Scenario: Doctor detects no VCS initialized
    Given no VCS is initialized
    When I run the doctor command
    Then the "[1/5] Checking VCS" check should fail
    And the output should contain "No VCS found"
    And the suggestion should include "scp init --vcs jj"
    And the exit code should be 130

  # ==========================================================================
  # Scenario: Doctor detects missing dependencies
  # ==========================================================================
  Scenario: Doctor detects missing dependencies
    Given JJ is not installed
    And git is not installed
    When I run the doctor command
    Then the "[2/5] Checking dependencies" check should fail
    And the output should contain "No VCS CLI found"
    And the exit code should be 130

  # ==========================================================================
  # Scenario: Doctor detects missing config
  # ==========================================================================
  Scenario: Doctor detects missing config file
    Given no config file exists
    When I run the doctor command
    Then the "[3/5] Checking configuration" check should warn
    And the output should contain "No config found"

  # ==========================================================================
  # Scenario: Doctor shows workspace count
  # ==========================================================================
  Scenario: Doctor shows workspace count
    Given there are workspaces in the repository
    When I run the doctor command
    Then the "[4/5] Checking workspaces" check should pass
    And the output should contain "workspace(s) found"

  # ==========================================================================
  # Scenario: Doctor shows no workspaces hint
  # ==========================================================================
  Scenario: Doctor shows no workspaces hint
    Given no workspaces exist
    When I run the doctor command
    Then the "[4/5] Checking workspaces" check should report info
    And the output should contain "No workspaces"
    And the output should contain "scp workspace spawn"

  # ==========================================================================
  # Scenario: Full diagnostics mode
  # ==========================================================================
  Scenario: Full diagnostics mode
    Given the system is in a healthy state
    When I run the doctor command with --full flag
    Then the "[5/5] Running full diagnostics" check should run
    And the output should contain disk information
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Full diagnostics skips without --full
  # ==========================================================================
  Scenario: Full diagnostics skips without --full flag
    Given the system is in a healthy state
    When I run the doctor command without --full flag
    Then the output should contain "Skipping full diagnostics"
    And the output should contain "use --full"

  # ==========================================================================
  # Scenario: All checks pass for healthy system
  # ==========================================================================
  Scenario: All checks pass for healthy system
    Given the system is in a healthy state
    And all dependencies are installed
    When I run the doctor command
    Then all checks should pass
    And the exit code should be 0
    And the output should contain "All checks passed"

  # ==========================================================================
  # Scenario: Safety - check mode is read-only
  # ==========================================================================
  Scenario: Safety - check mode is read-only
    Given the system has various issues
    When I run the doctor command
    Then no changes should be made to the system
    And no files should be modified
    And no database records should be deleted
    And the output should only report issues

  # ==========================================================================
  # Pending scenarios for future doctor capabilities
  # ==========================================================================

  @pending
  # NOTE: --fix flag does not exist on doctor command.
  #       This scenario describes desired future behavior.
  Scenario: Fix mode with auto-fixable issues
    Given there are 2 orphaned workspaces
    And there are 3 stale sessions
    When I run the doctor command with --fix flag
    Then the orphaned workspaces should be removed
    And the stale sessions should be removed
    And the output should show fix results

  @pending
  # NOTE: --fix flag does not exist on doctor command.
  Scenario: Fix idempotency - running twice is safe
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix flag
    And I run the doctor command with --fix flag again
    Then the second run should report no issues to fix
    And both runs should complete successfully

  @pending
  # NOTE: --fix and --dry-run flags do not exist on doctor command.
  Scenario: Dry-run mode shows what would be fixed
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix --dry-run flags
    Then no changes should be made to the system
    And the output should show what would be fixed
    And the output should contain "Dry-run mode"

  @pending
  # NOTE: Doctor does not check for orphaned workspaces yet.
  Scenario: Doctor detects orphaned workspaces
    Given there are 2 workspaces without session records
    When I run the doctor command
    Then the "Orphaned Workspaces" check should warn
    And the output should show 2 orphaned workspaces
    And the issue should be auto-fixable

  @pending
  # NOTE: Doctor does not check for stale sessions yet.
  Scenario: Doctor detects stale sessions
    Given there are 3 sessions in "creating" status for over 5 minutes
    When I run the doctor command
    Then the "Stale Sessions" check should warn
    And the output should show 3 stale sessions

  @pending
  # NOTE: Doctor does not check database integrity yet.
  Scenario: Database integrity check
    Given the state database is corrupted
    When I run the doctor command
    Then the "State Database" check should fail

  @pending
  # NOTE: Doctor does not have database recovery capability yet.
  Scenario: Database recovery with fix
    Given the state database is corrupted
    When I run the doctor command with --fix flag
    Then the corrupted database should be handled
    And the fix result should be reported

  @pending
  # NOTE: Doctor does not check for pending add operations yet.
  Scenario: Pending add operations check
    Given there are 5 pending add operations in the journal
    When I run the doctor command
    Then the "Pending Add Operations" check should fail
    And the output should show 5 pending operations

  @pending
  # NOTE: Doctor does not check workspace integrity yet.
  Scenario: Workspace integrity check
    Given session "feature-1" has workspace at "/workspaces/feature-1"
    And the workspace directory does not exist
    When I run the doctor command
    Then the "Workspace Integrity" check should fail
    And the output should show the missing workspace

  @pending
  # NOTE: Doctor does not have workspace rebind capability yet.
  Scenario: Workspace integrity fix with rebind
    Given session "feature-1" has workspace at "/old/path/feature-1"
    And the workspace exists at "/new/path/feature-1"
    When I run the doctor command with --fix flag
    Then the session workspace path should be updated
    And the fix should be reported

  @pending
  # NOTE: Doctor does not have a workflow health check yet.
  Scenario: Workflow health check - on main with active sessions
    Given the current directory is the main workspace
    And there are 2 active sessions
    When I run the doctor command
    Then the "Workflow Health" check should warn
    And the suggestion should include "scp workspace spawn"

  @pending
  # NOTE: Doctor does not detect recent recovery yet.
  Scenario: Recent recovery detection
    Given recovery occurred in the last 5 minutes
    When I run the doctor command
    Then the "State Database" check should warn
    And the output should indicate recovery detected

  @pending
  # NOTE: Doctor does not check for beads CLI yet.
  Scenario: Beads integration check (optional)
    Given beads CLI is not installed
    When I run the doctor command
    Then the "Beads Integration" check should pass
    And the message should include "optional"

  @pending
  # NOTE: Doctor does NOT output JSON. This describes desired future behavior.
  Scenario: JSON validity invariant - all doctor outputs are valid JSON
    Given the system is in any state
    When I run the doctor command
    Then the output must be valid JSON
    And the output must have a "$schema" field
    And the output must have a "_schema_version" field
    And the output must have a "success" field

  @pending
  # NOTE: Doctor does NOT have --verbose flag. Global --verbose exists but doesn't
  #       affect doctor output format. This describes desired future behavior.
  Scenario: Verbose output shows fix details
    Given there are 2 orphaned workspaces
    When I run the doctor command with --fix --verbose flags
    Then each fix action should be reported
    And the output should include action status

  # ==========================================================================
  # Scenario: Exit codes follow conventions
  # ==========================================================================
  Scenario: Exit codes follow conventions
    Given the system has 0 errors and 2 warnings
    When I run the doctor command
    Then the exit code should be 0

  Scenario: Exit code 130 for errors
    Given the system has failing diagnostics
    When I run the doctor command
    Then the exit code should be 130
