//! Version command

use scp_core::Result;

pub fn run() -> Result<()> {
    println!("scp v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
