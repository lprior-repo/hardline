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
        let mut request = match step.method {
            HttpMethod::Get => self.client.get(&step.url),
            HttpMethod::Post => self.client.post(&step.url),
            HttpMethod::Put => self.client.put(&step.url),
            HttpMethod::Patch => self.client.patch(&step.url),
            HttpMethod::Delete => self.client.delete(&step.url),
        };

        // Add headers
        for (key, value) in &step.headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body) = &step.body {
            let body_str = serde_json::to_string(body)
                .map_err(|e| RunnerError::SerializationError(e.to_string()))
                .unwrap_or_default();
            request = request.body(body_str);
        }

        // Execute request
        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let mut headers = HashMap::new();
                for (key, value) in response.headers() {
                    if let Ok(v) = value.to_str() {
                        headers.insert(key.to_string(), v.to_string());
                    }
                }

                let body = match response.json::<Value>().await {
                    Ok(v) => v,
                    Err(_) => Value::Null,
                };

                context.last_response = Some(HttpResponseData {
                    status,
                    headers,
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
            Err(e) => StepResult {
                step_index: 0,
                step_type: "http".to_string(),
                passed: false,
                error: Some(format!("Request failed: {e}")),
            },
        }
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

        // Simple JSONPath-like extraction (supports "$.path.to.value" and "path.to.value")
        let value = Self::extract_json_path(&response.body, &step.path);

        match value {
            Some(val) => {
                let val_str = if let Some(s) = val.as_str() {
                    s.to_string()
                } else {
                    serde_json::to_string(&val).unwrap_or_default()
                };

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

    /// Simple `JSONPath` extraction
    fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
        let path = path.trim_start_matches('$').trim_start_matches('.');

        if path.is_empty() {
            return Some(value.clone());
        }

        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value.clone();

        for part in parts {
            // Handle array indexing like "items[0]"
            let (key, index) = if let Some(idx_start) = part.find('[') {
                let key = &part[..idx_start];
                let idx_str = part[idx_start + 1..].trim_end_matches(']');
                let idx = idx_str.parse::<usize>().ok()?;
                (key, Some(idx))
            } else {
                (part, None)
            };

            current = match &current {
                Value::Object(map) => map.get(key)?.clone(),
                Value::Array(arr) => {
                    let idx = index.unwrap_or(0);
                    arr.get(idx)?.clone()
                }
                _ => return None,
            };
        }

        Some(current)
    }

    /// Execute an assert step
    fn execute_assert(step: &AssertStep, index: usize, context: &RunContext) -> StepResult {
        let assertion = step.assertion;

        let result: Result<bool, RunnerError> = match assertion {
            AssertionType::Equals => {
                let actual =
                    ScenarioRunner::resolve_template(step.equals.as_deref().unwrap_or(""), context);
                let expected = step.expected.as_deref().unwrap_or("");
                Ok(actual == expected)
            }
            AssertionType::NotEquals => {
                let actual =
                    ScenarioRunner::resolve_template(step.equals.as_deref().unwrap_or(""), context);
                let expected = step.expected.as_deref().unwrap_or("");
                Ok(actual != expected)
            }
            AssertionType::Exists => {
                let value = step.exists.as_deref().unwrap_or("");
                let resolved = ScenarioRunner::resolve_template(value, context);
                Ok(!resolved.is_empty())
            }
            AssertionType::NotExists => {
                let value = step.not_exists.as_deref().unwrap_or("");
                let resolved = ScenarioRunner::resolve_template(value, context);
                Ok(resolved.is_empty())
            }
            AssertionType::Contains => {
                let actual =
                    ScenarioRunner::resolve_template(step.equals.as_deref().unwrap_or(""), context);
                let expected = step.expected.as_deref().unwrap_or("");
                Ok(actual.contains(expected))
            }
            AssertionType::NotContains => {
                let actual =
                    ScenarioRunner::resolve_template(step.equals.as_deref().unwrap_or(""), context);
                let expected = step.expected.as_deref().unwrap_or("");
                Ok(!actual.contains(expected))
            }
        };

        match result {
            Ok(passed) => StepResult {
                step_index: index,
                step_type: "assert".to_string(),
                passed,
                error: if passed {
                    None
                } else {
                    Some("Assertion failed".to_string())
                },
            },
            Err(e) => StepResult {
                step_index: index,
                step_type: "assert".to_string(),
                passed: false,
                error: Some(e.to_string()),
            },
        }
    }

    /// Resolve template variables in a string
    /// Replaces `{{variable_name}}` with the actual value
    fn resolve_template(template: &str, context: &RunContext) -> String {
        let mut result = template.to_string();

        // Simple regex to match {{variable_name}}
        // This regex is guaranteed to be valid
        let Ok(re) = regex::Regex::new(r"\{\{(\w+)\}\}") else {
            return result;
        };

        for cap in re.captures_iter(template) {
            if let Some(var_name) = cap.get(1) {
                let var = var_name.as_str();
                if let Some(value) = context.variables.get(var) {
                    result = result.replace(&format!("{{{{{var}}}}}"), value);
                }
            }
        }

        result
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
