use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: scp"));
}

#[test]
fn test_queue_persists_across_cli_invocations() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Queue is empty"));

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("enqueue")
        .arg("feature-persist-test")
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-persist-test"));

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("feature-persist-test"));

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_queue_dequeue_persists_state() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("enqueue")
        .arg("dequeue-me")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("enqueue")
        .arg("dequeue-me-too")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("2 items"));

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("dequeue")
        .assert()
        .success()
        .stdout(predicate::str::contains("dequeue-me"));

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", &db_path)
        .arg("queue")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total items: 2"));
}

#[test]
fn test_queue_database_flag_sets_path() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("flag-test.db");

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("--database")
        .arg(&db_path)
        .arg("queue")
        .arg("enqueue")
        .arg("flag-test-branch")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("--database")
        .arg(&db_path)
        .arg("queue")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("flag-test-branch"));
}

#[test]
fn test_init_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("init").arg("--help").assert().success();
}

#[test]
fn test_status_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("status").arg("--help").assert().success();
}

#[test]
fn test_workspace_add_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("workspace")
        .arg("add")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_workspace_commit_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("workspace")
        .arg("commit")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_stash_save_help() {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.arg("stash")
        .arg("save")
        .arg("--help")
        .assert()
        .success();
}
