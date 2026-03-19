//! Cat command - show file contents using jj cat

use scp_core::{get_jj_command_sync, Error, Result};
use std::process::Output;
use tap::pipe::Pipe;

fn build_jj_cat_command(path: &str, revision: Option<&str>) -> std::process::Command {
    let args = std::iter::once("cat")
        .chain(std::iter::once(path))
        .chain(revision.map(|r| ["--revision", r]).into_iter().flatten());

    args.fold(get_jj_command_sync(), |cmd, arg| {
        let mut c = cmd;
        c.arg(arg);
        c
    })
}

pub fn run(path: &str, revision: Option<&str>) -> Result<()> {
    run_jj_cat(path, revision).and_then(print_output)
}

fn run_jj_cat(path: &str, revision: Option<&str>) -> Result<Output> {
    build_jj_cat_command(path, revision)
        .output()
        .map_err(into_jj_command_error("jj cat"))
        .and_then(check_jj_cat_status)
}

fn check_jj_cat_status(output: Output) -> Result<Output> {
    output.status.success().pipe(|success| match success {
        true => Ok(output),
        false => Err(create_jj_error_from_output(&output)),
    })
}

fn create_jj_error_from_output(output: &Output) -> Error {
    Error::JjCommandError {
        operation: "jj cat".to_string(),
        msg: String::from_utf8_lossy(&output.stderr).to_string(),
        is_not_found: false,
    }
}

fn into_jj_command_error(operation: &str) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |e| Error::JjCommandError {
        operation: operation.to_string(),
        msg: e.to_string(),
        is_not_found: e.kind() == std::io::ErrorKind::NotFound,
    }
}

fn print_output(output: Output) -> Result<()> {
    String::from_utf8_lossy(&output.stdout)
        .pipe(|stdout| {
            print!("{stdout}");
        })
        .pipe(|_| ())
        .pipe(Ok)
}
