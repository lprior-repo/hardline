//! Scenarios - Behavioral scenario vault with information barrier
//!
//! Provides a system for running black-box behavioral scenarios against
//! twin universes, with sanitized feedback to prevent scenario leakage.
//!
//! # Key Features
//! - Scenario YAML parsing and validation
//! - HTTP step execution against twin
//! - Value extraction from responses
//! - Assertion validation
//! - Multi-level feedback sanitization (information barrier)
//!
//! # Information Barrier
//!
//! The sanitizer ensures that agents cannot access scenario details:
//! - Level 1: Pass/fail only
//! - Level 2: +error type
//! - Level 3: +stack trace (no values)
//! - Level 4: +assertion locations (no values)
//! - Level 5: Full (development only)
//!
//! # Example
//!
//! ```ignore
//! use scenarios::{FeedbackLevel, Scenario, ScenarioRunner};
//!
//! let yaml = r#"
//! name: "Test scenario"
//! description: "A test"
//! steps:
//!   - http:
//!       url: "http://localhost:3001/api/test"
//!       method: GET
//! "#;
//!
//! let scenario = Scenario::from_yaml(yaml).unwrap();
//! let runner = ScenarioRunner::with_default_config().unwrap();
//! let feedback = runner
//!     .run_with_sanitized_feedback(&scenario, FeedbackLevel::Level2)
//!     .await;
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

pub mod runner;
pub mod sanitizer;
pub mod scenario;

pub use runner::{RunnerConfig, ScenarioResult, ScenarioRunner, StepResult};
pub use sanitizer::{FeedbackLevel, Sanitizer};
pub use scenario::{AssertStep, AssertionType, ExtractStep, HttpMethod, HttpStep, Scenario, Step};
