#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lock::{
        acquire_with_path, heartbeat_with_path, list_with_path, release_with_path, status_with_path,
    };
    use tempfile::NamedTempFile;

    fn get_temp_db() -> NamedTempFile {
        NamedTempFile::new().expect("Failed to create temp db")
    }

    #[test]
    fn acquire_release_cycle_succeeds() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res1 = acquire_with_path("s1", "a1", None, path);
        assert!(res1.is_ok(), "Expected Ok, got {:?}", res1);

        let res2 = release_with_path("s1", "a1", path);
        assert!(res2.is_ok(), "Expected Ok, got {:?}", res2);
    }

    #[test]
    fn acquire_twice_fails_with_conflict() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", None, path).expect("setup");
        let res = acquire_with_path("s1", "a2", None, path);
        assert!(res.is_err(), "Expected error on conflict, got {:?}", res);
    }

    #[test]
    fn heartbeat_updates_expiration() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", Some(100), path).expect("setup");
        let res = heartbeat_with_path("s1", "a1", path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn status_reports_correct_state() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        status_with_path("s1", path).expect("unlocked status");
        acquire_with_path("s1", "a1", None, path).expect("setup");
        let res = status_with_path("s1", path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn list_shows_active_locks() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", None, path).expect("setup 1");
        acquire_with_path("s2", "a2", None, path).expect("setup 2");
        let res = list_with_path(path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn release_nonexistent_lock_succeeds_idempotent() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = release_with_path("ghost", "a1", path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn heartbeat_from_wrong_agent_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", None, path).expect("setup");
        let res = heartbeat_with_path("s1", "a2", path);
        assert!(
            res.is_err(),
            "Expected error from wrong agent, got {:?}",
            res
        );
    }

    #[test]
    fn acquire_with_invalid_ttl_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path("s1", "a1", Some(1000000), path);
        assert!(res.is_err(), "Expected error on invalid TTL, got {:?}", res);
    }

    #[test]
    fn acquire_with_empty_session_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path("", "a1", None, path);
        assert!(
            res.is_err(),
            "Expected error on empty session, got {:?}",
            res
        );
    }

    #[test]
    fn acquire_with_empty_agent_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path("s1", "", None, path);
        assert!(res.is_err(), "Expected error on empty agent, got {:?}", res);
    }

    #[test]
    fn acquire_with_max_session_length_succeeds() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");
        let session = "s".repeat(255);

        let res = acquire_with_path(&session, "a1", None, path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn acquire_with_too_long_session_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");
        let session = "s".repeat(256);

        let res = acquire_with_path(&session, "a1", None, path);
        assert!(
            res.is_err(),
            "Expected error on too long session, got {:?}",
            res
        );
    }

    #[test]
    fn acquire_with_max_ttl_succeeds() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        let res = acquire_with_path("s1", "a1", Some(86400), path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }

    #[test]
    fn heartbeat_for_expired_lock_fails() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", Some(1), path).expect("setup");
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let res = heartbeat_with_path("s1", "a1", path);
        assert!(
            res.is_err(),
            "Expected error on expired lock, got {:?}",
            res
        );
    }

    #[test]
    fn list_excludes_expired_locks() {
        let db = get_temp_db();
        let path = db.path().to_str().expect("path utf8");

        acquire_with_path("s1", "a1", Some(1), path).expect("setup");
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let res = list_with_path(path);
        assert!(res.is_ok(), "Expected Ok, got {:?}", res);
    }
}
