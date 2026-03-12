use crate::config::{Config, ConfigManager, ConfigScope};

#[test]
fn test_config_basic() {
    let mut config = Config::new();
    assert!(config.get("test").is_none());

    config.set("test", "value");
    assert_eq!(config.get("test"), Some(&"value".to_string()));

    config.remove("test");
    assert!(config.get("test").is_none());
}

#[test]
fn test_config_validation_valid_vcs_type() {
    let mut config = Config::new();
    config.set("vcs.type", "jj");

    let errors = ConfigManager::validate(&config);
    assert!(errors.is_empty(), "jj should be valid: {:?}", errors);
}

#[test]
fn test_config_validation_invalid_vcs_type() {
    let mut config = Config::new();
    config.set("vcs.type", "invalid");

    let errors = ConfigManager::validate(&config);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("VCS")));
}

#[test]
fn test_config_validation_valid_log_level() {
    let mut config = Config::new();
    config.set("logging.level", "debug");

    let errors = ConfigManager::validate(&config);
    assert!(errors.is_empty(), "debug should be valid: {:?}", errors);
}

#[test]
fn test_config_validation_all_log_levels() {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];

    for level in valid_levels {
        let mut config = Config::new();
        config.set("logging.level", level);

        let errors = ConfigManager::validate(&config);
        assert!(errors.is_empty(), "{} should be valid: {:?}", level, errors);
    }
}

#[test]
fn test_config_validation_invalid_log_level() {
    let mut config = Config::new();
    config.set("logging.level", "verbose");

    let errors = ConfigManager::validate(&config);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("log")));
}

#[test]
fn test_config_multiple_errors() {
    let mut config = Config::new();
    config.set("vcs.type", "svn");
    config.set("logging.level", " TRACE ");

    let errors = ConfigManager::validate(&config);
    assert!(errors.len() >= 2);
}

#[test]
fn test_config_scope_display() {
    assert_eq!(ConfigScope::Global.to_string(), "Global");
    assert_eq!(ConfigScope::Project.to_string(), "Project");
    assert_eq!(ConfigScope::Env.to_string(), "Env");
}

#[test]
fn test_config_value_creation() {
    use crate::config::ConfigValue;

    let cv = ConfigValue::new("key", "value", ConfigScope::Global);
    assert_eq!(cv.key, "key");
    assert_eq!(cv.value, "value");
    assert_eq!(cv.scope, ConfigScope::Global);
}

#[test]
fn test_config_with_source() {
    use crate::config::ConfigValue;

    let cv = ConfigValue::with_source("key", "value", ConfigScope::Project, "/path/to/config");
    assert_eq!(cv.source.to_str(), Some("/path/to/config"));
}

#[test]
fn test_config_iteration() {
    let mut config = Config::new();
    config.set("key1", "value1");
    config.set("key2", "value2");

    let keys: Vec<_> = config.keys().collect();
    assert_eq!(keys.len(), 2);

    let pairs: Vec<_> = config.iter().collect();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn test_config_contains_key() {
    let mut config = Config::new();
    config.set("present", "value");

    assert!(config.contains_key("present"));
    assert!(!config.contains_key("absent"));
}

#[test]
fn test_config_source_priority() {
    let mut config = Config::new();
    config.add_source(std::path::PathBuf::from("/low"), ConfigScope::Global, 1);
    config.add_source(std::path::PathBuf::from("/high"), ConfigScope::Project, 3);
    config.add_source(std::path::PathBuf::from("/medium"), ConfigScope::Env, 2);

    let sources = config.sources();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0].priority, 3);
    assert_eq!(sources[1].priority, 2);
    assert_eq!(sources[2].priority, 1);
}

#[test]
fn test_config_empty_validation() {
    let config = Config::new();
    let errors = ConfigManager::validate(&config);
    assert!(errors.is_empty(), "Empty config should have no errors");
}
