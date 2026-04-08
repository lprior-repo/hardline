// Session<Failed> does not expose sync() — Failed→Syncing is invalid.
// The typestate pattern enforces: Failed → Created (retry) → Active → Syncing.
use scp_session::domain::entities::session::{Failed, Session};
use scp_session::SessionName;

fn main() {
    let name = SessionName::parse("test").unwrap();
    let session = Session::create(name).unwrap();
    let failed: Session<Failed> = session.fail().unwrap();
    failed.sync();
}
