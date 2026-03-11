use crate::domain::entities::{Session, SessionId, SessionState};
use crate::domain::events::{
    SessionCompletedEvent, SessionCreatedEvent, SessionEvent, SessionFailedEvent,
};
use crate::domain::value_objects::SessionName;
use crate::error::{Result, SessionError};

pub struct SessionService;

impl SessionService {
    pub fn create_session(name: SessionName) -> Result<Session> {
        Session::create(name)
    }

    pub fn activate_session(session: Session) -> Result<Session> {
        session.transition(SessionEvent::Activated)
    }

    pub fn complete_session(session: Session) -> Result<Session> {
        session.transition(SessionEvent::Completed)
    }

    pub fn fail_session(session: Session, reason: String) -> Result<Session> {
        let _ = reason;
        session.transition(SessionEvent::Failed)
    }

    pub fn list_sessions() -> Result<Vec<Session>> {
        Ok(Vec::new())
    }

    pub fn get_session(_id: SessionId) -> Result<Session> {
        Err(SessionError::NotFound("not implemented".into()))
    }
}
