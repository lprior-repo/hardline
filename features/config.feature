# Feature: Config Command
#
# As an agent in the SCP control plane
# I want to view and modify configuration settings
# So that I can customize SCP behavior for different projects
#
# Dan North BDD Style - Given/When/Then syntax
# ATDD Phase: These tests define expected behavior before implementation
#
# Actual CLI commands (from crates/cli/src/cli/args.rs):
#   scp config get <key>   -> get a config value
#   scp config set <key> <value>  -> set a config value
#   scp config list        -> list all config
#
# NOTE: --json and --global flags do NOT exist on config subcommands.
# NOTE: Output format is implementation-specific (not guaranteed TOML or JSON).
#
# Actual error types (from crates/scp-error/src/lib.rs):
#   ConfigNotFound  -> "Configuration not found: {0}"  exit 60
#   ConfigInvalid   -> "Configuration invalid: {0}"   exit 61
#   InvalidConfig   -> "Invalid configuration: {0}"   exit 63
#   ValidationError -> "Validation error: {0}"        exit 90
#
# Valid config keys (see crates/core/src/config/config_core.rs VALID_CONFIG_KEYS):
#   watch.enabled, watch.debounce_ms, watch.paths
#   conflict_resolution.mode, conflict_resolution.autonomy,
#     conflict_resolution.security_keywords, conflict_resolution.log_resolutions
#   session.auto_commit, session.commit_prefix, session.max_sessions
#   vcs.type, vcs.default_branch
#   workspace.directory, workspace.auto_rebase, workspace.auto_push
#   queue.default, logging.level, remote.push, remote.fetch, editor

Feature: Config Command

  Background:
    Given a JJ repository is initialized
    And SCP is initialized

  # ==========================================================================
  # Scenario: List all configuration
  # ==========================================================================
  Scenario: List all configuration
    Given a valid config exists
    When I run "scp config list"
    Then all config values should be displayed
    And the output should include workspace.directory
    And the output should include vcs.default_branch
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Get specific config value
  # ==========================================================================
  Scenario: Get specific config value
    Given a valid config exists
    When I run "scp config get workspace.directory"
    Then the value should be displayed
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set config value
  # ==========================================================================
  Scenario: Set config value
    Given a valid config exists
    When I run "scp config set workspace.directory ../custom_workspaces"
    Then the config value should be updated
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set integer value
  # ==========================================================================
  Scenario: Set integer value
    Given a valid config exists
    When I run "scp config set session.max_sessions 10"
    Then the integer should be stored as a proper integer
    And reading it back should return "10" not a string
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Set array value
  # ==========================================================================
  Scenario: Set array value
    Given a valid config exists
    When I run "scp config set watch.paths '[\".beads/beads.db\", \"src/\"]'"
    Then the array should be stored properly
    And reading it back should show the array
    And the exit code should be 0

  # ==========================================================================
  # Scenario: Invalid key is rejected
  # ==========================================================================
  Scenario: Invalid key is rejected
    Given a valid config exists
    When I run "scp config get invalid..key"
    Then the operation should fail
    And the error message should explain the key format
    And the exit code should match the config error code

  # ==========================================================================
  # Scenario: Non-existent key is rejected
  # ==========================================================================
  Scenario: Non-existent key is rejected
    Given a valid config exists
    When I run "scp config get nonexistent.key"
    Then the operation should fail
    And the exit code should be 60

  # ==========================================================================
  # Scenario: Config key validation prevents injection
  # ==========================================================================
  Scenario: Config key validation prevents injection
    Given a valid config exists
    When I run "scp config set '../../../etc/passwd' value"
    Then the operation should fail
    And the error should explain invalid key format
    And the exit code should match the config error code

  # ==========================================================================
  # Scenario: Concurrent config writes are serialized
  # ==========================================================================
  Scenario: Concurrent config writes are serialized
    Given a valid config exists
    When multiple processes write to config simultaneously
    Then no data should be lost
    And all writes should succeed
    And the final config should contain all changes

  # ==========================================================================
  # Scenario: Invalid TOML value is rejected
  # ==========================================================================
  Scenario: Invalid value is rejected
    Given a valid config exists
    When I run "scp config set watch.paths '[invalid'"
    Then the operation should fail
    And the error should explain the value format issue
    And the exit code should be 61

  # ==========================================================================
  # Pending scenarios for future config capabilities
  # ==========================================================================

  @pending
  # NOTE: --json flag does not exist on "scp config list".
  #       This scenario describes desired future behavior.
  Scenario: List all configuration in JSON format
    Given a valid config exists
    When I run "scp config list --json"
    Then the output should be valid JSON
    And the JSON should contain workspace.directory
    And the JSON should contain vcs.default_branch
    And the exit code should be 0

  @pending
  # NOTE: --json flag does not exist on "scp config get".
  Scenario: Get config value in JSON format
    Given a valid config exists
    When I run "scp config get workspace.directory --json"
    Then the output should be valid JSON
    And the JSON should contain the key field
    And the JSON should contain the value field
    And the exit code should be 0

  @pending
  # NOTE: --global flag does not exist on "scp config set".
  Scenario: Global config scope
    Given a valid config exists
    When I run "scp config set --global workspace.directory ../global_workspaces"
    Then the value should be set in global config
    And the output should indicate global scope
    And the exit code should be 0

  @pending
  # NOTE: --global flag does not exist on "scp config list".
  Scenario: View global config only
    Given a valid config exists
    When I run "scp config list --global"
    Then only global config should be displayed
    And the output should indicate global scope
    And the exit code should be 0

  @pending
  # NOTE: Merged config view is not currently implemented.
  Scenario: View merged config (default)
    Given global config exists
    And project config exists
    And project config overrides global settings
    When I run "scp config list"
    Then merged config should be displayed
    And project values should override global values
    And the output should show config sources
    And the exit code should be 0
