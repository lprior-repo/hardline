use scp_core::domain::identifiers::{AgentId, SessionName, WorkspaceName};

#[test]
fn test_session_name_boundary() {
    let min_minus_1 = "";
    assert!(
        SessionName::parse(min_minus_1).is_err(),
        "empty string should fail"
    );

    let min = "a";
    assert!(SessionName::parse(min).is_ok(), "1 char should pass");

    let max = "a".repeat(63);
    assert!(SessionName::parse(&max).is_ok(), "63 chars should pass");

    let max_plus_1 = "a".repeat(64);
    assert!(
        SessionName::parse(&max_plus_1).is_err(),
        "64 chars should fail"
    );
}

#[test]
fn test_workspace_name_boundary() {
    let min_minus_1 = "";
    assert!(
        WorkspaceName::parse(min_minus_1).is_err(),
        "empty string should fail"
    );

    let min = "a";
    assert!(WorkspaceName::parse(min).is_ok(), "1 char should pass");

    let max = "a".repeat(255);
    assert!(WorkspaceName::parse(&max).is_ok(), "255 chars should pass");

    let max_plus_1 = "a".repeat(256);
    assert!(
        WorkspaceName::parse(&max_plus_1).is_err(),
        "256 chars should fail"
    );

    // Multibyte check: If length is validated by bytes but meant to be characters.
    // 128 "あ" characters = 384 bytes
    let multibyte_max = "あ".repeat(128);
    let res = WorkspaceName::parse(&multibyte_max);
    if res.is_err() {
        println!("BUG: WorkspaceName length is validated by bytes, not chars!");
    } else {
        println!("WorkspaceName multibyte is fine.");
    }
}
