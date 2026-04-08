// Session<Completed> does not expose fail() — Completed→Failed is invalid.
// Completed is a terminal state; the only exit is restart() → Created.
use scp_session::domain::entities::session::{Completed, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session = Session::create(name).unwrap();
    let active = session.activate().unwrap();
    let completed: Session<Completed> = active.complete().unwrap();
    completed.fail();
}
