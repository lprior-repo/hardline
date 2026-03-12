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
    pub fn from_yaml(yaml: &str) -> Result<Self, DefinitionError> {
        let def = serde_yaml::from_str::<Self>(yaml)?;
        def.validate()?;
        Ok(def)
    }

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
        let def = TwinDefinition::from_yaml(VALID_YAML).expect("Should parse valid YAML");
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
}
