//! Config commands

use rpds::HashTrieMap;
use scp_core::{Error, Result};
use std::fs;
use std::path::PathBuf;

/// Get the config directory
fn get_config_dir() -> Result<PathBuf> {
    let dir = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::ConfigNotFound("Could not determine config directory".into()))?;
    Ok(dir.config_dir().to_path_buf())
}

/// Get or create config file path
fn get_config_file() -> Result<PathBuf> {
    let dir = get_config_dir()?;
    Ok(dir.join("config.toml"))
}

/// Parse a single TOML config line into an optional key-value pair
fn parse_config_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        None
    } else {
        trimmed
            .split_once('=')
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
    }
}

/// Parse config file contents into a HashTrieMap
fn parse_config_contents(contents: &str) -> HashTrieMap<String, String> {
    contents.lines().filter_map(parse_config_line).collect()
}

/// Load config from file
fn load_config() -> Result<HashTrieMap<String, String>> {
    let config_file = get_config_file()?;
    if config_file.exists() {
        let contents = fs::read_to_string(&config_file).map_err(Error::Io)?;
        Ok(parse_config_contents(&contents))
    } else {
        Ok(HashTrieMap::new())
    }
}

/// Format a single config entry as a TOML line
fn format_config_entry(key: &str, value: &str) -> String {
    format!("{} = {}\n", key, value)
}

/// Serialize config map to TOML-formatted string
fn serialize_config(config: &HashTrieMap<String, String>) -> String {
    let header = String::from("# SCP Configuration\n\n");
    let entries = config
        .iter()
        .map(|(k, v)| format_config_entry(k, v))
        .collect::<Vec<_>>()
        .join("");
    format!("{}{}", header, entries)
}

/// Save config to file
fn save_config(config: &HashTrieMap<String, String>) -> Result<()> {
    let config_file = get_config_file()?;
    config_file
        .parent()
        .map(fs::create_dir_all)
        .transpose()
        .map_err(Error::Io)?;
    fs::write(&config_file, serialize_config(config)).map_err(Error::Io)
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
            Err(Error::ConfigNotFound(key.to_string()))
        }
    }
}

/// Set config value
pub fn set(key: &str, value: &str) -> Result<()> {
    if key.is_empty() {
        return Err(Error::ConfigInvalid("Key cannot be empty".into()));
    }
    load_config()
        .map(|config| config.insert(key.to_string(), value.to_string()))
        .and_then(|new_config| save_config(&new_config))
        .map(|()| {
            println!("✓ Set {} = {}", key, value);
        })
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
