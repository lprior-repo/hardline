//! Cat command - show file contents using jj cat

use scp_core::{get_jj_command_sync, Error, Result};
use std::process::Output;

fn build_jj_cat_command(path: &str, revision: Option<&str>) -> std::process::Command {
    let mut cmd = get_jj_command_sync();
    cmd.arg("cat").arg(path);

    if let Some(rev) = revision {
        cmd.arg("--revision");
        cmd.arg(rev);
    }

    cmd
}

pub fn run(path: &str, revision: Option<&str>) -> Result<()> {
    let output = run_jj_cat(path, revision)?;
    print_output(output)
}

fn run_jj_cat(path: &str, revision: Option<&str>) -> Result<Output> {
    build_jj_cat_command(path, revision)
        .output()
        .map_err(|e| Error::JjCommandError {
            operation: "jj cat".to_string(),
            msg: e.to_string(),
            is_not_found: e.kind() == std::io::ErrorKind::NotFound,
        })
        .and_then(|output| {
            if output.status.success() {
                Ok(output)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(Error::JjCommandError {
                    operation: "jj cat".to_string(),
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
