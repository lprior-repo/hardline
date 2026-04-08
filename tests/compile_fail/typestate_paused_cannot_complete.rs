// Session<Paused> does not expose complete() — Paused→Completed is invalid.
// The typestate pattern enforces: Paused → Active (resume) → Completed (complete).
use scp_session::domain::entities::session::{Paused, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session = Session::create(name).unwrap();
    let active = session.activate().unwrap();
    let paused: Session<Paused> = active.pause().unwrap();
    paused.complete();
}
