//! Durable execution context traits and implementations.
//!
//! ## Context Traits
//!
//! | Trait | Methods | Description |
//! |--------|---------|-------------|
//! | `DurableContext` | Combined interface | Main trait for handlers |
//! | `ContextSideEffects` | `run`, `random_seed`, `rand`, `rand_uuid` | Journaling and RNG |
//! | `ContextTimers` | `sleep` | Durable sleep that survives crashes |
//! | `ContextReadState` | `get`, `get_keys` | Read from state store |
//! | `ContextWriteState` | `set`, `clear`, `clear_all` | Write to state store |
//!
//! ## Usage
//!
//! ```rust,ignore
//! async fn my_handler(ctx: &impl DurableContext, req: Request) -> Result<Response, HandlerError> {
//!     // Journaled non-deterministic operation
//!     let result = ctx.run(|| async { external_api_call().await }).await?;
//!
//!     // Durable sleep
//!     ctx.sleep(Duration::from_secs(30)).await?;
//!
//!     // State operations (for Virtual Objects/Workflows)
//!     ctx.set("counter", 42)?;
//!     let count: Option<i32> = ctx.get("counter").await?;
//!
//!     Ok(Response { result })
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use rand::rngs::StdRng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::infrastructure::restate::errors::{HandlerResult, TerminalError};

pub type DurableFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type RunFuture<T, E = TerminalError> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

pub trait RunClosure {
    type Fut: Future<Output = HandlerResult<Self::Output>> + Send;
    type Output: Serialize + DeserializeOwned + 'static;

    fn run(self) -> Self::Fut;
}

impl<F, Fut, T> RunClosure for F
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = HandlerResult<T>> + Send,
    T: Serialize + DeserializeOwned + 'static,
{
    type Fut = Fut;
    type Output = T;

    fn run(self) -> Self::Fut {
        self()
    }
}

pub trait ContextSideEffects {
    fn run<R, F, T>(&self, run_closure: R) -> RunFuture<T, TerminalError>
    where
        R: RunClosure<Output = T, Fut = F> + Send,
        F: Future<Output = HandlerResult<T>> + Send,
        T: Serialize + DeserializeOwned + 'static;

    fn random_seed(&self) -> u64;

    fn rand(&mut self) -> &mut StdRng;

    fn rand_uuid(&mut self) -> Uuid;
}

pub trait ContextTimers {
    fn sleep(&self, duration: Duration) -> DurableFuture<Result<(), TerminalError>>;
}

pub trait ContextReadState {
    fn get<T: DeserializeOwned + 'static>(
        &self,
        key: &str,
    ) -> DurableFuture<Result<Option<T>, TerminalError>>;

    fn get_keys(&self) -> DurableFuture<Result<Vec<String>, TerminalError>>;
}

pub trait ContextWriteState {
    fn set<T: Serialize + 'static>(&self, key: &str, value: T);

    fn clear(&self, key: &str);

    fn clear_all(&self);
}

pub trait DurableContext:
    ContextSideEffects + ContextTimers + ContextReadState + ContextWriteState
{
    fn run<R, F, T>(&self, run_closure: R) -> RunFuture<T, TerminalError>
    where
        R: RunClosure<Output = T, Fut = F> + Send,
        F: Future<Output = HandlerResult<T>> + Send,
        T: Serialize + DeserializeOwned + 'static;

    fn sleep(&self, duration: Duration) -> DurableFuture<Result<(), TerminalError>>;

    fn get_state<T: DeserializeOwned + 'static>(
        &self,
        key: &str,
    ) -> DurableFuture<Result<Option<T>, TerminalError>>;

    fn set_state<T: Serialize + 'static>(&self, key: &str, value: T);

    fn clear_state(&self, key: &str);

    fn clear_all_state(&self);

    fn get_state_keys(&self) -> DurableFuture<Result<Vec<String>, TerminalError>>;

    fn random_seed(&self) -> u64;

    fn rand_uuid(&mut self) -> Uuid;

    fn rand(&mut self) -> &mut StdRng;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_closure_impl() {
        fn assert_run_closure<F, Fut, T>()
        where
            F: RunClosure<Output = T, Fut = Fut>,
            Fut: Future<Output = HandlerResult<T>> + Send,
            T: Serialize + DeserializeOwned + 'static,
        {
        }

        fn check<F, Fut, T>(_f: F)
        where
            F: FnOnce() -> Fut + Send,
            Fut: Future<Output = HandlerResult<T>> + Send,
            T: Serialize + DeserializeOwned + 'static,
        {
            assert_run_closure::<F, Fut, T>()
        }

        check(|| async { Ok(42i32) });
    }
}
