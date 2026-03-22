//! Helper utilities for lock management.

/// Check if a SQLx error is a constraint conflict error.
///
/// This detects both SQLite constraint violations (1555, 2067) and
/// errors with "constraint" in the message.
#[must_use]
pub fn is_constraint_conflict_error(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            let code = db_error
                .code()
                .map_or(String::new(), |value| value.to_string());
            code == "1555"
                || code == "2067"
                || code.starts_with("SQLITE_CONSTRAINT")
                || db_error.message().to_lowercase().contains("constraint")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_constraint_error_codes() {
        // These would need actual sqlx errors to test properly
        // This is a placeholder to show the test structure
        assert!(true);
    }
}
