// Session<Created> does not expose sync() — Created→Syncing is invalid.
// The typestate pattern enforces: Created → Active → Syncing.
use scp_session::domain::entities::session::{Created, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session: Session<Created> = Session::create(name).unwrap();
    session.sync();
}
