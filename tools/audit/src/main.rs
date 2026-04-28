//! Isolate Codebase Auditor
//!
//! Systematic detection of forbidden patterns in Rust codebase.
//! Searches for: unwrap, expect, panic, todo, unimplemented, unsafe
//!
//! Usage: cargo run --manifest-path tools/audit/Cargo.toml

use std::{fs, path::Path};

use regex::Regex;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct Violation {
    file: String,
    line: usize,
    pattern: String,
    context: String,
}

#[derive(Debug)]
struct AuditReport {
    production_violations: Vec<Violation>,
    test_violations: Vec<Violation>,
}

impl AuditReport {
    fn total_violations(&self) -> usize {
        self.production_violations.len() + self.test_violations.len()
    }

    fn is_clean(&self) -> bool {
        self.production_violations.is_empty()
    }
}

fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!")
}

fn audit_codebase(root: &Path) -> Result<AuditReport, Box<dyn std::error::Error>> {
    let mut all_violations = Vec::new();

    // Forbidden patterns and their names
    let forbidden_patterns = vec![
        (r"\.unwrap\(\)", "unwrap"),
        (r"\.expect\(", "expect"),
        (r"panic!\(", "panic!"),
        (r"\btodo!\(", "todo!"),
        (r"\bunimplemented!\(", "unimplemented!"),
        (r"unsafe\s*\{", "unsafe"),
    ];

    let patterns: Vec<_> = forbidden_patterns
        .iter()
        .map(|(pat, name)| (Regex::new(pat).map(|r| (r, *name))))
        .collect::<Result<Vec<_>, _>>()?;

    // Walk all .rs files in crates/ directory
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let path = entry.path();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_num, line) in content.lines().enumerate() {
            // Skip comment-only lines
            if is_comment_line(line) {
                continue;
            }

            for (regex, pattern_name) in &patterns {
                if regex.is_match(line) {
                    all_violations.push(Violation {
                        file: path
                            .strip_prefix(root)
                            .unwrap_or(path)
                            .display()
                            .to_string(),
                        line: line_num + 1,
                        pattern: pattern_name.to_string(),
                        context: line.trim().to_string(),
                    });
                }
            }
        }
    }

    // Separate production and test violations
    let (test_violations, production_violations): (Vec<_>, Vec<_>) = all_violations
        .into_iter()
        .partition(|v| v.file.contains("/tests/") || v.file.contains("/test_"));

    Ok(AuditReport {
        production_violations,
        test_violations,
    })
}

fn print_report(report: &AuditReport) {
    println!("\n=== Isolate CODEBASE AUDIT REPORT ===");
    println!();

    // Production violations (critical)
    if report.production_violations.is_empty() {
        println!("[OK] Production Code: CLEAN (0 violations)");
    } else {
        println!(
            "[!] Production Code: {} VIOLATIONS",
            report.production_violations.len()
        );
        for v in &report.production_violations {
            println!("  [X] {}:{} [{}] {}", v.file, v.line, v.pattern, v.context);
        }
    }

    println!();

    // Test violations (warnings)
    if report.test_violations.is_empty() {
        println!("[OK] Test Code: CLEAN (0 violations)");
    } else {
        println!("[!] Test Code: {} violations", report.test_violations.len());

        let mut by_pattern: std::collections::HashMap<String, Vec<&Violation>> =
            std::collections::HashMap::new();
        for v in &report.test_violations {
            by_pattern.entry(v.pattern.clone()).or_default().push(v);
        }

        for (pattern, violations) in by_pattern {
            println!("  [!] {} ({} occurrences)", pattern, violations.len());
            for v in violations.iter().take(5) {
                println!("    -> {}:{}", v.file, v.line);
            }
            if violations.len() > 5 {
                println!("    -> ... and {} more", violations.len() - 5);
            }
        }
    }

    println!();
    println!("=== SUMMARY ===");
    println!();

    if report.is_clean() {
        println!("[OK] Production code passes all checks!");
    } else {
        println!(
            "[X] Production code has {} critical violations",
            report.production_violations.len()
        );
    }

    if report.test_violations.is_empty() {
        println!("[OK] Test code passes all checks!");
    } else {
        println!(
            "[!] Test code has {} violations (low priority)",
            report.test_violations.len()
        );
    }

    println!();
    println!("[INFO] Total violations: {}", report.total_violations());
    println!();

    let grade = if report.is_clean() && report.test_violations.is_empty() {
        "A+"
    } else if report.is_clean() {
        "A"
    } else if report.production_violations.len() < 5 {
        "B"
    } else {
        "C"
    };

    println!("[INFO] Overall Grade: {}", grade);
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crates_dir = Path::new("./crates");

    if !crates_dir.exists() {
        eprintln!("[X] Crates directory not found. Run from project root.");
        std::process::exit(1);
    }

    println!();
    println!("[INFO] Scanning codebase...");
    println!();

    let report = audit_codebase(crates_dir)?;
    print_report(&report);

    if !report.is_clean() {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_comment_line() {
        assert!(is_comment_line("// This is a comment"));
        assert!(is_comment_line("/// Doc comment"));
        assert!(is_comment_line("//! Module doc"));
        assert!(!is_comment_line("let x = 5; // comment"));
        assert!(!is_comment_line("    // comment"));
    }
}
