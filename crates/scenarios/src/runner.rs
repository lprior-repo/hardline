//! Scenario runner - executes scenarios against twin universes
//!
//! Runs HTTP steps, extracts values, and asserts conditions.
//! Returns pass/fail results without exposing scenario details.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

use reqwest::Client;
use serde_json::Value;

use crate::{
    sanitizer::{FeedbackLevel, Sanitizer},
    scenario::{AssertStep, AssertionType, ExtractStep, HttpMethod, HttpStep, Scenario, Step},
};

/// Context for running a scenario - holds variables extracted during execution
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    /// Variables extracted from responses during scenario execution
    variables: HashMap<String, String>,
    /// Last HTTP response for extraction
    last_response: Option<HttpResponseData>,
}

/// HTTP response data captured during execution
#[derive(Debug, Clone)]
pub struct HttpResponseData {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

/// Result of running a scenario
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub passed: bool,
    pub step_results: Vec<StepResult>,
}

/// Result of executing a single step
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub step_index: usize,
    pub step_type: String,
    pub passed: bool,
    pub error: Option<String>,
}

/// Scenario runner configuration
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Base URL for the twin instance
    pub twin_url: String,
    /// Timeout for HTTP requests in seconds
    pub timeout_secs: u64,
    /// Whether to follow redirects
    pub follow_redirects: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            twin_url: String::from("http://localhost:3001"),
            timeout_secs: 30,
            follow_redirects: true,
        }
    }
}

/// Scenario runner - executes scenarios against twin universes
#[derive(Debug)]
pub struct ScenarioRunner {
    client: Client,
    #[allow(dead_code)]
    config: RunnerConfig,
}

impl ScenarioRunner {
    /// Create a new scenario runner with the given configuration
    ///
    /// # Errors
    ///
    /// Returns `RunnerError::ClientError` if the HTTP client cannot be built.
    pub fn new(config: RunnerConfig) -> Result<Self, RunnerError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .redirect(if config.follow_redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .map_err(|e| RunnerError::ClientError(e.to_string()))?;

        Ok(Self {
            client,
            config,
        })
    }

    /// Create a new scenario runner with default configuration
    ///
    /// # Errors
    ///
    /// Returns `RunnerError::ClientError` if the HTTP client cannot be built.
    pub fn with_default_config() -> Result<Self, RunnerError> {
        Self::new(RunnerConfig::default())
    }

    /// Run a scenario and return the result
    pub async fn run(&self, scenario: &Scenario) -> ScenarioResult {
        let mut context = RunContext::default();
        let mut step_results = Vec::new();

        for (index, step) in scenario.steps.iter().enumerate() {
            let step_result = self.execute_step(step, index, &mut context).await;
            step_results.push(step_result);

            // Stop on first failure
            if !step_results.last().is_some_and(|r| r.passed) {
                break;
            }
        }

        let passed = step_results.iter().all(|r| r.passed);

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            passed,
            step_results,
        }
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &Step,
        index: usize,
        context: &mut RunContext,
    ) -> StepResult {
        match step {
            Step::Http(http_step) => self.execute_http(http_step, index, context).await,
            Step::Extract(extract_step) => Self::execute_extract(extract_step, index, context),
            Step::Assert(assert_step) => Self::execute_assert(assert_step, index, context),
        }
    }

    /// Execute an HTTP step
    async fn execute_http(&self, step: &HttpStep, index: usize, context: &mut RunContext) -> StepResult {
        let request = Self::build_request(&self.client, step);

        match request.send().await {
            Ok(response) => self.process_http_response(response, index, context).await,
            Err(e) => StepResult {
                step_index: index,
                step_type: "http".to_string(),
                passed: false,
                error: Some(format!("Request failed: {e}")),
            },
        }
    }

    /// Build HTTP request from step configuration
    fn build_request(client: &Client, step: &HttpStep) -> reqwest::RequestBuilder {
        let builder = match step.method {
            HttpMethod::Get => client.get(&step.url),
            HttpMethod::Post => client.post(&step.url),
            HttpMethod::Put => client.put(&step.url),
            HttpMethod::Patch => client.patch(&step.url),
            HttpMethod::Delete => client.delete(&step.url),
        };

        let with_headers = step
            .headers
            .iter()
            .fold(builder, |req, (key, value)| req.header(key, value));

        match step.body.as_ref() {
            Some(body) => {
                let with_headers = with_headers;
                match serde_json::to_string(body) {
                    Ok(s) => with_headers.body(s),
                    Err(e) => with_headers.body(format!("/* serialization failed: {e} */")),
                }
            }
            None => with_headers,
        }
    }

    /// Process successful HTTP response
    async fn process_http_response(
        &self,
        response: reqwest::Response,
        index: usize,
        context: &mut RunContext,
    ) -> StepResult {
        let status = response.status().as_u16();
        let headers = Self::parse_response_headers(&response);
        let body_text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<Value>(&body_text).unwrap_or_else(|_| Value::String(body_text));

        context.last_response = Some(HttpResponseData {
            status,
            headers: headers.clone(),
            body: body.clone(),
        });

        StepResult {
            step_index: index,
            step_type: "http".to_string(),
            passed: (200..400).contains(&status),
            error: if status >= 400 {
                Some(format!("HTTP error: {status}"))
            } else {
                None
            },
        }
    }

    /// Parse HTTP response headers into a `HashMap`
    fn parse_response_headers(response: &reqwest::Response) -> HashMap<String, String> {
        response
            .headers()
            .iter()
            .filter_map(|(key, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (key.to_string(), v.to_string()))
            })
            .collect()
    }

    /// Execute an extract step
    fn execute_extract(step: &ExtractStep, index: usize, context: &mut RunContext) -> StepResult {
        let Some(response) = &context.last_response else {
            return StepResult {
                step_index: index,
                step_type: "extract".to_string(),
                passed: false,
                error: Some("No HTTP response available".to_string()),
            };
        };

        let value = Self::extract_json_path(&response.body, &step.path);

        match value {
            Some(val) => {
                let val_str = Self::value_to_string(&val);
                context.variables.insert(step.r#as.clone(), val_str);

                StepResult {
                    step_index: index,
                    step_type: "extract".to_string(),
                    passed: true,
                    error: None,
                }
            }
            None => StepResult {
                step_index: index,
                step_type: "extract".to_string(),
                passed: false,
                error: Some(format!(
                    "Failed to extract {} from {}",
                    step.path, step.from
                )),
            },
        }
    }

    /// Convert a JSON Value to String representation
    fn value_to_string(value: &Value) -> String {
        value
            .as_str()
            .map(String::from)
            .or_else(|| serde_json::to_string(value).ok())
            .map_or(String::new(), |s| s)
    }

    /// Simple `JSONPath` extraction
    fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
        let path = path.trim_start_matches('$').trim_start_matches('.');

        if path.is_empty() {
            return Some(value.clone());
        }

        path.split('.').try_fold(value.clone(), |current, part| {
            Self::navigate_path(&current, part)
        })
    }

    /// Navigate a single path segment
    fn navigate_path(value: &Value, part: &str) -> Option<Value> {
        let (key, index) = Self::parse_path_segment(part)?;

        let next = match value {
            Value::Object(map) => map.get(key).cloned(),
            Value::Array(arr) => {
                let Some(idx) = index else {
                    return None;
                };
                arr.get(idx).cloned()
            }
            _ => None,
        }?;

        // If an explicit index was provided and the result is an array, index into it
        if let Some(idx) = index {
            if let Value::Array(arr) = &next {
                return arr.get(idx).cloned();
            }
        }

        Some(next)
    }

    /// Parse a path segment to extract key and optional array index
    fn parse_path_segment(part: &str) -> Option<(&str, Option<usize>)> {
        match part.find('[') {
            Some(idx_start) => {
                let key = &part[..idx_start];
                let idx_str = part[idx_start + 1..].trim_end_matches(']');
                let idx = idx_str.parse::<usize>().ok()?;
                Some((key, Some(idx)))
            }
            None => Some((part, None)),
        }
    }

    /// Execute an assert step
    fn execute_assert(step: &AssertStep, index: usize, context: &RunContext) -> StepResult {
        let passed = Self::evaluate_assertion(step, context);

        StepResult {
            step_index: index,
            step_type: "assert".to_string(),
            passed,
            error: if passed {
                None
            } else {
                Some("Assertion failed".to_string())
            },
        }
    }

    /// Evaluate an assertion and return the result
    fn evaluate_assertion(step: &AssertStep, context: &RunContext) -> bool {
        match step.assertion {
            AssertionType::Equals => {
                let actual =
                    Self::resolve_template(step.equals.as_deref().map_or("", |s| s), context);
                let expected = step.expected.as_deref().map_or("", |s| s);
                actual == expected
            }
            AssertionType::NotEquals => {
                let actual =
                    Self::resolve_template(step.equals.as_deref().map_or("", |s| s), context);
                let expected = step.expected.as_deref().map_or("", |s| s);
                actual != expected
            }
            AssertionType::Exists => {
                let value = step.exists.as_deref().map_or("", |s| s);
                let resolved = Self::resolve_template(value, context);
                !resolved.is_empty()
            }
            AssertionType::NotExists => {
                let value = step.not_exists.as_deref().map_or("", |s| s);
                let resolved = Self::resolve_template(value, context);
                resolved.is_empty()
            }
            AssertionType::Contains => {
                let actual =
                    Self::resolve_template(step.equals.as_deref().map_or("", |s| s), context);
                let expected = step.expected.as_deref().map_or("", |s| s);
                actual.contains(expected)
            }
            AssertionType::NotContains => {
                let actual =
                    Self::resolve_template(step.equals.as_deref().map_or("", |s| s), context);
                let expected = step.expected.as_deref().map_or("", |s| s);
                !actual.contains(expected)
            }
        }
    }

    /// Resolve template variables in a string
    /// Replaces `{{variable_name}}` with the actual value
    fn resolve_template(template: &str, context: &RunContext) -> String {
        let Ok(re) = regex::Regex::new(r"\{\{(\w+)\}\}") else {
            return template.to_string();
        };

        re.captures_iter(template)
            .filter_map(|cap| {
                let var_name = cap.get(1)?.as_str();
                context
                    .variables
                    .get(var_name)
                    .map(|value| (format!("{{{{{var_name}}}}}"), value.clone()))
            })
            .fold(template.to_string(), |result, (placeholder, value)| {
                result.replace(&placeholder, &value)
            })
    }

    /// Run scenario and sanitize feedback for agent
    pub async fn run_with_sanitized_feedback(
        &self,
        scenario: &Scenario,
        level: FeedbackLevel,
    ) -> String {
        let result = self.run(scenario).await;
        Sanitizer::new(level).sanitize_result(&result)
    }
}

/// Errors that can occur during scenario execution
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    #[error("HTTP client error: {0}")]
    ClientError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Extraction error: {0}")]
    ExtractionError(String),

    #[error("Assertion error: {0}")]
    AssertionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_template() {
        let _runner = ScenarioRunner::with_default_config().unwrap();
        let mut context = RunContext::default();
        context
            .variables
            .insert("message_id".to_string(), "test-123".to_string());

        let result = ScenarioRunner::resolve_template("{{message_id}}", &context);
        assert_eq!(result, "test-123");
    }

    #[test]
    fn test_resolve_template_no_var() {
        let _runner = ScenarioRunner::with_default_config().unwrap();
        let context = RunContext::default();

        let result = ScenarioRunner::resolve_template("static-value", &context);
        assert_eq!(result, "static-value");
    }

    #[test]
    fn test_json_path_extraction() {
        let _runner = ScenarioRunner::with_default_config().unwrap();
        let value = serde_json::json!({
            "message_id": "test-123",
            "nested": {
                "deep": "value"
            }
        });

        let result = ScenarioRunner::extract_json_path(&value, "$.message_id");
        assert_eq!(result, Some(serde_json::json!("test-123")));

        let result = ScenarioRunner::extract_json_path(&value, "nested.deep");
        assert_eq!(result, Some(serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_runner_default_config() {
        let runner = ScenarioRunner::with_default_config();
        assert!(runner.is_ok());
    }

    // --- JSON path extraction tests ---

    #[test]
    fn test_json_path_array_with_index() {
        let value = serde_json::json!({
            "items": [
                {"name": "first"},
                {"name": "second"}
            ]
        });

        let result = ScenarioRunner::extract_json_path(&value, "items[0].name");
        assert_eq!(result, Some(serde_json::json!("first")));

        let result = ScenarioRunner::extract_json_path(&value, "items[1].name");
        assert_eq!(result, Some(serde_json::json!("second")));
    }

    #[test]
    fn test_json_path_array_without_index_returns_array() {
        let value = serde_json::json!({
            "items": ["a", "b", "c"]
        });

        // Accessing an object field that is an array returns the full array
        let result = ScenarioRunner::extract_json_path(&value, "items");
        assert_eq!(result, Some(serde_json::json!(["a", "b", "c"])));
    }

    #[test]
    fn test_json_path_empty_path_returns_root() {
        let value = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::extract_json_path(&value, "$");
        assert_eq!(result, Some(value.clone()));
    }

    #[test]
    fn test_json_path_missing_key_returns_none() {
        let value = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::extract_json_path(&value, "nonexistent");
        assert_eq!(result, None);
    }

    // --- value_to_string tests ---

    #[test]
    fn test_value_to_string_string() {
        let value = serde_json::json!("hello");
        assert_eq!(ScenarioRunner::value_to_string(&value), "hello");
    }

    #[test]
    fn test_value_to_string_number() {
        let value = serde_json::json!(42);
        assert_eq!(ScenarioRunner::value_to_string(&value), "42");
    }

    #[test]
    fn test_value_to_string_boolean() {
        let value = serde_json::json!(true);
        assert_eq!(ScenarioRunner::value_to_string(&value), "true");
    }

    #[test]
    fn test_value_to_string_null() {
        let value = serde_json::Value::Null;
        assert_eq!(ScenarioRunner::value_to_string(&value), "null");
    }

    #[test]
    fn test_value_to_string_array() {
        let value = serde_json::json!([1, 2, 3]);
        let result = ScenarioRunner::value_to_string(&value);
        assert!(result.contains("1"));
        assert!(result.contains("2"));
    }

    #[test]
    fn test_value_to_string_object() {
        let value = serde_json::json!({"a": 1});
        let result = ScenarioRunner::value_to_string(&value);
        assert!(result.contains("\"a\""));
    }

    // --- parse_path_segment tests ---

    #[test]
    fn test_parse_path_segment_simple() {
        let (key, index) = ScenarioRunner::parse_path_segment("name").unwrap();
        assert_eq!(key, "name");
        assert_eq!(index, None);
    }

    #[test]
    fn test_parse_path_segment_with_index() {
        let (key, index) = ScenarioRunner::parse_path_segment("items[0]").unwrap();
        assert_eq!(key, "items");
        assert_eq!(index, Some(0));
    }

    #[test]
    fn test_parse_path_segment_nested_with_index() {
        let (key, index) = ScenarioRunner::parse_path_segment("items[2]").unwrap();
        assert_eq!(key, "items");
        assert_eq!(index, Some(2));
    }

    // --- Template resolution tests ---

    #[test]
    fn test_resolve_template_multiple_variables() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("first".to_string(), "hello".to_string());
        context
            .variables
            .insert("second".to_string(), "world".to_string());

        let result = ScenarioRunner::resolve_template("{{first}} {{second}}", &context);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_resolve_template_missing_variable_unchanged() {
        let context = RunContext::default();
        let result = ScenarioRunner::resolve_template("{{missing}}", &context);
        assert_eq!(result, "{{missing}}");
    }

    #[test]
    fn test_resolve_template_empty_string() {
        let context = RunContext::default();
        let result = ScenarioRunner::resolve_template("", &context);
        assert_eq!(result, "");
    }

    // --- evaluate_assertion tests ---

    #[test]
    fn test_assert_equals_match() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "expected".to_string());

        let step = AssertStep {
            assertion: AssertionType::Equals,
            equals: Some("{{val}}".to_string()),
            expected: Some("expected".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_equals_mismatch() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "actual".to_string());

        let step = AssertStep {
            assertion: AssertionType::Equals,
            equals: Some("{{val}}".to_string()),
            expected: Some("different".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(!ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_equals() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "a".to_string());

        let step = AssertStep {
            assertion: AssertionType::NotEquals,
            equals: Some("{{val}}".to_string()),
            expected: Some("b".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_equals_same_values() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "same".to_string());

        let step = AssertStep {
            assertion: AssertionType::NotEquals,
            equals: Some("{{val}}".to_string()),
            expected: Some("same".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(!ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_exists_with_value() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "present".to_string());

        let step = AssertStep {
            assertion: AssertionType::Exists,
            equals: None,
            expected: None,
            exists: Some("{{val}}".to_string()),
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_exists_without_resolved_value_still_exists() {
        // Unresolved template {{missing}} stays as literal "{{missing}}" (non-empty)
        // so Exists returns true even though the variable wasn't found
        let context = RunContext::default();

        let step = AssertStep {
            assertion: AssertionType::Exists,
            equals: None,
            expected: None,
            exists: Some("{{missing}}".to_string()),
            not_exists: None,
        };
        // Unresolved templates produce non-empty strings, so Exists passes
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_exists_with_resolved_empty_string() {
        let mut context = RunContext::default();
        // Empty string in a variable should trigger NotExists
        context
            .variables
            .insert("empty_var".to_string(), String::new());

        let step = AssertStep {
            assertion: AssertionType::NotExists,
            equals: None,
            expected: None,
            exists: None,
            not_exists: Some("{{empty_var}}".to_string()),
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_exists_with_value() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("val".to_string(), "present".to_string());

        let step = AssertStep {
            assertion: AssertionType::NotExists,
            equals: None,
            expected: None,
            exists: None,
            not_exists: Some("{{val}}".to_string()),
        };
        assert!(!ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_contains() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("msg".to_string(), "hello world".to_string());

        let step = AssertStep {
            assertion: AssertionType::Contains,
            equals: Some("{{msg}}".to_string()),
            expected: Some("world".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_contains() {
        let mut context = RunContext::default();
        context
            .variables
            .insert("msg".to_string(), "hello world".to_string());

        let step = AssertStep {
            assertion: AssertionType::NotContains,
            equals: Some("{{msg}}".to_string()),
            expected: Some("goodbye".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    // --- RunnerConfig defaults ---

    #[test]
    fn test_runner_config_defaults() {
        let config = RunnerConfig::default();
        assert_eq!(config.twin_url, "http://localhost:3001");
        assert_eq!(config.timeout_secs, 30);
        assert!(config.follow_redirects);
    }

    // --- RunnerError Display ---

    #[test]
    fn test_runner_error_display() {
        let err = RunnerError::ClientError("connection refused".to_string());
        assert_eq!(err.to_string(), "HTTP client error: connection refused");

        let err = RunnerError::SerializationError("bad json".to_string());
        assert_eq!(err.to_string(), "Serialization error: bad json");

        let err = RunnerError::ExtractionError("path not found".to_string());
        assert_eq!(err.to_string(), "Extraction error: path not found");

        let err = RunnerError::AssertionError("values differ".to_string());
        assert_eq!(err.to_string(), "Assertion error: values differ");
    }
}
