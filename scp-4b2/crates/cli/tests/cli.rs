use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("scp").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: scp"));
}

#[test]
fn test_init_help() {
    let mut cmd = Command::cargo_bin("scp").unwrap();
    cmd.arg("init").arg("--help").assert().success();
}

#[test]
fn test_status_help() {
    let mut cmd = Command::cargo_bin("scp").unwrap();
    cmd.arg("status").arg("--help").assert().success();
}

#[test]
fn test_workspace_add_help() {
    let mut cmd = Command::cargo_bin("scp").unwrap();
    cmd.arg("workspace")
        .arg("add")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_workspace_commit_help() {
    let mut cmd = Command::cargo_bin("scp").unwrap();
    cmd.arg("workspace")
        .arg("commit")
        .arg("--help")
        .assert()
        .success();
}
