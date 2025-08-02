use crate::component::Component;
use crate::{bus::BusHandle, message::Message, address::Address};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// Import specialized modules (OAuth, IRC, types remain as specialized modules)
pub mod irc;
pub mod oauth;
pub mod types;
pub mod user_validator;

use types::{TwitchConfig, TwitchEvent, TwitchStatus, TwitchErrorType, TwitchErrorState};

/// 📤 Component response builder - eliminates repetitive JSON construction
#[derive(Debug, Clone, Serialize)]
struct ComponentResponse {
    success: bool,
    action: String,
    version: u64,
    component_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    messages_sent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_remaining: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    messages_removed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ComponentResponse {
    /// Create success response with required context fields
    fn success(action: &str, component_id: &str, version: u64) -> Self {
        Self {
            success: true,
            action: action.to_string(),
            version,
            component_id: component_id.to_string(),
            timestamp: None,
            oauth_url: None,
            status: None,
            messages_sent: None,
            rate_limit_remaining: None,
            messages_removed: None,
            attempt: None,
            message: None,
            error: None,
        }
    }

    /// Create error response
    fn error(error: &str) -> Self {
        Self {
            success: false,
            action: String::new(),
            version: 0,
            component_id: String::new(),
            timestamp: None,
            oauth_url: None,
            status: None,
            messages_sent: None,
            rate_limit_remaining: None,
            messages_removed: None,
            attempt: None,
            message: None,
            error: Some(error.to_string()),
        }
    }

    /// Add timestamp (current UTC)
    fn with_timestamp(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now().to_rfc3339());
        self
    }

    /// Add OAuth URL
    fn with_oauth_url(mut self, oauth_url: &str) -> Self {
        self.oauth_url = Some(oauth_url.to_string());
        self
    }

    /// Add status
    fn with_status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }

    /// Add message stats
    fn with_message_stats(mut self, messages_sent: u64, rate_limit_remaining: u32) -> Self {
        self.messages_sent = Some(messages_sent);
        self.rate_limit_remaining = Some(rate_limit_remaining);
        self
    }

    /// Add clear history info
    fn with_clear_history(mut self, messages_removed: usize) -> Self {
        self.messages_removed = Some(messages_removed);
        self
    }

    /// Add reconnect attempt
    fn with_reconnect_attempt(mut self, attempt: u32) -> Self {
        self.attempt = Some(attempt);
        self
    }

    /// Add message
    fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    /// Convert to JSON Value
    fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({"error": "Failed to serialize response"}))
    }
}

/// ⚙️ Twitch component configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Twitch username
    pub username: String,
    
    /// Target channel name
    pub channel: String,
    
    /// Client ID from Twitch developer console
    pub client_id: Option<String>,
    
    /// Client secret from Twitch developer console
    pub client_secret: Option<String>,
    
    /// Maximum message length
    #[serde(default = "default_max_message_length")]
    pub max_message_length: usize,
    
    /// Rate limit: messages per minute
    #[serde(default = "default_max_messages_per_minute")]
    pub max_messages_per_minute: u32,
    
    /// Token validation interval in seconds
    #[serde(default = "default_token_validation_interval")]
    pub token_validation_interval_secs: u64,
}

fn default_max_message_length() -> usize {
    500
}

fn default_max_messages_per_minute() -> u32 {
    20
}

fn default_token_validation_interval() -> u64 {
    3600
}

/// 🎮 Twitch Component - Works with macro-generated infrastructure
#[crate::component(template = "chat.hbs", config = "Config")]
pub struct Twitch {
    // User fields only - macro handles all framework fields
    /// 💬 Chat message history
    chat_messages: Arc<RwLock<Vec<TwitchEvent>>>,
    /// 📊 Real-time status tracking
    status: Arc<RwLock<TwitchStatus>>,
    /// 🔐 OAuth manager with automatic refresh
    oauth_manager: Arc<RwLock<Option<oauth::TwitchOAuth>>>,
    /// 📤 Message sender for rate-limited chat
    message_sender: Arc<RwLock<Option<mpsc::UnboundedSender<String>>>>,
    /// 🔄 Background tasks for lifecycle management
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
    /// 👤 User validator for enhanced security
    user_validator: Arc<RwLock<Option<user_validator::UserValidator>>>,
    /// Internal bus handle reference (derived from system)
    bus_handle: Option<BusHandle>,
}

impl Twitch {
    /// 🔧 Called after component creation - initialize user fields
    /// NOTE: This is called by macro-generated constructor
    pub async fn new_component(&mut self) -> Result<()> {
        // Initialize user fields with defaults
        self.chat_messages = Arc::new(RwLock::new(Vec::new()));
        self.background_tasks = Arc::new(RwLock::new(Vec::new()));
        self.oauth_manager = Arc::new(RwLock::new(None));
        self.message_sender = Arc::new(RwLock::new(None));
        self.user_validator = Arc::new(RwLock::new(None));
        
        // Get bus handle from system reference
        self.bus_handle = Some(self.system.get_bus_handle().clone());
        
        // Initialize status from configuration
        let status = TwitchStatus {
            connected: false,
            channel: self.cfg.channel.clone(),
            username: self.cfg.username.clone(),
            authenticated: false,
            messages_received: 0,
            messages_sent: 0,
            last_activity: None,
            connection_uptime: None,
            error_state: None,
            oauth_validated_at: None,
            last_message_at: None,
            rate_limit_remaining: self.cfg.max_messages_per_minute,
            connection_quality: 0,
            irc_capabilities: Vec::new(),
            avg_processing_time_ms: 0.0,
            reconnection_attempts: 0,
            memory_usage_bytes: 0,
        };
        self.status = Arc::new(RwLock::new(status));
        
        // Try to load saved state
        let state_key = format!("twitch:{}:state", self.id());
        if let Ok(Some(state)) = self.load_state(&state_key).await {
            self.load_twitch_state_from_data(&state).await?;
            debug!("📖 Loaded Twitch state for '{}'", self.id());
        }
        
        // Initialize OAuth and other services
        if let Err(e) = self.initialize_connections().await {
            warn!("Failed to initialize connections: {}", e);
        }
        
        // Add welcome message
        self.add_welcome_message().await;
        
        // Start background services
        if let Err(e) = self.start_background_services().await {
            warn!("Failed to start background services: {}", e);
        }
        
        info!("🎮 Twitch component '{}' initialized", self.id());
        Ok(())
    }

    /// 🔗 Connect to Twitch - routed by macro
    pub async fn connect(&mut self) -> Result<Value> {
        info!("🔗 Enhanced connection with automatic OAuth");
        
        self.chat_messages.write().await.push(TwitchEvent::ConnectionStatus {
            connected: false,
            reason: Some("🔗 Enhanced connection initiated".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("connecting".to_string()),
            duration: None,
        });
        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save state: {}", e);
        }

        if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            if oauth_manager.has_valid_token().await {
                info!("⚡ Valid token found");
                
                let mut status = self.status.write().await;
                status.connected = true;
                status.authenticated = true;
                status.error_state = None;
                
                Ok(ComponentResponse::success("connect", self.id(), self.current_version())
                    .with_status("connected")
                    .with_message("Connected with existing token")
                    .with_timestamp()
                    .to_json())
            } else {
                // Start OAuth flow
                self.initiate_oauth().await?;
                Ok(ComponentResponse::success("connect", self.id(), self.current_version())
                    .with_status("oauth_started")
                    .with_message("OAuth flow started")
                    .with_timestamp()
                    .to_json())
            }
        } else {
            Ok(ComponentResponse::error("OAuth manager not initialized").to_json())
        }
    }

    /// 🚪 Disconnect from Twitch - routed by macro
    pub async fn disconnect(&mut self) -> Result<Value> {
        info!("🚪 Enhanced disconnection");

        let mut status = self.status.write().await;
        status.connected = false;
        status.authenticated = false;
        status.error_state = None;

        self.chat_messages.write().await.push(TwitchEvent::ConnectionStatus {
            connected: false,
            reason: Some("🚪 Disconnected by user request".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: None,
            duration: None,
        });
        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save state: {}", e);
        }

        Ok(ComponentResponse::success("disconnect", self.id(), self.current_version())
            .with_status("disconnected")
            .with_message("Disconnected successfully")
            .with_timestamp()
            .to_json())
    }

    /// 💬 Send message to chat - routed by macro
    pub async fn send_message(&mut self, text: &str) -> Result<Value> {
        if text.trim().is_empty() {
            return Ok(ComponentResponse::error("Cannot send empty message").to_json());
        }

        if let Some(sender) = self.message_sender.read().await.as_ref() {
            match sender.send(text.to_string()) {
                Ok(_) => {
                    // Update statistics
                    let mut status = self.status.write().await;
                    status.messages_sent += 1;
                    status.last_activity = Some(TwitchEvent::now_timestamp());
                    status.rate_limit_remaining = status.rate_limit_remaining.saturating_sub(1);

                    info!("📤 Enhanced message sent: {}", text);
                    if let Err(e) = self.save_component_state().await {
                        warn!("Failed to save state: {}", e);
                    }
                    
                    Ok(ComponentResponse::success("send_message", self.id(), self.current_version())
                        .with_message_stats(status.messages_sent, status.rate_limit_remaining)
                        .with_message("Message sent successfully")
                        .with_timestamp()
                        .to_json())
                }
                Err(_) => {
                    Ok(ComponentResponse::error("Failed to send message - IRC connection not ready").to_json())
                }
            }
        } else {
            Ok(ComponentResponse::error("Not connected to chat").to_json())
        }
    }

    /// 🔐 Start OAuth flow - routed by macro
    pub async fn oauth_start(&mut self) -> Result<Value> {
        info!("🔐 Enhanced OAuth flow start");
        
        if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            match oauth_manager.start_oauth_flow().await {
                Ok(oauth_url) => {
                    self.chat_messages.write().await.push(TwitchEvent::ConnectionStatus {
                        connected: false,
                        reason: Some("🔐 Enhanced OAuth flow started - check browser".to_string()),
                        timestamp: TwitchEvent::now_timestamp(),
                        quality: Some("authenticating".to_string()),
                        duration: None,
                    });
                    self.increment_version();
                    if let Err(e) = self.save_component_state().await {
                        warn!("Failed to save state: {}", e);
                    }
                    
                    Ok(ComponentResponse::success("oauth_start", self.id(), self.current_version())
                        .with_oauth_url(&oauth_url)
                        .with_message("Enhanced OAuth flow started")
                        .with_timestamp()
                        .to_json())
                }
                Err(e) => {
                    error!("❌ Enhanced OAuth start failed: {}", e);
                    Ok(ComponentResponse::error(&format!("OAuth start failed: {e}")).to_json())
                }
            }
        } else {
            Ok(ComponentResponse::error("OAuth manager not available").to_json())
        }
    }

    /// 🔄 Handle OAuth callback - routed by macro
    pub async fn handle_oauth(&mut self, params: std::collections::HashMap<String, String>) -> Result<Value> {
        info!("🔄 Enhanced OAuth callback processing");

        if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            match oauth_manager.handle_oauth_callback(params).await {
                Ok(_) => {
                    self.chat_messages.write().await.push(TwitchEvent::ConnectionStatus {
                        connected: false,
                        reason: Some("✅ Enhanced OAuth completed - connecting to IRC".to_string()),
                        timestamp: TwitchEvent::now_timestamp(),
                        quality: Some("authenticated".to_string()),
                        duration: None,
                    });
                    self.increment_version();
                    if let Err(e) = self.save_component_state().await {
                        warn!("Failed to save state: {}", e);
                    }
                    
                    // Automatically start IRC connection after successful OAuth
                    self.start_irc_connection().await;
                    
                    Ok(ComponentResponse::success("handle_oauth", self.id(), self.current_version())
                        .with_message("Enhanced OAuth completed successfully")
                        .with_timestamp()
                        .to_json())
                }
                Err(e) => {
                    error!("❌ Enhanced OAuth callback failed: {}", e);
                    Ok(ComponentResponse::error(&format!("OAuth callback failed: {e}")).to_json())
                }
            }
        } else {
            Ok(ComponentResponse::error("OAuth manager not available").to_json())
        }
    }

    /// 🔄 Reconnect to Twitch - routed by macro
    pub async fn reconnect(&mut self) -> Result<Value> {
        info!("🔄 Enhanced manual reconnection");

        self.chat_messages.write().await.push(TwitchEvent::ConnectionStatus {
            connected: false,
            reason: Some("🔄 Enhanced reconnection initiated".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("reconnecting".to_string()),
            duration: None,
        });
        
        let mut status = self.status.write().await;
        status.error_state = None;
        status.reconnection_attempts += 1;
        
        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save state: {}", e);
        }

        // Attempt to reconnect IRC
        self.start_irc_connection().await;

        Ok(ComponentResponse::success("reconnect", self.id(), self.current_version())
            .with_status("reconnecting")
            .with_reconnect_attempt(status.reconnection_attempts)
            .with_message("Enhanced reconnection initiated")
            .with_timestamp()
            .to_json())
    }

    /// 📊 Get comprehensive metrics - routed by macro
    pub async fn get(&self, key: &str) -> Result<Value> {
        match key {
            "metrics" => {
                let metrics = self.get_comprehensive_metrics().await;
                Ok(json!({
                    "success": true,
                    "metrics": metrics
                }))
            }
            "status" => {
                let status = self.status.read().await;
                let messages = self.chat_messages.read().await;
                
                Ok(json!({
                    "connected": status.connected,
                    "authenticated": status.authenticated,
                    "channel": status.channel,
                    "username": status.username,
                    "messages_received": status.messages_received,
                    "messages_sent": status.messages_sent,
                    "connection_quality": status.connection_quality,
                    "rate_limit_remaining": status.rate_limit_remaining,
                    "message_count": messages.len(),
                    "version": self.current_version(),
                    "error_state": status.error_state.as_ref().map(|e| json!({
                        "message": e.message,
                        "error_type": e.error_type.to_string(),
                        "is_permanent": e.is_permanent()
                    }))
                }))
            }
            _ => Ok(json!({
                "error": format!("Unknown key: {}", key),
                "available_keys": ["metrics", "status"]
            }))
        }
    }

    /// 🧹 Clear chat history - routed by macro
    pub async fn clear_history(&mut self) -> Result<Value> {
        let mut messages = self.chat_messages.write().await;
        let original_count = messages.len();
        messages.clear();
        
        // Add enhanced system message
        messages.push(TwitchEvent::ConnectionStatus {
            connected: true,
            reason: Some(format!("🧹 Enhanced history cleared ({original_count} messages removed)")),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("maintenance".to_string()),
            duration: None,
        });
        
        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save state: {}", e);
        }
        
        Ok(ComponentResponse::success("clear_history", self.id(), self.current_version())
            .with_clear_history(original_count)
            .with_message("Chat history cleared successfully")
            .with_timestamp()
            .to_json())
    }

    /// 👤 Validate user - routed by macro
    pub async fn validate_user(&mut self) -> Result<Value> {
        if let Some(_validator) = self.user_validator.read().await.as_ref() {
            info!("👤 Enhanced user validation requested");
            Ok(ComponentResponse::success("validate_user", self.id(), self.current_version())
                .with_message("Enhanced user validation completed")
                .with_timestamp()
                .to_json())
        } else {
            Ok(ComponentResponse::error("User validator not available").to_json())
        }
    }

    /// 🔄 Refresh OAuth token - routed by macro
    pub async fn refresh_token(&mut self) -> Result<Value> {
        if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            match oauth_manager.validate_token().await {
                Ok(is_valid) => {
                    Ok(ComponentResponse::success("refresh_token", self.id(), self.current_version())
                        .with_status(if is_valid { "valid" } else { "needs_refresh" })
                        .with_message(if is_valid { 
                            "Token is valid" 
                        } else { 
                            "Token needs refresh" 
                        })
                        .with_timestamp()
                        .to_json())
                }
                Err(e) => {
                    Ok(ComponentResponse::error(&format!("Token validation failed: {e}")).to_json())
                }
            }
        } else {
            Ok(ComponentResponse::error("OAuth manager not available").to_json())
        }
    }

    // ================================================================================
    // 🔧 HELPER METHODS
    // ================================================================================

    /// 💾 Save component state using macro-generated storage helpers
    async fn save_component_state(&self) -> Result<()> {
        let status = self.status.read().await;
        let state = json!({
            "status": {
                "connected": status.connected,
                "authenticated": status.authenticated,
                "messages_received": status.messages_received,
                "messages_sent": status.messages_sent,
                "connection_quality": status.connection_quality,
                "rate_limit_remaining": status.rate_limit_remaining,
                "reconnection_attempts": status.reconnection_attempts
            },
            "version": self.current_version(),
            "last_updated": chrono::Utc::now().to_rfc3339()
        });
        
        let state_key = format!("twitch:{}:state", self.id());
        self.save_state(&state_key, state).await
    }

    /// 📂 Load Twitch state from saved data
    async fn load_twitch_state_from_data(&mut self, state: &Value) -> Result<()> {
        if let Some(status_obj) = state.get("status").and_then(|v| v.as_object()) {
            let mut status = self.status.write().await;
            
            if let Some(connected) = status_obj.get("connected").and_then(|v| v.as_bool()) {
                status.connected = connected;
            }
            if let Some(authenticated) = status_obj.get("authenticated").and_then(|v| v.as_bool()) {
                status.authenticated = authenticated;
            }
            if let Some(messages_received) = status_obj.get("messages_received").and_then(|v| v.as_u64()) {
                status.messages_received = messages_received;
            }
            if let Some(messages_sent) = status_obj.get("messages_sent").and_then(|v| v.as_u64()) {
                status.messages_sent = messages_sent;
            }
            if let Some(quality) = status_obj.get("connection_quality").and_then(|v| v.as_u64()) {
                status.connection_quality = quality as u8;
            }
            if let Some(remaining) = status_obj.get("rate_limit_remaining").and_then(|v| v.as_u64()) {
                status.rate_limit_remaining = remaining as u32;
            }
            if let Some(attempts) = status_obj.get("reconnection_attempts").and_then(|v| v.as_u64()) {
                status.reconnection_attempts = attempts as u32;
            }
        }
        Ok(())
    }

    /// 🚀 Initialize OAuth and IRC connections
    async fn initialize_connections(&mut self) -> Result<()> {
        // Initialize OAuth if needed
        self.initialize_oauth().await?;
        
        // Initialize user validator
        self.initialize_user_validator().await;
        
        info!("🚀 Twitch connections initialized");
        Ok(())
    }

    /// 🔐 Initialize OAuth manager
    async fn initialize_oauth(&mut self) -> Result<()> {
        if let Some(bus_handle) = &self.bus_handle {
            // Convert Config to TwitchConfig
            let twitch_config = TwitchConfig {
                username: self.cfg.username.clone(),
                channel: self.cfg.channel.clone(),
                client_id: self.cfg.client_id.clone(),
                client_secret: self.cfg.client_secret.clone(),
                oauth_scopes: Some("chat:read chat:edit".to_string()),
                oauth_timeout_secs: 300,
                max_message_length: self.cfg.max_message_length,
                max_stored_messages: 100,
                reconnect_delays: vec![1, 2, 4, 8, 16, 30, 60],
                irc_server: "irc.chat.twitch.tv".to_string(),
                irc_port: 6697,
                max_messages_per_minute: self.cfg.max_messages_per_minute,
                token_validation_interval_secs: self.cfg.token_validation_interval_secs,
                enable_user_validation: true,
                enable_metrics: true,
                enable_auto_reconnect: true,
                max_reconnect_attempts: 10,
                enable_enhanced_parsing: true,
                message_queue_size: 1000,
                enable_rate_limiting: true,
            };

            let oauth_manager = oauth::TwitchOAuth::new(
                bus_handle.clone(),
                twitch_config,
                self.id().to_string(),
            );

            *self.oauth_manager.write().await = Some(oauth_manager);
            info!("🔐 OAuth manager initialized");
        }
        Ok(())
    }

    /// 👤 Initialize user validator
    async fn initialize_user_validator(&mut self) {
        if let Some(bus_handle) = &self.bus_handle {
            let validator = user_validator::UserValidator::new(bus_handle.clone());
            *self.user_validator.write().await = Some(validator);
            info!("👤 User validator initialized");
        }
    }

    /// 💬 Add welcome message with enhanced features
    async fn add_welcome_message(&self) {
        let mut messages = self.chat_messages.write().await;
        
        messages.push(TwitchEvent::ConnectionStatus {
            connected: false,
            reason: Some("🚀 Macro-based component initialized - enhanced features ready".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("initializing".to_string()),
            duration: None,
        });

        messages.push(TwitchEvent::ChatMessage {
            user: "system".to_string(),
            message: format!(
                "🎮 Welcome to #{} chat! Macro-based features:\n\
                ✨ Automatic OAuth with token refresh\n\
                🛡️ Intelligent error recovery with classification\n\
                📊 Real-time metrics and health monitoring\n\
                🔄 Advanced rate limiting and message validation\n\
                🎨 Modern UI with smooth incremental updates",
                self.cfg.channel
            ),
            color: Some("#9146FF".to_string()),
            badges: vec!["system".to_string()],
            timestamp: TwitchEvent::now_timestamp(),
            is_action: false,
            message_id: Some(format!("sys-{}", uuid::Uuid::new_v4())),
            display_name: Some("ControlRake Macro".to_string()),
            user_id: Some("system".to_string()),
            is_subscriber: false,
            is_moderator: true,
            is_vip: false,
            is_first_message: false,
        });

        self.increment_version();
    }

    /// 🔐 Enhanced OAuth initiation
    async fn initiate_oauth(&self) -> Result<()> {
        if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            match oauth_manager.start_oauth_flow().await {
                Ok(oauth_url) if oauth_url == "existing_token_found" => {
                    self.start_irc_connection().await;
                }
                Ok(oauth_url) => {
                    info!("🌐 OAuth URL generated: {}", oauth_url);
                    self.add_connection_status("🌐 Browser opened for authentication - please authorize").await;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// 🚀 Enhanced IRC connection with comprehensive error handling
    async fn start_irc_connection(&self) {
        info!("🚀 Starting enhanced IRC connection");

        if let Some(bus_handle) = &self.bus_handle {
            // Get authentication with validation
            let auth = {
                let oauth_manager_guard = self.oauth_manager.read().await;
                let oauth_manager = match oauth_manager_guard.as_ref() {
                    Some(manager) => manager,
                    None => {
                        error!("❌ OAuth manager not initialized");
                        return;
                    }
                };

                match oauth_manager.get_stored_token().await {
                    Ok(token) => token,
                    Err(e) => {
                        error!("❌ Failed to get stored token: {}", e);
                        self.set_error_state(TwitchErrorType::PermanentAuth, 
                                             format!("Authentication token not available: {e}")).await;
                        return;
                    }
                }
            };

            // Convert Config to TwitchConfig
            let config = TwitchConfig {
                username: self.cfg.username.clone(),
                channel: self.cfg.channel.clone(),
                client_id: self.cfg.client_id.clone(),
                client_secret: self.cfg.client_secret.clone(),
                oauth_scopes: Some("chat:read chat:edit".to_string()),
                oauth_timeout_secs: 300,
                max_message_length: self.cfg.max_message_length,
                max_stored_messages: 100,
                reconnect_delays: vec![1, 2, 4, 8, 16, 30, 60],
                irc_server: "irc.chat.twitch.tv".to_string(),
                irc_port: 6697,
                max_messages_per_minute: self.cfg.max_messages_per_minute,
                token_validation_interval_secs: self.cfg.token_validation_interval_secs,
                enable_user_validation: true,
                enable_metrics: true,
                enable_auto_reconnect: true,
                max_reconnect_attempts: 10,
                enable_enhanced_parsing: true,
                message_queue_size: 1000,
                enable_rate_limiting: true,
            };
            
            // Create enhanced message channels
            let (event_sender, event_receiver) = mpsc::unbounded_channel::<TwitchEvent>();
            let (message_sender, message_receiver) = mpsc::unbounded_channel::<String>();
            
            // Store message sender for component use
            *self.message_sender.write().await = Some(message_sender);

            // Create IRC client with enhanced features
            let mut irc_client = irc::IrcClient::new(
                auth,
                config.channel.clone(),
                config,
                event_sender,
                message_receiver,
            );

            // Spawn IRC client task with error handling
            let chat_messages = self.chat_messages.clone();
            let status = self.status.clone();
            let version = Arc::new(AtomicU64::new(self.current_version()));
            
            let irc_task = tokio::spawn(async move {
                if let Err(e) = irc_client.start().await {
                    error!("❌ IRC client failed: {}", e);
                }
            });

            // Enhanced event handling with broadcasting
            self.spawn_event_handler(event_receiver, chat_messages, status, version, bus_handle.clone()).await;

            // Store IRC task handle
            let mut tasks = self.background_tasks.write().await;
            tasks.push(irc_task);
        }
    }

    /// 📡 Enhanced event handler with broadcasting
    async fn spawn_event_handler(
        &self,
        mut event_receiver: mpsc::UnboundedReceiver<TwitchEvent>,
        chat_messages: Arc<RwLock<Vec<TwitchEvent>>>,
        status: Arc<RwLock<TwitchStatus>>,
        version: Arc<AtomicU64>,
        bus_handle: BusHandle,
    ) {
        let component_id = self.id().to_string();
        
        let event_task = tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                // Add event to chat messages
                {
                    let mut messages = chat_messages.write().await;
                    messages.push(event.clone());
                    
                    // Intelligent message management
                    if messages.len() > 1000 {
                        messages.remove(0);
                    }
                }

                // Enhanced status updates based on event type
                match &event {
                    TwitchEvent::ConnectionStatus { connected, .. } => {
                        let mut status_guard = status.write().await;
                        status_guard.connected = *connected;
                        status_guard.last_activity = Some(TwitchEvent::now_timestamp());
                        if *connected {
                            status_guard.authenticated = true;
                            status_guard.error_state = None;
                            status_guard.connection_quality = 100;
                        }
                    }
                    TwitchEvent::ChatMessage { .. } => {
                        let mut status_guard = status.write().await;
                        status_guard.messages_received += 1;
                        status_guard.last_activity = Some(TwitchEvent::now_timestamp());
                        status_guard.last_message_at = Some(TwitchEvent::now_timestamp());
                    }
                    _ => {
                        let mut status_guard = status.write().await;
                        status_guard.last_activity = Some(TwitchEvent::now_timestamp());
                    }
                }

                // Enhanced event broadcasting to HTTP component
                let event_type = match &event {
                    TwitchEvent::ChatMessage { .. } => "chat_message",
                    TwitchEvent::ConnectionStatus { .. } => "connection_status", 
                    TwitchEvent::Join { .. } => "user_join",
                    TwitchEvent::Part { .. } => "user_part",
                    TwitchEvent::Subscription { .. } => "subscription",
                    TwitchEvent::Raid { .. } => "raid",
                    TwitchEvent::Bits { .. } => "bits",
                    TwitchEvent::Host { .. } => "host",
                    TwitchEvent::UserState { .. } => "user_state",
                };

                // Enhanced broadcast payload with metadata
                let broadcast_payload = json!({
                    "type": "event",
                    "event_type": event_type,
                    "source": format!("twitch:{component_id}"),
                    "event": event,
                    "timestamp": TwitchEvent::now_timestamp(),
                    "component_version": version.load(Ordering::SeqCst)
                });

                let message = Message {
                    target: "http:.broadcast".parse().unwrap_or_else(|_| {
                        warn!("Failed to parse HTTP broadcast address");
                        Address::parse("http:").unwrap()
                    }),
                    source: Some(format!("twitch:{component_id}").parse().unwrap_or_else(|_| {
                        warn!("Failed to parse source address"); 
                        Address::parse("twitch").unwrap()
                    })),
                    payload: broadcast_payload,
                    id: Some(crate::message::MessageId::new()),
                    reply: None,
                    finish: false,
                };

                if let Err(e) = bus_handle.send(message).await {
                    warn!("Failed to broadcast event: {}", e);
                }

                // Update version for incremental updates
                version.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Store event task handle
        let mut tasks = self.background_tasks.write().await;
        tasks.push(event_task);
    }

    /// 🔄 Start background services with comprehensive monitoring
    async fn start_background_services(&self) -> Result<()> {
        let mut tasks = self.background_tasks.write().await;
        
        // Token validation service (Twitch requirement - hourly)
        let token_task = self.spawn_token_validation_service().await;
        tasks.push(token_task);

        // Health monitoring with connection quality tracking
        let health_task = self.spawn_health_monitoring_service().await;
        tasks.push(health_task);

        // Memory usage monitoring
        let memory_task = self.spawn_memory_monitoring_service().await;
        tasks.push(memory_task);

        info!("✅ Enhanced background services started ({} tasks)", tasks.len());
        Ok(())
    }

    /// 🕐 Enhanced token validation service
    async fn spawn_token_validation_service(&self) -> tokio::task::JoinHandle<()> {
        let oauth_manager = self.oauth_manager.clone();
        let status = self.status.clone();
        let interval_secs = self.cfg.token_validation_interval_secs;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(interval_secs)
            );
            
            loop {
                interval.tick().await;
                
                if let Some(oauth) = oauth_manager.read().await.as_ref() {
                    match oauth.validate_token().await {
                        Ok(is_valid) => {
                            let mut status_guard = status.write().await;
                            status_guard.oauth_validated_at = Some(chrono::Utc::now().to_rfc3339());
                            
                            if !is_valid {
                                warn!("🚨 Token validation failed - may need refresh");
                                // Could trigger automatic refresh here if needed
                            }
                        }
                        Err(e) => {
                            debug!("⚠️ Token validation error: {}", e);
                        }
                    }
                }
            }
        })
    }

    /// 🏥 Enhanced health monitoring service
    async fn spawn_health_monitoring_service(&self) -> tokio::task::JoinHandle<()> {
        let status = self.status.clone();
        let chat_messages = self.chat_messages.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(30)
            );
            
            loop {
                interval.tick().await;
                
                let mut status_guard = status.write().await;
                
                if status_guard.connected {
                    // Calculate connection quality based on multiple factors
                    let mut quality_score = 100;
                    
                    // Factor 1: Time since last activity
                    if let Some(last_activity) = &status_guard.last_activity {
                        if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(last_activity) {
                            let now = chrono::Utc::now();
                            let elapsed = now.signed_duration_since(last_time.with_timezone(&chrono::Utc));
                            
                            if elapsed.num_minutes() > 30 {
                                quality_score -= 50; // Significant degradation
                            } else if elapsed.num_minutes() > 15 {
                                quality_score -= 25; // Moderate degradation
                            } else if elapsed.num_minutes() > 5 {
                                quality_score -= 10; // Minor degradation
                            }
                        }
                    }
                    
                    // Factor 2: Error state impact
                    if let Some(error_state) = &status_guard.error_state {
                        if error_state.is_permanent() {
                            quality_score = 0; // Critical failure
                        } else {
                            quality_score -= 20; // Temporary issue
                        }
                    }
                    
                    // Factor 3: Reconnection attempts
                    if status_guard.reconnection_attempts > 0 {
                        quality_score -= (status_guard.reconnection_attempts * 5).min(30);
                    }
                    
                    status_guard.connection_quality = quality_score.clamp(0, 255) as u8;
                }
                
                // Update memory usage estimate
                let message_count = chat_messages.read().await.len();
                status_guard.memory_usage_bytes = (message_count * 256) as u64; // Rough estimate
                
                drop(status_guard);
            }
        })
    }

    /// 💾 Memory monitoring service
    async fn spawn_memory_monitoring_service(&self) -> tokio::task::JoinHandle<()> {
        let chat_messages = self.chat_messages.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(300) // Every 5 minutes
            );
            
            loop {
                interval.tick().await;
                
                // Clean up old messages to prevent memory leaks
                let mut messages = chat_messages.write().await;
                if messages.len() > 1000 {
                    let excess = messages.len() - 800; // Keep some buffer
                    messages.drain(0..excess);
                    info!("🧹 Cleaned up {} old messages", excess);
                }
            }
        })
    }

    /// 🚨 Enhanced error state management
    async fn set_error_state(&self, error_type: TwitchErrorType, message: String) {
        let error_state = TwitchErrorState::new(error_type, message, None);
        
        {
            let mut status = self.status.write().await;
            status.error_state = Some(error_state.clone());
            status.connected = false;
            status.authenticated = false;
            status.connection_quality = 0;
        }

        // Add error message to chat with enhanced formatting
        self.add_connection_status(&format!("❌ Error: {}", error_state.message)).await;
        self.increment_version();
    }

    /// 📢 Helper to add connection status messages
    async fn add_connection_status(&self, message: &str) {
        let mut messages = self.chat_messages.write().await;
        messages.push(TwitchEvent::ConnectionStatus {
            connected: false,
            reason: Some(message.to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("info".to_string()),
            duration: None,
        });
    }

    /// 📊 Get comprehensive component metrics
    pub async fn get_comprehensive_metrics(&self) -> Value {
        let status = self.status.read().await;
        let messages = self.chat_messages.read().await;
        
        // Get OAuth metrics if available
        let oauth_metrics = if let Some(oauth_manager) = self.oauth_manager.read().await.as_ref() {
            oauth_manager.get_metrics().await
        } else {
            oauth::OAuthMetrics::default()
        };

        json!({
            "component": {
                "id": self.id(),
                "version": self.current_version(),
                "uptime_seconds": 0, // warn!("TODO: Parse connection_uptime string to seconds")
            },
            "connection": {
                "connected": status.connected,
                "authenticated": status.authenticated,
                "quality": status.connection_quality,
                "reconnection_attempts": status.reconnection_attempts,
                "last_activity": status.last_activity,
            },
            "messages": {
                "received": status.messages_received,
                "sent": status.messages_sent,
                "history_count": messages.len(),
                "rate_limit_remaining": status.rate_limit_remaining,
                "last_message_at": status.last_message_at,
            },
            "performance": {
                "avg_processing_time_ms": status.avg_processing_time_ms,
                "memory_usage_bytes": status.memory_usage_bytes,
            },
            "oauth": {
                "flows_started": oauth_metrics.oauth_flows_started,
                "flows_completed": oauth_metrics.oauth_flows_completed,
                "flows_failed": oauth_metrics.oauth_flows_failed,
                "tokens_cached": oauth_metrics.tokens_cached,
                "cache_hit_rate": oauth_metrics.cache_hit_rate,
                "avg_oauth_duration_ms": oauth_metrics.avg_oauth_duration_ms,
            },
            "errors": {
                "current_error": status.error_state.as_ref().map(|e| e.error_type.to_string()),
                "error_message": status.error_state.as_ref().map(|e| e.message.clone()),
                "is_permanent": status.error_state.as_ref().map(|e| e.is_permanent()).unwrap_or(false),
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }
}
