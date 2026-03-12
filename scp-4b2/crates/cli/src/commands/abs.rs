//! Abs command - show abstract using jj abs

use scp_core::{get_jj_command_sync, Error, Result};
use std::process::Output;

fn build_jj_abs_command(revision: Option<&str>) -> std::process::Command {
    let mut cmd = get_jj_command_sync();
    cmd.arg("abs");

    if let Some(rev) = revision {
        cmd.arg(rev);
    }

    cmd
}

pub fn run(revision: Option<&str>) -> Result<()> {
    let output = run_jj_abs(revision)?;
    print_output(output)
}

fn run_jj_abs(revision: Option<&str>) -> Result<Output> {
    build_jj_abs_command(revision)
        .output()
        .map_err(|e| Error::JjCommandError {
            operation: "jj abs".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })
        .and_then(|output| {
            if output.status.success() {
                Ok(output)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(Error::JjCommandError {
                    operation: "jj abs".to_string(),
                    msg: stderr.to_string(),
                    is_not_found: false,
                })
            }
        })
}

fn print_output(output: Output) -> Result<()> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    Ok(())
}
