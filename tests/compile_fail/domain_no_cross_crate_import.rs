// This should fail to compile - stack domain cannot import vcs domain internals
use scp_vcs::domain::types::CommitHash;

fn main() {}
