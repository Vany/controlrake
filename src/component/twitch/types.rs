use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 🎮 Enhanced Twitch events with comprehensive metadata support
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TwitchEvent {
    /// 💬 Enhanced chat message with rich user metadata
    ChatMessage {
        user: String,
        message: String,
        color: Option<String>,
        badges: Vec<String>,
        timestamp: String,
        /// 🎭 True if this is a "/me" action message
        is_action: bool,
        /// 🆔 Unique message identifier from Twitch
        message_id: Option<String>,
        /// 👤 Display name (may differ from username)
        display_name: Option<String>,
        /// 🔢 Twitch user ID
        user_id: Option<String>,
        /// ⭐ Subscriber status
        is_subscriber: bool,
        /// 🛡️ Moderator status
        is_moderator: bool,
        /// 💎 VIP status
        is_vip: bool,
        /// 🆕 First-time chatter
        is_first_message: bool,
    },

    /// 📥 Enhanced user joined with display name
    Join { 
        user: String, 
        timestamp: String,
        /// 👤 Display name if available
        display_name: Option<String>,
    },

    /// 📤 Enhanced user left with display name
    Part { 
        user: String, 
        timestamp: String,
        /// 👤 Display name if available
        display_name: Option<String>,
    },

    /// 🏴‍☠️ Enhanced raid event with comprehensive data
    Raid {
        from_channel: String,
        viewer_count: u32,
        timestamp: String,
        /// 👤 Display name of raiding channel
        display_name: Option<String>,
    },

    /// ⭐ Enhanced subscription event with tier and streak info
    Subscription {
        user: String,
        months: u32,
        message: Option<String>,
        timestamp: String,
        /// 🎁 Gift subscription details
        is_gift: bool,
        gifted_by: Option<String>,
        /// 💰 Subscription tier (1000, 2000, 3000)
        tier: Option<String>,
        /// 🔥 Consecutive months streak
        streak_months: Option<u32>,
    },

    /// 💎 Bits/cheer donation
    Bits {
        user: String,
        amount: u32,
        message: Option<String>,
        timestamp: String,
    },

    /// 🏠 Enhanced host event with auto-host detection
    Host {
        from_channel: String,
        viewer_count: Option<u32>,
        timestamp: String,
        /// 🎯 Target channel being hosted
        target_channel: String,
        /// 👤 Display name if available
        display_name: Option<String>,
        /// 🤖 Whether this is an auto-host
        is_auto_host: bool,
    },

    /// 👤 User state information from Twitch IRC USERSTATE command
    UserState {
        /// 👤 Display name of the authenticated user
        display_name: Option<String>,
        /// 🎨 User's chat color
        color: Option<String>,
        /// 🎖️ User badges (subscriber, moderator, etc.)
        badges: Vec<String>,
        /// 🆔 Twitch user ID
        user_id: Option<String>,
        /// ⭐ Subscriber status
        is_subscriber: bool,
        /// 🛡️ Moderator status
        is_moderator: bool,
        /// 💎 VIP status
        is_vip: bool,
        /// 😀 Available emote sets
        emote_sets: Vec<String>,
        /// 🕒 Timestamp when state was received
        timestamp: String,
        /// 📺 Channel this state applies to
        channel: String,
    },

    /// 🔗 Enhanced connection status with quality metrics
    /// 🔗 Enhanced connection status with quality metrics
    ConnectionStatus {
        connected: bool,
        reason: Option<String>,
        timestamp: String,
        /// 📊 Connection quality indicator
        quality: Option<String>,
        /// ⏱️ Connection duration
        duration: Option<String>,
    },
}

/// 🔐 Enhanced OAuth token storage with validation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchAuth {
    pub token: String,
    pub username: String,
    pub obtained_at: String,
    /// 🕒 Token expiration (if available from OAuth response)
    pub expires_at: Option<String>,
    /// 🔑 Refresh token (for OAuth2 support)
    pub refresh_token: Option<String>,
    /// 🎯 OAuth scopes granted
    pub scopes: Vec<String>,
    /// 🆔 Twitch user ID
    pub user_id: Option<String>,
    /// 📊 Number of times token has been validated
    pub validation_count: u32,
    /// ⏰ Last validation timestamp
    pub last_validated: Option<String>,
}

/// ⚙️ Comprehensive Twitch component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchConfig {
    /// 👤 Twitch username to authenticate as
    pub username: String,
    /// 📺 Channel to join (without #)
    pub channel: String,
    /// 🔑 Twitch application client ID (optional, uses default if not provided)
    pub client_id: Option<String>,
    /// 🔐 Twitch application client secret (required for OAuth token exchange)
    pub client_secret: Option<String>,
    /// 🎯 OAuth scopes to request (defaults to "chat:read chat:edit")
    pub oauth_scopes: Option<String>,

    // 🆕 Enhanced timing and limits
    #[serde(default = "default_oauth_timeout_secs")]
    pub oauth_timeout_secs: u64, // default 300 (5 minutes)

    #[serde(default = "default_max_message_length")]
    pub max_message_length: usize, // default 500

    #[serde(default = "default_max_stored_messages")]
    pub max_stored_messages: usize, // default 100

    #[serde(default = "default_reconnect_delays")]
    pub reconnect_delays: Vec<u64>, // default [1, 2, 4, 8, 16, 30, 60]

    #[serde(default = "default_irc_server")]
    pub irc_server: String, // default "irc.chat.twitch.tv"

    #[serde(default = "default_irc_port")]
    pub irc_port: u16, // default 6697

    // 🚀 New enhanced configuration options
    #[serde(default = "default_max_messages_per_minute")]
    pub max_messages_per_minute: u32, // default 20

    #[serde(default = "default_token_validation_interval_secs")]
    pub token_validation_interval_secs: u64, // default 3600 (1 hour)

    #[serde(default = "default_enable_user_validation")]
    pub enable_user_validation: bool, // default true

    #[serde(default = "default_enable_metrics")]
    pub enable_metrics: bool, // default true

    #[serde(default = "default_enable_auto_reconnect")]
    pub enable_auto_reconnect: bool, // default true

    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: usize, // default 10

    #[serde(default = "default_enable_enhanced_parsing")]
    pub enable_enhanced_parsing: bool, // default true

    #[serde(default = "default_message_queue_size")]
    pub message_queue_size: usize, // default 1000

    #[serde(default = "default_enable_rate_limiting")]
    pub enable_rate_limiting: bool, // default true
}

impl Default for TwitchConfig {
    fn default() -> Self {
        Self {
            username: "default_user".to_string(),
            channel: "default_channel".to_string(),
            client_id: None,
            client_secret: None,
            oauth_scopes: Some("chat:read chat:edit".to_string()),
            oauth_timeout_secs: default_oauth_timeout_secs(),
            max_message_length: default_max_message_length(),
            max_stored_messages: default_max_stored_messages(),
            reconnect_delays: default_reconnect_delays(),
            irc_server: default_irc_server(),
            irc_port: default_irc_port(),
            max_messages_per_minute: default_max_messages_per_minute(),
            token_validation_interval_secs: default_token_validation_interval_secs(),
            enable_user_validation: default_enable_user_validation(),
            enable_metrics: default_enable_metrics(),
            enable_auto_reconnect: default_enable_auto_reconnect(),
            max_reconnect_attempts: default_max_reconnect_attempts(),
            enable_enhanced_parsing: default_enable_enhanced_parsing(),
            message_queue_size: default_message_queue_size(),
            enable_rate_limiting: default_enable_rate_limiting(),
        }
    }
}

/// 📨 Enhanced IRC message with comprehensive metadata
#[derive(Debug, Clone)]
pub struct IrcMessage {
    pub tags: HashMap<String, String>,
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<String>,
    pub raw: String,
    /// ⏰ When the message was received
    pub received_at: std::time::SystemTime,
    /// 📏 Message size in bytes
    pub size: usize,
}

/// 📊 Enhanced component status with comprehensive monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TwitchStatus {
    pub connected: bool,
    pub channel: String,
    pub username: String,
    pub authenticated: bool,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub last_activity: Option<String>,
    pub connection_uptime: Option<String>,
    /// 🚨 Current error state (if any)
    pub error_state: Option<TwitchErrorState>,
    /// ⏰ Last OAuth validation timestamp
    pub oauth_validated_at: Option<String>,
    /// 📨 Last message timestamp
    pub last_message_at: Option<String>,
    /// 🚦 Remaining rate limit capacity
    pub rate_limit_remaining: u32,
    /// 📊 Connection quality score (0-100)
    pub connection_quality: u8,
    /// 🎯 Negotiated IRC capabilities
    pub irc_capabilities: Vec<String>,
    /// ⚡ Average message processing time
    pub avg_processing_time_ms: f64,
    /// 🔄 Number of reconnection attempts
    pub reconnection_attempts: u32,
    /// 💾 Memory usage in bytes
    pub memory_usage_bytes: u64,
}

/// 🚨 Enhanced error state management with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchErrorState {
    /// 🔍 Error type determines handling strategy
    pub error_type: TwitchErrorType,
    /// 📝 Human-readable error message for UI display
    pub message: String,
    /// 🕒 When the error first occurred
    pub first_occurred: String,
    /// 🔄 How many retry attempts have been made (for temporary errors)
    pub retry_count: u32,
    /// ⏰ When the next retry attempt will happen (for temporary errors)
    pub next_retry_at: Option<String>,
    /// 📊 Additional context for debugging
    pub details: Option<String>,
    /// 🚨 Error severity level
    pub severity: String,
    /// 💡 Suggested action for user
    pub suggested_action: Option<String>,
    /// 🔗 Help documentation URL
    pub help_url: Option<String>,
}

/// 🎯 Enhanced error classification for intelligent handling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TwitchErrorType {
    /// 🔐 OAuth/Authentication errors - require user intervention
    #[serde(rename = "permanent_auth")]
    PermanentAuth,
    /// ⚙️ Configuration errors - require config fix
    #[serde(rename = "permanent_config")]
    PermanentConfig,
    /// 🌐 Network connectivity issues - can auto-retry
    #[serde(rename = "temporary_network")]
    TemporaryNetwork,
    /// 🔌 IRC protocol errors - can auto-retry
    #[serde(rename = "temporary_irc")]
    TemporaryIrc,
    /// 💾 Storage system errors - can auto-retry
    #[serde(rename = "temporary_storage")]
    TemporaryStorage,
    /// 🚦 Rate limiting errors - can auto-retry with backoff
    #[serde(rename = "temporary_rate_limit")]
    TemporaryRateLimit,
    /// 🌐 API errors from Twitch - limited retry
    #[serde(rename = "api_error")]
    ApiError,
}

impl fmt::Display for TwitchErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwitchErrorType::PermanentAuth => write!(f, "Authentication Error"),
            TwitchErrorType::PermanentConfig => write!(f, "Configuration Error"),
            TwitchErrorType::TemporaryNetwork => write!(f, "Network Error"),
            TwitchErrorType::TemporaryIrc => write!(f, "IRC Error"),
            TwitchErrorType::TemporaryStorage => write!(f, "Storage Error"),
            TwitchErrorType::TemporaryRateLimit => write!(f, "Rate Limit Error"),
            TwitchErrorType::ApiError => write!(f, "API Error"),
        }
    }
}

impl TwitchErrorState {
    /// ✨ Create a new enhanced error state with comprehensive information
    pub fn new(error_type: TwitchErrorType, message: String, details: Option<String>) -> Self {
        let (severity, suggested_action, help_url) = match error_type {
            TwitchErrorType::PermanentAuth => (
                "error".to_string(),
                Some("Re-authenticate with Twitch by clicking the 'Retry Connection' button".to_string()),
                Some("https://dev.twitch.tv/docs/authentication/".to_string()),
            ),
            TwitchErrorType::PermanentConfig => (
                "error".to_string(),
                Some("Check your config.toml file and ensure client_id and client_secret are set".to_string()),
                Some("https://dev.twitch.tv/console/apps".to_string()),
            ),
            TwitchErrorType::TemporaryNetwork => (
                "warn".to_string(),
                Some("Check your internet connection. Automatic retry in progress".to_string()),
                None,
            ),
            TwitchErrorType::TemporaryIrc => (
                "warn".to_string(),
                Some("IRC connection issue. Automatic retry in progress".to_string()),
                None,
            ),
            TwitchErrorType::TemporaryStorage => (
                "warn".to_string(),
                Some("Storage system issue. Automatic retry in progress".to_string()),
                None,
            ),
            TwitchErrorType::TemporaryRateLimit => (
                "info".to_string(),
                Some("Rate limit reached. Please wait before sending more messages".to_string()),
                None,
            ),
            TwitchErrorType::ApiError => (
                "warn".to_string(),
                Some("Twitch API issue. Limited automatic retry in progress".to_string()),
                Some("https://dev.twitch.tv/docs/api/".to_string()),
            ),
        };

        Self {
            error_type,
            message,
            first_occurred: TwitchEvent::now_timestamp(),
            retry_count: 0,
            next_retry_at: None,
            details,
            severity,
            suggested_action,
            help_url,
        }
    }

    /// 🔄 Increment retry count and set next retry time
    pub fn increment_retry(&mut self, retry_delay_secs: u64) {
        self.retry_count += 1;
        let next_retry = chrono::Utc::now() + chrono::Duration::seconds(retry_delay_secs as i64);
        self.next_retry_at = Some(next_retry.to_rfc3339());
    }

    /// ❓ Check if this is a permanent error requiring user intervention
    pub fn is_permanent(&self) -> bool {
        matches!(
            self.error_type,
            TwitchErrorType::PermanentAuth | TwitchErrorType::PermanentConfig
        )
    }

    /// 🔄 Check if this error should be automatically retried
    pub fn should_auto_retry(&self) -> bool {
        !self.is_permanent() && self.retry_count < 10 // Max 10 retries
    }

    /// ⏰ Check if enough time has passed for the next retry attempt
    pub fn can_retry_now(&self) -> bool {
        if let Some(next_retry_str) = &self.next_retry_at {
            if let Ok(next_retry) = chrono::DateTime::parse_from_rfc3339(next_retry_str) {
                return chrono::Utc::now() >= next_retry.with_timezone(&chrono::Utc);
            }
        }
        true // If no retry time set, can retry immediately
    }

    /// 👤 Get user-friendly error description
    pub fn get_user_description(&self) -> String {
        match self.error_type {
            TwitchErrorType::PermanentAuth => {
                "Your Twitch authentication has expired or been revoked. Please re-authenticate to continue using chat features.".to_string()
            }
            TwitchErrorType::PermanentConfig => {
                "The Twitch component configuration is incomplete or invalid. Please check your settings and try again.".to_string()
            }
            TwitchErrorType::TemporaryNetwork => {
                "There's a temporary network connectivity issue. The system will automatically retry the connection.".to_string()
            }
            TwitchErrorType::TemporaryIrc => {
                "The IRC connection to Twitch encountered an issue. The system will automatically attempt to reconnect.".to_string()
            }
            TwitchErrorType::TemporaryStorage => {
                "The storage system is temporarily unavailable. The system will automatically retry the operation.".to_string()
            }
            TwitchErrorType::TemporaryRateLimit => {
                "You're sending messages too quickly. Please wait a moment before sending another message.".to_string()
            }
            TwitchErrorType::ApiError => {
                "The Twitch API is temporarily unavailable. The system will automatically retry with limited attempts.".to_string()
            }
        }
    }
}

/// 🎯 Methods supported by the Twitch component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum TwitchMethod {
    /// 🔐 Handle OAuth callback from browser
    HandleOauth {
        state: String,
        token: Option<String>,
        error: Option<String>,
    },

    /// 📤 Send chat message to the channel
    SendMessage { text: String },

    /// 📊 Get current component status
    GetStatus,

    /// 🔄 Reconnect IRC connection
    Reconnect,

    /// 🚪 Leave channel and disconnect
    Disconnect,
}

impl TwitchEvent {
    /// 🕒 Create timestamp string for current time
    pub fn now_timestamp() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    /// 🎨 Get display color for the event type
    pub fn display_color(&self) -> &'static str {
        match self {
            TwitchEvent::ChatMessage { .. } => "#FFFFFF",
            TwitchEvent::Join { .. } => "#00FF00",
            TwitchEvent::Part { .. } => "#FF6B6B",
            TwitchEvent::Raid { .. } => "#9146FF",
            TwitchEvent::Subscription { .. } => "#FFD700",
            TwitchEvent::Bits { .. } => "#00E5FF",
            TwitchEvent::Host { .. } => "#FF6600",
            TwitchEvent::UserState { .. } => "#00BFFF",
            TwitchEvent::ConnectionStatus { .. } => "#808080",
        }
    }

    /// 📝 Get user-friendly display text
    pub fn display_text(&self) -> String {
        match self {
            TwitchEvent::ChatMessage {
                user,
                message,
                is_action,
                display_name,
                ..
            } => {
                let username = display_name.as_ref().unwrap_or(user);
                if *is_action {
                    format!("* {username} {message}")
                } else {
                    format!("<{username}> {message}")
                }
            }
            TwitchEvent::Join { user, display_name, .. } => {
                let username = display_name.as_ref().unwrap_or(user);
                format!("→ {username} joined")
            }
            TwitchEvent::Part { user, display_name, .. } => {
                let username = display_name.as_ref().unwrap_or(user);
                format!("← {username} left")
            }
            TwitchEvent::Raid {
                from_channel,
                viewer_count,
                display_name,
                ..
            } => {
                let channel_name = display_name.as_ref().unwrap_or(from_channel);
                format!("🏴‍☠️ {channel_name} raided with {viewer_count} viewers!")
            }
            TwitchEvent::Subscription {
                user,
                months,
                is_gift,
                gifted_by,
                tier,
                streak_months,
                ..
            } => {
                let tier_text = tier.as_ref().map(|t| match t.as_str() {
                    "1000" => " (Tier 1)",
                    "2000" => " (Tier 2)", 
                    "3000" => " (Tier 3)",
                    _ => "",
                }).unwrap_or("");
                
                let streak_text = streak_months.map(|s| format!(" ({s} month streak)")).unwrap_or_default();
                
                if *is_gift {
                    if let Some(gifter) = gifted_by {
                        format!("⭐ {gifter} gifted a subscription{tier_text} to {user}!")
                    } else {
                        format!("⭐ Anonymous gift subscription{tier_text} to {user}!")
                    }
                } else {
                    format!("⭐ {user} subscribed{tier_text} for {months} months!{streak_text}")
                }
            }
            TwitchEvent::Bits { user, amount, .. } => format!("💎 {user} cheered {amount} bits!"),
            TwitchEvent::Host {
                target_channel,
                viewer_count,
                display_name,
                is_auto_host,
                ..
            } => {
                let channel_name = display_name.as_ref().unwrap_or(target_channel);
                let auto_text = if *is_auto_host { " (auto)" } else { "" };
                
                if let Some(count) = viewer_count {
                    format!("🏠 Now hosting {channel_name} with {count} viewers{auto_text}!")
                } else {
                    format!("🏠 Now hosting {channel_name}{auto_text}!")
                }
            }
            TwitchEvent::ConnectionStatus {
                connected, reason, quality, ..
            } => {
                let quality_text = quality.as_ref().map(|q| format!(" ({q})")).unwrap_or_default();
                
                if *connected {
                    format!("🔗 Connected to Twitch IRC{quality_text}")
                } else if let Some(reason) = reason {
                    format!("❌ Disconnected: {reason}")
                } else {
                    "❌ Disconnected from Twitch IRC".to_string()
                }
            }
            TwitchEvent::UserState {
                display_name,
                color,
                badges,
                is_subscriber,
                is_moderator,
                is_vip,
                channel,
                ..
            } => {
                let username = display_name.as_ref().map(|s| s.as_str()).unwrap_or("Unknown");
                let color_text = color.as_ref().map(|c| format!(" ({c})")).unwrap_or_default();
                let badges_text = if !badges.is_empty() {
                    format!(" [{}]", badges.join(", "))
                } else {
                    String::new()
                };
                let status_parts = [
                    if *is_moderator { Some("MOD") } else { None },
                    if *is_vip { Some("VIP") } else { None },
                    if *is_subscriber { Some("SUB") } else { None },
                ].into_iter().flatten().collect::<Vec<_>>();
                let status_text = if !status_parts.is_empty() {
                    format!(" ({})", status_parts.join("/"))
                } else {
                    String::new()
                };
                
                format!("👤 User state updated: {username}{color_text}{badges_text}{status_text} in #{channel}")
            }
        }
    }

    /// 🔢 Get event priority for UI sorting and filtering
    pub fn get_priority(&self) -> u8 {
        match self {
            TwitchEvent::ConnectionStatus { connected: false, .. } => 100, // Critical errors
            TwitchEvent::Subscription { .. } => 90, // High priority events
            TwitchEvent::Raid { .. } => 85,
            TwitchEvent::Bits { .. } => 80,
            TwitchEvent::Host { .. } => 75,
            TwitchEvent::ConnectionStatus { connected: true, .. } => 70, // Connection success
            TwitchEvent::ChatMessage { is_moderator: true, .. } => 60, // Moderator messages
            TwitchEvent::ChatMessage { is_vip: true, .. } => 55, // VIP messages  
            TwitchEvent::ChatMessage { is_subscriber: true, .. } => 50, // Subscriber messages
            TwitchEvent::ChatMessage { .. } => 40, // Regular chat
            TwitchEvent::UserState { .. } => 25, // User state updates (informational)
            TwitchEvent::Join { .. } => 20, // Lower priority
            TwitchEvent::Part { .. } => 10, // Lowest priority
        }
    }
}

/// 🛡️ Constants for Twitch API and IRC
pub mod constants {
    /// 🌐 Twitch OAuth authorization URL
    pub const OAUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";

    /// 🔑 Twitch OAuth token exchange URL
    pub const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";

    /// 🔄 Twitch OAuth token refresh URL
    pub const REFRESH_URL: &str = "https://id.twitch.tv/oauth2/token";

    /// ✅ Twitch OAuth token validation URL
    pub const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";

    /// 📡 Twitch IRC server address
    pub const IRC_SERVER: &str = "irc.chat.twitch.tv";

    /// 🔌 Twitch IRC server port (TLS)
    pub const IRC_PORT: u16 = 6697;

    /// ⏱️ OAuth timeout in seconds
    pub const OAUTH_TIMEOUT_SECS: u64 = 300; // 5 minutes

    /// 🔄 IRC reconnection delays (exponential backoff)
    pub const RECONNECT_DELAYS: &[u64] = &[1, 2, 4, 8, 16, 30, 60];

    /// 💬 Maximum chat message length
    pub const MAX_MESSAGE_LENGTH: usize = 500;

    /// 📚 Maximum stored chat messages (for web UI)
    pub const MAX_STORED_MESSAGES: usize = 100;
}

// 🔧 Enhanced default value functions for TwitchConfig serde
pub fn default_oauth_timeout_secs() -> u64 {
    300 // 5 minutes
}

pub fn default_max_message_length() -> usize {
    500
}

pub fn default_max_stored_messages() -> usize {
    100
}

pub fn default_reconnect_delays() -> Vec<u64> {
    vec![1, 2, 4, 8, 16, 30, 60]
}

pub fn default_irc_server() -> String {
    "irc.chat.twitch.tv".to_string()
}

pub fn default_irc_port() -> u16 {
    6697
}

pub fn default_max_messages_per_minute() -> u32 {
    20
}

pub fn default_token_validation_interval_secs() -> u64 {
    3600 // 1 hour
}

pub fn default_enable_user_validation() -> bool {
    true
}

pub fn default_enable_metrics() -> bool {
    true
}

pub fn default_enable_auto_reconnect() -> bool {
    true
}

pub fn default_max_reconnect_attempts() -> usize {
    10
}

pub fn default_enable_enhanced_parsing() -> bool {
    true
}

pub fn default_message_queue_size() -> usize {
    1000
}

pub fn default_enable_rate_limiting() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_twitch_event_display() {
        let chat_msg = TwitchEvent::ChatMessage {
            user: "testuser".to_string(),
            message: "Hello world!".to_string(),
            color: Some("#FF0000".to_string()),
            badges: vec!["subscriber".to_string()],
            timestamp: TwitchEvent::now_timestamp(),
            is_action: false,
            message_id: Some("msg-123".to_string()),
            display_name: Some("TestUser".to_string()),
            user_id: Some("12345".to_string()),
            is_subscriber: true,
            is_moderator: false,
            is_vip: false,
            is_first_message: false,
        };

        assert_eq!(chat_msg.display_text(), "<TestUser> Hello world!");
        assert_eq!(chat_msg.display_color(), "#FFFFFF");
        assert_eq!(chat_msg.get_priority(), 50); // Subscriber message
    }

    #[test]
    fn test_enhanced_action_message_display() {
        let action_msg = TwitchEvent::ChatMessage {
            user: "testuser".to_string(),
            message: "waves".to_string(),
            color: None,
            badges: vec![],
            timestamp: TwitchEvent::now_timestamp(),
            is_action: true,
            message_id: None,
            display_name: Some("TestUser".to_string()),
            user_id: None,
            is_subscriber: false,
            is_moderator: false,
            is_vip: false,
            is_first_message: false,
        };

        assert_eq!(action_msg.display_text(), "* TestUser waves");
    }

    #[test]
    fn test_enhanced_subscription_display() {
        let sub = TwitchEvent::Subscription {
            user: "subscriber".to_string(),
            months: 12,
            message: Some("Love this stream!".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            is_gift: false,
            gifted_by: None,
            tier: Some("2000".to_string()),
            streak_months: Some(6),
        };

        assert!(sub.display_text().contains("subscriber subscribed (Tier 2) for 12 months! (6 month streak)"));
        assert_eq!(sub.get_priority(), 90); // High priority
    }

    #[test]
    fn test_enhanced_error_state() {
        let mut error = TwitchErrorState::new(
            TwitchErrorType::TemporaryNetwork,
            "Connection timeout".to_string(),
            Some("Additional context".to_string()),
        );

        assert!(!error.is_permanent());
        assert!(error.should_auto_retry());
        assert!(error.can_retry_now());
        assert_eq!(error.severity, "warn");
        assert!(error.suggested_action.is_some());

        error.increment_retry(30);
        assert_eq!(error.retry_count, 1);
        assert!(error.next_retry_at.is_some());
    }

    #[test]
    fn test_enhanced_config_defaults() {
        let config = TwitchConfig::default();
        
        assert_eq!(config.max_messages_per_minute, 20);
        assert_eq!(config.token_validation_interval_secs, 3600);
        assert!(config.enable_user_validation);
        assert!(config.enable_metrics);
        assert!(config.enable_auto_reconnect);
        assert_eq!(config.max_reconnect_attempts, 10);
    }

    #[test]
    fn test_error_type_display() {
        assert_eq!(TwitchErrorType::PermanentAuth.to_string(), "Authentication Error");
        assert_eq!(TwitchErrorType::TemporaryNetwork.to_string(), "Network Error");
        assert_eq!(TwitchErrorType::TemporaryRateLimit.to_string(), "Rate Limit Error");
    }

    #[test]
    fn test_user_description_generation() {
        let auth_error = TwitchErrorState::new(
            TwitchErrorType::PermanentAuth,
            "Auth failed".to_string(),
            None,
        );

        let description = auth_error.get_user_description();
        assert!(description.contains("authentication"));
        assert!(description.contains("re-authenticate"));
    }
}