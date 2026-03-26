use crate::domain::entities::session::{Active, Completed, Created, Failed, Session, SessionId};
use crate::domain::value_objects::SessionName;
use crate::error::{Result, SessionError};

pub struct SessionService;

impl SessionService {
    pub fn create_session(name: SessionName) -> Result<Session<Created>> {
        Session::create(name)
    }

    pub fn activate_session(session: Session<Created>) -> Result<Session<Active>> {
        session.activate()
    }

    pub fn complete_session(session: Session<Active>) -> Result<Session<Completed>> {
        session.complete()
    }

    pub fn fail_session(session: Session<Active>) -> Result<Session<Failed>> {
        session.fail()
    }

    pub fn list_sessions() -> Result<Vec<Session<Created>>> {
        Ok(Vec::new())
    }

    pub fn get_session(_id: SessionId) -> Result<Session<Created>> {
        Err(SessionError::NotFound("not implemented".into()))
    }
}
