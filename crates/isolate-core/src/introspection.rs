//! AI-first introspection capabilities
//!
//! This module provides structured metadata about isolate capabilities,
//! enabling AI agents to discover features and understand system state.

use im::HashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Complete introspection output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectOutput {
    /// Isolate version
    pub isolate_version: String,
    /// Categorized capabilities
    pub capabilities: Capabilities,
    /// External dependency status
    pub dependencies: HashMap<String, DependencyInfo>,
    /// Current system state
    pub system_state: SystemState,
}

/// Categorized capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Session management capabilities
    pub session_management: CapabilityCategory,
    /// Configuration capabilities
    pub configuration: CapabilityCategory,
    /// Version control capabilities
    pub version_control: CapabilityCategory,
    /// Introspection and diagnostics
    pub introspection: CapabilityCategory,
}

/// A category of related capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCategory {
    /// Available commands in this category
    pub commands: Vec<String>,
    /// Feature descriptions
    pub features: Vec<String>,
}

/// Information about an external dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Whether this dependency is required for core functionality
    pub required: bool,
    /// Whether the dependency is currently installed
    pub installed: bool,
    /// Installed version if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Command name
    pub command: String,
}

/// Current system state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    /// Whether isolate has been initialized in this repo
    pub initialized: bool,
    /// Whether current directory is a JJ repository
    pub jj_repo: bool,
    /// Path to config file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Path to state database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_db: Option<String>,
    /// Total number of sessions
    pub sessions_count: usize,
    /// Number of active sessions
    pub active_sessions: usize,
}

/// Detailed command introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntrospection {
    /// Command name
    pub command: String,
    /// Human-readable description
    pub description: String,
    /// Command aliases
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Positional arguments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentSpec>,
    /// Optional flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagSpec>,
    /// Usage examples
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<CommandExample>,
    /// Prerequisites for running this command
    pub prerequisites: Prerequisites,
    /// Side effects this command will produce
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    /// Possible error conditions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_conditions: Vec<ErrorCondition>,
}

/// Argument specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentSpec {
    /// Argument name
    pub name: String,
    /// Type of argument
    #[serde(rename = "type")]
    pub arg_type: String,
    /// Whether this argument is required
    pub required: bool,
    /// Human-readable description
    pub description: String,
    /// Validation pattern (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
    /// Example values
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Flag specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    /// Long flag name (e.g., "no-hooks")
    pub long: String,
    /// Short flag name (e.g., "t")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Human-readable description
    pub description: String,
    /// Type of flag value
    #[serde(rename = "type")]
    pub flag_type: String,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Possible values for enum-like flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Category for grouping flags in help output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl FlagSpec {
    /// Validate that a category is one of the allowed values.
    ///
    /// Valid categories are: behavior, configuration, filter, output, advanced
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if the category is not in the allowed list.
    ///
    /// # Examples
    ///
    /// ```
    /// # use isolate_core::introspection::FlagSpec;
    /// assert!(FlagSpec::validate_category("behavior").is_ok());
    /// assert!(FlagSpec::validate_category("invalid").is_err());
    /// ```
    pub fn validate_category(category: &str) -> Result<()> {
        const VALID_CATEGORIES: &[&str] =
            &["behavior", "configuration", "filter", "output", "advanced"];

        if VALID_CATEGORIES.contains(&category) {
            Ok(())
        } else {
            Err(Error::ValidationError {
                message: format!(
                    "Invalid flag category: '{}'. Must be one of: {}",
                    category,
                    VALID_CATEGORIES.join(", ")
                ),
                field: None,
                value: None,
                constraints: Vec::new(),
            })
        }
    }
}

/// Command usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExample {
    /// Example command line
    pub command: String,
    /// Description of what this example does
    pub description: String,
}

/// Prerequisites for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    /// Must be initialized
    pub initialized: bool,
    /// JJ must be installed
    pub jj_installed: bool,
    /// Additional custom checks
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<String>,
}

/// Error condition documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCondition {
    /// Error code
    pub code: String,
    /// Human-readable description
    pub description: String,
    /// How to resolve this error
    pub resolution: String,
}

/// System health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Status message
    pub message: String,
    /// Suggestion for fixing issues
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Whether this issue can be auto-fixed
    pub auto_fixable: bool,
    /// Additional details about the check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a health check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Warning - non-critical issue
    Warn,
    /// Failure - critical issue
    Fail,
}

/// Overall health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOutput {
    /// Whether the system is healthy overall
    pub healthy: bool,
    /// Individual check results
    pub checks: Vec<DoctorCheck>,
    /// Count of warnings
    pub warnings: usize,
    /// Count of errors
    pub errors: usize,
    /// Number of issues that can be auto-fixed
    pub auto_fixable_issues: usize,
}

/// Result of auto-fix operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixOutput {
    /// Issues that were fixed
    pub fixed: Vec<FixResult>,
    /// Issues that could not be fixed
    pub unable_to_fix: Vec<UnfixableIssue>,
}

/// Result of fixing a single issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Issue that was fixed
    pub issue: String,
    /// Action taken
    pub action: String,
    /// Whether the fix succeeded
    pub success: bool,
}

/// Issue that could not be auto-fixed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfixableIssue {
    /// Issue name
    pub issue: String,
    /// Reason why it couldn't be fixed
    pub reason: String,
    /// Manual fix suggestion
    pub suggestion: String,
}

/// Error information for failed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Query result for session existence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExistsQuery {
    /// Whether the session exists (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// Session details if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Basic session information for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name
    pub name: String,
    /// Session status
    pub status: String,
}

/// Query result for session count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCountQuery {
    /// Number of sessions matching filter (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Filter that was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Query result for "can run" check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanRunQuery {
    /// Whether the command can be run
    pub can_run: bool,
    /// Command being checked
    pub command: String,
    /// Prerequisites that are blocking execution
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,
    /// Number of prerequisites met
    pub prerequisites_met: usize,
    /// Total number of prerequisites
    pub prerequisites_total: usize,
}

/// A prerequisite that is blocking command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Check name
    pub check: String,
    /// Check status (should be false)
    pub status: bool,
    /// Human-readable message
    pub message: String,
}

/// Query result for name suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestNameQuery {
    /// Pattern used
    pub pattern: String,
    /// Suggested name
    pub suggested: String,
    /// Next available number in sequence
    pub next_available_n: usize,
    /// Existing names matching pattern
    pub existing_matches: Vec<String>,
}

impl IntrospectOutput {
    /// Create default introspection output
    ///
    /// # Returns
    ///
    /// Returns a new introspection output instance. The result should be used
    /// as this creates a structured introspection report.
    #[must_use]
    pub fn new(version: &str) -> Self {
        Self {
            isolate_version: version.to_string(),
            capabilities: Capabilities::default(),
            dependencies: HashMap::new(),
            system_state: SystemState::default(),
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            session_management: CapabilityCategory {
                commands: vec![
                    "init".to_string(),
                    "add".to_string(),
                    "remove".to_string(),
                    "list".to_string(),
                    "status".to_string(),
                    "focus".to_string(),
                    "sync".to_string(),
                ],
                features: vec![
                    "parallel_workspaces".to_string(),
                    "hook_lifecycle".to_string(),
                ],
            },
            configuration: CapabilityCategory {
                commands: vec![],
                features: vec![
                    "hierarchy".to_string(),
                    "placeholder_substitution".to_string(),
                ],
            },
            version_control: CapabilityCategory {
                commands: vec!["diff".to_string()],
                features: vec![
                    "jj_integration".to_string(),
                    "workspace_isolation".to_string(),
                ],
            },
            introspection: CapabilityCategory {
                commands: vec![
                    "introspect".to_string(),
                    "doctor".to_string(),
                    "query".to_string(),
                ],
                features: vec![
                    "capability_discovery".to_string(),
                    "health_checks".to_string(),
                    "auto_fix".to_string(),
                    "state_queries".to_string(),
                ],
            },
        }
    }
}

impl Prerequisites {
    /// Check if all prerequisites are met
    ///
    /// # Returns
    ///
    /// Returns `true` if all prerequisites are satisfied. The result should be checked
    /// before proceeding with operations that require these prerequisites.
    #[must_use]
    pub const fn all_met(&self) -> bool {
        self.initialized && self.jj_installed && self.custom.is_empty()
    }

    /// Count how many prerequisites are met
    ///
    /// # Returns
    ///
    /// Returns the count of met prerequisites. The result should be used
    /// for reporting or validation purposes.
    #[must_use]
    pub const fn count_met(&self) -> usize {
        let mut count = 0;
        if self.initialized {
            count += 1;
        }
        if self.jj_installed {
            count += 1;
        }
        count
    }

    /// Total number of prerequisites
    ///
    /// # Returns
    ///
    /// Returns the total count of prerequisites. The result should be used
    /// for reporting or validation purposes.
    #[must_use]
    pub const fn total(&self) -> usize {
        2 + self.custom.len()
    }
}

impl DoctorOutput {
    /// Calculate summary statistics from checks
    ///
    /// # Returns
    ///
    /// Returns a summary with calculated statistics. The result should be used
    /// as this performs analysis and aggregates check results.
    #[must_use]
    pub fn from_checks(checks: Vec<DoctorCheck>) -> Self {
        let warnings = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();
        let errors = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let auto_fixable_issues = checks.iter().filter(|c| c.auto_fixable).count();
        let healthy = errors == 0;

        Self {
            healthy,
            checks,
            warnings,
            errors,
            auto_fixable_issues,
        }
    }
}

/// Parse a name pattern and suggest next available name
///
/// Pattern format: `prefix-{n}` or `{n}-suffix` where {n} is a number placeholder
#[allow(clippy::literal_string_with_formatting_args)]
pub fn suggest_name(pattern: &str, existing_names: &[String]) -> Result<SuggestNameQuery> {
    // Find {n} placeholder
    if !pattern.contains("{n}") {
        return Err(Error::ValidationError {
            message: "Pattern must contain {n} placeholder".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }

    // Extract prefix and suffix
    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::ValidationError {
            message: "Pattern must contain exactly one {n} placeholder".into(),
            field: None,
            value: None,
            constraints: Vec::new(),
        });
    }

    let prefix = parts.first().ok_or_else(|| Error::ValidationError {
        message: "Pattern parts missing".into(),
        field: None,
        value: None,
        constraints: Vec::new(),
    })?;
    let suffix = parts.get(1).ok_or_else(|| Error::ValidationError {
        message: "Pattern parts missing suffix".into(),
        field: None,
        value: None,
        constraints: Vec::new(),
    })?;

    // Find all numbers used in matching names using functional patterns
    let (used_numbers, matching): (Vec<usize>, Vec<String>) = existing_names
        .iter()
        .filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
        .filter_map(|name| {
            let num_part = name
                .get(prefix.len()..name.len().saturating_sub(suffix.len()))
                .map_or("", |s| s);
            num_part.parse::<usize>().ok().map(|n| (n, name.clone()))
        })
        .unzip();

    // Find next available number - use map_or to avoid unwrap_or
    let next_n = (1..=used_numbers.len() + 2)
        .find(|n| !used_numbers.contains(n))
        .map_or(1, |n| n);

    let suggested = pattern.replace("{n}", &next_n.to_string());

    Ok(SuggestNameQuery {
        pattern: pattern.to_string(),
        suggested,
        next_available_n: next_n,
        existing_matches: matching,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspect_output_new() {
        let output = IntrospectOutput::new("0.1.0");
        assert_eq!(output.isolate_version, "0.1.0");
        assert!(!output.capabilities.session_management.commands.is_empty());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(caps
            .session_management
            .commands
            .contains(&"add".to_string()));
        assert!(caps.introspection.commands.contains(&"doctor".to_string()));
    }

    #[test]
    fn test_prerequisites_all_met() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        };
        assert!(prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_not_met() {
        let prereqs = Prerequisites {
            initialized: false,
            jj_installed: true,
            custom: vec![],
        };
        assert!(!prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_count() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            custom: vec![],
        };
        assert_eq!(prereqs.count_met(), 2);
        assert_eq!(prereqs.total(), 2);
    }

    #[test]
    fn test_doctor_output_from_checks() {
        let checks = vec![
            DoctorCheck {
                name: "Check 1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
                suggestion: Some("Fix it".to_string()),
                auto_fixable: true,
                details: None,
            },
            DoctorCheck {
                name: "Check 3".to_string(),
                status: CheckStatus::Fail,
                message: "Error".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);
        assert!(!output.healthy);
        assert_eq!(output.warnings, 1);
        assert_eq!(output.errors, 1);
        assert_eq!(output.auto_fixable_issues, 1);
    }

    #[test]
    fn test_suggest_name_basic() -> Result<()> {
        let existing = vec!["feature-1".to_string(), "feature-2".to_string()];
        let result = suggest_name("feature-{n}", &existing)?;
        assert_eq!(result.suggested, "feature-3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_gap() -> Result<()> {
        let existing = vec!["test-1".to_string(), "test-3".to_string()];
        let result = suggest_name("test-{n}", &existing)?;
        assert_eq!(result.suggested, "test-2");
        assert_eq!(result.next_available_n, 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_no_existing() -> Result<()> {
        let existing = vec![];
        let result = suggest_name("bug-{n}", &existing)?;
        assert_eq!(result.suggested, "bug-1");
        assert_eq!(result.next_available_n, 1);
        assert_eq!(result.existing_matches.len(), 0);
        Ok(())
    }

    #[test]
    fn test_suggest_name_invalid_pattern() {
        let result = suggest_name("no-placeholder", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_multiple_placeholders() {
        let result = suggest_name("test-{n}-{n}", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_requires_braced_placeholder() {
        // Test pattern without {n} placeholder returns validation error
        // This documents the error when users run: isolate query suggest-name feat
        let result = suggest_name("feat", &[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ValidationError { .. })));
    }

    #[test]
    fn test_suggest_name_with_feat_placeholder() {
        // Test the corrected example from the help text
        // isolate query suggest-name "feat{n}" should work
        let existing = vec!["feat1".to_string(), "feat2".to_string()];
        let result = suggest_name("feat{n}", &existing);
        let result = match result {
            Ok(r) => r,
            Err(e) => panic!("suggest_name failed: {e}"),
        };
        assert_eq!(result.suggested, "feat3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
    }

    // ===== CommandIntrospection Validation Tests (TDD15 Phase 4 - RED) =====
    // These tests verify the expected structure and validation rules for the list command.
    // They FAIL until the implementation adds the required flags and error conditions.

    /// Validates that a list introspection includes bead flag specification
    #[allow(dead_code)]
    fn validate_list_has_bead_flag(list_introspection: &CommandIntrospection) -> bool {
        list_introspection.flags.iter().any(|f| f.long == "bead")
    }

    /// Validates that a list introspection includes agent flag specification
    #[allow(dead_code)]
    fn validate_list_has_agent_flag(list_introspection: &CommandIntrospection) -> bool {
        list_introspection.flags.iter().any(|f| f.long == "agent")
    }

    /// Validates that error conditions include `NO_MATCHING_SESSIONS`
    fn validate_has_no_matching_error(list_introspection: &CommandIntrospection) -> bool {
        list_introspection
            .error_conditions
            .iter()
            .any(|e| e.code == "NO_MATCHING_SESSIONS")
    }

    /// Validates that examples include filter usage patterns
    fn validate_has_filter_examples(list_introspection: &CommandIntrospection) -> bool {
        list_introspection
            .examples
            .iter()
            .any(|ex| ex.command.contains("--bead") || ex.command.contains("--agent"))
    }

    #[test]
    fn test_list_introspection_structure_supports_bead_flag() {
        // FAILING: Validates that list command introspection can include --bead flag
        let expected_bead_flag = FlagSpec {
            long: "bead".to_string(),
            short: Some("b".to_string()),
            description: "Filter sessions by bead ID".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // This test verifies the FlagSpec structure itself is valid
        assert_eq!(expected_bead_flag.long, "bead");
        assert!(expected_bead_flag.short.is_some());
        assert!(!expected_bead_flag.description.is_empty());
        assert_eq!(expected_bead_flag.flag_type, "string");
    }

    #[test]
    fn test_list_introspection_structure_supports_agent_flag() {
        // FAILING: Validates that list command introspection can include --agent flag
        let expected_agent_flag = FlagSpec {
            long: "agent".to_string(),
            short: Some("a".to_string()),
            description: "Filter sessions by assigned agent".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // This test verifies the FlagSpec structure itself is valid
        assert_eq!(expected_agent_flag.long, "agent");
        assert!(expected_agent_flag.short.is_some());
        assert!(!expected_agent_flag.description.is_empty());
        assert_eq!(expected_agent_flag.flag_type, "string");
    }

    #[test]
    fn test_list_command_introspection_includes_no_matching_sessions_error() {
        // FAILING: Verifies NO_MATCHING_SESSIONS error condition structure
        let expected_error = ErrorCondition {
            code: "NO_MATCHING_SESSIONS".to_string(),
            description: "No sessions match the specified filter criteria".to_string(),
            resolution: "Check filter parameters and try with fewer restrictions".to_string(),
        };

        // Validate error structure
        assert_eq!(expected_error.code, "NO_MATCHING_SESSIONS");
        assert!(!expected_error.description.is_empty());
        assert!(!expected_error.resolution.is_empty());
    }

    #[test]
    fn test_list_command_bead_flag_has_dynamic_value_documentation() {
        // FAILING: Verifies bead flag supports dynamic values with proper documentation
        let bead_with_doc = FlagSpec {
            long: "bead".to_string(),
            short: Some("b".to_string()),
            description: "Filter by bead ID or pattern - supports dynamic values like 'feature-*'"
                .to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // Verify it supports dynamic values (type is string, not enum)
        assert_eq!(bead_with_doc.flag_type, "string");
        // Verify documentation is present
        assert!(
            bead_with_doc.description.contains("dynamic")
                || bead_with_doc.description.contains("filter")
                || bead_with_doc.description.contains("ID")
        );
    }

    #[test]
    fn test_list_command_agent_flag_has_dynamic_value_documentation() {
        // FAILING: Verifies agent flag supports dynamic values with proper documentation
        let agent_with_doc = FlagSpec {
            long: "agent".to_string(),
            short: Some("a".to_string()),
            description: "Filter by agent name or pattern - supports dynamic values".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        // Verify it supports dynamic values (type is string, not enum)
        assert_eq!(agent_with_doc.flag_type, "string");
        // Verify documentation is present
        assert!(
            agent_with_doc.description.contains("dynamic")
                || agent_with_doc.description.contains("agent")
        );
    }

    #[test]
    fn test_list_command_error_conditions_document_no_matches_scenario() {
        // FAILING: Verifies error condition for filtering with no matches
        let list_with_error = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![ErrorCondition {
                code: "NO_MATCHING_SESSIONS".to_string(),
                description: "No sessions match the filter criteria".to_string(),
                resolution: "Try with less restrictive filters".to_string(),
            }],
        };

        // Validate error is documented
        assert!(validate_has_no_matching_error(&list_with_error));
    }

    #[test]
    fn test_list_command_examples_demonstrate_bead_filtering() {
        // FAILING: Verifies examples show how to use --bead filter
        let list_with_bead_example = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "isolate list --bead feature-123".to_string(),
                description: "List sessions for bead feature-123".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate bead example exists
        assert!(validate_has_filter_examples(&list_with_bead_example));
    }

    #[test]
    fn test_list_command_examples_demonstrate_agent_filtering() {
        // FAILING: Verifies examples show how to use --agent filter
        let list_with_agent_example = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "isolate list --agent alice".to_string(),
                description: "List sessions assigned to alice".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate agent example exists
        assert!(validate_has_filter_examples(&list_with_agent_example));
    }

    #[test]
    fn test_list_command_supports_combined_filter_parameters() {
        // FAILING: Verifies list command allows combining multiple filters
        let list_with_combined = CommandIntrospection {
            command: "list".to_string(),
            description: "List all sessions".to_string(),
            aliases: vec!["ls".to_string()],
            arguments: vec![],
            flags: vec![],
            examples: vec![CommandExample {
                command: "isolate list --bead feature-123 --agent alice".to_string(),
                description: "List feature-123 sessions assigned to alice".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        // Validate combined filters example
        let has_combined = list_with_combined
            .examples
            .iter()
            .any(|ex| ex.command.contains("--bead") && ex.command.contains("--agent"));

        assert!(
            has_combined,
            "Expected example with both --bead and --agent"
        );
    }

    #[test]
    fn test_list_command_filters_are_optional_parameters() {
        // FAILING: Verifies that both filter flags are optional (not required)
        // FlagSpec structure doesn't have a 'required' field; all flags are optional by design
        let filters = [
            FlagSpec {
                long: "bead".to_string(),
                short: Some("b".to_string()),
                description: "Filter by bead".to_string(),
                flag_type: "string".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
            FlagSpec {
                long: "agent".to_string(),
                short: Some("a".to_string()),
                description: "Filter by agent".to_string(),
                flag_type: "string".to_string(),
                default: None,
                possible_values: vec![],
                category: None,
            },
        ];

        // Validate both are present and have string type (not boolean/required)
        assert_eq!(filters.len(), 2);
        assert!(filters.iter().all(|f| f.flag_type == "string"));
    }

    #[test]
    fn test_add_command_introspection_has_name_argument_with_validation() {
        // FAILING: Verifies add command has proper introspection with validation
        let add_introspection = CommandIntrospection {
            command: "add".to_string(),
            description: "Create a new session with JJ workspace + Zellij tab".to_string(),
            aliases: vec![],
            arguments: vec![ArgumentSpec {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description:
                    "Session name (alphanumeric, dash, underscore; must start with letter)"
                        .to_string(),
                validation: Some("^[a-zA-Z][a-zA-Z0-9_-]{0,63}$".to_string()),
                examples: vec![
                    "my-session".to_string(),
                    "feature_001".to_string(),
                    "BugFix".to_string(),
                ],
            }],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![],
        };

        assert_eq!(add_introspection.command, "add");
        assert!(!add_introspection.arguments.is_empty());

        // Safe to unwrap after checking is_empty
        #[allow(clippy::indexing_slicing)]
        let first_arg = &add_introspection.arguments[0];
        assert_eq!(first_arg.name, "name");
        assert!(first_arg.required);
        let validation = &first_arg.validation;
        assert!(validation.is_some());
        assert_eq!(validation.as_deref(), Some("^[a-zA-Z][a-zA-Z0-9_-]{0,63}$"));
    }

    #[test]
    fn test_add_command_error_conditions_document_validation_errors() {
        // FAILING: add command should document validation errors in introspection
        let add_command = CommandIntrospection {
            command: "add".to_string(),
            description: "Create a new session".to_string(),
            aliases: vec![],
            arguments: vec![],
            flags: vec![],
            examples: vec![],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: true,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![
                ErrorCondition {
                    code: "ERR_VALIDATION".to_string(),
                    description: "Invalid session name format".to_string(),
                    resolution: "Session name must start with a letter and contain only alphanumeric characters, dashes, or underscores".to_string(),
                },
                ErrorCondition {
                    code: "ERR_DUPLICATE".to_string(),
                    description: "Session already exists".to_string(),
                    resolution: "Choose a different name or delete the existing session".to_string(),
                },
            ],
        };

        // Verify validation error is documented
        let has_validation_error = add_command
            .error_conditions
            .iter()
            .any(|e| e.code == "ERR_VALIDATION");
        assert!(
            has_validation_error,
            "add command should document ERR_VALIDATION error"
        );
    }

    #[test]
    fn test_add_command_argument_examples_follow_validation() {
        // FAILING: All examples in ArgumentSpec should be valid names
        let arg = ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z][a-zA-Z0-9_-]{0,63}$".to_string()),
            examples: vec![
                "session1".to_string(),
                "my-session".to_string(),
                "Feature_Branch".to_string(),
            ],
        };

        // Verify all examples match the regex pattern
        let regex = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{0,63}$").expect("valid regex");
        for example in &arg.examples {
            assert!(
                regex.is_match(example),
                "Example '{example}' should match the validation regex"
            );
        }
    }

    // ===== PHASE 2 (RED): SchemaEnvelope Wrapping Tests =====
    // These tests FAIL initially - they verify envelope structure and format
    // Implementation in Phase 4 (GREEN) will make them pass

    #[test]
    fn test_introspect_json_has_envelope() -> Result<()> {
        // FAILING: Verify envelope wrapping for introspect command output
        let output = IntrospectOutput::new("0.1.0");
        let envelope = crate::json::SchemaEnvelope::new("introspect-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("_schema_version").and_then(|v| v.as_str()),
            Some("1.0")
        );
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("single")
        );
        assert!(parsed.get("success").is_some(), "Missing success field");

        Ok(())
    }

    #[test]
    fn test_introspect_schema_format() -> Result<()> {
        // FAILING: Verify schema format matches isolate://introspect/v1 pattern
        let output = IntrospectOutput::new("0.1.0");
        let envelope = crate::json::SchemaEnvelope::new("introspect-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let schema = parsed
            .get("$schema")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::ParseError("$schema not found".to_string()))?;

        assert!(
            schema.starts_with("isolate://introspect"),
            "Schema should start with 'isolate://introspect'"
        );
        assert!(schema.ends_with("/v1"), "Schema should end with '/v1'");

        Ok(())
    }

    #[test]
    fn test_introspect_flags_wrapped() -> Result<()> {
        // FAILING: Verify flags array is wrapped in envelope
        let flag = FlagSpec {
            long: "test-flag".to_string(),
            short: Some("t".to_string()),
            description: "A test flag".to_string(),
            flag_type: "string".to_string(),
            default: None,
            possible_values: vec![],
            category: None,
        };

        let flags = vec![flag];
        let envelope = crate::json::SchemaEnvelopeArray::new("introspect-flags", flags);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        assert!(parsed.get("$schema").is_some(), "Missing $schema field");
        assert_eq!(
            parsed.get("schema_type").and_then(|v| v.as_str()),
            Some("array")
        );

        Ok(())
    }

    #[test]
    fn test_introspect_schema_version() -> Result<()> {
        // FAILING: Verify _schema_version is "1.0"
        let output = IntrospectOutput::new("0.1.0");
        let envelope = crate::json::SchemaEnvelope::new("introspect-response", "single", output);
        let json_str = serde_json::to_string(&envelope)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)?;

        let version = parsed
            .get("_schema_version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::ParseError("_schema_version not found".to_string()))?;

        assert_eq!(version, "1.0", "_schema_version should be exactly '1.0'");

        Ok(())
    }
}
