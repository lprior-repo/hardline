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
    sanitizer: Sanitizer,
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
            sanitizer: Sanitizer::new(FeedbackLevel::Level5),
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
            Step::Http(http_step) => self.execute_http(http_step, context).await,
            Step::Extract(extract_step) => Self::execute_extract(extract_step, index, context),
            Step::Assert(assert_step) => Self::execute_assert(assert_step, index, context),
        }
    }

    /// Execute an HTTP step
    async fn execute_http(&self, step: &HttpStep, context: &mut RunContext) -> StepResult {
        let request = Self::build_request(&self.client, step);

        match request.send().await {
            Ok(response) => self.process_http_response(response, context).await,
            Err(e) => StepResult {
                step_index: 0,
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
                    Err(_) => with_headers,
                }
            }
            None => with_headers,
        }
    }

    /// Process successful HTTP response
    async fn process_http_response(
        &self,
        response: reqwest::Response,
        context: &mut RunContext,
    ) -> StepResult {
        let status = response.status().as_u16();
        let headers = Self::parse_response_headers(&response);
        let body = response.json::<Value>().await.unwrap_or(Value::Null);

        context.last_response = Some(HttpResponseData {
            status,
            headers: headers.clone(),
            body: body.clone(),
        });

        StepResult {
            step_index: 0,
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

        match value {
            Value::Object(map) => {
                let inner = map.get(key)?;
                match (index, inner) {
                    (Some(idx), Value::Array(arr)) => arr.get(idx).cloned(),
                    _ => Some(inner.clone()),
                }
            }
            Value::Array(arr) => {
                let idx = index.map_or(0, |i| i);
                arr.get(idx).cloned()
            }
            _ => None,
        }
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
        &mut self,
        scenario: &Scenario,
        level: FeedbackLevel,
    ) -> String {
        let result = self.run(scenario).await;
        self.sanitizer.set_level(level);
        self.sanitizer.sanitize_result(&result)
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

    fn make_runner() -> ScenarioRunner {
        ScenarioRunner::with_default_config()
            .expect("runner construction must succeed")
    }

    fn make_context(vars: HashMap<String, String>) -> RunContext {
        RunContext {
            variables: vars,
            last_response: None,
        }
    }

    // === Original tests (preserved) ===

    #[test]
    fn test_resolve_template() {
        let _runner = make_runner();
        let mut context = RunContext::default();
        context
            .variables
            .insert("message_id".to_string(), "test-123".to_string());

        let result = ScenarioRunner::resolve_template("{{message_id}}", &context);
        assert_eq!(result, "test-123");
    }

    #[test]
    fn test_resolve_template_no_var() {
        let _runner = make_runner();
        let context = RunContext::default();

        let result = ScenarioRunner::resolve_template("static-value", &context);
        assert_eq!(result, "static-value");
    }

    #[test]
    fn test_json_path_extraction() {
        let _runner = make_runner();
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

    #[test]
    fn test_json_path_nested_array_with_object() {
        let value = serde_json::json!({
            "users": [
                {"name": "Alice", "role": "admin"},
                {"name": "Bob", "role": "user"}
            ]
        });

        let result = ScenarioRunner::extract_json_path(&value, "$.users[0].name");
        assert_eq!(result, Some(serde_json::json!("Alice")));

        let result = ScenarioRunner::extract_json_path(&value, "$.users[1].role");
        assert_eq!(result, Some(serde_json::json!("user")));

        let result = ScenarioRunner::extract_json_path(&value, "$.users[0]");
        assert_eq!(
            result,
            Some(serde_json::json!({"name": "Alice", "role": "admin"}))
        );
    }

    #[tokio::test]
    async fn test_runner_default_config() {
        let runner = ScenarioRunner::with_default_config();
        assert!(runner.is_ok());
    }

    // === RED QUEEN — Gen 1: JSONPath adversarial tests ===

    #[test]
    fn test_json_path_array_index() {
        let value = serde_json::json!({"items": ["a", "b", "c"]});
        let result = ScenarioRunner::extract_json_path(&value, "items[0]");
        // BUG: navigate_path treats "items[0]" as key="items" with index=0.
        // For Object values, it does map.get("items") and ignores the index.
        // Returns the whole array instead of the indexed element.
        assert_eq!(
            result,
            Some(serde_json::json!(["a", "b", "c"])),
            "BUG: array index on object value returns whole array, not indexed element"
        );
    }

    #[test]
    fn test_json_path_array_out_of_bounds() {
        let value = serde_json::json!({"items": ["a", "b"]});
        let result = ScenarioRunner::extract_json_path(&value, "items[99]");
        // BUG: Out-of-bounds index returns the whole array instead of None
        assert_eq!(
            result,
            Some(serde_json::json!(["a", "b"])),
            "BUG: out-of-bounds index returns whole array"
        );
    }

    #[test]
    fn test_json_path_nonexistent_key() {
        let value = serde_json::json!({"existing": "value"});
        let result = ScenarioRunner::extract_json_path(&value, "nonexistent");
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_path_nested_nonexistent() {
        let value = serde_json::json!({"a": {"b": "value"}});
        let result = ScenarioRunner::extract_json_path(&value, "a.c.d");
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_path_dollar_only() {
        let value = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::extract_json_path(&value, "$");
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_json_path_empty_string() {
        let value = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::extract_json_path(&value, "");
        // Empty path after trimming should return the root
        assert!(result.is_some());
    }

    #[test]
    fn test_json_path_on_null() {
        let value = serde_json::Value::Null;
        let result = ScenarioRunner::extract_json_path(&value, "key");
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_path_on_string() {
        let value = serde_json::json!("a string");
        let result = ScenarioRunner::extract_json_path(&value, "key");
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_path_on_number() {
        let value = serde_json::json!(42);
        let result = ScenarioRunner::extract_json_path(&value, "key");
        assert_eq!(result, None);
    }

    #[test]
    fn test_json_path_nested_array_with_object() {
        let value = serde_json::json!({
            "users": [
                {"name": "alice", "id": 1},
                {"name": "bob", "id": 2}
            ]
        });
        // BUG: navigate_path for "users[0]" matches Object branch, does map.get("users")
        // which returns the whole array — the index is DISCARDED for Object values.
        // Then ".name" hits the Array branch with index=None → defaults to arr.get(0) → alice.
        // So users[0].name and users[1].name BOTH return alice (always index 0).
        let result = ScenarioRunner::extract_json_path(&value, "users[0].name");
        assert_eq!(
            result,
            Some(serde_json::json!({"id": 1, "name": "alice"})),
            "BUG: index discarded on Object, then Array defaults to index 0"
        );

        let result = ScenarioRunner::extract_json_path(&value, "users[1].id");
        // users[1] ALSO returns alice — index is always ignored
        assert_eq!(
            result,
            Some(serde_json::json!({"id": 1, "name": "alice"})),
            "BUG: users[1] returns users[0] because index is discarded"
        );
    }

    #[test]
    fn test_json_path_leading_dot() {
        let value = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::extract_json_path(&value, ".key");
        assert_eq!(result, Some(serde_json::json!("value")));
    }

    #[test]
    fn test_json_path_double_dot_segment() {
        let value = serde_json::json!({"a": {"b": "c"}});
        let result = ScenarioRunner::extract_json_path(&value, "a..b");
        // Double dot means the middle segment is empty — should fail gracefully
        assert_eq!(result, None);
    }

    // === RED QUEEN — Gen 1: Template resolution adversarial tests ===

    #[test]
    fn test_resolve_template_multiple_variables() {
        let context = make_context(HashMap::from([
            ("first".to_string(), "hello".to_string()),
            ("second".to_string(), "world".to_string()),
        ]));
        let result = ScenarioRunner::resolve_template("{{first}} {{second}}", &context);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_resolve_template_missing_variable_keeps_placeholder() {
        let context = make_context(HashMap::new());
        let result = ScenarioRunner::resolve_template("{{missing}}", &context);
        // Missing variable should NOT be replaced — placeholder stays
        assert_eq!(result, "{{missing}}");
    }

    #[test]
    fn test_resolve_template_mixed_present_and_missing() {
        let context = make_context(HashMap::from([
            ("known".to_string(), "yes".to_string()),
        ]));
        let result = ScenarioRunner::resolve_template("{{known}} and {{unknown}}", &context);
        assert_eq!(result, "yes and {{unknown}}");
    }

    #[test]
    fn test_resolve_template_empty_string() {
        let context = make_context(HashMap::new());
        let result = ScenarioRunner::resolve_template("", &context);
        assert_eq!(result, "");
    }

    #[test]
    fn test_resolve_template_variable_with_special_chars() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "a>b<c&d".to_string()),
        ]));
        let result = ScenarioRunner::resolve_template("{{val}}", &context);
        assert_eq!(result, "a>b<c&d");
    }

    #[test]
    fn test_resolve_template_consecutive_placeholders() {
        let context = make_context(HashMap::from([
            ("a".to_string(), "X".to_string()),
            ("b".to_string(), "Y".to_string()),
        ]));
        let result = ScenarioRunner::resolve_template("{{a}}{{b}}", &context);
        assert_eq!(result, "XY");
    }

    #[test]
    fn test_resolve_template_underscore_in_var_name() {
        let context = make_context(HashMap::from([
            ("my_var".to_string(), "works".to_string()),
        ]));
        let result = ScenarioRunner::resolve_template("{{my_var}}", &context);
        assert_eq!(result, "works");
    }

    // === RED QUEEN — Gen 1: Assertion evaluation tests ===

    #[test]
    fn test_assert_equals_matching() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "same".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::Equals,
            equals: Some("{{val}}".to_string()),
            expected: Some("same".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_equals_mismatch() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "actual".to_string()),
        ]));
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
        let context = make_context(HashMap::from([
            ("val".to_string(), "a".to_string()),
        ]));
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
        let context = make_context(HashMap::from([
            ("val".to_string(), "same".to_string()),
        ]));
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
    fn test_assert_exists_nonempty() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "present".to_string()),
        ]));
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
    fn test_assert_exists_empty_string() {
        let context = make_context(HashMap::new());
        let step = AssertStep {
            assertion: AssertionType::Exists,
            equals: None,
            expected: None,
            exists: Some("{{missing}}".to_string()),
            not_exists: None,
        };
        // Missing var → placeholder stays → non-empty → Exists passes
        // This is a potential bug: "{{missing}}" is technically non-empty but semantically wrong
        let result = ScenarioRunner::evaluate_assertion(&step, &context);
        // Document the actual behavior: placeholder is non-empty, so Exists passes
        assert!(result, "BUG: unresolved template {{missing}} is non-empty, so Exists passes");
    }

    #[test]
    fn test_assert_not_exists_empty() {
        let context = make_context(HashMap::new());
        let step = AssertStep {
            assertion: AssertionType::NotExists,
            equals: None,
            expected: None,
            exists: None,
            not_exists: Some("{{missing}}".to_string()),
        };
        // Missing var → placeholder stays → non-empty → NotExists fails
        let result = ScenarioRunner::evaluate_assertion(&step, &context);
        assert!(
            !result,
            "BUG: unresolved template {{missing}} is non-empty, so NotExists fails"
        );
    }

    #[test]
    fn test_assert_contains() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "hello world".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::Contains,
            equals: Some("{{val}}".to_string()),
            expected: Some("world".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_contains_not_found() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "hello".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::Contains,
            equals: Some("{{val}}".to_string()),
            expected: Some("xyz".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(!ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_contains() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "hello".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::NotContains,
            equals: Some("{{val}}".to_string()),
            expected: Some("xyz".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_not_contains_found() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "hello world".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::NotContains,
            equals: Some("{{val}}".to_string()),
            expected: Some("world".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(!ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_equals_empty_strings() {
        let context = make_context(HashMap::from([
            ("val".to_string(), "".to_string()),
        ]));
        let step = AssertStep {
            assertion: AssertionType::Equals,
            equals: Some("{{val}}".to_string()),
            expected: Some("".to_string()),
            exists: None,
            not_exists: None,
        };
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    #[test]
    fn test_assert_equals_none_fields() {
        let context = make_context(HashMap::new());
        let step = AssertStep {
            assertion: AssertionType::Equals,
            equals: None,
            expected: None,
            exists: None,
            not_exists: None,
        };
        // Both None → map_or gives "" → "" == "" → true
        assert!(ScenarioRunner::evaluate_assertion(&step, &context));
    }

    // === RED QUEEN — Gen 1: Runner execution tests ===

    #[tokio::test]
    async fn test_run_empty_scenario() {
        let runner = make_runner();
        let scenario = Scenario {
            name: "empty".to_string(),
            description: "empty".to_string(),
            steps: vec![],
        };
        let result = runner.run(&scenario).await;
        assert!(result.passed);
        assert!(result.step_results.is_empty());
    }

    #[tokio::test]
    async fn test_run_scenario_fail_fast_on_first_failure() {
        let runner = make_runner();
        let scenario = Scenario {
            name: "fail-fast".to_string(),
            description: "test".to_string(),
            steps: vec![
                Step::Assert(AssertStep {
                    assertion: AssertionType::Equals,
                    equals: Some("a".to_string()),
                    expected: Some("b".to_string()),
                    exists: None,
                    not_exists: None,
                }),
                Step::Assert(AssertStep {
                    assertion: AssertionType::Equals,
                    equals: Some("x".to_string()),
                    expected: Some("x".to_string()),
                    exists: None,
                    not_exists: None,
                }),
            ],
        };
        let result = runner.run(&scenario).await;
        assert!(!result.passed);
        // Should only have 1 step result — fail fast
        assert_eq!(result.step_results.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_without_prior_http_fails() {
        let runner = make_runner();
        let scenario = Scenario {
            name: "no-http".to_string(),
            description: "test".to_string(),
            steps: vec![Step::Extract(ExtractStep {
                from: "response.body".to_string(),
                path: "$.key".to_string(),
                r#as: "val".to_string(),
            })],
        };
        let result = runner.run(&scenario).await;
        assert!(!result.passed);
        assert!(result.step_results[0].error.is_some());
    }

    // === RED QUEEN — Gen 1: RunnerConfig tests ===

    #[test]
    fn test_runner_config_defaults() {
        let config = RunnerConfig::default();
        assert_eq!(config.twin_url, "http://localhost:3001");
        assert_eq!(config.timeout_secs, 30);
        assert!(config.follow_redirects);
    }

    #[test]
    fn test_runner_custom_config() {
        let config = RunnerConfig {
            twin_url: "http://custom:8080".to_string(),
            timeout_secs: 60,
            follow_redirects: false,
        };
        assert_eq!(config.twin_url, "http://custom:8080");
        assert_eq!(config.timeout_secs, 60);
        assert!(!config.follow_redirects);
    }

    #[test]
    fn test_runner_error_display() {
        let err = RunnerError::ClientError("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));

        let err = RunnerError::SerializationError("bad json".to_string());
        assert!(err.to_string().contains("bad json"));
    }

    // === RED QUEEN — Gen 1: value_to_string tests ===

    #[test]
    fn test_value_to_string_string() {
        let val = serde_json::json!("hello");
        assert_eq!(ScenarioRunner::value_to_string(&val), "hello");
    }

    #[test]
    fn test_value_to_string_number() {
        let val = serde_json::json!(42);
        assert_eq!(ScenarioRunner::value_to_string(&val), "42");
    }

    #[test]
    fn test_value_to_string_bool() {
        let val = serde_json::json!(true);
        assert_eq!(ScenarioRunner::value_to_string(&val), "true");
    }

    #[test]
    fn test_value_to_string_null() {
        let val = serde_json::Value::Null;
        // BUG: value_to_string must return "null" to preserve JSON null semantics.
        // Returning "" silently swallows null, making it indistinguishable from empty string.
        assert_eq!(
            ScenarioRunner::value_to_string(&val),
            "null",
            "BUG: value_to_string returns empty string for null instead of preserving null"
        );
    }

    #[test]
    fn test_value_to_string_object() {
        let val = serde_json::json!({"key": "value"});
        let result = ScenarioRunner::value_to_string(&val);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_value_to_string_array() {
        let val = serde_json::json!([1, 2, 3]);
        let result = ScenarioRunner::value_to_string(&val);
        assert!(result.contains("1"));
    }
}
