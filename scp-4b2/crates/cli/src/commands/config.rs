//! Config commands

use itertools::Itertools;
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
    (trimmed.is_empty() || trimmed.starts_with('#'))
        .not()
        .then(|| ())
        .and_then(|()| trimmed.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
}

/// Parse config file contents into a HashTrieMap
fn parse_config_contents(contents: &str) -> HashTrieMap<String, String> {
    contents.lines().filter_map(parse_config_line).collect()
}

/// Load config from file
fn load_config() -> Result<HashTrieMap<String, String>> {
    get_config_file()
        .and_then(|config_file| {
            config_file
                .exists()
                .then_some(config_file)
                .map(|_| fs::read_to_string(&config_file).map_err(|e| Error::Io(e)))
                .unwrap_or_else(|| Ok(String::new()))
        })
        .map(parse_config_contents)
}

/// Save config to file
fn save_config(config: &HashTrieMap<String, String>) -> Result<()> {
    get_config_file().and_then(|config_file| {
        config_file
            .parent()
            .map(|parent| fs::create_dir_all(parent).map_err(|e| Error::Io(e)))
            .unwrap_or_else(|| Ok(()))?;
        let contents = ["# SCP Configuration", ""]
            .iter()
            .chain(config.iter().map(|(k, v)| format!("{} = {}", k, v)))
            .join("\n");
        fs::write(&config_file, contents).map_err(|e| Error::Io(e))
    })
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
    (key.is_empty())
        .then(|| Err(Error::ConfigInvalid("Key cannot be empty".into())))
        .unwrap_or_else(|| {
            load_config()
                .map(|config| config.insert(key.to_string(), value.to_string()))
                .and_then(|new_config| save_config(&new_config))
                .map(|()| {
                    println!("✓ Set {} = {}", key, value);
                })
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
