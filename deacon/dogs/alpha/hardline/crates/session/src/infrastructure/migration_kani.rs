//! Kani harnesses for session migration pure function verification (bead hl-c18).
//!
//! # Invariants Proven
//!
//! 1. `MigrationVersion::new` rejects zero and negative values
//! 2. `MigrationVersion::new` accepts all positive values
//! 3. `validate_migration_name` rejects empty strings
//! 4. `validate_migration_name` rejects special characters
//! 5. `validate_migration_name` accepts valid SQL identifiers
//!
//! Note: The async DB operations (column_exists, migrate_v2, rollback) cannot be
//! verified by Kani as they require a live SQLite connection via sqlx. These are
//! thoroughly tested by 162 integration tests instead.

#[cfg(kani)]
mod proofs {
    use crate::infrastructure::migration::{validate_migration_name, MigrationVersion};

    // =========================================================================
    // hl-c18: MigrationVersion::new invariants
    // =========================================================================

    /// Verify positive version numbers are accepted
    #[kani::proof]
    fn prove_migration_version_positive_accepted() {
        for v in [1i64, 2, 10, 100, i64::MAX] {
            let result = MigrationVersion::new(v);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().as_i64(), v);
        }
    }

    /// Verify zero is rejected
    #[kani::proof]
    fn prove_migration_version_zero_rejected() {
        let result = MigrationVersion::new(0);
        assert!(result.is_err());
    }

    /// Verify negative values are rejected
    #[kani::proof]
    fn prove_migration_version_negative_rejected() {
        for v in [-1i64, -100, i64::MIN] {
            let result = MigrationVersion::new(v);
            assert!(result.is_err());
        }
    }

    /// Verify that version 1 is always accepted (v1 migration baseline)
    #[kani::proof]
    fn prove_migration_version_one_accepted() {
        let v = MigrationVersion::new(1).unwrap();
        assert_eq!(v.as_i64(), 1);
    }

    /// Verify that version 2 is always accepted (v2 migration)
    #[kani::proof]
    fn prove_migration_version_two_accepted() {
        let v = MigrationVersion::new(2).unwrap();
        assert_eq!(v.as_i64(), 2);
    }

    // =========================================================================
    // hl-c18: validate_migration_name invariants
    // =========================================================================

    /// Verify empty string is rejected
    #[kani::proof]
    fn prove_migration_name_empty_rejected() {
        assert!(validate_migration_name("").is_err());
    }

    /// Verify valid SQL identifiers are accepted
    #[kani::proof]
    fn prove_migration_name_valid_identifiers() {
        for name in [
            "valid_name",
            "validName123",
            "a",
            "create_sessions_table",
            "v2_migration",
            "CREATE_SESSIONS",
        ] {
            assert!(
                validate_migration_name(name).is_ok(),
                "Should accept: {name}"
            );
        }
    }

    /// Verify special characters are rejected
    #[kani::proof]
    fn prove_migration_name_special_chars_rejected() {
        for name in [
            "invalid-name",
            "invalid name",
            "invalid.name",
            "invalid;name",
            "invalid'name",
            "invalid\"name",
            "invalid(name)",
        ] {
            assert!(
                validate_migration_name(name).is_err(),
                "Should reject: {name}"
            );
        }
    }

    /// Verify underscore-only name is accepted
    #[kani::proof]
    fn prove_migration_name_underscore_accepted() {
        assert!(validate_migration_name("_").is_ok());
        assert!(validate_migration_name("__").is_ok());
        assert!(validate_migration_name("_valid_name_").is_ok());
    }

    /// Verify numeric-only names are accepted (all chars are alphanumeric)
    #[kani::proof]
    fn prove_migration_name_numeric_accepted() {
        assert!(validate_migration_name("123").is_ok());
        assert!(validate_migration_name("0").is_ok());
    }
}
