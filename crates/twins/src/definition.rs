#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Twin definition parsing module
//!
//! Parses twin definition YAML files into structured types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DefinitionError {
    #[error("Failed to parse YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GET => write!(f, "GET"),
            Self::POST => write!(f, "POST"),
            Self::PUT => write!(f, "PUT"),
            Self::DELETE => write!(f, "DELETE"),
            Self::PATCH => write!(f, "PATCH"),
            Self::OPTIONS => write!(f, "OPTIONS"),
            Self::HEAD => write!(f, "HEAD"),
        }
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = DefinitionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::GET),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "DELETE" => Ok(Self::DELETE),
            "PATCH" => Ok(Self::PATCH),
            "OPTIONS" => Ok(Self::OPTIONS),
            "HEAD" => Ok(Self::HEAD),
            _ => Err(DefinitionError::InvalidMethod(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointResponse {
    pub status: u16,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub path: String,
    pub method: HttpMethod,
    pub response: EndpointResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinDefinition {
    pub name: String,
    pub port: u16,
    pub endpoints: Vec<Endpoint>,
}

impl TwinDefinition {
    /// Parses a YAML string into a `TwinDefinition`.
    ///
    /// # Errors
    ///
    /// Returns `DefinitionError` if the YAML is invalid or fails validation.
    pub fn from_yaml(yaml: &str) -> Result<Self, DefinitionError> {
        let def = serde_yaml::from_str::<Self>(yaml)?;
        def.validate()?;
        Ok(def)
    }

    /// Parses YAML bytes into a `TwinDefinition`.
    ///
    /// # Errors
    ///
    /// Returns `DefinitionError` if the bytes are invalid YAML or fail validation.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self, DefinitionError> {
        let def = serde_yaml::from_slice::<Self>(bytes)?;
        def.validate()?;
        Ok(def)
    }

    fn validate(&self) -> Result<(), DefinitionError> {
        if self.name.is_empty() {
            return Err(DefinitionError::MissingField("name".to_string()));
        }
        if self.port == 0 {
            return Err(DefinitionError::MissingField("port".to_string()));
        }
        if self.endpoints.is_empty() {
            return Err(DefinitionError::MissingField("endpoints".to_string()));
        }
        for (i, endpoint) in self.endpoints.iter().enumerate() {
            if !endpoint.path.starts_with('/') {
                return Err(DefinitionError::InvalidEndpoint(format!(
                    "Endpoint {i}: path must start with /"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    const VALID_YAML: &str = r"
name: sendgrid
port: 3001
endpoints:
  - path: /v3/mail/send
    method: POST
    response:
      status: 200
      body:
        message_id: 'test-123'
";

    #[test]
    fn test_parse_valid_yaml() {
        let def = TwinDefinition::from_yaml(VALID_YAML);
        assert!(def.is_ok(), "Should parse valid YAML: {:?}", def.err());
        let def = def.unwrap();
        assert_eq!(def.name, "sendgrid");
        assert_eq!(def.port, 3001);
        assert_eq!(def.endpoints.len(), 1);
        assert_eq!(def.endpoints[0].path, "/v3/mail/send");
        assert_eq!(def.endpoints[0].method, HttpMethod::POST);
    }

    #[test]
    fn test_missing_name() {
        let yaml = r"
port: 3001
endpoints: []
";
        let result = TwinDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_path() {
        let yaml = r"
name: test
port: 3001
endpoints:
  - path: invalid
    method: GET
    response:
      status: 200
      body: {}
";
        let result = TwinDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    // ── Red Queen: adversarial parsing tests ──

    #[test]
    fn test_completely_empty_yaml() {
        let result = TwinDefinition::from_yaml("");
        assert!(result.is_err());
    }

    #[test]
    fn test_yaml_with_only_whitespace() {
        let result = TwinDefinition::from_yaml("   \n\t\n  ");
        assert!(result.is_err());
    }

    #[test]
    fn test_port_zero_is_rejected() {
        let yaml = r"
name: test
port: 0
endpoints:
  - path: /ok
    method: GET
    response:
      status: 200
      body: {}
";
        let result = TwinDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_endpoints_list_is_rejected() {
        let yaml = r"
name: test
port: 3001
endpoints: []
";
        let result = TwinDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_name_is_rejected() {
        let yaml = r"
name: ''
port: 3001
endpoints:
  - path: /ok
    method: GET
    response:
      status: 200
      body: {}
";
        let result = TwinDefinition::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_without_leading_slash_reports_endpoint_index() {
        let yaml = r"
name: test
port: 3001
endpoints:
  - path: /ok
    method: GET
    response:
      status: 200
      body: {}
  - path: bad
    method: POST
    response:
      status: 200
      body: {}
";
        let result = TwinDefinition::from_yaml(yaml);
        let err = result.expect_err("Should fail for second endpoint");
        let msg = err.to_string();
        assert!(
            msg.contains("Endpoint 1"),
            "Error should mention endpoint index 1: {msg}"
        );
    }

    #[test]
    fn test_random_bytes_rejected_gracefully() {
        let result = TwinDefinition::from_yaml_bytes(b"\x00\x01\x02\xff\xfe");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_yaml_bytes_roundtrip_with_from_yaml() {
        let def = TwinDefinition::from_yaml(VALID_YAML).expect("parse");
        let yaml_str = serde_yaml::to_string(&def).expect("serialize to yaml");
        let def2 = TwinDefinition::from_yaml_bytes(yaml_str.as_bytes()).expect("re-parse");
        assert_eq!(def.name, def2.name);
        assert_eq!(def.port, def2.port);
        assert_eq!(def.endpoints.len(), def2.endpoints.len());
    }

    #[test]
    fn test_all_http_methods_parse_case_insensitively() {
        for method_str in &[
            "GET", "get", "GeT", "POST", "post", "PUT", "put",
            "DELETE", "delete", "PATCH", "patch", "OPTIONS", "options", "HEAD", "head",
        ] {
            let result = HttpMethod::from_str(method_str);
            assert!(result.is_ok(), "Should parse method '{method_str}'");
        }
    }

    #[test]
    fn test_unknown_http_method_rejected() {
        let result = HttpMethod::from_str("CONNECT");
        assert!(result.is_err());
        let result = HttpMethod::from_str("TRACE");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string_method_rejected() {
        let result = HttpMethod::from_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_http_method_display_matches_standard() {
        assert_eq!(HttpMethod::GET.to_string(), "GET");
        assert_eq!(HttpMethod::POST.to_string(), "POST");
        assert_eq!(HttpMethod::DELETE.to_string(), "DELETE");
        assert_eq!(HttpMethod::PATCH.to_string(), "PATCH");
        assert_eq!(HttpMethod::OPTIONS.to_string(), "OPTIONS");
        assert_eq!(HttpMethod::HEAD.to_string(), "HEAD");
        assert_eq!(HttpMethod::PUT.to_string(), "PUT");
    }

    #[test]
    fn test_all_methods_exhaustive_match() {
        // Regression: if a new HttpMethod variant is added without Display/FromStr,
        // this test will fail to compile (or fail on the assert).
        let all = [
            HttpMethod::GET, HttpMethod::POST, HttpMethod::PUT,
            HttpMethod::DELETE, HttpMethod::PATCH, HttpMethod::OPTIONS, HttpMethod::HEAD,
        ];
        for m in &all {
            let s = m.to_string();
            let reparsed = HttpMethod::from_str(&s).expect(&format!("reparse {s}"));
            assert_eq!(*m, reparsed, "Display→FromStr roundtrip for {s}");
        }
    }

    #[test]
    fn test_response_defaults_empty_body_and_headers() {
        let yaml = r"
name: test
port: 3001
endpoints:
  - path: /ok
    method: GET
    response:
      status: 204
";
        let def = TwinDefinition::from_yaml(yaml).expect("parse");
        let ep = &def.endpoints[0];
        assert_eq!(ep.response.status, 204);
        assert_eq!(ep.response.body, serde_json::Value::Null);
        assert!(ep.response.headers.is_empty());
    }

    #[test]
    fn test_multiple_endpoints_same_path_different_methods() {
        let yaml = r"
name: multi
port: 3001
endpoints:
  - path: /resource
    method: GET
    response:
      status: 200
      body: {}
  - path: /resource
    method: DELETE
    response:
      status: 204
      body: null
  - path: /resource
    method: POST
    response:
      status: 201
      body:
        id: 42
";
        let def = TwinDefinition::from_yaml(yaml).expect("parse");
        assert_eq!(def.endpoints.len(), 3);
        assert_eq!(def.endpoints[0].method, HttpMethod::GET);
        assert_eq!(def.endpoints[1].method, HttpMethod::DELETE);
        assert_eq!(def.endpoints[2].method, HttpMethod::POST);
    }

    #[test]
    fn test_endpoint_response_with_custom_headers() {
        let yaml = r"
name: headers-test
port: 3001
endpoints:
  - path: /custom
    method: GET
    response:
      status: 200
      headers:
        x-custom: value123
        content-type: text/plain
      body:
        msg: hi
";
        let def = TwinDefinition::from_yaml(yaml).expect("parse");
        let headers = &def.endpoints[0].response.headers;
        assert_eq!(headers.get("x-custom"), Some(&"value123".to_string()));
        assert_eq!(headers.get("content-type"), Some(&"text/plain".to_string()));
    }

    #[test]
    fn test_definition_error_variants_display() {
        // Use a struct that will actually fail YAML parsing (not String which accepts anything)
        #[derive(Debug, serde::Deserialize)]
        struct StrictParse {
            #[allow(dead_code)]
            required: u32,
        }
        let err = DefinitionError::ParseError(
            serde_yaml::from_str::<StrictParse>("not: valid: at: all: :::")
                .unwrap_err(),
        );
        assert!(err.to_string().contains("Failed to parse YAML"));

        let err = DefinitionError::MissingField("port".to_string());
        assert!(err.to_string().contains("port"));

        let err = DefinitionError::InvalidEndpoint("bad path".to_string());
        assert!(err.to_string().contains("bad path"));

        let err = DefinitionError::InvalidMethod("CONNECT".to_string());
        assert!(err.to_string().contains("CONNECT"));
    }

    #[test]
    fn test_max_valid_port() {
        let yaml = format!(
            r"
name: test
port: 65535
endpoints:
  - path: /ok
    method: GET
    response:
      status: 200
      body: {{}}
"
        );
        let def = TwinDefinition::from_yaml(&yaml);
        assert!(def.is_ok(), "Port 65535 should be valid");
    }

    #[test]
    fn test_large_number_of_endpoints() {
        let mut endpoints = String::new();
        for i in 0..100 {
            endpoints.push_str(&format!(
                "  - path: /ep{i}\n    method: GET\n    response:\n      status: 200\n      body: {{}}\n"
            ));
        }
        let yaml = format!(
            "name: bulk\nport: 3001\nendpoints:\n{endpoints}"
        );
        let def = TwinDefinition::from_yaml(&yaml).expect("Should parse 100 endpoints");
        assert_eq!(def.endpoints.len(), 100);
    }

    #[test]
    fn test_endpoint_struct_clone_independence() {
        let ep = Endpoint {
            path: "/original".to_string(),
            method: HttpMethod::GET,
            response: EndpointResponse {
                status: 200,
                body: serde_json::json!({"key": "val"}),
                headers: HashMap::new(),
            },
        };
        let mut cloned = ep.clone();
        cloned.path = "/modified".to_string();
        assert_eq!(ep.path, "/original", "Clone must be independent");
    }

    #[test]
    fn test_twin_definition_serde_json_roundtrip() {
        let def = TwinDefinition::from_yaml(VALID_YAML).expect("parse yaml");
        let json = serde_json::to_string(&def).expect("serialize to json");
        let def2: TwinDefinition = serde_json::from_str(&json).expect("deserialize from json");
        assert_eq!(def.name, def2.name);
        assert_eq!(def.port, def2.port);
        assert_eq!(def.endpoints.len(), def2.endpoints.len());
    }
}
