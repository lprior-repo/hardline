// Session<Syncing> does not expose complete() — Syncing→Completed is invalid.
// The typestate pattern enforces: Syncing → Synced → Completed.
use scp_session::domain::entities::session::{Syncing, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session = Session::create(name).unwrap();
    let active = session.activate().unwrap();
    let syncing: Session<Syncing> = active.sync().unwrap();
    syncing.complete();
}
