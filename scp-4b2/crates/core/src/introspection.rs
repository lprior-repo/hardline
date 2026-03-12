//! AI-first introspection capabilities
//!
//! This module provides structured metadata about SCP capabilities,
//! enabling AI agents to discover features and understand system state.

use im::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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
    /// # use scp_core::introspection::FlagSpec;
    /// assert!(FlagSpec::validate_category("behavior").is_ok());
    /// assert!(FlagSpec::validate_category("invalid").is_err());
    /// ```
    pub fn validate_category(category: &str) -> Result<()> {
        const VALID_CATEGORIES: &[&str] =
            &["behavior", "configuration", "filter", "output", "advanced"];

        if VALID_CATEGORIES.contains(&category) {
            Ok(())
        } else {
            Err(Error::ValidationError(format!(
                "Invalid flag category: '{}'. Must be one of: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )))
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
    if !pattern.contains("{n}") {
        return Err(Error::ValidationError(
            "Pattern must contain {n} placeholder".to_string(),
        ));
    }

    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::ValidationError(
            "Pattern must contain exactly one {n} placeholder".to_string(),
        ));
    }

    let prefix = parts
        .first()
        .ok_or_else(|| Error::ValidationError("Pattern parts missing".to_string()))?;
    let suffix = parts
        .get(1)
        .ok_or_else(|| Error::ValidationError("Pattern parts missing suffix".to_string()))?;

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
        let result = suggest_name("feat", &[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::ValidationError(_))));
    }

    #[test]
    fn test_suggest_name_with_feat_placeholder() {
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

    #[test]
    fn test_flag_validate_category_valid() {
        assert!(FlagSpec::validate_category("behavior").is_ok());
        assert!(FlagSpec::validate_category("configuration").is_ok());
        assert!(FlagSpec::validate_category("filter").is_ok());
        assert!(FlagSpec::validate_category("output").is_ok());
        assert!(FlagSpec::validate_category("advanced").is_ok());
    }

    #[test]
    fn test_flag_validate_category_invalid() {
        assert!(FlagSpec::validate_category("invalid").is_err());
        assert!(FlagSpec::validate_category("").is_err());
    }

    #[test]
    fn test_doctor_check_serialization() {
        let check = DoctorCheck {
            name: "test".to_string(),
            status: CheckStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"status\":\"pass\""));
    }

    #[test]
    fn test_command_introspection_full() {
        let cmd = CommandIntrospection {
            command: "test".to_string(),
            description: "A test command".to_string(),
            aliases: vec!["t".to_string()],
            arguments: vec![ArgumentSpec {
                name: "name".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Test argument".to_string(),
                validation: None,
                examples: vec![],
            }],
            flags: vec![FlagSpec {
                long: "verbose".to_string(),
                short: Some("v".to_string()),
                description: "Enable verbose output".to_string(),
                flag_type: "boolean".to_string(),
                default: None,
                possible_values: vec![],
                category: Some("output".to_string()),
            }],
            examples: vec![CommandExample {
                command: "scp test --verbose".to_string(),
                description: "Run with verbose output".to_string(),
            }],
            prerequisites: Prerequisites {
                initialized: true,
                jj_installed: false,
                custom: vec![],
            },
            side_effects: vec![],
            error_conditions: vec![ErrorCondition {
                code: "ERR_TEST".to_string(),
                description: "Test error".to_string(),
                resolution: "Fix it".to_string(),
            }],
        };

        assert_eq!(cmd.command, "test");
        assert!(!cmd.aliases.is_empty());
        assert!(!cmd.arguments.is_empty());
        assert!(!cmd.flags.is_empty());
        assert!(!cmd.examples.is_empty());
        assert!(!cmd.error_conditions.is_empty());
    }

    #[test]
    fn test_system_state_default() {
        let state = SystemState::default();
        assert!(!state.initialized);
        assert!(!state.jj_repo);
        assert!(state.config_path.is_none());
        assert!(state.state_db.is_none());
        assert_eq!(state.sessions_count, 0);
        assert_eq!(state.active_sessions, 0);
    }

    #[test]
    fn test_dependency_info_serialization() {
        let dep = DependencyInfo {
            required: true,
            installed: true,
            version: Some("1.0.0".to_string()),
            command: "jj".to_string(),
        };
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("\"required\":true"));
        assert!(json.contains("\"installed\":true"));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }
}
