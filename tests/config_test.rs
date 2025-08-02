// Configuration management tests for controlrake
// Tests TOML configuration loading, validation, and default handling

use controlrake::config::{Config, SystemConfig, HttpConfig, UpdateConfig};
use serde_json::json;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_default_config_creation() {
    let config = Config::default();

    // Verify system defaults
    assert_eq!(config.system.shutdown_timeout, 30);
    assert_eq!(config.system.log_level, "info");
    assert_eq!(config.system.test_delay_ms, 10);

    // Verify HTTP defaults
    assert_eq!(config.http.port, 8888);
    assert_eq!(config.http.bind_address, "0.0.0.0");
    assert_eq!(config.http.static_folder, "./static");
    assert_eq!(config.http.base_url, "http://localhost");
    assert!(!config.http.auth_key.is_empty());

    // Verify Twitch is None by default
    assert!(config.twitch.is_none());

    // Verify update defaults
    assert_eq!(config.update.max_text_length, 1000);
}

#[test]
fn test_config_deserialization() {
    let toml_content = r#"
        [system]
        shutdown_timeout = 60
        log_level = "debug"
        test_delay_ms = 20

        [http]
        port = 9090
        auth_key = "my-custom-key"
        static_folder = "./custom_static"
        bind_address = "192.168.1.100"
        base_url = "https://example.com"

        [update]
        max_text_length = 2000
    "#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(config.system.shutdown_timeout, 60);
    assert_eq!(config.system.log_level, "debug");
    assert_eq!(config.system.test_delay_ms, 20);

    assert_eq!(config.http.port, 9090);
    assert_eq!(config.http.auth_key, "my-custom-key");
    assert_eq!(config.http.static_folder, "./custom_static");
    assert_eq!(config.http.bind_address, "192.168.1.100");
    assert_eq!(config.http.base_url, "https://example.com");

    assert_eq!(config.update.max_text_length, 2000);
}

#[test]
fn test_partial_config_with_defaults() {
    let toml_content = r#"
        [system]
        log_level = "error"

        [http]
        port = 7777
    "#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse partial TOML");

    // Verify provided values
    assert_eq!(config.system.log_level, "error");
    assert_eq!(config.http.port, 7777);

    // Verify defaults are used for missing values
    assert_eq!(config.system.shutdown_timeout, 30); // default
    assert_eq!(config.system.test_delay_ms, 10); // default
    assert_eq!(config.http.bind_address, "0.0.0.0"); // default
    assert_eq!(config.update.max_text_length, 1000); // default
}

#[test]
fn test_update_config_structure() {
    let update_config = UpdateConfig {
        max_text_length: 500,
    };

    assert_eq!(update_config.max_text_length, 500);
}

#[test]
fn test_update_config_defaults() {
    let update_config = UpdateConfig::default();
    assert_eq!(update_config.max_text_length, 1000);
}

#[test]
fn test_config_serialization() {
    let config = Config::default();

    // Test that config can be serialized back to JSON
    let config_json = serde_json::to_value(&config).expect("Should serialize to JSON");

    // Verify structure
    assert!(config_json["system"].is_object());
    assert!(config_json["http"].is_object());
    assert!(config_json["update"].is_object());

    // Verify some key values
    assert_eq!(config_json["system"]["shutdown_timeout"], 30);
    assert_eq!(config_json["http"]["port"], 8888);
    assert_eq!(config_json["update"]["max_text_length"], 1000);
}

#[test]
fn test_config_from_file() {
    let config_content = r#"
        [system]
        shutdown_timeout = 45
        log_level = "warn"
        test_delay_ms = 5

        [http]
        port = 8080
        auth_key = "file-test-key"
        static_folder = "./test_static"

        [update]
        max_text_length = 1500
    "#;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", config_content).expect("Failed to write to temp file");

    // Parse the configuration
    let file_content = std::fs::read_to_string(temp_file.path()).expect("Failed to read temp file");
    let config: Config = toml::from_str(&file_content).expect("Failed to parse config from file");

    assert_eq!(config.system.shutdown_timeout, 45);
    assert_eq!(config.system.log_level, "warn");
    assert_eq!(config.system.test_delay_ms, 5);
    assert_eq!(config.http.port, 8080);
    assert_eq!(config.http.auth_key, "file-test-key");
    assert_eq!(config.update.max_text_length, 1500);
}

#[test]
fn test_extreme_config_values() {
    let extreme_config = Config {
        system: SystemConfig {
            shutdown_timeout: 1,
            log_level: "trace".to_string(),
            test_delay_ms: 1,
        },
        http: HttpConfig {
            port: 65535,
            auth_key: "x".repeat(100),
            static_folder: "/tmp/extreme".to_string(),
            bind_address: "127.0.0.1".to_string(),
            base_url: "https://extreme.example.com".to_string(),
        },
        twitch: None,
        update: UpdateConfig {
            max_text_length: 1, // Very small limit
        },
    };

    assert_eq!(extreme_config.system.shutdown_timeout, 1);
    assert_eq!(extreme_config.update.max_text_length, 1);
}

#[test]
fn test_config_serialization_roundtrip() {
    let original_config = Config::default();

    // Serialize to JSON
    let json_str = serde_json::to_string(&original_config).expect("Should serialize");

    // Deserialize back
    let deserialized: Config = serde_json::from_str(&json_str).expect("Should deserialize");

    // Verify key fields match
    assert_eq!(original_config.system.shutdown_timeout, deserialized.system.shutdown_timeout);
    assert_eq!(original_config.http.port, deserialized.http.port);
    assert_eq!(original_config.update.max_text_length, deserialized.update.max_text_length);
}
