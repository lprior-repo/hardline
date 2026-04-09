use async_trait::async_trait;

use crate::domain::entities::session::StateInfo;
use crate::domain::entities::{Session, SessionState};
use crate::domain::value_objects::{AgentId, SessionName};
use crate::error::SessionError;

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save<S: StateInfo + std::marker::Sync>(&self, session: &Session<S>) -> Result<(), SessionError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Session>, SessionError>;
    async fn find_by_name(&self, name: &SessionName) -> Result<Option<Session>, SessionError>;
    async fn list(&self) -> Result<Vec<Session>, SessionError>;
    async fn delete(&self, id: &str) -> Result<(), SessionError>;
    async fn find_by_state(&self, state: SessionState) -> Result<Vec<Session>, SessionError>;
    async fn find_by_agent(&self, agent: &AgentId) -> Result<Vec<Session>, SessionError>;
}
