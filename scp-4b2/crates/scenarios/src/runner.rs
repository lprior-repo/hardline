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
        let context = RunContext::default();
        let step_results = Self::execute_scenario_steps(scenario, self).await;
        let passed = step_results.iter().all(|r| r.passed);

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            passed,
            step_results,
        }
    }

    /// Execute all scenario steps with early exit on failure - pure async iteration
    async fn execute_scenario_steps(scenario: &Scenario, runner: &ScenarioRunner) -> Vec<StepResult> {
        let mut context = RunContext::default();
        let mut results = Vec::new();

        for (index, step) in scenario.steps.iter().enumerate() {
            let step_result = runner.execute_step(step, index, &mut context).await;
            results.push(step_result);

            if !results.last().is_some_and(|r| r.passed) {
                break;
            }
        }

        results
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
        let request = self.build_request(step);

        match request.send().await {
            Ok(response) => self.handle_http_response(response, context).await,
            Err(e) => Self::make_http_error_result(e),
        }
    }

    /// Build HTTP request from step config - pure calculation
    fn build_request(&self, step: &HttpStep) -> reqwest::RequestBuilder {
        let request = match step.method {
            HttpMethod::Get => self.client.get(&step.url),
            HttpMethod::Post => self.client.post(&step.url),
            HttpMethod::Put => self.client.put(&step.url),
            HttpMethod::Patch => self.client.patch(&step.url),
            HttpMethod::Delete => self.client.delete(&step.url),
        };

        let with_headers = step
            .headers
            .iter()
            .fold(request, |req, (key, value)| req.header(key, value));

        match step.body.as_ref() {
            Some(body) => {
                let body_str = serde_json::to_string(body).map_or(String::new(), |s| s);
                with_headers.body(body_str)
            }
            None => with_headers,
        }
    }

    /// Handle successful HTTP response - pure calculation
    fn handle_http_response(
        &self,
        response: reqwest::Response,
        context: &mut RunContext,
    ) -> StepResult {
        let status = response.status().as_u16();
        let headers = Self::collect_response_headers(&response);
        let body = Self::extract_response_body(response);

        context.last_response = Some(HttpResponseData {
            status,
            headers: headers.clone(),
            body: body.clone(),
        });

        Self::make_http_success_result(status, headers, body)
    }

    /// Collect headers from HTTP response - pure calculation
    fn collect_response_headers(response: &reqwest::Response) -> HashMap<String, String> {
        response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
            .collect()
    }

    /// Extract body from HTTP response - async but pure transformation
    async fn extract_response_body(response: reqwest::Response) -> Value {
        response.json::<Value>().await.unwrap_or(Value::Null)
    }

    /// Build HTTP success result - pure calculation
    fn make_http_success_result(
        status: u16,
        _headers: HashMap<String, String>,
        _body: Value,
    ) -> StepResult {
        StepResult {
            step_index: 0,
            step_type: "http".to_string(),
            passed: (200..400).contains(&status),
            error: (status >= 400).then(|| format!("HTTP error: {status}")),
        }
    }

    /// Build HTTP error result - pure calculation
    fn make_http_error_result(e: reqwest::Error) -> StepResult {
        StepResult {
            step_index: 0,
            step_type: "http".to_string(),
            passed: false,
            error: Some(format!("Request failed: {e}")),
        }
    }

    /// Execute an extract step
    fn execute_extract(step: &ExtractStep, index: usize, context: &mut RunContext) -> StepResult {
        match context.last_response.as_ref() {
            None => Self::make_extract_no_response_result(index),
            Some(response) => Self::execute_extract_from_response(step, index, context, response),
        }
    }

    /// Handle extract when no response available - pure calculation
    fn make_extract_no_response_result(index: usize) -> StepResult {
        StepResult {
            step_index: index,
            step_type: "extract".to_string(),
            passed: false,
            error: Some("No HTTP response available".to_string()),
        }
    }

    /// Execute extraction from response - pure calculation
    fn execute_extract_from_response(
        step: &ExtractStep,
        index: usize,
        context: &mut RunContext,
        response: &HttpResponseData,
    ) -> StepResult {
        Self::extract_json_path(&response.body, &step.path).map_or_else(
            || Self::make_extract_failure_result(step, index),
            |val| Self::store_extracted_value(step, index, context, val),
        )
    }

    /// Store extracted value in context - pure calculation
    fn store_extracted_value(
        step: &ExtractStep,
        index: usize,
        context: &mut RunContext,
        val: Value,
    ) -> StepResult {
        let val_str = Self::value_to_string(&val);
        context.variables.insert(step.r#as.clone(), val_str);
        StepResult {
            step_index: index,
            step_type: "extract".to_string(),
            passed: true,
            error: None,
        }
    }

    /// Convert JSON value to string - pure calculation
    fn value_to_string(val: &Value) -> String {
        val.as_str()
            .map(String::from)
            .or_else(|| serde_json::to_string(val).ok())
            .unwrap_or_default()
    }

    /// Build extract failure result - pure calculation
    fn make_extract_failure_result(step: &ExtractStep, index: usize) -> StepResult {
        StepResult {
            step_index: index,
            step_type: "extract".to_string(),
            passed: false,
            error: Some(format!(
                "Failed to extract {} from {}",
                step.path, step.from
            )),
        }
    }

    /// Simple `JSONPath` extraction
    fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
        let path = path.trim_start_matches('$').trim_start_matches('.');

        if path.is_empty() {
            return Some(value.clone());
        }

        path.split('.').try_fold(value.clone(), |current, part| {
            Self::navigate_path_part(&current, part)
        })
    }

    /// Navigate a single path part - pure calculation
    fn navigate_path_part(current: &Value, part: &str) -> Option<Value> {
        let (key, idx) = Self::parse_path_part(part);

        match (current, key, idx) {
            (Value::Object(map), Some(k), None) => map.get(k).cloned(),
            (Value::Array(arr), None, Some(i)) => arr.get(i).cloned(),
            (Value::Array(arr), Some(k), Some(i)) => arr.get(i).and_then(|item| {
                if let Value::Object(map) = item {
                    map.get(k).cloned()
                } else {
                    None
                }
            }),
            _ => None,
        }
    }

    /// Parse a path part into (key, array_index) - pure calculation
    fn parse_path_part(part: &str) -> (Option<&str>, Option<usize>) {
        part.find('[').map_or((Some(part), None), |idx_start| {
            let key = &part[..idx_start];
            let idx_str = part[idx_start + 1..].trim_end_matches(']');
            let idx = idx_str.parse::<usize>().ok();
            (Some(key), idx)
        })
    }

    /// Execute an assert step
    fn execute_assert(step: &AssertStep, index: usize, context: &RunContext) -> StepResult {
        Self::run_assertion(&step.assertion, step, context).map_or_else(
            |e| StepResult {
                step_index: index,
                step_type: "assert".to_string(),
                passed: false,
                error: Some(e.to_string()),
            },
            |passed| StepResult {
                step_index: index,
                step_type: "assert".to_string(),
                passed,
                error: passed.then_some("Assertion failed".to_string()),
            },
        )
    }

    /// Run assertion and return pass/fail - pure calculation
    fn run_assertion(
        assertion: &AssertionType,
        step: &AssertStep,
        context: &RunContext,
    ) -> Result<bool, RunnerError> {
        match assertion {
            AssertionType::Equals => Ok(Self::assert_equals(step, context)),
            AssertionType::NotEquals => Ok(Self::assert_not_equals(step, context)),
            AssertionType::Exists => Ok(Self::assert_exists(step, context)),
            AssertionType::NotExists => Ok(Self::assert_not_exists(step, context)),
            AssertionType::Contains => Ok(Self::assert_contains(step, context)),
            AssertionType::NotContains => Ok(Self::assert_not_contains(step, context)),
        }
    }

    /// Assert equals - pure calculation
    fn assert_equals(step: &AssertStep, context: &RunContext) -> bool {
        let actual = Self::resolve_or_empty(step.equals.as_deref(), context);
        let expected = Self::resolve_or_empty(step.expected.as_deref(), context);
        actual == expected
    }

    /// Assert not equals - pure calculation
    fn assert_not_equals(step: &AssertStep, context: &RunContext) -> bool {
        let actual = Self::resolve_or_empty(step.equals.as_deref(), context);
        let expected = Self::resolve_or_empty(step.expected.as_deref(), context);
        actual != expected
    }

    /// Assert exists - pure calculation
    fn assert_exists(step: &AssertStep, context: &RunContext) -> bool {
        !Self::resolve_or_empty(step.exists.as_deref(), context).is_empty()
    }

    /// Assert not exists - pure calculation
    fn assert_not_exists(step: &AssertStep, context: &RunContext) -> bool {
        Self::resolve_or_empty(step.not_exists.as_deref(), context).is_empty()
    }

    /// Assert contains - pure calculation
    fn assert_contains(step: &AssertStep, context: &RunContext) -> bool {
        let actual = Self::resolve_or_empty(step.equals.as_deref(), context);
        let expected = Self::resolve_or_empty(step.expected.as_deref(), context);
        actual.contains(&expected)
    }

    /// Assert not contains - pure calculation
    fn assert_not_contains(step: &AssertStep, context: &RunContext) -> bool {
        let actual = Self::resolve_or_empty(step.equals.as_deref(), context);
        let expected = Self::resolve_or_empty(step.expected.as_deref(), context);
        !actual.contains(&expected)
    }

    /// Resolve template or return empty string - pure calculation
    fn resolve_or_empty(opt: Option<&str>, context: &RunContext) -> String {
        opt.map_or_else(|| String::new(), |s| Self::resolve_template(s, context))
    }

    /// Resolve template variables in a string
    /// Replaces `{{variable_name}}` with the actual value
    fn resolve_template(template: &str, context: &RunContext) -> String {
        let re = match regex::Regex::new(r"\{\{(\w+)\}\}") {
            Ok(re) => re,
            Err(_) => return template.to_string(),
        };

        let ctx_vars = &context.variables;
        re.replace_all(template, |caps: &regex::Captures| {
            caps.get(1)
                .and_then(|m| ctx_vars.get(m.as_str()).cloned())
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string()
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
}
