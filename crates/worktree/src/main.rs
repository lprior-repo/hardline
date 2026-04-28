//! Worktree CLI - Command-line interface for worktree management

use std::env;

use worktree::{AbsolutePath, WorktreeId, WorktreeName, WorktreeTypeEnum};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "create" => handle_create(&args[2..]),
        "list" => handle_list(),
        "info" => handle_info(&args[2..]),
        "remove" => handle_remove(&args[2..]),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Worktree CLI - Git worktree management");
    println!();
    println!("Usage: worktree <command> [options]");
    println!();
    println!("Commands:");
    println!("  create <name> <path> <parent> [--type <type>] [--branch <branch>]");
    println!("  list [--state <state>] [--type <type>]");
    println!("  info <id>");
    println!("  remove <id>");
    println!("  help");
    println!();
    println!("Types: development, testing, review, debugging, research");
    println!("States: creating, incomplete, active, suspended, removing, removed");
}

fn handle_create(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Error: create requires name, path, and parent");
        std::process::exit(1);
    }

    let name = args[0].clone();
    let path = args[1].clone();
    let parent = args[2].clone();

    let type_str = args.get(3).map(|s| s.as_str()).unwrap_or("development");
    let worktree_type = parse_type(type_str);

    // Warn if an unrecognized type was provided
    if !is_known_type(type_str) {
        eprintln!(
            "Warning: Unknown type '{}', defaulting to Development",
            type_str
        );
    }

    let branch = parse_branch(args, 4);

    if let Some(Err(ref e)) = branch {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let branch_value = branch.and_then(|r| r.ok());

    match WorktreeName::new(&name) {
        Ok(name) => {
            match AbsolutePath::new(path) {
                Ok(path) => {
                    match AbsolutePath::new(parent) {
                        Ok(parent) => {
                            println!("Creating worktree:");
                            println!("  Name: {}", name);
                            println!("  Path: {}", path);
                            println!("  Parent: {}", parent);
                            println!("  Type: {}", worktree_type);
                            if let Some(ref b) = branch_value {
                                println!("  Branch: {}", b);
                            }

                            // In a real implementation, this would call the service
                            println!("  Status: Created (simulated)");
                        }
                        Err(e) => {
                            eprintln!("Error: Invalid parent path: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: Invalid path: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Invalid name: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_list() {
    println!("Listing worktrees (simulated)...");
    println!("No worktrees found.");
}

fn handle_info(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: info requires worktree ID");
        std::process::exit(1);
    }

    let id_str = args[0].clone();
    match WorktreeId::from_string(&id_str) {
        Ok(_id) => {
            println!("Worktree info for: {}", id_str);
            println!("  (Simulated - no database connected)");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_remove(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: remove requires worktree ID");
        std::process::exit(1);
    }

    let id_str = args[0].clone();
    match WorktreeId::from_string(&id_str) {
        Ok(_id) => {
            println!("Removing worktree: {}", id_str);
            println!("  (Simulated - no database connected)");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn parse_type(s: &str) -> WorktreeTypeEnum {
    match s.to_lowercase().as_str() {
        "testing" | "test" => WorktreeTypeEnum::Testing,
        "review" => WorktreeTypeEnum::Review,
        "debugging" | "debug" => WorktreeTypeEnum::Debugging,
        "research" => WorktreeTypeEnum::Research,
        _ => WorktreeTypeEnum::Development,
    }
}

fn is_known_type(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "development" | "testing" | "test" | "review" | "debugging" | "debug" | "research"
    )
}

fn parse_branch(
    args: &[String],
    start: usize,
) -> Option<Result<worktree::BranchName, worktree::WorktreeDomainError>> {
    if args.get(start).map(|s| s.as_str()) == Some("--branch") {
        match args.get(start + 1) {
            Some(val) => Some(worktree::BranchName::new(val)),
            None => Some(Err(worktree::WorktreeDomainError::InvalidBranch(
                "--branch requires a value".to_string(),
            ))),
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type_development_string_returns_development_type() {
        assert_eq!(parse_type("development"), WorktreeTypeEnum::Development);
        assert_eq!(parse_type("DEV"), WorktreeTypeEnum::Development);
    }

    #[test]
    fn parse_type_testing_string_returns_testing_type() {
        assert_eq!(parse_type("testing"), WorktreeTypeEnum::Testing);
        assert_eq!(parse_type("test"), WorktreeTypeEnum::Testing);
    }

    #[test]
    fn parse_type_review_string_returns_review_type() {
        assert_eq!(parse_type("review"), WorktreeTypeEnum::Review);
    }

    #[test]
    fn parse_type_debugging_string_returns_debugging_type() {
        assert_eq!(parse_type("debugging"), WorktreeTypeEnum::Debugging);
        assert_eq!(parse_type("debug"), WorktreeTypeEnum::Debugging);
    }

    #[test]
    fn parse_type_research_string_returns_research_type() {
        assert_eq!(parse_type("research"), WorktreeTypeEnum::Research);
    }

    #[test]
    fn parse_type_unknown_string_returns_development_type() {
        assert_eq!(parse_type("unknown"), WorktreeTypeEnum::Development);
    }
}
