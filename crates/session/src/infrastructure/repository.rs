use async_trait::async_trait;

use crate::{
    domain::{entities::Session, value_objects::SessionName},
    error::SessionError,
};

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn save(&self, session: &Session) -> Result<(), SessionError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Session>, SessionError>;
    async fn find_by_name(&self, name: &SessionName) -> Result<Option<Session>, SessionError>;
    async fn list(&self) -> Result<Vec<Session>, SessionError>;
    async fn delete(&self, id: &str) -> Result<(), SessionError>;
}
