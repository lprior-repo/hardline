//! Scenario YAML schema and parser
//!
//! Defines the structure for behavioral scenarios that can be executed
//! against a twin universe for black-box testing.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A complete scenario with metadata and execution steps
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    /// Unique identifier for the scenario
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Ordered list of steps to execute
    pub steps: Vec<Step>,
}

/// A single step in a scenario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Step {
    /// HTTP request step
    Http(HttpStep),
    /// Extract value from response
    Extract(ExtractStep),
    /// Assert a condition
    Assert(AssertStep),
}

/// HTTP request configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpStep {
    /// Target URL
    pub url: String,
    /// HTTP method
    #[serde(default)]
    pub method: HttpMethod,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (for POST/PUT/PATCH)
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// HTTP methods supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Extract a value from a response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractStep {
    /// Source location (response.body, response.headers, etc.)
    pub from: String,
    /// `JSONPath` or similar selector
    pub path: String,
    /// Variable name to store the extracted value
    pub r#as: String,
}

/// Assert a condition on extracted values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertStep {
    /// Assertion type
    pub assertion: AssertionType,
    /// The actual value (can be a template like {{variable}})
    pub equals: Option<String>,
    /// The expected value
    pub expected: Option<String>,
    /// The value to check existence for
    pub exists: Option<String>,
    /// The value to check for absence
    pub not_exists: Option<String>,
}

/// Types of assertions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    #[default]
    Equals,
    NotEquals,
    Exists,
    NotExists,
    Contains,
    NotContains,
}

impl Scenario {
    /// Parse a scenario from YAML string
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError::ParseError` if YAML is invalid.
    pub fn from_yaml(yaml_str: &str) -> Result<Self, ScenarioError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ScenarioError::ParseError(e.to_string()))
    }

    /// Parse a scenario from YAML bytes
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError::ParseError` if YAML is invalid.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self, ScenarioError> {
        serde_yaml::from_slice(bytes).map_err(|e| ScenarioError::ParseError(e.to_string()))
    }
}

/// Errors that can occur when working with scenarios
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScenarioError {
    #[error("Failed to parse scenario YAML: {0}")]
    ParseError(String),

    #[error("Invalid step: {0}")]
    InvalidStep(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SCENARIO: &str = r#"
name: "SendGrid email validation"
description: "Validates email sending flow"
steps:
  - type: http
    url: "http://localhost:3001/v3/mail/send"
    method: POST
    headers:
      Authorization: "Bearer test-key"
      Content-Type: "application/json"
    body:
      personalizations:
        - to:
            - email: "test@example.com"
      from:
        email: "sender@example.com"
      subject: "Test"
      content:
        - type: "text"
          value: "Test email"
  - type: extract
    from: response.body
    path: "$.message_id"
    as: "message_id"
  - type: assert
    assertion: equals
    equals: "{{message_id}}"
    expected: "test-123"
"#;

    #[test]
    fn test_scenario_parsing() {
        let scenario = Scenario::from_yaml(VALID_SCENARIO).expect("Failed to parse scenario");

        assert_eq!(scenario.name, "SendGrid email validation");
        assert_eq!(scenario.description, "Validates email sending flow");
        assert_eq!(scenario.steps.len(), 3);

        // First step is HTTP
        match &scenario.steps[0] {
            Step::Http(http) => {
                assert_eq!(http.url, "http://localhost:3001/v3/mail/send");
                assert_eq!(http.method, HttpMethod::Post);
            }
            _ => panic!("Expected HTTP step"),
        }

        // Second step is extract
        match &scenario.steps[1] {
            Step::Extract(ext) => {
                assert_eq!(ext.from, "response.body");
                assert_eq!(ext.path, "$.message_id");
                assert_eq!(ext.r#as, "message_id");
            }
            _ => panic!("Expected Extract step"),
        }

        // Third step is assert
        match &scenario.steps[2] {
            Step::Assert(assert) => {
                assert_eq!(assert.equals.as_deref(), Some("{{message_id}}"));
                assert_eq!(assert.expected.as_deref(), Some("test-123"));
            }
            _ => panic!("Expected Assert step"),
        }
    }

    #[test]
    fn test_scenario_default_method() {
        let yaml = r#"
name: "Test"
description: "Test"
steps:
  - type: http
    url: "http://localhost:3001/test"
"#;
        let scenario = Scenario::from_yaml(yaml).expect("Failed to parse");

        match &scenario.steps[0] {
            Step::Http(http) => {
                assert_eq!(http.method, HttpMethod::Get);
            }
            _ => panic!("Expected HTTP step"),
        }
    }

    #[test]
    fn test_from_yaml_bytes() {
        let yaml = br#"
name: "Bytes test"
description: "Test parsing from bytes"
steps: []
"#;
        let scenario = Scenario::from_yaml_bytes(yaml).expect("Failed to parse bytes");
        assert_eq!(scenario.name, "Bytes test");
        assert!(scenario.steps.is_empty());
    }

    #[test]
    fn test_from_yaml_invalid_yaml() {
        let result = Scenario::from_yaml("not valid yaml: [");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Failed to parse scenario YAML"));
    }

    #[test]
    fn test_from_yaml_bytes_invalid() {
        let result = Scenario::from_yaml_bytes(b"not valid yaml: {");
        assert!(result.is_err());
    }

    #[test]
    fn test_scenario_error_parse_error_display() {
        let err = ScenarioError::ParseError("bad yaml at line 5".to_string());
        assert_eq!(err.to_string(), "Failed to parse scenario YAML: bad yaml at line 5");
    }

    #[test]
    fn test_scenario_error_invalid_step_display() {
        let err = ScenarioError::InvalidStep("missing type".to_string());
        assert_eq!(err.to_string(), "Invalid step: missing type");
    }

    #[test]
    fn test_scenario_error_variable_not_found_display() {
        let err = ScenarioError::VariableNotFound("user_id".to_string());
        assert_eq!(err.to_string(), "Variable not found: user_id");
    }

    #[test]
    fn test_scenario_error_extraction_failed_display() {
        let err = ScenarioError::ExtractionFailed("$.path".to_string());
        assert_eq!(err.to_string(), "Extraction failed: $.path");
    }

    #[test]
    fn test_scenario_error_assertion_failed_display() {
        let err = ScenarioError::AssertionFailed("expected X got Y".to_string());
        assert_eq!(err.to_string(), "Assertion failed: expected X got Y");
    }

    #[test]
    fn test_scenario_error_invalid_template_display() {
        let err = ScenarioError::InvalidTemplate("unclosed {{".to_string());
        assert_eq!(err.to_string(), "Invalid template: unclosed {{");
    }

    #[test]
    fn test_scenario_empty_steps() {
        let yaml = r#"
name: "Empty"
description: "No steps"
steps: []
"#;
        let scenario = Scenario::from_yaml(yaml).expect("Failed to parse");
        assert_eq!(scenario.name, "Empty");
        assert!(scenario.steps.is_empty());
    }

    #[test]
    fn test_scenario_all_step_types() {
        let yaml = r#"
name: "All types"
description: "Tests all step types"
steps:
  - type: http
    url: "http://localhost:3001/test"
    method: GET
  - type: http
    url: "http://localhost:3001/data"
    method: POST
    body:
      key: "value"
  - type: extract
    from: response.body
    path: "$.id"
    as: "extracted_id"
  - type: assert
    assertion: equals
    equals: "{{extracted_id}}"
    expected: "123"
"#;
        let scenario = Scenario::from_yaml(yaml).expect("Failed to parse");
        assert_eq!(scenario.steps.len(), 4);
        assert!(matches!(&scenario.steps[0], Step::Http(_)));
        assert!(matches!(&scenario.steps[1], Step::Http(_)));
        assert!(matches!(&scenario.steps[2], Step::Extract(_)));
        assert!(matches!(&scenario.steps[3], Step::Assert(_)));
    }

    #[test]
    fn test_scenario_all_http_methods() {
        for method_yaml in [
            "method: GET",
            "method: POST",
            "method: PUT",
            "method: PATCH",
            "method: DELETE",
        ] {
            let yaml = format!(
                r#"
name: "Method test"
description: "Test"
steps:
  - type: http
    url: "http://localhost:3001/test"
    {method_yaml}
"#,
                method_yaml = method_yaml
            );
            assert!(
                Scenario::from_yaml(&yaml).is_ok(),
                "Failed to parse scenario with {method_yaml}"
            );
        }
    }
}
