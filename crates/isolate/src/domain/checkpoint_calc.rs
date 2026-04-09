//! Checkpoint calculation functions — pure, no side effects.
//!
//! Classifies commands into risk levels and provides pure helpers
//! for checkpoint-related decisions.

use super::checkpoint_types::OperationRisk;

/// Classifies a command name into its risk level.
#[must_use]
pub fn classify_command(command: &str) -> OperationRisk {
    match command {
        "batch" | "spawn" | "remove" | "cleanup" | "rebase" | "squash" => OperationRisk::Risky,
        _ => OperationRisk::Safe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_safe_commands() {
        for cmd in [
            "list", "status", "context", "focus", "help", "version", "show",
        ] {
            assert_eq!(
                classify_command(cmd),
                OperationRisk::Safe,
                "expected '{cmd}' to be Safe"
            );
        }
    }

    #[test]
    fn classify_risky_commands() {
        for cmd in ["batch", "spawn", "remove", "cleanup", "rebase", "squash"] {
            assert_eq!(
                classify_command(cmd),
                OperationRisk::Risky,
                "expected '{cmd}' to be Risky"
            );
        }
    }

    #[test]
    fn classify_empty_string_is_safe() {
        assert_eq!(classify_command(""), OperationRisk::Safe);
    }

    #[test]
    fn classify_unknown_is_safe() {
        assert_eq!(classify_command("foobar"), OperationRisk::Safe);
    }

    #[test]
    fn classify_case_sensitive() {
        assert_eq!(classify_command("Batch"), OperationRisk::Safe);
        assert_eq!(classify_command("batch"), OperationRisk::Risky);
    }
}
