//! Service client traits for Restate SDK integration.
//!
//! ## Client Types
//!
//! | Type | Method | Description |
//! |------|--------|-------------|
//! | `ServiceClient` | `service_client()` | Stateless service invocations |
//! | `ObjectClient` | `object_client(key)` | Stateful keyed entity invocations |
//! | `WorkflowClient` | `workflow_client(key)` | Long-running workflow invocations |
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Service/Object/Workflow clients require Restate runtime integration.
//! // See the RequestTarget constructors for usage patterns:
//! use scp_core::infrastructure::restate::clients::RequestTarget;
//!
//! let service = RequestTarget::service("my-service");
//! let object = RequestTarget::object("entity-key", "my-object");
//! let workflow = RequestTarget::workflow("workflow-id", "my-workflow");
//! ```

use std::future::Future;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::infrastructure::restate::errors::HandlerError;

/// Request target for service calls.
///
/// Indicates whether to call a service, object, or workflow.
#[derive(Debug, Clone)]
pub enum RequestTarget {
    Service(String),
    Object { key: String, service: String },
    Workflow { key: String, workflow: String },
}

impl RequestTarget {
    pub fn service(name: impl Into<String>) -> Self {
        Self::Service(name.into())
    }

    pub fn object(key: impl Into<String>, service: impl Into<String>) -> Self {
        Self::Object {
            key: key.into(),
            service: service.into(),
        }
    }

    pub fn workflow(key: impl Into<String>, workflow: impl Into<String>) -> Self {
        Self::Workflow {
            key: key.into(),
            workflow: workflow.into(),
        }
    }
}

/// Trait for making service calls.
///
/// Provides access to service, object, and workflow clients.
pub trait ContextClient {
    fn service_client<C>(&self) -> C
    where
        C: IntoServiceClient;

    fn object_client<C>(&self, key: impl Into<String>) -> C
    where
        C: IntoObjectClient;

    fn workflow_client<C>(&self, key: impl Into<String>) -> C
    where
        C: IntoWorkflowClient;

    fn request<Req, Res>(&self, target: RequestTarget, req: Req) -> Request<'_, Req, Res>
    where
        Req: Serialize + 'static,
        Res: DeserializeOwned + 'static;
}

/// Trait for service client creation.
pub trait IntoServiceClient {
    type Client: ServiceClient;

    fn into_client(self) -> Self::Client;
}

/// Trait for object client creation.
pub trait IntoObjectClient {
    type Client: ObjectClient;

    fn into_client(self) -> Self::Client;
}

/// Trait for workflow client creation.
pub trait IntoWorkflowClient {
    type Client: WorkflowClient;

    fn into_client(self) -> Self::Client;
}

/// Trait for service (stateless) clients.
pub trait ServiceClient: Send + Sync {
    fn call<Req, Res>(&self, method: &str, req: Req) -> Request<'_, Req, Res>
    where
        Req: Serialize + 'static,
        Res: DeserializeOwned + 'static;
}

/// Trait for object (keyed stateful) clients.
pub trait ObjectClient: Send + Sync {
    fn call<Req, Res>(&self, method: &str, req: Req) -> Request<'_, Req, Res>
    where
        Req: Serialize + 'static,
        Res: DeserializeOwned + 'static;
}

/// Trait for workflow clients.
pub trait WorkflowClient: Send + Sync {
    fn call<Req, Res>(&self, method: &str, req: Req) -> Request<'_, Req, Res>
    where
        Req: Serialize + 'static,
        Res: DeserializeOwned + 'static;
}

/// A pending request to a service.
///
/// This is a future that resolves when the response is received.
pub struct Request<'ctx, Req, Res> {
    target: RequestTarget,
    req: Req,
    _phantom: std::marker::PhantomData<(&'ctx (), Res)>,
}

impl<'ctx, Req, Res> Request<'ctx, Req, Res>
where
    Req: Serialize + 'static,
    Res: DeserializeOwned + 'static,
{
    pub fn new(target: RequestTarget, req: Req) -> Self {
        Self {
            target,
            req,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn target(&self) -> &RequestTarget {
        &self.target
    }

    pub fn request(&self) -> &Req {
        &self.req
    }
}

impl<Req, Res> Future for Request<'_, Req, Res>
where
    Req: Serialize + 'static,
    Res: DeserializeOwned + 'static,
{
    type Output = Result<Res, HandlerError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        todo!("Request implementation requires Restate runtime")
    }
}

/// Trait for obtaining an invocation handle.
///
/// Used to interact with an ongoing invocation.
pub trait InvocationHandle {
    fn invocation_id(&self) -> &str;

    fn is_replay(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_target_service() {
        let target = RequestTarget::service("my-service");
        match target {
            RequestTarget::Service(name) => assert_eq!(name, "my-service"),
            _ => panic!("Expected Service variant"),
        }
    }

    #[test]
    fn test_request_target_object() {
        let target = RequestTarget::object("entity-key", "my-object");
        match target {
            RequestTarget::Object { key, service } => {
                assert_eq!(key, "entity-key");
                assert_eq!(service, "my-object");
            }
            _ => panic!("Expected Object variant"),
        }
    }

    #[test]
    fn test_request_target_workflow() {
        let target = RequestTarget::workflow("workflow-id", "my-workflow");
        match target {
            RequestTarget::Workflow { key, workflow } => {
                assert_eq!(key, "workflow-id");
                assert_eq!(workflow, "my-workflow");
            }
            _ => panic!("Expected Workflow variant"),
        }
    }
}
