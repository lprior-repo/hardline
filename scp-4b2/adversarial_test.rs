use proptest::prelude::*;

// We will test both SessionName implementations to find edge cases.
#[path = "crates/core/src/types.rs"]
mod core_types;

#[path = "crates/session/src/domain/value_objects/mod.rs"]
mod session_types;

proptest! {
    #[test]
    fn test_session_name_equivalence(s in ".*") {
        let core_res = core_types::SessionName::parse(s.clone());
        let session_res = session_types::SessionName::parse(s.clone());

        // We expect both to either succeed or fail together.
        // If they don't, we found an inconsistency.
        prop_assert_eq!(
            core_res.is_ok(),
            session_res.is_ok(),
            "Inconsistency found for input: {:?}\ncore: {:?}\nsession: {:?}",
            s,
            core_res.map(|n| n.as_str().to_string()),
            session_res.map(|n| n.as_str().to_string())
        );

        // If both succeed, the resulting string should be the same
        if let (Ok(core_name), Ok(session_name)) = (core_res, session_res) {
            prop_assert_eq!(
                core_name.as_str(),
                session_name.as_str(),
                "Parsed values differ for input: {:?}", s
            );
        }
    }

    #[test]
    fn test_core_session_name_adversarial(s in ".*") {
        let res = core_types::SessionName::parse(s.clone());
        let trimmed = s.trim();
        let is_valid = !trimmed.is_empty()
            && trimmed.len() <= 63
            && trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
            && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        prop_assert_eq!(res.is_ok(), is_valid, "Failed for string: {:?}", s);
    }

    #[test]
    fn test_session_session_name_adversarial(s in ".*") {
        let res = session_types::SessionName::parse(s.clone());
        let trimmed = s.trim();
        let is_valid = !trimmed.is_empty()
            && trimmed.len() <= 63
            && trimmed.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        prop_assert_eq!(res.is_ok(), is_valid, "Failed for string: {:?}", s);
    }
}
