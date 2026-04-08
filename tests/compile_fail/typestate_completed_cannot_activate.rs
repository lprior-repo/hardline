// Session<Completed> does not expose activate() — Completed→Active is invalid.
// The typestate pattern enforces: Completed → Created (restart) → Active (activate).
use scp_session::domain::entities::session::{Completed, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session = Session::create(name).unwrap();
    let active = session.activate().unwrap();
    let completed: Session<Completed> = active.complete().unwrap();
    completed.activate();
}
