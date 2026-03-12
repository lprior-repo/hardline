//! Config commands

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
    let config_file = dir.join("config.toml");
    Ok(config_file)
}

/// Load config from file
fn load_config() -> Result<std::collections::HashMap<String, String>> {
    let config_file = get_config_file()?;

    let mut config = std::collections::HashMap::new();

    if config_file.exists() {
        let contents = fs::read_to_string(&config_file).map_err(|e| Error::Io(e))?;

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
        fs::create_dir_all(parent).map_err(|e| Error::Io(e))?;
    }

    // Write config
    let mut contents = String::new();
    contents.push_str("# SCP Configuration\n\n");

    for (key, value) in config.iter() {
        contents.push_str(&format!("{} = {}\n", key, value));
    }

    fs::write(&config_file, contents).map_err(|e| Error::Io(e))?;

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
            Err(Error::ConfigNotFound(key.to_string()))
        }
    }
}

/// Set config value
pub fn set(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;

    // Validate key
    if key.is_empty() {
        return Err(Error::ConfigInvalid("Key cannot be empty".into()));
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
