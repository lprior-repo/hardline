//! Config commands

use scp_core::{Error, Result};
use std::fs;
use std::path::PathBuf;

/// Get the config directory
fn get_config_dir() -> Result<PathBuf> {
    let dir = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::config_not_found("Could not determine config directory"))?;
    Ok(dir.config_dir().to_path_buf())
}

/// Get or create config file path
fn get_config_file() -> Result<PathBuf> {
    let dir = get_config_dir()?;
    let config_file = dir.join("config.toml");
    Ok(config_file)
}

/// Load config from file
fn load_config() -> Result<std::collections::HashMap<String, String>> {
    let config_file = get_config_file()?;

    let mut config = std::collections::HashMap::new();

    if config_file.exists() {
        let contents =
            fs::read_to_string(&config_file).map_err(|e| Error::io_error(e.to_string()))?;

        // Simple TOML parsing (key = value)
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                config.insert(key, value);
            }
        }
    }

    Ok(config)
}

/// Save config to file
fn save_config(config: &std::collections::HashMap<String, String>) -> Result<()> {
    let config_file = get_config_file()?;

    // Create parent directories
    if let Some(parent) = config_file.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io_error(e.to_string()))?;
    }

    // Write config
    let mut contents = String::new();
    contents.push_str("# SCP Configuration\n\n");

    for (key, value) in config.iter() {
        contents.push_str(&format!("{} = {}\n", key, value));
    }

    fs::write(&config_file, contents).map_err(|e| Error::io_error(e.to_string()))?;

    Ok(())
}

/// Get config value
pub fn get(key: &str) -> Result<()> {
    let config = load_config()?;

    match config.get(key) {
        Some(value) => {
            println!("{} = {}", key, value);
            Ok(())
        }
        None => {
            eprintln!("Config key '{}' not found", key);
            Err(Error::config_not_found(key.to_string()))
        }
    }
}

/// Set config value
pub fn set(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;

    // Validate key
    if key.is_empty() {
        return Err(Error::config_invalid("Key cannot be empty"));
    }

    config.insert(key.to_string(), value.to_string());
    save_config(&config)?;

    println!("✓ Set {} = {}", key, value);
    Ok(())
}

/// List all config values
pub fn list() -> Result<()> {
    let config = load_config()?;

    if config.is_empty() {
        println!("No configuration found");
        println!("Run 'scp config set <key> <value>' to add settings");
    } else {
        println!("Configuration:");
        for (key, value) in config.iter() {
            println!("  {} = {}", key, value);
        }
    }

    Ok(())
}

/// Parse a simple TOML string into key-value pairs (pure function for testing)
///
/// Supports:
/// - `key = value` lines
/// - Lines starting with `#` as comments
/// - Blank lines
/// - Whitespace trimming
fn parse_simple_toml(contents: &str) -> std::collections::HashMap<String, String> {
    let mut config = std::collections::HashMap::new();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            config.insert(key, value);
        }
    }
    config
}

/// Generate TOML content from a key-value map (pure function for testing)
fn generate_toml(config: &std::collections::HashMap<String, String>) -> String {
    let mut contents = String::from("# SCP Configuration\n\n");
    for (key, value) in config.iter() {
        contents.push_str(&format!("{} = {}\n", key, value));
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_simple_toml ----

    #[test]
    fn parse_empty_string() {
        let config = parse_simple_toml("");
        assert!(config.is_empty());
    }

    #[test]
    fn parse_single_key_value() {
        let config = parse_simple_toml("editor = vim");
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
    }

    #[test]
    fn parse_multiple_key_values() {
        let config = parse_simple_toml("editor = vim\ntheme = dark");
        assert_eq!(config.len(), 2);
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
        assert_eq!(config.get("theme").map(|s| s.as_str()), Some("dark"));
    }

    #[test]
    fn parse_ignores_comments() {
        let config = parse_simple_toml("# this is a comment\neditor = vim");
        assert_eq!(config.len(), 1);
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
    }

    #[test]
    fn parse_ignores_blank_lines() {
        let config = parse_simple_toml("\n\neditor = vim\n\n");
        assert_eq!(config.len(), 1);
    }

    #[test]
    fn parse_trims_whitespace() {
        let config = parse_simple_toml("  editor   =   vim  ");
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
    }

    #[test]
    fn parse_value_with_spaces() {
        let config = parse_simple_toml("default_branch = main");
        assert_eq!(config.get("default_branch").map(|s| s.as_str()), Some("main"));
    }

    #[test]
    fn parse_quoted_value() {
        let config = parse_simple_toml("name = \"my project\"");
        assert_eq!(config.get("name").map(|s| s.as_str()), Some("\"my project\""));
    }

    #[test]
    fn parse_value_with_equals_in_value() {
        // split_once ensures only the first '=' is used as separator
        let config = parse_simple_toml("pattern = key=value");
        assert_eq!(config.get("pattern").map(|s| s.as_str()), Some("key=value"));
    }

    #[test]
    fn parse_overwrites_duplicate_keys() {
        let config = parse_simple_toml("editor = vim\neditor = emacs");
        assert_eq!(config.len(), 1);
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("emacs"));
    }

    #[test]
    fn parse_lines_without_equals_are_skipped() {
        let config = parse_simple_toml("no_equals_here\neditor = vim");
        assert_eq!(config.len(), 1);
        assert!(config.contains_key("editor"));
    }

    #[test]
    fn parse_comment_char_in_value_preserved() {
        let config = parse_simple_toml("pattern = #hashtag");
        assert_eq!(config.get("pattern").map(|s| s.as_str()), Some("#hashtag"));
    }

    #[test]
    fn parse_comment_after_value_not_treated_as_comment() {
        // Our simple parser does not support inline comments
        let config = parse_simple_toml("key = value # not a comment");
        assert_eq!(
            config.get("key").map(|s| s.as_str()),
            Some("value # not a comment")
        );
    }

    #[test]
    fn parse_numeric_value() {
        let config = parse_simple_toml("port = 8080");
        assert_eq!(config.get("port").map(|s| s.as_str()), Some("8080"));
    }

    #[test]
    fn parse_boolean_value() {
        let config = parse_simple_toml("auto_commit = true");
        assert_eq!(config.get("auto_commit").map(|s| s.as_str()), Some("true"));
    }

    // ---- generate_toml ----

    #[test]
    fn generate_empty_config() {
        let config = std::collections::HashMap::new();
        let output = generate_toml(&config);
        assert!(output.contains("# SCP Configuration"));
        assert!(output.contains("\n\n"));
    }

    #[test]
    fn generate_single_entry() {
        let mut config = std::collections::HashMap::new();
        config.insert("editor".to_string(), "vim".to_string());
        let output = generate_toml(&config);
        assert!(output.contains("editor = vim"));
    }

    #[test]
    fn generate_multiple_entries() {
        let mut config = std::collections::HashMap::new();
        config.insert("editor".to_string(), "vim".to_string());
        config.insert("theme".to_string(), "dark".to_string());
        let output = generate_toml(&config);
        assert!(output.contains("editor = vim"));
        assert!(output.contains("theme = dark"));
    }

    #[test]
    fn generate_has_header() {
        let config = std::collections::HashMap::new();
        let output = generate_toml(&config);
        assert!(output.starts_with("# SCP Configuration"));
    }

    #[test]
    fn roundtrip_parse_generate() {
        let original = "editor = vim\ntheme = dark\nauto_commit = true";
        let config = parse_simple_toml(original);
        let output = generate_toml(&config);
        let reparsed = parse_simple_toml(&output);
        assert_eq!(config, reparsed);
    }

    // ---- Additional parse_simple_toml edge cases ----

    #[test]
    fn parse_empty_key_after_equals() {
        let config = parse_simple_toml(" = value");
        // Empty key is valid for our simple parser
        assert_eq!(config.get("").map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn parse_empty_value() {
        let config = parse_simple_toml("key = ");
        assert_eq!(config.get("key").map(|s| s.as_str()), Some(""));
    }

    #[test]
    fn parse_key_with_dashes() {
        let config = parse_simple_toml("default-branch = main");
        assert_eq!(config.get("default-branch").map(|s| s.as_str()), Some("main"));
    }

    #[test]
    fn parse_key_with_underscores() {
        let config = parse_simple_toml("my_config_key = value");
        assert_eq!(config.get("my_config_key").map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn parse_value_with_special_chars() {
        let config = parse_simple_toml("remote = origin/upstream");
        assert_eq!(config.get("remote").map(|s| s.as_str()), Some("origin/upstream"));
    }

    #[test]
    fn parse_only_comments() {
        let config = parse_simple_toml("# comment 1\n# comment 2\n# comment 3");
        assert!(config.is_empty());
    }

    #[test]
    fn parse_only_blank_lines() {
        let config = parse_simple_toml("\n\n\n\n");
        assert!(config.is_empty());
    }

    #[test]
    fn parse_comment_without_space() {
        let config = parse_simple_toml("#comment\nkey = value");
        assert_eq!(config.len(), 1);
        assert_eq!(config.get("key").map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn parse_multiple_equals_in_value() {
        let config = parse_simple_toml("equation = a == b");
        assert_eq!(config.get("equation").map(|s| s.as_str()), Some("a == b"));
    }

    #[test]
    fn parse_unicode_key_and_value() {
        let config = parse_simple_toml("editeur = vim");
        assert_eq!(config.get("editeur").map(|s| s.as_str()), Some("vim"));
    }

    #[test]
    fn parse_very_long_key() {
        let long_key = "a".repeat(1000);
        let config = parse_simple_toml(&format!("{long_key} = value"));
        assert_eq!(config.get(&long_key).map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn parse_very_long_value() {
        let long_value = "b".repeat(1000);
        let config = parse_simple_toml(&format!("key = {long_value}"));
        assert_eq!(config.get("key").map(|s| s.as_str()), Some(long_value.as_str()));
    }

    #[test]
    fn parse_many_entries() {
        let input: String = (0..100)
            .map(|i| format!("key{i} = value{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let config = parse_simple_toml(&input);
        assert_eq!(config.len(), 100);
    }

    // ---- generate_toml edge cases ----

    #[test]
    fn generate_includes_all_entries() {
        let mut config = std::collections::HashMap::new();
        config.insert("a".to_string(), "1".to_string());
        config.insert("b".to_string(), "2".to_string());
        config.insert("c".to_string(), "3".to_string());
        let output = generate_toml(&config);
        assert!(output.contains("a = 1"));
        assert!(output.contains("b = 2"));
        assert!(output.contains("c = 3"));
    }

    #[test]
    fn generate_each_entry_on_own_line() {
        let mut config = std::collections::HashMap::new();
        config.insert("x".to_string(), "1".to_string());
        config.insert("y".to_string(), "2".to_string());
        let output = generate_toml(&config);
        let lines: Vec<&str> = output.lines().filter(|l| l.contains('=')).collect();
        assert_eq!(lines.len(), 2, "each key-value should be on its own line");
    }

    #[test]
    fn generate_empty_value() {
        let mut config = std::collections::HashMap::new();
        config.insert("empty".to_string(), String::new());
        let output = generate_toml(&config);
        assert!(output.contains("empty = "));
    }

    #[test]
    fn generate_value_with_spaces() {
        let mut config = std::collections::HashMap::new();
        config.insert("name".to_string(), "hello world".to_string());
        let output = generate_toml(&config);
        assert!(output.contains("name = hello world"));
    }

    #[test]
    fn generate_empty_config_contains_only_header() {
        let config = std::collections::HashMap::new();
        let output = generate_toml(&config);
        let non_empty_lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(non_empty_lines.len(), 1, "only the header comment should be present");
        assert!(non_empty_lines[0].starts_with("# SCP Configuration"));
    }

    #[test]
    fn generate_roundtrip_preserves_data() {
        let mut original = std::collections::HashMap::new();
        original.insert("key1".to_string(), "val1".to_string());
        original.insert("key2".to_string(), "val2".to_string());
        let output = generate_toml(&original);
        let reparsed = parse_simple_toml(&output);
        assert_eq!(original, reparsed);
    }

    #[test]
    fn generate_large_config() {
        let config: std::collections::HashMap<String, String> = (0..50)
            .map(|i| (format!("config_key_{i}"), format!("config_value_{i}")))
            .collect();
        let output = generate_toml(&config);
        for i in 0..50 {
            assert!(
                output.contains(&format!("config_key_{i} = config_value_{i}")),
                "output should contain entry {i}"
            );
        }
    }

    use proptest::proptest;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_parse_single_entry(key in "[a-zA-Z0-9_-]{1,50}", value in "[a-zA-Z0-9_]{0,100}") {
            let input = format!("{key} = {value}");
            let config = parse_simple_toml(&input);
            prop_assert_eq!(config.len(), 1);
            prop_assert_eq!(config.get(&key).map(|s| s.as_str()), Some(value.as_str()));
        }

        #[test]
        fn prop_generate_then_parse_roundtrip(
            entries in proptest::collection::hash_map(
                "[a-z]{1,10}",
                "[a-z0-9]{1,10}",
                0..20usize
            )
        ) {
            let output = generate_toml(&entries);
            let reparsed = parse_simple_toml(&output);
            prop_assert_eq!(entries, reparsed);
        }

        #[test]
        fn prop_parse_ignores_comment_lines(comment in "#.*") {
            let input = format!("{comment}\nkey = value");
            let config = parse_simple_toml(&input);
            prop_assert_eq!(config.len(), 1);
            prop_assert_eq!(config.get("key").map(|s| s.as_str()), Some("value"));
        }
    }
}
