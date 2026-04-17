//! Promises and Awakeables for workflow signaling.
//!
//! ## Promises
//!
//! Promises are named future values that can be resolved or rejected by one
//! part of a workflow and awaited by another.
//!
//! | Method | Description |
//! |--------|-------------|
//! | `promise(key)` | Create/await a promise |
//! | `peek_promise(key)` | Check if promise is resolved without consuming |
//! | `resolve_promise(key, value)` | Resolve a promise with a value |
//! | `reject_promise(key, error)` | Reject a promise with an error |
//!
//! ## Awakeables
//!
//! Awakeables are external completion handles. Unlike promises, they can
//! be completed by external systems (via the Restate API).
//!
//! | Method | Description |
//! |--------|-------------|
//! | `awakeable()` | Create an awakeable and get completion key |
//! | `resolve_awakeable(key, value)` | Resolve an awakeable |
//! | `reject_awakeable(key, error)` | Reject an awakeable |
//!
//! ## Usage
//!
//! ### Promises
//!
//! ```rust,ignore
//! // ContextPromises requires Restate runtime integration.
//! // See trait definition for available methods:
//! // - promise(key), peek_promise(key), resolve_promise(key, value), reject_promise(key, error)
//! ```
//!
//! ### Awakeables
//!
//! ```rust,ignore
//! // ContextAwakeables requires Restate runtime integration.
//! // See trait definition for available methods:
//! // - awakeable(), resolve_awakeable(key, value), reject_awakeable(key, error)
//! ```

use std::future::Future;
use std::pin::Pin;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::infrastructure::restate::errors::TerminalError;

/// Unique identifier for an awakeable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AwakeableId(String);

impl AwakeableId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AwakeableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A promise that can be resolved or rejected.
///
/// Created by `ContextPromises::promise()` and awaited to get the result.
#[derive(Debug)]
pub struct Promise<T> {
    key: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Promise<T> {
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// Resolver for a promise.
///
/// Allows resolving or rejecting a promise from outside the waiting context.
#[derive(Debug)]
pub struct PromiseResolver {
    key: String,
}

impl PromiseResolver {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn into_promise<T: DeserializeOwned + 'static>(self) -> Promise<T> {
        Promise {
            key: self.key,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Trait for workflow promise operations.
pub trait ContextPromises {
    fn promise<T: DeserializeOwned + 'static>(&self, key: &str) -> PromiseFuture<T>;

    fn peek_promise<T: DeserializeOwned + 'static>(&self, key: &str) -> PeekPromiseFuture<T>;

    fn resolve_promise<T: Serialize + 'static>(&self, key: &str, value: T);

    fn reject_promise(&self, key: &str, failure: TerminalError);
}

/// Future returned by `ContextPromises::promise`.
pub type PromiseFuture<T> = Pin<Box<dyn Future<Output = Result<T, TerminalError>> + Send>>;

/// Future returned by `ContextPromises::peek_promise`.
pub type PeekPromiseFuture<T> =
    Pin<Box<dyn Future<Output = Result<Option<T>, TerminalError>> + Send>>;

/// Trait for awakeable operations.
///
/// Awakeables are handles for external completion. They can be resolved
/// or rejected by external systems via the Restate API.
pub trait ContextAwakeables {
    fn awakeable<T: DeserializeOwned + 'static>(&self) -> (AwakeableId, AwakeableFuture<T>);

    fn resolve_awakeable<T: Serialize + 'static>(&self, key: &str, value: T);

    fn reject_awakeable(&self, key: &str, failure: TerminalError);
}

/// Future returned by `ContextAwakeables::awakeable`.
pub type AwakeableFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, TerminalError>> + Send + 'static>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_awakeable_id() {
        let id = AwakeableId::new("test-id");
        assert_eq!(id.as_str(), "test-id");
        assert_eq!(id.to_string(), "test-id");
    }

    #[test]
    fn test_promise_key() {
        let promise: Promise<i32> = Promise {
            key: "test-key".to_string(),
            _marker: std::marker::PhantomData,
        };
        assert_eq!(promise.key(), "test-key");
    }

    #[test]
    fn test_promise_resolver_key() {
        let resolver = PromiseResolver {
            key: "resolver-key".to_string(),
        };
        assert_eq!(resolver.key(), "resolver-key");

        let promise: Promise<String> = resolver.into_promise();
        assert_eq!(promise.key(), "resolver-key");
    }
}
