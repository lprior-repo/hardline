use clap::Arg;

/// Create the JSON argument (common to all commands)
pub fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Output as JSON (machine-parseable format)")
}

/// Create the verbose argument
pub fn verbose_arg() -> Arg {
    Arg::new("verbose")
        .long("verbose")
        .short('v')
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Enable verbose output")
}

/// Create the dry-run argument
pub fn dry_run_arg() -> Arg {
    Arg::new("dry-run")
        .long("dry-run")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("Preview without executing")
}

/// Create the contract argument (AI: Show machine-readable contract)
pub fn contract_arg() -> Arg {
    Arg::new("contract")
        .long("contract")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show machine-readable contract (JSON schema of inputs/outputs)")
}

/// Create the ai-hints argument (AI: Show execution hints)
pub fn ai_hints_arg() -> Arg {
    Arg::new("ai-hints")
        .long("ai-hints")
        .action(clap::ArgAction::SetTrue)
        .default_value("false")
        .help("AI: Show execution hints and common patterns")
}

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};

    use super::*;

    #[test]
    fn json_arg_has_long_flag() {
        let arg = json_arg();
        assert_eq!(arg.get_long(), Some("json"));
    }

    #[test]
    fn json_arg_get_flag_returns_false_by_default() {
        let cmd = Command::new("test").arg(json_arg());
        let matches = cmd.try_get_matches_from(["test"]).expect("valid");
        assert!(!matches.get_flag("json"));
    }

    #[test]
    fn json_arg_get_flag_returns_true_when_set() {
        let cmd = Command::new("test").arg(json_arg());
        let matches = cmd.try_get_matches_from(["test", "--json"]).expect("valid");
        assert!(matches.get_flag("json"));
    }

    #[test]
    fn verbose_arg_has_short_and_long() {
        let arg = verbose_arg();
        assert_eq!(arg.get_long(), Some("verbose"));
        assert_eq!(arg.get_short(), Some('v'));
    }

    #[test]
    fn verbose_arg_default_false() {
        let cmd = Command::new("test").arg(verbose_arg());
        let matches = cmd.try_get_matches_from(["test"]).expect("valid");
        assert!(!matches.get_flag("verbose"));
    }

    #[test]
    fn verbose_arg_set_via_short() {
        let cmd = Command::new("test").arg(verbose_arg());
        let matches = cmd.try_get_matches_from(["test", "-v"]).expect("valid");
        assert!(matches.get_flag("verbose"));
    }

    #[test]
    fn verbose_arg_set_via_long() {
        let cmd = Command::new("test").arg(verbose_arg());
        let matches = cmd.try_get_matches_from(["test", "--verbose"]).expect("valid");
        assert!(matches.get_flag("verbose"));
    }

    #[test]
    fn dry_run_arg_has_long_flag() {
        let arg = dry_run_arg();
        assert_eq!(arg.get_long(), Some("dry-run"));
    }

    #[test]
    fn dry_run_arg_default_false() {
        let cmd = Command::new("test").arg(dry_run_arg());
        let matches = cmd.try_get_matches_from(["test"]).expect("valid");
        assert!(!matches.get_flag("dry-run"));
    }

    #[test]
    fn dry_run_arg_set() {
        let cmd = Command::new("test").arg(dry_run_arg());
        let matches = cmd.try_get_matches_from(["test", "--dry-run"]).expect("valid");
        assert!(matches.get_flag("dry-run"));
    }

    #[test]
    fn contract_arg_has_long_flag() {
        let arg = contract_arg();
        assert_eq!(arg.get_long(), Some("contract"));
    }

    #[test]
    fn contract_arg_default_false() {
        let cmd = Command::new("test").arg(contract_arg());
        let matches = cmd.try_get_matches_from(["test"]).expect("valid");
        assert!(!matches.get_flag("contract"));
    }

    #[test]
    fn ai_hints_arg_has_long_flag() {
        let arg = ai_hints_arg();
        assert_eq!(arg.get_long(), Some("ai-hints"));
    }

    #[test]
    fn ai_hints_arg_default_false() {
        let cmd = Command::new("test").arg(ai_hints_arg());
        let matches = cmd.try_get_matches_from(["test"]).expect("valid");
        assert!(!matches.get_flag("ai-hints"));
    }

    #[test]
    fn multiple_args_combine() {
        let cmd = Command::new("test")
            .arg(json_arg())
            .arg(verbose_arg())
            .arg(dry_run_arg());
        let matches = cmd
            .try_get_matches_from(["test", "--json", "-v", "--dry-run"])
            .expect("valid");
        assert!(matches.get_flag("json"));
        assert!(matches.get_flag("verbose"));
        assert!(matches.get_flag("dry-run"));
    }

    #[test]
    fn json_arg_help_contains_machine_parseable() {
        let arg = json_arg();
        let help = arg.get_help().map(|h| h.to_string());
        assert!(help.is_some());
        assert!(help.unwrap().contains("JSON"));
    }

    #[test]
    fn verbose_arg_help_contains_verbose() {
        let arg = verbose_arg();
        let help = arg.get_help().map(|h| h.to_string());
        assert!(help.is_some());
        assert!(help.unwrap().contains("verbose"));
    }

    #[test]
    fn dry_run_arg_help_is_non_empty() {
        let arg = dry_run_arg();
        let help = arg.get_help();
        assert!(help.is_some());
        let help_str = help.expect("help exists").to_string();
        assert!(!help_str.is_empty(), "dry-run arg should have help text");
    }
}
