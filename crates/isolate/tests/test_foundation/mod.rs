//! Test Foundation Module
//!
//! This module provides the testing infrastructure for Isolate:
//! - Property-based testing with proptest (deterministic with seeds)
//! - BDD-style testing with Given/When/Then syntax
//!
//! ## Design Principles
//!
//! - Zero panics: All fallible operations return `Result<T, E>`
//! - Deterministic: All tests use fixed seeds for reproducibility
//! - Isolation: No shared mutable state between tests

#![allow(dead_code)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

pub mod bdd;
pub mod proptest_config;
pub mod string_validation;