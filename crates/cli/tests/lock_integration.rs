use assert_cmd::Command;
use tempfile::NamedTempFile;

fn scp_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("scp-cli").unwrap();
    cmd.env("SCP_DATABASE_PATH", db_path);
    cmd
}

#[test]
fn cli_lock_basic_lifecycle() {
    let db_file = NamedTempFile::new().unwrap();
    let db_path = db_file.path().to_str().unwrap();

    // Acquire
    scp_cmd(db_path)
        .arg("lock")
        .arg("acquire")
        .arg("s1")
        .arg("--agent")
        .arg("a1")
        .assert()
        .success();

    // Status
    scp_cmd(db_path)
        .arg("lock")
        .arg("status")
        .arg("s1")
        .assert()
        .success()
        .stdout(predicates::str::contains("Locked"));

    // Heartbeat
    scp_cmd(db_path)
        .arg("lock")
        .arg("heartbeat")
        .arg("s1")
        .arg("--agent")
        .arg("a1")
        .assert()
        .success();

    // List
    scp_cmd(db_path)
        .arg("lock")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("s1"));

    // Release
    scp_cmd(db_path)
        .arg("lock")
        .arg("release")
        .arg("s1")
        .arg("--agent")
        .arg("a1")
        .assert()
        .success();

    // Verify Unlocked
    scp_cmd(db_path)
        .arg("lock")
        .arg("status")
        .arg("s1")
        .assert()
        .success()
        .stdout(predicates::str::contains("Unlocked"));
}

#[test]
fn cli_lock_conflict_prevention() {
    let db_file = NamedTempFile::new().unwrap();
    let db_path = db_file.path().to_str().unwrap();

    scp_cmd(db_path)
        .arg("lock")
        .arg("acquire")
        .arg("s1")
        .arg("--agent")
        .arg("a1")
        .assert()
        .success();
    scp_cmd(db_path)
        .arg("lock")
        .arg("acquire")
        .arg("s1")
        .arg("--agent")
        .arg("a2")
        .assert()
        .failure();
}

#[test]
fn cli_lock_heartbeat_failure_for_non_holder() {
    let db_file = NamedTempFile::new().unwrap();
    let db_path = db_file.path().to_str().unwrap();

    scp_cmd(db_path)
        .arg("lock")
        .arg("acquire")
        .arg("s1")
        .arg("--agent")
        .arg("a1")
        .assert()
        .success();
    scp_cmd(db_path)
        .arg("lock")
        .arg("heartbeat")
        .arg("s1")
        .arg("--agent")
        .arg("a2")
        .assert()
        .failure();
}

#[test]
fn cli_lock_status_for_nonexistent_session() {
    let db_file = NamedTempFile::new().unwrap();
    let db_path = db_file.path().to_str().unwrap();

    scp_cmd(db_path)
        .arg("lock")
        .arg("status")
        .arg("ghost")
        .assert()
        .success()
        .stdout(predicates::str::contains("Unlocked"));
}
