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
