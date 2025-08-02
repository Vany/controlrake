use crate::component::twitch::types::{
    default_irc_port, default_irc_server, default_max_message_length, default_max_stored_messages,
    default_oauth_timeout_secs, default_reconnect_delays, TwitchConfig,
};
use serde::{Deserialize, Serialize};

// 📊 Configuration constants - centralized defaults to eliminate magic numbers
/// Default HTTP server port
pub const DEFAULT_HTTP_PORT: u16 = 8888;
/// Default system shutdown timeout in seconds
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;
/// Default test delay in milliseconds
pub const DEFAULT_TEST_DELAY_MS: u64 = 10;
/// Default maximum text length for content validation
pub const DEFAULT_MAX_TEXT_LENGTH: usize = 1000;
/// Default authentication key for testing
pub const DEFAULT_AUTH_KEY: &str = "test-key";
/// Default static file serving directory
pub const DEFAULT_STATIC_FOLDER: &str = "./static";
/// Default base URL for HTTP server
pub const DEFAULT_BASE_URL: &str = "http://localhost";

/// ⚙️ Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub system: SystemConfig,
    pub http: HttpConfig,
    #[serde(default)]
    pub twitch: Option<TwitchConfig>,
    #[serde(default)]
    pub update: UpdateConfig,
}

impl Default for Config {
    fn default() -> Self {
        test_config()
    }
}

/// 🎛️ System configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemConfig {
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: u64, // seconds, default 30

    #[serde(default = "default_log_level")]
    pub log_level: String, // default "info"

    #[serde(default = "default_test_delay_ms")]
    pub test_delay_ms: u64, // milliseconds, default 10
}

/// 🌐 HTTP configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpConfig {
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default = "default_auth_key")]
    pub auth_key: String,
    #[serde(default = "default_static_folder")]
    pub static_folder: String,

    #[serde(default = "default_bind_address")]
    pub bind_address: String, // default "0.0.0.0"

    #[serde(default = "default_base_url")]
    pub base_url: String, // default "http://localhost"
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: default_http_port(),
            auth_key: default_auth_key(),
            static_folder: default_static_folder(),
            bind_address: default_bind_address(),
            base_url: default_base_url(),
        }
    }
}

/// 🔄 Update system configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateConfig {
    #[serde(default = "default_max_text_length")]
    pub max_text_length: usize, // default 1000
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            max_text_length: default_max_text_length(),
        }
    }
}

/// 📁 Load configuration with environment overrides
pub fn load_config() -> anyhow::Result<Config> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config")) // Look for config.{toml,json,yaml,etc}
        .add_source(config::Environment::with_prefix("CONTROLRAKE"));

    // Try to build config
    let settings = builder.build()?;

    // Deserialize into our Config struct
    let config: Config = settings.try_deserialize()?;

    // Validate configuration
    validate_config(&config)?;

    Ok(config)
}

/// ✅ Validate configuration values
fn validate_config(config: &Config) -> anyhow::Result<()> {
    // Validate HTTP configuration
    if config.http.port == 0 {
        anyhow::bail!("HTTP port must be greater than 0");
    }

    if config.http.auth_key.is_empty() {
        anyhow::bail!("HTTP auth_key cannot be empty");
    }

    if config.http.auth_key.len() < 8 {
        anyhow::bail!("HTTP auth_key must be at least 8 characters long");
    }

    // Validate static folder exists
    if !std::path::Path::new(&config.http.static_folder).exists() {
        anyhow::bail!(
            "Static folder does not exist: {}",
            config.http.static_folder
        );
    }

    // Validate system configuration
    if config.system.shutdown_timeout == 0 {
        anyhow::bail!("System shutdown_timeout must be greater than 0");
    }

    let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_log_levels.contains(&config.system.log_level.as_str()) {
        anyhow::bail!(
            "Invalid log level '{}'. Must be one of: {}",
            config.system.log_level,
            valid_log_levels.join(", ")
        );
    }

    Ok(())
}

/// 📝 Generate example configuration file
pub fn generate_example_config() -> Config {
    Config {
        system: SystemConfig {
            shutdown_timeout: default_shutdown_timeout(),
            log_level: default_log_level(),
            test_delay_ms: default_test_delay_ms(),
        },
        http: HttpConfig {
            port: DEFAULT_HTTP_PORT,
            auth_key: "your-secret-key-here".to_string(),
            static_folder: DEFAULT_STATIC_FOLDER.to_string(),
            bind_address: default_bind_address(),
            base_url: default_base_url(),
        },
        twitch: Some(TwitchConfig {
            username: "your_twitch_username".to_string(),
            channel: "target_channel".to_string(),
            client_id: None,
            client_secret: None,
            oauth_scopes: None,
            oauth_timeout_secs: default_oauth_timeout_secs(),
            max_message_length: default_max_message_length(),
            max_stored_messages: default_max_stored_messages(),
            reconnect_delays: default_reconnect_delays(),
            irc_server: default_irc_server(),
            irc_port: default_irc_port(),
            max_messages_per_minute: 20,
            token_validation_interval_secs: 3600,
            enable_user_validation: true,
            enable_metrics: true,
            enable_auto_reconnect: true,
            max_reconnect_attempts: 10,
            enable_enhanced_parsing: true,
            message_queue_size: 1000,
            enable_rate_limiting: true,
        }),
        update: UpdateConfig {
            max_text_length: default_max_text_length(),
        },
    }
}

/// 💾 Save configuration to file
pub fn save_config(config: &Config, path: &str) -> anyhow::Result<()> {
    let toml_str = toml::to_string_pretty(config)?;
    std::fs::write(path, toml_str)?;
    Ok(())
}

// Default value functions for serde
fn default_shutdown_timeout() -> u64 {
    DEFAULT_SHUTDOWN_TIMEOUT_SECS
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_test_delay_ms() -> u64 {
    DEFAULT_TEST_DELAY_MS
}
fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}
fn default_base_url() -> String {
    "http://localhost".to_string()
}
fn default_max_text_length() -> usize {
    DEFAULT_MAX_TEXT_LENGTH
}

/// Default HTTP port
fn default_http_port() -> u16 {
    DEFAULT_HTTP_PORT
}

/// Default authentication key
fn default_auth_key() -> String {
    DEFAULT_AUTH_KEY.to_string()
}

/// Default static folder path
fn default_static_folder() -> String {
    DEFAULT_STATIC_FOLDER.to_string()
}

/// 🧪 Get test delay duration from configuration (with fallback for tests)
pub fn test_delay() -> std::time::Duration {
    std::time::Duration::from_millis(
        load_config().map(|c| c.system.test_delay_ms).unwrap_or(10), // Fallback to 10ms if config loading fails
    )
}

/// 🧪 Helper function for tests - create minimal config with required fields
pub fn test_config() -> Config {
    Config {
        system: SystemConfig {
            shutdown_timeout: 30,
            log_level: "info".to_string(),
            test_delay_ms: 10,
        },
        http: HttpConfig {
            port: 8888,
            auth_key: "test-key".to_string(),
            static_folder: "./static".to_string(),
            bind_address: "0.0.0.0".to_string(),
            base_url: "http://localhost".to_string(),
        },
        twitch: None,
        update: UpdateConfig {
            max_text_length: 1000,
        },
    }
}
