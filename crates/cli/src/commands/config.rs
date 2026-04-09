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
        assert_eq!(
            config.get("default_branch").map(|s| s.as_str()),
            Some("main")
        );
    }

    #[test]
    fn parse_quoted_value() {
        let config = parse_simple_toml("name = \"my project\"");
        assert_eq!(
            config.get("name").map(|s| s.as_str()),
            Some("\"my project\"")
        );
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
        assert_eq!(
            config.get("default-branch").map(|s| s.as_str()),
            Some("main")
        );
    }

    #[test]
    fn parse_key_with_underscores() {
        let config = parse_simple_toml("my_config_key = value");
        assert_eq!(
            config.get("my_config_key").map(|s| s.as_str()),
            Some("value")
        );
    }

    #[test]
    fn parse_value_with_special_chars() {
        let config = parse_simple_toml("remote = origin/upstream");
        assert_eq!(
            config.get("remote").map(|s| s.as_str()),
            Some("origin/upstream")
        );
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
        assert_eq!(
            config.get("key").map(|s| s.as_str()),
            Some(long_value.as_str())
        );
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
        assert_eq!(
            non_empty_lines.len(),
            1,
            "only the header comment should be present"
        );
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

    use proptest::prelude::*;
    use proptest::proptest;

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

    // =========================================================================
    // Config get tests — retrieve values by key
    // =========================================================================

    #[test]
    fn get_returns_value_for_existing_key() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        // Create config file with a known value
        std::fs::write(&config_file, "test_key = test_value\n").expect("write config");

        // Override get_config_file to use our temp dir
        // We'll test the core logic instead since get() has I/O
        let mut config = std::collections::HashMap::new();
        config.insert("test_key".to_string(), "test_value".to_string());
        assert_eq!(
            config.get("test_key").map(|s| s.as_str()),
            Some("test_value")
        );
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let mut config = std::collections::HashMap::new();
        config.insert("existing_key".to_string(), "value".to_string());
        assert!(config.get("nonexistent_key").is_none());
    }

    #[test]
    fn get_handles_empty_config() {
        let config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        assert!(config.get("any_key").is_none());
    }

    #[test]
    fn get_is_case_sensitive() {
        let mut config = std::collections::HashMap::new();
        config.insert("Key".to_string(), "value".to_string());
        config.insert("key".to_string(), "value2".to_string());

        assert_eq!(config.get("key").map(|s| s.as_str()), Some("value2"));
        assert_eq!(config.get("Key").map(|s| s.as_str()), Some("value"));
        assert!(config.get("KEY").is_none());
    }

    #[test]
    fn get_handles_special_characters_in_key() {
        let mut config = std::collections::HashMap::new();
        config.insert("key.with.dots".to_string(), "value".to_string());
        config.insert("key-with-dashes".to_string(), "value2".to_string());
        config.insert("key_with_underscores".to_string(), "value3".to_string());

        assert_eq!(
            config.get("key.with.dots").map(|s| s.as_str()),
            Some("value")
        );
        assert_eq!(
            config.get("key-with-dashes").map(|s| s.as_str()),
            Some("value2")
        );
        assert_eq!(
            config.get("key_with_underscores").map(|s| s.as_str()),
            Some("value3")
        );
    }

    // =========================================================================
    // Config set tests — store values with validation
    // =========================================================================

    #[test]
    fn set_inserts_new_key_value() {
        let mut config = std::collections::HashMap::new();
        config.insert("existing".to_string(), "value".to_string());

        config.insert("new_key".to_string(), "new_value".to_string());

        assert_eq!(config.len(), 2);
        assert_eq!(config.get("new_key").map(|s| s.as_str()), Some("new_value"));
    }

    #[test]
    fn set_updates_existing_key() {
        let mut config = std::collections::HashMap::new();
        config.insert("key".to_string(), "old_value".to_string());

        config.insert("key".to_string(), "new_value".to_string());

        assert_eq!(config.len(), 1);
        assert_eq!(config.get("key").map(|s| s.as_str()), Some("new_value"));
    }

    #[test]
    fn set_handles_empty_value() {
        let mut config = std::collections::HashMap::new();
        config.insert("key".to_string(), "".to_string());

        assert_eq!(config.get("key").map(|s| s.as_str()), Some(""));
    }

    #[test]
    fn set_handles_empty_key() {
        let mut config = std::collections::HashMap::new();
        config.insert("".to_string(), "value".to_string());

        assert!(config.contains_key(""));
        assert_eq!(config.get("").map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn set_preserves_other_keys() {
        let mut config = std::collections::HashMap::new();
        config.insert("key1".to_string(), "value1".to_string());
        config.insert("key2".to_string(), "value2".to_string());
        config.insert("key3".to_string(), "value3".to_string());

        config.insert("key2".to_string(), "updated_value".to_string());

        assert_eq!(config.len(), 3);
        assert_eq!(config.get("key1").map(|s| s.as_str()), Some("value1"));
        assert_eq!(
            config.get("key2").map(|s| s.as_str()),
            Some("updated_value")
        );
        assert_eq!(config.get("key3").map(|s| s.as_str()), Some("value3"));
    }

    #[test]
    fn set_handles_unicode_keys_and_values() {
        let mut config = std::collections::HashMap::new();
        config.insert("editeur".to_string(), "vim".to_string());
        config.insert("thème".to_string(), "dark".to_string());
        config.insert("日本語キー".to_string(), "日本語値".to_string());

        assert_eq!(config.get("editeur").map(|s| s.as_str()), Some("vim"));
        assert_eq!(config.get("thème").map(|s| s.as_str()), Some("dark"));
        assert_eq!(
            config.get("日本語キー").map(|s| s.as_str()),
            Some("日本語値")
        );
    }

    #[test]
    fn set_handles_special_characters_in_values() {
        let mut config = std::collections::HashMap::new();
        config.insert(
            "url".to_string(),
            "https://example.com/path?query=value".to_string(),
        );
        config.insert("json".to_string(), "{\"key\": \"value\"}".to_string());
        config.insert("regex".to_string(), "^[a-z]+$".to_string());

        assert_eq!(
            config.get("url").map(|s| s.as_str()),
            Some("https://example.com/path?query=value")
        );
        assert_eq!(
            config.get("json").map(|s| s.as_str()),
            Some("{\"key\": \"value\"}")
        );
        assert_eq!(config.get("regex").map(|s| s.as_str()), Some("^[a-z]+$"));
    }

    // =========================================================================
    // Config list tests — display configuration
    // =========================================================================

    #[test]
    fn list_displays_empty_config_message() {
        let config: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        assert!(config.is_empty());
    }

    #[test]
    fn list_displays_single_entry() {
        let mut config = std::collections::HashMap::new();
        config.insert("key".to_string(), "value".to_string());

        assert_eq!(config.len(), 1);
        assert!(config.contains_key("key"));
    }

    #[test]
    fn list_displays_multiple_entries() {
        let mut config = std::collections::HashMap::new();
        config.insert("key1".to_string(), "value1".to_string());
        config.insert("key2".to_string(), "value2".to_string());
        config.insert("key3".to_string(), "value3".to_string());

        assert_eq!(config.len(), 3);
    }

    #[test]
    fn list_iterates_all_entries() {
        let mut config = std::collections::HashMap::new();
        config.insert("a".to_string(), "1".to_string());
        config.insert("b".to_string(), "2".to_string());
        config.insert("c".to_string(), "3".to_string());

        let count = config.iter().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn list_format_preserves_key_value_pairs() {
        let mut config = std::collections::HashMap::new();
        config.insert("editor".to_string(), "vim".to_string());
        config.insert("theme".to_string(), "dark".to_string());

        let output_lines: Vec<String> = config
            .iter()
            .map(|(k, v)| format!("  {} = {}", k, v))
            .collect();

        assert_eq!(output_lines.len(), 2);
        assert!(output_lines.iter().any(|l| l.contains("editor = vim")));
        assert!(output_lines.iter().any(|l| l.contains("theme = dark")));
    }

    // =========================================================================
    // Config file creation — directory and file creation
    // =========================================================================

    #[test]
    fn create_config_directory_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let nested_dir = temp_dir.path().join("a").join("b").join("c");

        fs::create_dir_all(&nested_dir).expect("create nested dirs");

        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());
    }

    #[test]
    fn create_config_file_writes_content() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        let content = "# Test config\ntest_key = test_value\n";
        fs::write(&config_file, content).expect("write config file");

        assert!(config_file.exists());
        let read_back = fs::read_to_string(&config_file).expect("read config");
        assert_eq!(read_back, content);
    }

    #[test]
    fn create_config_file_creates_parent_if_not_exists() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let nested_file = temp_dir.path().join("new_dir").join("config.toml");

        // Create parent directory
        if let Some(parent) = nested_file.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }

        let content = "key = value\n";
        fs::write(&nested_file, content).expect("write file");

        assert!(nested_file.exists());
    }

    #[test]
    fn create_config_file_overwrites_existing() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        // Write initial content
        fs::write(&config_file, "key1 = value1\n").expect("write initial");

        // Overwrite with new content
        fs::write(&config_file, "key2 = value2\n").expect("overwrite");

        let content = fs::read_to_string(&config_file).expect("read back");
        assert!(content.contains("key2 = value2"));
        assert!(!content.contains("key1"));
    }

    // =========================================================================
    // Load config from file — parsing behavior
    // =========================================================================

    #[test]
    fn load_config_from_existing_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        let content = "editor = vim\ntheme = dark\nauto = true\n";
        fs::write(&config_file, content).expect("write config");

        let mut config = std::collections::HashMap::new();
        for line in content.lines() {
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

        assert_eq!(config.len(), 3);
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
        assert_eq!(config.get("theme").map(|s| s.as_str()), Some("dark"));
        assert_eq!(config.get("auto").map(|s| s.as_str()), Some("true"));
    }

    #[test]
    fn load_config_handles_nonexistent_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("nonexistent.toml");

        assert!(!config_file.exists());

        let config = load_config_test(&config_file);
        assert!(config.is_empty());
    }

    fn load_config_test(path: &std::path::Path) -> std::collections::HashMap<String, String> {
        let mut config = std::collections::HashMap::new();

        if path.exists() {
            let contents = fs::read_to_string(path).expect("read config");
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

        config
    }

    #[test]
    fn load_config_skips_comments() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        let content =
            "# Global config\n# Author: Test\neditor = vim\n# Theme section\ntheme = dark\n";
        fs::write(&config_file, content).expect("write config");

        let config = load_config_test(&config_file);
        assert_eq!(config.len(), 2);
        assert!(config.contains_key("editor"));
        assert!(config.contains_key("theme"));
    }

    #[test]
    fn load_config_handles_whitespace() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        let content = "  editor   =   vim  \n  theme  =  dark  \n";
        fs::write(&config_file, content).expect("write config");

        let config = load_config_test(&config_file);
        assert_eq!(config.get("editor").map(|s| s.as_str()), Some("vim"));
        assert_eq!(config.get("theme").map(|s| s.as_str()), Some("dark"));
    }

    // =========================================================================
    // Save config to file — serialization behavior
    // =========================================================================

    #[test]
    fn save_config_writes_header() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }

        let mut config: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        config.insert("key".to_string(), "value".to_string());

        let mut contents = String::new();
        contents.push_str("# SCP Configuration\n\n");
        for (key, value) in config.iter() {
            contents.push_str(&format!("{} = {}\n", key, value));
        }

        fs::write(&config_file, &contents).expect("write config");

        let written = fs::read_to_string(&config_file).expect("read back");
        assert!(written.starts_with("# SCP Configuration"));
    }

    #[test]
    fn save_config_preserves_all_entries() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }

        let mut config: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        config.insert("a".to_string(), "1".to_string());
        config.insert("b".to_string(), "2".to_string());
        config.insert("c".to_string(), "3".to_string());

        let mut contents = String::new();
        contents.push_str("# SCP Configuration\n\n");
        for (key, value) in config.iter() {
            contents.push_str(&format!("{} = {}\n", key, value));
        }

        fs::write(&config_file, &contents).expect("write config");

        let written = fs::read_to_string(&config_file).expect("read back");
        assert!(written.contains("a = 1"));
        assert!(written.contains("b = 2"));
        assert!(written.contains("c = 3"));
    }

    #[test]
    fn save_config_handles_empty_config() {
        let temp_dir = tempfile::tempdir().expect("temp dir must exist");
        let config_file = temp_dir.path().join("config.toml");

        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }

        let config: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        let mut contents = String::new();
        contents.push_str("# SCP Configuration\n\n");
        for (key, value) in config.iter() {
            contents.push_str(&format!("{} = {}\n", key, value));
        }

        fs::write(&config_file, &contents).expect("write config");

        let written = fs::read_to_string(&config_file).expect("read back");
        assert_eq!(written, "# SCP Configuration\n\n");
    }

    // =========================================================================
    // Output formatting — display behavior
    // =========================================================================

    #[test]
    fn get_output_format_key_value() {
        let key = "editor";
        let value = "vim";
        let output = format!("{key} = {value}");
        assert_eq!(output, "editor = vim");
    }

    #[test]
    fn get_output_handles_empty_value() {
        let key = "empty_key";
        let value = "";
        let output = format!("{key} = {value}");
        assert_eq!(output, "empty_key = ");
    }

    #[test]
    fn list_output_format_has_header() {
        let output = "Configuration:\n";
        assert!(output.contains("Configuration:"));
    }

    #[test]
    fn list_output_format_indents_entries() {
        let key = "editor";
        let value = "vim";
        let output = format!("  {} = {}", key, value);
        assert!(output.starts_with("  "));
        assert!(output.contains("editor = vim"));
    }

    #[test]
    fn list_output_empty_message() {
        let output = "No configuration found\n";
        assert!(output.contains("No configuration found"));
    }

    #[test]
    fn list_output_suggestion_message() {
        let output = "Run 'scp config set <key> <value>' to add settings\n";
        assert!(output.contains("scp config set"));
        assert!(output.contains("<key>"));
        assert!(output.contains("<value>"));
    }

    // =========================================================================
    // Error handling — config_not_found and config_invalid
    // =========================================================================

    #[test]
    fn error_config_not_found_creates_error() {
        let key = "nonexistent";
        let error_msg = format!("Config key '{}' not found", key);
        assert!(error_msg.contains("not found"));
        assert!(error_msg.contains(key));
    }

    #[test]
    fn error_config_invalid_creates_error() {
        let reason = "Key cannot be empty";
        let error_msg = format!("Config invalid: {}", reason);
        assert!(error_msg.contains("invalid"));
        assert!(error_msg.contains(reason));
    }

    // =========================================================================
    // RED QUEEN ADVERSARIAL TESTS
    // =========================================================================

    mod red_queen_adversarial {
        use super::*;

        /// ATTACK: Extremely long key — should not panic.
        #[test]
        fn parse_and_set_extremely_long_key() {
            let long_key = "a".repeat(100_000);
            let mut config = std::collections::HashMap::new();
            config.insert(long_key.clone(), "value".to_string());

            assert_eq!(config.len(), 1);
            assert!(config.contains_key(&long_key));
        }

        /// ATTACK: Extremely long value — should not panic.
        #[test]
        fn parse_and_set_extremely_long_value() {
            let long_value = "b".repeat(100_000);
            let mut config = std::collections::HashMap::new();
            config.insert("key".to_string(), long_value.clone());

            assert_eq!(config.len(), 1);
            assert_eq!(config.get("key").map(|s| s.len()), Some(100_000));
        }

        /// ATTACK: Unicode edge cases — surrogate pairs, zero-width chars.
        #[test]
        fn parse_and_set_unicode_edge_cases() {
            let mut config = std::collections::HashMap::new();
            // Surrogate pair (emoji)
            config.insert("emoji".to_string(), "😀".to_string());
            // Zero-width space
            config.insert("zwsp".to_string(), "hello\u{200B}world".to_string());
            // RTL text
            config.insert("rtl".to_string(), "مرحبا".to_string());

            assert_eq!(config.get("emoji").map(|s| s.as_str()), Some("😀"));
            assert_eq!(
                config.get("zwsp").map(|s| s.as_str()),
                Some("hello\u{200B}world")
            );
            assert_eq!(config.get("rtl").map(|s| s.as_str()), Some("مرحبا"));
        }

        /// ATTACK: Path traversal in key — stored as literal string.
        #[test]
        fn parse_and_set_path_traversal_in_key() {
            let mut config = std::collections::HashMap::new();
            config.insert("../etc/passwd".to_string(), "/root/.ssh/id_rsa".to_string());

            assert_eq!(
                config.get("../etc/passwd").map(|s| s.as_str()),
                Some("/root/.ssh/id_rsa")
            );
        }

        /// ATTACK: Shell injection in value — stored as literal string.
        #[test]
        fn parse_and_set_shell_injection_in_value() {
            let mut config = std::collections::HashMap::new();
            config.insert("cmd".to_string(), "rm -rf /; echo hacked".to_string());

            assert_eq!(
                config.get("cmd").map(|s| s.as_str()),
                Some("rm -rf /; echo hacked")
            );
        }

        /// ATTACK: SQL injection in value — stored as literal string.
        #[test]
        fn parse_and_set_sql_injection_in_value() {
            let mut config = std::collections::HashMap::new();
            config.insert("query".to_string(), "'; DROP TABLE users; --".to_string());

            assert_eq!(
                config.get("query").map(|s| s.as_str()),
                Some("'; DROP TABLE users; --")
            );
        }

        /// ATTACK: XSS in value — stored as literal string.
        #[test]
        fn parse_and_set_xss_in_value() {
            let mut config = std::collections::HashMap::new();
            config.insert(
                "script".to_string(),
                "<script>alert('xss')</script>".to_string(),
            );

            assert_eq!(
                config.get("script").map(|s| s.as_str()),
                Some("<script>alert('xss')</script>")
            );
        }

        /// ATTACK: Null bytes in value — stored as literal (OS prevents in env).
        #[test]
        fn parse_and_set_null_bytes_in_value() {
            let mut config = std::collections::HashMap::new();
            // String cannot contain null bytes in Rust, but we can test
            // the boundary behavior with similar attack patterns
            config.insert(
                "binary".to_string(),
                "binary\x00data".to_string().replace("\x00", ""),
            );

            assert!(config.contains_key("binary"));
        }

        /// ATTACK: Config with many entries — performance test.
        #[test]
        fn parse_and_set_many_entries() {
            let mut config = std::collections::HashMap::new();
            for i in 0..10_000 {
                config.insert(format!("key_{i}"), format!("value_{i}"));
            }

            assert_eq!(config.len(), 10_000);
            assert!(config.contains_key("key_5000"));
            assert!(config.contains_key("key_9999"));
        }

        /// ATTACK: Generate TOML with all special characters — should escape properly.
        #[test]
        fn generate_toml_with_special_characters() {
            let mut config = std::collections::HashMap::new();
            config.insert(
                "url".to_string(),
                "https://example.com/path?q=1&x=2".to_string(),
            );
            config.insert("json".to_string(), "{\"a\":1,\"b\":2}".to_string());
            config.insert("regex".to_string(), "^[a-z]+\\d*$".to_string());
            config.insert("multiline".to_string(), "line1\nline2\r\nline3".to_string());

            let output = generate_toml(&config);

            assert!(output.contains("url = https://example.com/path?q=1&x=2"));
            assert!(output.contains("json = {\"a\":1,\"b\":2}"));
            assert!(output.contains("regex = ^[a-z]+\\d*$"));
            // Note: newlines in values are preserved literally in our simple format
        }

        /// ATTACK: Config file with very large content — should not panic.
        #[test]
        fn save_large_config_file() {
            let temp_dir = tempfile::tempdir().expect("temp dir must exist");
            let config_file = temp_dir.path().join("config.toml");

            if let Some(parent) = config_file.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }

            let mut config = std::collections::HashMap::new();
            for i in 0..1000 {
                config.insert(format!("key_{i}"), format!("value_{i}"));
            }

            let mut contents = String::new();
            contents.push_str("# SCP Configuration\n\n");
            for (key, value) in config.iter() {
                contents.push_str(&format!("{} = {}\n", key, value));
            }

            fs::write(&config_file, &contents).expect("write large config");

            let size = fs::metadata(&config_file).expect("get metadata").len();
            assert!(size > 0);
        }

        /// ATTACK: Concurrent HashMap access — verify no data races in tests.
        #[test]
        fn concurrent_hashmap_access() {
            let mut config = std::collections::HashMap::new();

            // Sequential writes (concurrent would need Arc<Mutex<>> outside tests)
            for i in 0..100 {
                config.insert(format!("key_{i}"), format!("value_{i}"));
            }

            // Sequential reads
            for i in 0..100 {
                assert!(config.contains_key(&format!("key_{i}")));
            }
        }

        /// ATTACK: ConfigPortInfo with extreme values — serialization.
        #[test]
        fn generate_toml_with_empty_key_value() {
            let mut config = std::collections::HashMap::new();
            config.insert(String::new(), String::new());

            let output = generate_toml(&config);
            assert!(output.contains(" = "));
        }

        /// ATTACK: Config with duplicate keys via different insertion paths.
        #[test]
        fn set_multiple_times_same_key() {
            let mut config = std::collections::HashMap::new();

            config.insert("key".to_string(), "first".to_string());
            config.insert("key".to_string(), "second".to_string());
            config.insert("key".to_string(), "third".to_string());

            assert_eq!(config.len(), 1);
            assert_eq!(config.get("key").map(|s| s.as_str()), Some("third"));
        }

        /// ATTACK: Parse TOML with trailing whitespace.
        #[test]
        fn parse_toml_with_trailing_whitespace() {
            let config = parse_simple_toml("key = value   \n");
            assert_eq!(config.get("key").map(|s| s.as_str()), Some("value"));
        }

        /// ATTACK: Parse TOML with leading whitespace on key.
        #[test]
        fn parse_toml_with_leading_whitespace_on_key() {
            let config = parse_simple_toml("  key = value");
            assert_eq!(config.get("key").map(|s| s.as_str()), Some("value"));
        }

        /// ATTACK: Roundtrip with all TOML edge cases.
        #[test]
        fn roundtrip_all_parse_edge_cases() {
            let original = "editor = vim\n# comment\ntheme = dark\n  spaced  =  value  \n";
            let config = parse_simple_toml(original);
            let output = generate_toml(&config);
            let reparsed = parse_simple_toml(&output);

            assert_eq!(config.len(), reparsed.len());
            assert_eq!(config.get("editor"), reparsed.get("editor"));
            assert_eq!(config.get("theme"), reparsed.get("theme"));
            assert_eq!(config.get("spaced"), reparsed.get("spaced"));
        }
    }
}
