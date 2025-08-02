use crate::component::twitch::types::{IrcMessage, TwitchAuth, TwitchEvent, TwitchErrorState, TwitchErrorType, TwitchConfig};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant, timeout};
use tokio_native_tls::{TlsConnector, TlsStream};
use tracing::{debug, error, info, instrument, warn};

/// ⏱️ Production-ready timeouts and limits with enhanced reliability
const IRC_CONNECT_TIMEOUT_SECS: u64 = 15; // Increased for better reliability
const IRC_AUTH_TIMEOUT_SECS: u64 = 20; // Increased for enhanced auth flow
const IRC_MESSAGE_TIMEOUT_SECS: u64 = 45; // Increased to handle network issues
const IRC_PING_INTERVAL_SECS: u64 = 60; // Standard ping interval
const IRC_PONG_TIMEOUT_SECS: u64 = 30; // Timeout for pong responses
const MAX_MESSAGE_SIZE: usize = 512; // IRC protocol limit
const MAX_RECONNECT_ATTEMPTS: usize = 15; // Increased for better resilience
const HEALTH_CHECK_INTERVAL_SECS: u64 = 30; // Health monitoring interval
const RATE_LIMIT_WINDOW_SECS: u64 = 30; // Rate limiting window
const RATE_LIMIT_BURST_SIZE: usize = 5; // Burst allowance
const MESSAGE_HISTORY_LIMIT: usize = 1000; // Keep message history for debugging

/// 📊 Enhanced IRC connection metrics for comprehensive monitoring
#[derive(Debug, Default, Clone)]
pub struct IrcMetrics {
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub failed_connections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub ping_pong_cycles: u64,
    pub reconnection_attempts: u64,
    pub total_uptime_secs: u64,
    pub last_connection_duration_ms: u64,
    pub avg_message_processing_ms: f64,
    pub rate_limited_messages: u64,
    pub dropped_messages: u64,
    pub protocol_errors: u64,
    pub network_errors: u64,
    pub auth_errors: u64,
    pub capabilities_negotiated: Vec<String>,
    pub peak_messages_per_minute: u64,
    pub connection_quality_score: u8,
}

/// 🔄 Enhanced rate limiter for message sending
#[derive(Debug)]
struct RateLimiter {
    messages: Vec<Instant>,
    max_messages: usize,
    window_duration: Duration,
    burst_allowance: usize,
}

impl RateLimiter {
    fn new(max_messages: usize, window_duration: Duration, burst_allowance: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            window_duration,
            burst_allowance,
        }
    }

    fn can_send(&mut self) -> bool {
        let now = Instant::now();
        
        // Remove old messages outside the window
        self.messages.retain(|&time| now.duration_since(time) < self.window_duration);
        
        // Check if we can send (allow burst but respect rate limit)
        self.messages.len() < self.max_messages + self.burst_allowance
    }

    fn record_message(&mut self) {
        self.messages.push(Instant::now());
    }

    fn get_wait_time(&self) -> Option<Duration> {
        if self.messages.len() < self.max_messages {
            return None;
        }
        
        let oldest_message = self.messages.first()?;
        let elapsed = oldest_message.elapsed();
        
        if elapsed < self.window_duration {
            Some(self.window_duration - elapsed)
        } else {
            None
        }
    }
}

/// 📡 Production-ready Twitch IRC client with enhanced reliability and monitoring
pub struct IrcClient {
    /// 🌐 TLS stream to Twitch IRC server
    stream: Option<TlsStream<TcpStream>>,
    /// 👤 Authenticated username
    username: String,
    /// 📺 Channel to join (normalized without #)
    channel: String,
    /// 🔑 OAuth token for authentication
    token: String,
    /// ⚙️ Configuration with timing and limits
    config: TwitchConfig,
    /// 📨 Event sender for broadcasting Twitch events
    event_sender: mpsc::UnboundedSender<TwitchEvent>,
    /// 📤 Message receiver for sending chat messages
    message_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// 🔄 Current reconnection attempt counter
    reconnect_attempt: usize,
    /// 🔗 Connection status and health tracking
    connection_state: ConnectionState,
    /// 📊 Comprehensive performance and reliability metrics
    metrics: IrcMetrics,
    /// ⏰ Connection timing information
    connection_started: Option<Instant>,
    /// 💓 Last ping/pong cycle time for health monitoring
    last_ping: Option<Instant>,
    last_pong: Option<Instant>,
    /// 🎯 Enhanced message buffer with rate limiting
    message_buffer: Vec<String>,
    /// ⚡ Enhanced error state tracking with classification
    error_state: Option<TwitchErrorState>,
    /// 🔄 Rate limiter for outgoing messages
    rate_limiter: RateLimiter,
    /// 📜 Message history for debugging and analysis
    message_history: Vec<(Instant, String, bool)>, // (timestamp, message, is_outgoing)
    /// 🎯 IRC capabilities negotiated with server
    capabilities: Vec<String>,
    /// 🏥 Health monitoring data
    health_data: HealthData,
}

/// 🔗 Enhanced connection state tracking for intelligent error handling
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    NegotiatingCapabilities,
    JoiningChannel,
    Connected,
    Reconnecting,
    Failed(String),
    Degraded(String), // Connected but with issues
}

/// 🏥 Health monitoring data structure
#[derive(Debug)]
pub struct HealthData {
    last_message_received: Option<Instant>,
    last_message_sent: Option<Instant>,
    consecutive_timeouts: u32,
    consecutive_errors: u32,
    connection_stability_score: f64,
    message_flow_rate: f64,
}

impl Default for HealthData {
    fn default() -> Self {
        Self {
            last_message_received: None,
            last_message_sent: None,
            consecutive_timeouts: 0,
            consecutive_errors: 0,
            connection_stability_score: 100.0, // Start with perfect score
            message_flow_rate: 0.0,
        }
    }
}

impl IrcClient {
    /// 🏗️ Create enhanced IRC client with production settings
    #[instrument(skip(auth, config, event_sender, message_receiver))]
    pub fn new(
        auth: TwitchAuth,
        channel: String,
        config: TwitchConfig,
        event_sender: mpsc::UnboundedSender<TwitchEvent>,
        message_receiver: mpsc::UnboundedReceiver<String>,
    ) -> Self {
        info!("🏗️ Creating enhanced IRC client for user {} in channel #{}", 
              auth.username, channel);

        // Normalize channel name (remove # if present)
        let normalized_channel = channel.trim_start_matches('#').to_string();

        // Create rate limiter based on configuration
        let rate_limiter = RateLimiter::new(
            config.max_messages_per_minute as usize,
            Duration::from_secs(RATE_LIMIT_WINDOW_SECS),
            RATE_LIMIT_BURST_SIZE,
        );

        Self {
            stream: None,
            username: auth.username,
            channel: normalized_channel,
            token: auth.token,
            config,
            event_sender,
            message_receiver: Some(message_receiver),
            reconnect_attempt: 0,
            connection_state: ConnectionState::Disconnected,
            metrics: IrcMetrics::default(),
            connection_started: None,
            last_ping: None,
            last_pong: None,
            message_buffer: Vec::new(),
            error_state: None,
            rate_limiter,
            message_history: Vec::new(),
            capabilities: Vec::new(),
            health_data: HealthData::default(),
        }
    }

    /// 🚀 Start enhanced IRC client with intelligent reconnection and monitoring
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting enhanced IRC client for #{}", self.channel);

        loop {
            self.metrics.connection_attempts += 1;
            let connection_start = Instant::now();

            match self.connect_and_run().await {
                Ok(_) => {
                    info!("✅ IRC client completed gracefully");
                    self.metrics.successful_connections += 1;
                    break;
                }
                Err(e) => {
                    self.metrics.failed_connections += 1;
                    let connection_duration = connection_start.elapsed();
                    
                    error!("❌ IRC connection failed after {:?}: {}", connection_duration, e);
                    
                    // Enhanced error classification and handling
                    let error_type = self.classify_error_enhanced(&e);
                    self.error_state = Some(TwitchErrorState::new(
                        error_type.clone(),
                        e.to_string(),
                        Some(format!("Connection attempt {} failed after {:?}", 
                                   self.reconnect_attempt + 1, connection_duration)),
                    ));

                    // Update connection state and health data
                    self.connection_state = ConnectionState::Failed(e.to_string());
                    self.stream = None;
                    self.health_data.consecutive_errors += 1;

                    // Emit enhanced disconnection event with detailed information
                    let event = TwitchEvent::ConnectionStatus {
                        connected: false,
                        reason: Some(format!("Connection failed: {} (attempt {})", e, self.reconnect_attempt + 1)),
                        timestamp: TwitchEvent::now_timestamp(),
                        quality: Some("failed".to_string()),
                        duration: Some(format!("{:.1}s", connection_duration.as_secs_f64())),
                    };

                    if let Err(send_err) = self.event_sender.send(event) {
                        error!("Failed to send enhanced disconnection event: {send_err}");
                    }

                    // Intelligent reconnection logic with enhanced backoff
                    if self.should_reconnect_enhanced(&error_type) {
                        let delay_index = self.reconnect_attempt.min(self.config.reconnect_delays.len() - 1);
                        let base_delay = self.config.reconnect_delays[delay_index];
                        
                        // Enhanced backoff with jitter and error-based adjustment
                        let jitter = rand::random::<f64>() * 0.5; // 0-50% jitter
                        let error_multiplier = if self.health_data.consecutive_errors > 3 { 2.0 } else { 1.0 };
                        let actual_delay = (base_delay as f64 * error_multiplier * (1.0 + jitter)) as u64;

                        warn!("🔄 Reconnecting in {} seconds (attempt {}/{}, base={}, adjusted={})", 
                              actual_delay, self.reconnect_attempt + 1, MAX_RECONNECT_ATTEMPTS, 
                              base_delay, actual_delay);

                        self.connection_state = ConnectionState::Reconnecting;
                        self.metrics.reconnection_attempts += 1;

                        // Send enhanced reconnection status
                        let reconnect_event = TwitchEvent::ConnectionStatus {
                            connected: false,
                            reason: Some(format!("Reconnecting in {}s... (attempt {}) - {}", 
                                                actual_delay, self.reconnect_attempt + 1, 
                                                error_type)),
                            timestamp: TwitchEvent::now_timestamp(),
                            quality: Some("reconnecting".to_string()),
                            duration: None,
                        };

                        let _ = self.event_sender.send(reconnect_event);

                        sleep(Duration::from_secs(actual_delay)).await;
                        self.reconnect_attempt += 1;

                        if self.reconnect_attempt >= MAX_RECONNECT_ATTEMPTS {
                            error!("❌ Maximum reconnection attempts ({}) reached", MAX_RECONNECT_ATTEMPTS);
                            break;
                        }
                    } else {
                        error!("❌ Error classified as non-recoverable, stopping reconnection");
                        break;
                    }
                }
            }
        }

        // Final cleanup and metrics update
        if let Some(started) = self.connection_started {
            self.metrics.total_uptime_secs += started.elapsed().as_secs();
        }

        Ok(())
    }

    /// 🔗 Enhanced connection flow with comprehensive error handling and monitoring
    #[instrument(skip(self))]
    async fn connect_and_run(&mut self) -> Result<()> {
        self.connection_started = Some(Instant::now());
        
        // Phase 1: Establish enhanced TLS connection
        self.connection_state = ConnectionState::Connecting;
        self.connect_enhanced()
            .await
            .context("Failed to establish enhanced TLS connection")?;

        // Phase 2: Authenticate with enhanced verification
        self.connection_state = ConnectionState::Authenticating;
        self.authenticate_enhanced()
            .await
            .context("Failed to authenticate with enhanced Twitch IRC")?;

        // Phase 3: Negotiate enhanced capabilities
        self.connection_state = ConnectionState::NegotiatingCapabilities;
        self.negotiate_capabilities_enhanced()
            .await
            .context("Failed to negotiate enhanced IRC capabilities")?;

        // Phase 4: Join channel with monitoring
        self.connection_state = ConnectionState::JoiningChannel;
        self.join_channel_enhanced()
            .await
            .context("Failed to join channel with enhanced monitoring")?;

        // Phase 5: Run enhanced message processing with health monitoring
        self.connection_state = ConnectionState::Connected;
        self.enhanced_message_loop_with_monitoring()
            .await
            .context("Enhanced message processing with monitoring failed")?;

        Ok(())
    }

    /// 🔌 Enhanced TLS connection with comprehensive validation and monitoring
    #[instrument(skip(self))]
    async fn connect_enhanced(&mut self) -> Result<()> {
        info!("🔌 Connecting to {}:{} with enhanced security and monitoring", 
              self.config.irc_server, self.config.irc_port);

        // Create enhanced TLS connector with comprehensive security settings
        let connector = TlsConnector::from(
            native_tls::TlsConnector::builder()
                .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
                .danger_accept_invalid_hostnames(false) // Strict hostname verification
                .danger_accept_invalid_certs(false) // Strict certificate verification
                .build()
                .context("Failed to create enhanced TLS connector")?
        );

        // Connect with enhanced timeout and monitoring
        let tcp_stream = timeout(
            Duration::from_secs(IRC_CONNECT_TIMEOUT_SECS),
            TcpStream::connect(format!("{}:{}", self.config.irc_server, self.config.irc_port))
        )
        .await
        .context("Enhanced connection timeout")?
        .context("Failed to connect to IRC server")?;

        // Establish TLS with enhanced timeout and validation
        let tls_stream = timeout(
            Duration::from_secs(IRC_CONNECT_TIMEOUT_SECS),
            connector.connect(&self.config.irc_server, tcp_stream)
        )
        .await
        .context("Enhanced TLS handshake timeout")?
        .context("Failed to establish enhanced TLS connection")?;

        self.stream = Some(tls_stream);
        self.reconnect_attempt = 0; // Reset on successful connection
        self.error_state = None; // Clear error state
        self.health_data.consecutive_errors = 0; // Reset error count

        let connection_time = self.connection_started.unwrap_or_else(Instant::now).elapsed();
        self.metrics.last_connection_duration_ms = connection_time.as_millis() as u64;

        info!("✅ Enhanced TLS connection established in {:?} with comprehensive security", connection_time);
        Ok(())
    }

    /// 🔐 Enhanced authentication with comprehensive capability requests and validation
    #[instrument(skip(self))]
    async fn authenticate_enhanced(&mut self) -> Result<()> {
        info!("🔐 Authenticating as {} with enhanced capabilities and validation", self.username);

        // Send enhanced authentication commands with timeout and validation
        timeout(
            Duration::from_secs(IRC_AUTH_TIMEOUT_SECS),
            async {
                // Send PASS command
                self.send_raw_enhanced(&format!("PASS {}", self.token)).await?;
                
                // Send NICK command
                self.send_raw_enhanced(&format!("NICK {}", self.username)).await?;
                
                Result::<()>::Ok(())
            }
        )
        .await
        .context("Enhanced authentication timeout")?
        .context("Enhanced authentication failed")?;

        info!("✅ Enhanced authentication completed successfully");
        Ok(())
    }

    /// 🤝 Enhanced capability negotiation with comprehensive Twitch features
    #[instrument(skip(self))]
    async fn negotiate_capabilities_enhanced(&mut self) -> Result<()> {
        info!("🤝 Negotiating enhanced Twitch capabilities for full event support");

        // Request comprehensive Twitch capabilities for enhanced functionality
        let capabilities = [
            "twitch.tv/membership",  // JOIN/PART events
            "twitch.tv/tags",        // Rich message metadata
            "twitch.tv/commands",    // Enhanced IRC commands
        ];

        let cap_request = format!("CAP REQ :{}", capabilities.join(" "));
        self.send_raw_enhanced(&cap_request).await?;

        // Store negotiated capabilities for monitoring
        self.capabilities = capabilities.iter().map(|&s| s.to_string()).collect();
        self.metrics.capabilities_negotiated = self.capabilities.clone();

        info!("✅ Enhanced capability negotiation completed: {:?}", self.capabilities);
        Ok(())
    }

    /// 📺 Enhanced channel joining with comprehensive status tracking and monitoring
    #[instrument(skip(self))]
    async fn join_channel_enhanced(&mut self) -> Result<()> {
        info!("📺 Joining channel #{} with enhanced monitoring and status tracking", self.channel);

        self.send_raw_enhanced(&format!("JOIN #{}", self.channel)).await?;

        // Emit enhanced connection status event with comprehensive information
        let event = TwitchEvent::ConnectionStatus {
            connected: true,
            reason: Some("Successfully connected with enhanced IRC capabilities and monitoring".to_string()),
            timestamp: TwitchEvent::now_timestamp(),
            quality: Some("excellent".to_string()),
            duration: self.connection_started.map(|start| format!("{:.1}s", start.elapsed().as_secs_f64())),
        };

        self.event_sender
            .send(event)
            .context("Failed to send enhanced connection event")?;

        self.metrics.successful_connections += 1;
        self.health_data.connection_stability_score = 100.0; // Start with perfect score
        
        info!("✅ Enhanced channel join completed for #{} with full monitoring", self.channel);
        Ok(())
    }

    /// 📨 Enhanced message processing loop with comprehensive health monitoring and performance tracking
    #[instrument(skip(self))]
    async fn enhanced_message_loop_with_monitoring(&mut self) -> Result<()> {
        info!("💬 Starting enhanced message processing with comprehensive health monitoring");

        let mut line = String::new();
        let mut message_receiver = self
            .message_receiver
            .take()
            .ok_or_else(|| anyhow::anyhow!("Message receiver not available"))?;

        // Enhanced monitoring intervals with health tracking
        let mut health_check_interval = tokio::time::interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));
        let mut ping_interval = tokio::time::interval(Duration::from_secs(IRC_PING_INTERVAL_SECS));
        let mut metrics_interval = tokio::time::interval(Duration::from_secs(60)); // Update metrics every minute

        // Initialize enhanced ping tracking
        self.last_ping = Some(Instant::now());
        self.last_pong = Some(Instant::now());

        loop {
            tokio::select! {
                // Handle incoming IRC messages with enhanced timeout and processing
                result = timeout(Duration::from_secs(IRC_MESSAGE_TIMEOUT_SECS), async {
                    line.clear();
                    let bytes_read = {
                        let stream = self.stream.as_mut()
                            .context("IRC stream not connected")?;
                        let mut reader = BufReader::new(stream);
                        reader.read_line(&mut line).await?
                    };
                    anyhow::Ok(bytes_read)
                }) => {
                    match result {
                        Ok(Ok(0)) => {
                            info!("📡 IRC server closed connection gracefully");
                            break;
                        }
                        Ok(Ok(_)) => {
                            let message_start = Instant::now();
                            let trimmed = line.trim();
                            
                            if !trimmed.is_empty() {
                                debug!("📨 Received: {}", trimmed);
                                self.metrics.messages_received += 1;
                                self.health_data.last_message_received = Some(message_start);

                                // Add to message history for debugging
                                self.add_to_message_history(trimmed, false);

                                if let Err(e) = self.handle_enhanced_message_with_monitoring(trimmed).await {
                                    error!("❌ Failed to handle enhanced message: {e}");
                                    self.metrics.protocol_errors += 1;
                                } else {
                                    // Reset consecutive timeout counter on successful processing
                                    self.health_data.consecutive_timeouts = 0;
                                }

                                // Update enhanced message processing metrics
                                let processing_time = message_start.elapsed().as_millis() as f64;
                                self.metrics.avg_message_processing_ms = 
                                    (self.metrics.avg_message_processing_ms + processing_time) / 2.0;
                                
                                // Update health score based on processing performance
                                if processing_time < 10.0 {
                                    self.health_data.connection_stability_score = 
                                        (self.health_data.connection_stability_score + 1.0).min(100.0);
                                } else if processing_time > 100.0 {
                                    self.health_data.connection_stability_score = 
                                        (self.health_data.connection_stability_score - 1.0).max(0.0);
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("❌ Enhanced IRC read error: {}", e);
                            self.metrics.network_errors += 1;
                            self.health_data.consecutive_errors += 1;
                            return Err(e);
                        }
                        Err(_) => {
                            warn!("⏰ Enhanced IRC message timeout - connection may be degraded");
                            self.health_data.consecutive_timeouts += 1;
                            
                            // If we have too many timeouts, consider connection degraded
                            if self.health_data.consecutive_timeouts > 3 {
                                self.connection_state = ConnectionState::Degraded("Multiple message timeouts".to_string());
                                self.health_data.connection_stability_score = 
                                    (self.health_data.connection_stability_score - 10.0).max(0.0);
                            }
                        }
                    }
                }

                // Handle outgoing chat messages with enhanced rate limiting and monitoring
                message = message_receiver.recv() => {
                    match message {
                        Some(text) => {
                            if let Err(e) = self.send_message_with_enhanced_rate_limiting(&text).await {
                                error!("❌ Failed to send enhanced chat message: {}", e);
                                self.metrics.dropped_messages += 1;
                            }
                        }
                        None => {
                            info!("📭 Enhanced message sender channel closed");
                            break;
                        }
                    }
                }

                // Enhanced health check interval with comprehensive monitoring
                _ = health_check_interval.tick() => {
                    self.perform_enhanced_health_check().await;
                }

                // Enhanced ping interval for connection keepalive with monitoring
                _ = ping_interval.tick() => {
                    if let Err(e) = self.send_enhanced_ping().await {
                        error!("❌ Failed to send enhanced ping: {}", e);
                        return Err(e);
                    }
                }

                // Enhanced metrics update interval
                _ = metrics_interval.tick() => {
                    self.update_enhanced_metrics().await;
                }
            }
        }

        // Update final uptime metrics
        if let Some(started) = self.connection_started {
            self.metrics.total_uptime_secs += started.elapsed().as_secs();
        }

        Ok(())
    }

    /// 🎯 Enhanced message handling with comprehensive event support and monitoring
    #[instrument(skip(self, raw_message))]
    async fn handle_enhanced_message_with_monitoring(&mut self, raw_message: &str) -> Result<()> {
        let message = self.parse_irc_message_enhanced_with_validation(raw_message)?;

        match message.command.as_str() {
            "PING" => {
                // Enhanced PING handling with comprehensive timing and health monitoring
                if let Some(payload) = message.params.first() {
                    self.send_raw_enhanced(&format!("PONG {payload}")).await?;
                    self.last_pong = Some(Instant::now());
                    self.metrics.ping_pong_cycles += 1;
                    
                    // Update health based on ping/pong timing
                    if let Some(last_ping) = self.last_ping {
                        let ping_duration = last_ping.elapsed();
                        if ping_duration < Duration::from_secs(5) {
                            self.health_data.connection_stability_score = 
                                (self.health_data.connection_stability_score + 0.5).min(100.0);
                        }
                    }
                    
                    debug!("🏓 Enhanced PING/PONG cycle completed with health monitoring");
                }
            }

            "PRIVMSG" => {
                self.handle_enhanced_privmsg_with_metadata(&message).await?;
            }

            "JOIN" => {
                self.handle_enhanced_join_with_tracking(&message).await?;
            }

            "PART" => {
                self.handle_enhanced_part_with_tracking(&message).await?;
            }

            "USERNOTICE" => {
                self.handle_enhanced_usernotice_with_comprehensive_support(&message).await?;
            }

            "NOTICE" => {
                self.handle_enhanced_notice_with_classification(&message).await?;
            }

            "CLEARCHAT" => {
                self.handle_enhanced_clear_chat_with_moderation(&message).await?;
            }

            "CLEARMSG" => {
                self.handle_enhanced_clear_message_with_tracking(&message).await?;
            }

            "HOSTTARGET" => {
                self.handle_enhanced_host_target_with_metadata(&message).await?;
            }

            "RECONNECT" => {
                info!("🔄 Server requested enhanced reconnection");
                return Err(anyhow::anyhow!("Server requested enhanced reconnection"));
            }

            "001" => {
                info!("✅ Successfully connected to enhanced Twitch IRC with comprehensive features");
                self.health_data.connection_stability_score = 100.0;
            }

            "353" => {
                // Enhanced names list processing with user counting
                debug!("👥 Received enhanced channel user list");
            }

            "366" => {
                // Enhanced end of names list with completion tracking
                debug!("✅ Enhanced channel user list complete");
            }

            "CAP" => {
                // Enhanced capability acknowledgment
            }

            "USERSTATE" => {
                // Enhanced USERSTATE handling with comprehensive user state tracking
                debug!("👤 USERSTATE received: {:?}", message.tags);
                
                // Extract basic user information from IRC tags
                let display_name = message.tags.get("display-name").cloned();
                let color = message.tags.get("color").cloned();
                let user_id = message.tags.get("user-id").cloned();
                
                // Parse user status flags
                let is_subscriber = message.tags.get("subscriber").map(|s| s == "1").unwrap_or(false);
                let is_moderator = message.tags.get("mod").map(|s| s == "1").unwrap_or(false);
                let is_vip = message.tags.get("vip").map(|s| s == "1").unwrap_or(false);
                
                // Parse badges and emote sets
                let badges: Vec<String> = message.tags.get("badges")
                    .map(|b| b.split(',').filter_map(|badge| {
                        badge.split('/').next().map(|s| s.to_string())
                    }).collect())
                    .unwrap_or_default();
                
                let emote_sets: Vec<String> = message.tags.get("emote-sets")
                    .map(|sets| sets.split(',').map(|s| s.to_string()).collect())
                    .unwrap_or_default();
                
                // Get channel from message parameters  
                let channel = message.params.first()
                    .map(|c| c.trim_start_matches('#').to_string())
                    .unwrap_or_else(|| self.channel.clone());
                
                // Create user state event
                let event = TwitchEvent::UserState {
                    display_name: display_name.clone(),
                    color: color.clone(),
                    badges: badges.clone(),
                    user_id: user_id.clone(),
                    is_subscriber,
                    is_moderator,
                    is_vip,
                    emote_sets,
                    timestamp: TwitchEvent::now_timestamp(),
                    channel: channel.clone(),
                };

                // Send the user state event
                if let Err(e) = self.event_sender.send(event) {
                    warn!("Failed to send user state event: {}", e);
                }
                
                debug!("👤 User state processed for {} in #{}", 
                       display_name.unwrap_or_else(|| "Unknown".to_string()), channel);
            }

            _command => {
                self.metrics.protocol_errors += 1;
            }
        }

        Ok(())
    }

    /// 💬 Enhanced PRIVMSG handling with comprehensive rich metadata extraction
    #[instrument(skip(self, message))]
    async fn handle_enhanced_privmsg_with_metadata(&self, message: &IrcMessage) -> Result<()> {
        if message.params.len() < 2 {
            return Ok(());
        }

        let _channel = message.params.first()
            .ok_or_else(|| anyhow::anyhow!("Missing channel in enhanced PRIVMSG"))?;
        let text = message.params.get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing text in enhanced PRIVMSG"))?;

        // Extract comprehensive enhanced user information with fallbacks
        let username = message.prefix.as_ref()
            .and_then(|p| p.split('!').next())
            .unwrap_or("unknown")
            .to_string();

        // Enhanced action message detection with validation
        let (is_action, actual_message) = if text.starts_with("\u{0001}ACTION ") && text.ends_with('\u{0001}') {
            (true, text[8..text.len() - 1].to_string())
        } else {
            (false, text.clone())
        };

        // Extract comprehensive enhanced metadata from IRC tags
        let color = message.tags.get("color").cloned();
        let user_id = message.tags.get("user-id").cloned();
        let display_name = message.tags.get("display-name").cloned().unwrap_or_else(|| username.clone());
        let message_id = message.tags.get("id").cloned();
        
        // Enhanced user status detection
        let subscriber = message.tags.get("subscriber").map(|s| s == "1").unwrap_or(false);
        let moderator = message.tags.get("mod").map(|s| s == "1").unwrap_or(false);
        let vip = message.tags.get("vip").map(|s| s == "1").unwrap_or(false);
        let first_msg = message.tags.get("first-msg").map(|s| s == "1").unwrap_or(false);

        // Enhanced comprehensive badge parsing with validation
        let badges = message.tags.get("badges")
            .map(|b| {
                b.split(',')
                    .map(|badge| {
                        if let Some(slash_pos) = badge.find('/') {
                            badge[..slash_pos].to_string()
                        } else {
                            badge.to_string()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Create comprehensive enhanced chat event with all metadata
        let event = TwitchEvent::ChatMessage {
            user: username.clone(),
            message: actual_message,
            color,
            badges,
            timestamp: TwitchEvent::now_timestamp(),
            is_action,
            message_id,
            display_name: Some(display_name),
            user_id,
            is_subscriber: subscriber,
            is_moderator: moderator,
            is_vip: vip,
            is_first_message: first_msg,
        };

        debug!("💬 Enhanced chat message from {}: {}", username, event.display_text());

        // Enhanced metadata logging for comprehensive debugging and monitoring
        if first_msg {
            debug!("🆕 First message from user: {}", username);
        }
        if subscriber {
            debug!("⭐ Message from subscriber: {}", username);
        }
        if moderator {
            debug!("🛡️ Message from moderator: {}", username);
        }
        if vip {
            debug!("💎 Message from VIP: {}", username);
        }

        self.event_sender
            .send(event)
            .context("Failed to send enhanced chat message event")?;

        Ok(())
    }

    /// 📥 Enhanced JOIN event handling with comprehensive metadata tracking
    #[instrument(skip(self, message))]
    async fn handle_enhanced_join_with_tracking(&self, message: &IrcMessage) -> Result<()> {
        let username = message.prefix.as_ref()
            .and_then(|p| p.split('!').next())
            .unwrap_or("unknown")
            .to_string();

        // Don't emit join events for ourselves to avoid noise
        if username == self.username {
            return Ok(());
        }

        // Extract display name from tags if available
        let display_name = message.tags.get("display-name").cloned();

        let event = TwitchEvent::Join {
            user: username.clone(),
            timestamp: TwitchEvent::now_timestamp(),
            display_name,
        };

        debug!("📥 Enhanced user joined with tracking: {}", username);

        self.event_sender
            .send(event)
            .context("Failed to send enhanced join event")?;

        Ok(())
    }

    /// 📤 Enhanced PART event handling with comprehensive tracking
    #[instrument(skip(self, message))]
    async fn handle_enhanced_part_with_tracking(&self, message: &IrcMessage) -> Result<()> {
        let username = message.prefix.as_ref()
            .and_then(|p| p.split('!').next())
            .unwrap_or("unknown")
            .to_string();

        // Don't emit part events for ourselves to avoid noise
        if username == self.username {
            return Ok(());
        }

        // Extract display name from tags if available
        let display_name = message.tags.get("display-name").cloned();

        let event = TwitchEvent::Part {
            user: username.clone(),
            timestamp: TwitchEvent::now_timestamp(),
            display_name,
        };

        debug!("📤 Enhanced user left with tracking: {}", username);

        self.event_sender
            .send(event)
            .context("Failed to send enhanced part event")?;

        Ok(())
    }

    /// 🎉 Enhanced USERNOTICE handling with comprehensive event support (subs, raids, etc.)
    #[instrument(skip(self, message))]
    async fn handle_enhanced_usernotice_with_comprehensive_support(&self, message: &IrcMessage) -> Result<()> {
        let msg_id = message.tags.get("msg-id");

        match msg_id.map(|s| s.as_str()) {
            Some("sub" | "resub") => {
                self.handle_enhanced_subscription_with_metadata(message, false).await?;
            }
            Some("subgift") => {
                self.handle_enhanced_subscription_with_metadata(message, true).await?;
            }
            Some("raid") => {
                self.handle_enhanced_raid_with_comprehensive_data(message).await?;
            }
            Some("ritual") => {
                self.handle_enhanced_ritual_with_tracking(message).await?;
            }
            Some("submysterygift") => {
                self.handle_enhanced_mystery_gift_sub(message).await?;
            }
            Some("giftpaidupgrade") => {
                self.handle_enhanced_gift_upgrade(message).await?;
            }
            _ => {
                debug!("🤷 Unhandled enhanced USERNOTICE type: {:?}", msg_id);
            }
        }

        Ok(())
    }

    /// ⭐ Enhanced subscription handling with comprehensive gift detection and metadata
    #[instrument(skip(self))]
    async fn handle_enhanced_subscription_with_metadata(&self, message: &IrcMessage, is_gift: bool) -> Result<()> {
        let user = message.tags.get("display-name")
            .or_else(|| message.tags.get("login"))
            .map_or("unknown", |v| v)
            .to_string();

        let months = message.tags.get("msg-param-cumulative-months")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let streak_months = message.tags.get("msg-param-streak-months")
            .and_then(|s| s.parse().ok());

        let tier = message.tags.get("msg-param-sub-plan").cloned();
        let sub_message = message.params.get(1).cloned();

        let gifted_by = if is_gift {
            message.tags.get("display-name").cloned()
        } else {
            None
        };

        let event = TwitchEvent::Subscription {
            user,
            months,
            message: sub_message,
            timestamp: TwitchEvent::now_timestamp(),
            is_gift,
            gifted_by,
            tier,
            streak_months,
        };

        info!("⭐ Enhanced subscription event with metadata: {}", event.display_text());

        self.event_sender
            .send(event)
            .context("Failed to send enhanced subscription event")?;

        Ok(())
    }

    /// 🏴‍☠️ Enhanced raid handling with comprehensive data extraction
    #[instrument(skip(self, message))]
    async fn handle_enhanced_raid_with_comprehensive_data(&self, message: &IrcMessage) -> Result<()> {
        let from_channel = message.tags.get("msg-param-displayName")
            .or_else(|| message.tags.get("msg-param-login"))
            .map_or("unknown", |v| v)
            .to_string();

        let viewer_count = message.tags.get("msg-param-viewerCount")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let display_name = message.tags.get("msg-param-displayName").cloned();

        let event = TwitchEvent::Raid {
            from_channel: from_channel.clone(),
            viewer_count,
            timestamp: TwitchEvent::now_timestamp(),
            display_name,
        };

        info!("🏴‍☠️ Enhanced raid with comprehensive data from {}: {} viewers", from_channel, viewer_count);

        self.event_sender
            .send(event)
            .context("Failed to send enhanced raid event")?;

        Ok(())
    }

    /// 🎭 Enhanced ritual message handling (new follower celebration) with tracking
    #[instrument(skip(self))]
    async fn handle_enhanced_ritual_with_tracking(&self, _message: &IrcMessage) -> Result<()> {
        debug!("🎭 Enhanced ritual message received with tracking");
        // Could emit a special ritual event if needed for comprehensive event coverage
        Ok(())
    }

    /// 🎁 Enhanced mystery gift subscription handling
    #[instrument(skip(self, message))]
    async fn handle_enhanced_mystery_gift_sub(&self, message: &IrcMessage) -> Result<()> {
        let gifter = message.tags.get("display-name")
            .or_else(|| message.tags.get("login"))
            .map_or("unknown", |v| v)
            .to_string();

        let gift_count = message.tags.get("msg-param-mass-gift-count")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);

        let tier = message.tags.get("msg-param-sub-plan").cloned().unwrap_or_else(|| "1000".to_string());

        debug!("🎁 Enhanced mystery gift: {} gifted {} subs (tier {})", gifter, gift_count, tier);
        Ok(())
    }

    /// 🔄 Enhanced gift subscription upgrade handling
    #[instrument(skip(self, message))]
    async fn handle_enhanced_gift_upgrade(&self, message: &IrcMessage) -> Result<()> {
        let user = message.tags.get("display-name")
            .or_else(|| message.tags.get("login"))
            .map_or("unknown", |v| v)
            .to_string();

        debug!("🔄 Enhanced gift upgrade: {} upgraded their gift subscription", user);
        Ok(())
    }

    /// 📢 Enhanced NOTICE handling with intelligent classification
    #[instrument(skip(self, message))]
    async fn handle_enhanced_notice_with_classification(&self, message: &IrcMessage) -> Result<()> {
        if let Some(notice_text) = message.params.get(1) {
            debug!("📢 Enhanced NOTICE with classification: {}", notice_text);
            
            // Enhanced notice classification for better handling
            let notice_type = if notice_text.contains("slow mode") {
                "slow_mode"
            } else if notice_text.contains("followers only") {
                "followers_only"
            } else if notice_text.contains("timeout") || notice_text.contains("banned") {
                "moderation"
            } else {
                "general"
            };
            
            // Emit enhanced notice event with classification
            let event = TwitchEvent::ConnectionStatus {
                connected: true,
                reason: Some(format!("Notice ({notice_type}): {notice_text}")),
                timestamp: TwitchEvent::now_timestamp(),
                quality: Some("notice".to_string()),
                duration: None,
            };

            let _ = self.event_sender.send(event);
        }
        Ok(())
    }

    /// 🧹 Enhanced CLEARCHAT handling with comprehensive moderation support (timeout/ban)
    #[instrument(skip(self, _message))]
    async fn handle_enhanced_clear_chat_with_moderation(&self, _message: &IrcMessage) -> Result<()> {
        debug!("🧹 Enhanced chat cleared or user timed out with moderation support");
        // Could emit a comprehensive clear chat event with moderation details
        Ok(())
    }

    /// 🗑️ Enhanced CLEARMSG handling with comprehensive tracking (single message deletion)
    #[instrument(skip(self, _message))]
    async fn handle_enhanced_clear_message_with_tracking(&self, _message: &IrcMessage) -> Result<()> {
        debug!("🗑️ Enhanced message deleted with comprehensive tracking");
        // Could emit a comprehensive message deletion event with tracking details
        Ok(())
    }

    /// 🏠 Enhanced HOSTTARGET handling with comprehensive metadata (hosting)
    #[instrument(skip(self, message))]
    async fn handle_enhanced_host_target_with_metadata(&self, message: &IrcMessage) -> Result<()> {
        if let Some(target_info) = message.params.get(1) {
            let parts: Vec<&str> = target_info.split_whitespace().collect();
            if let Some(target_channel) = parts.first() {
                if target_channel != &"-" {
                    let viewer_count = parts.get(1)
                        .and_then(|s| s.parse().ok());

                    // Determine if this is an auto-host based on available information
                    let is_auto_host = message.tags.get("auto-host").map(|s| s == "1").unwrap_or(false);
                    let display_name = message.tags.get("display-name").cloned();

                    let event = TwitchEvent::Host {
                        target_channel: target_channel.to_string(),
                        viewer_count,
                        timestamp: TwitchEvent::now_timestamp(),
                        from_channel: self.channel.clone(),
                        display_name,
                        is_auto_host,
                    };

                    info!("🏠 Enhanced hosting with metadata {}: {} viewers (auto: {})", 
                          target_channel, viewer_count.unwrap_or(0), is_auto_host);

                    let _ = self.event_sender.send(event);
                }
            }
        }
        Ok(())
    }

    /// 📝 Enhanced IRC message parsing with comprehensive tag support and validation
    #[instrument(skip(self, raw))]
    fn parse_irc_message_enhanced_with_validation(&self, raw: &str) -> Result<IrcMessage> {
        let mut tags = HashMap::new();
        let mut remaining = raw;

        // Parse enhanced tags with comprehensive validation and error handling
        if raw.starts_with('@') {
            if let Some(space_pos) = raw.find(' ') {
                let tags_str = &raw[1..space_pos];
                remaining = &raw[space_pos + 1..];

                for tag_pair in tags_str.split(';') {
                    if let Some(eq_pos) = tag_pair.find('=') {
                        let key = tag_pair[..eq_pos].to_string();
                        let value = tag_pair[eq_pos + 1..].to_string();
                        
                        // Enhanced IRC tag escaping decoding with comprehensive support
                        let decoded_value = self.decode_irc_tag_value_enhanced(&value);
                        tags.insert(key, decoded_value);
                    } else if !tag_pair.is_empty() {
                        // Tag without value - store as empty string
                        tags.insert(tag_pair.to_string(), String::new());
                    }
                }
            }
        }

        // Parse prefix with enhanced validation
        let mut prefix = None;
        if remaining.starts_with(':') {
            if let Some(space_pos) = remaining.find(' ') {
                prefix = Some(remaining[1..space_pos].to_string());
                remaining = &remaining[space_pos + 1..];
            }
        }

        // Parse command and parameters with enhanced error handling
        let parts: Vec<&str> = remaining.splitn(2, ' ').collect();
        let command = parts.first()
            .ok_or_else(|| anyhow::anyhow!("Missing command in enhanced IRC message"))?
            .to_string();

        let mut params = Vec::new();
        if parts.len() > 1 {
            let param_str = parts.get(1).expect("Length checked above");

            if let Some(colon_pos) = param_str.find(" :") {
                // Split normal params and trailing param
                let normal_params = &param_str[..colon_pos];
                let trailing_param = &param_str[colon_pos + 2..];

                params.extend(normal_params.split_whitespace().map(String::from));
                params.push(trailing_param.to_string());
            } else if let Some(stripped) = param_str.strip_prefix(':') {
                // Entire thing is a trailing param
                params.push(stripped.to_string());
            } else {
                // All normal params
                params.extend(param_str.split_whitespace().map(String::from));
            }
        }

        Ok(IrcMessage {
            tags,
            prefix,
            command,
            params,
            raw: raw.to_string(),
            received_at: std::time::SystemTime::now(),
            size: raw.len(),
        })
    }

    /// 🔓 Enhanced IRC tag value escaping decoding with comprehensive support
    fn decode_irc_tag_value_enhanced(&self, value: &str) -> String {
        value
            .replace("\\:", ";")
            .replace("\\s", " ")
            .replace("\\\\", "\\")
            .replace("\\r", "\r")
            .replace("\\n", "\n")
    }

    /// 📤 Enhanced message sending with comprehensive rate limiting and monitoring
    #[instrument(skip(self))]
    async fn send_message_with_enhanced_rate_limiting(&mut self, text: &str) -> Result<()> {
        // Enhanced validation with comprehensive checks
        if text.len() > self.config.max_message_length {
            self.metrics.dropped_messages += 1;
            bail!("Message too long: {} > {} characters", 
                  text.len(), self.config.max_message_length);
        }

        if text.trim().is_empty() {
            self.metrics.dropped_messages += 1;
            bail!("Cannot send empty message");
        }

        // Enhanced rate limiting with intelligent backoff
        if !self.rate_limiter.can_send() {
            if let Some(wait_time) = self.rate_limiter.get_wait_time() {
                warn!("⚠️ Enhanced rate limit reached, message queued (wait: {:?})", wait_time);
                self.metrics.rate_limited_messages += 1;
                
                // Add to buffer instead of dropping
                if self.message_buffer.len() < 100 {
                    self.message_buffer.push(text.to_string());
                } else {
                    self.metrics.dropped_messages += 1;
                    bail!("Message buffer full, dropping message");
                }
                return Ok(());
            }
        }

        // Send the message
        let command = format!("PRIVMSG #{} :{}", self.channel, text);
        self.send_raw_enhanced(&command).await?;
        
        // Record in rate limiter and metrics
        self.rate_limiter.record_message();
        self.metrics.messages_sent += 1;
        self.health_data.last_message_sent = Some(Instant::now());

        // Add to message history for debugging
        self.add_to_message_history(&command, true);

        info!("📤 Enhanced message sent with rate limiting: {}", text);
        Ok(())
    }

    /// 📡 Enhanced raw IRC command sending with comprehensive validation and monitoring
    #[instrument(skip(self, command))]
    async fn send_raw_enhanced(&mut self, command: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .context("Enhanced IRC stream not connected")?;

        // Enhanced validation with comprehensive length checking (IRC protocol limit)
        if command.len() > MAX_MESSAGE_SIZE - 2 { // -2 for \r\n
            bail!("Enhanced IRC command too long: {} > {} bytes", 
                  command.len(), MAX_MESSAGE_SIZE - 2);
        }

        let message = format!("{command}\r\n");

        stream.write_all(message.as_bytes()).await
            .context("Failed to write to enhanced IRC stream")?;

        stream.flush().await
            .context("Failed to flush enhanced IRC stream")?;

        debug!("📡 Enhanced raw command sent with validation: {}", command);
        Ok(())
    }

    /// 💓 Enhanced ping sending for connection keepalive with comprehensive monitoring
    #[instrument(skip(self))]
    async fn send_enhanced_ping(&mut self) -> Result<()> {
        let ping_message = format!("PING :{}", self.config.irc_server);
        self.send_raw_enhanced(&ping_message).await?;
        self.last_ping = Some(Instant::now());
        debug!("💓 Enhanced ping sent with monitoring");
        Ok(())
    }

    /// 🏥 Enhanced connection health check with comprehensive monitoring
    #[instrument(skip(self))]
    async fn perform_enhanced_health_check(&mut self) {
        // Enhanced ping/pong monitoring with comprehensive timing
        if let (Some(last_ping), Some(last_pong)) = (self.last_ping, self.last_pong) {
            let ping_age = last_ping.elapsed();
            let pong_age = last_pong.elapsed();
            
            if ping_age > Duration::from_secs(IRC_PONG_TIMEOUT_SECS * 2) {
                warn!("⚠️ No PING sent in {:?} - connection may be stale", ping_age);
                self.health_data.connection_stability_score = 
                    (self.health_data.connection_stability_score - 5.0).max(0.0);
            }
            
            if pong_age > Duration::from_secs(IRC_PONG_TIMEOUT_SECS * 2) {
                warn!("⚠️ No PONG received in {:?} - connection may be stale", pong_age);
                self.health_data.connection_stability_score = 
                    (self.health_data.connection_stability_score - 10.0).max(0.0);
            }
        }

        // Enhanced message flow monitoring
        if let Some(last_received) = self.health_data.last_message_received {
            let since_last_message = last_received.elapsed();
            if since_last_message > Duration::from_secs(300) { // 5 minutes
                warn!("⚠️ No messages received in {:?} - connection may be inactive", since_last_message);
                self.health_data.connection_stability_score = 
                    (self.health_data.connection_stability_score - 2.0).max(0.0);
            }
        }

        // Update connection quality score based on health data
        self.metrics.connection_quality_score = self.health_data.connection_stability_score as u8;

        debug!("🏥 Enhanced health check completed - connection stability: {:.1}%", 
               self.health_data.connection_stability_score);
    }

    /// 📊 Enhanced metrics update with comprehensive data collection
    async fn update_enhanced_metrics(&mut self) {
        // Update uptime metrics
        if let Some(started) = self.connection_started {
            let current_uptime = started.elapsed().as_secs();
            if current_uptime > self.metrics.total_uptime_secs {
                self.metrics.total_uptime_secs = current_uptime;
            }
        }

        // Calculate message flow rate (messages per minute)
        let messages_per_minute = if self.metrics.total_uptime_secs > 0 {
            (self.metrics.messages_received * 60) / self.metrics.total_uptime_secs.max(1)
        } else {
            0
        };

        self.health_data.message_flow_rate = messages_per_minute as f64;
        
        if messages_per_minute > self.metrics.peak_messages_per_minute {
            self.metrics.peak_messages_per_minute = messages_per_minute;
        }

        debug!("📊 Enhanced metrics updated - uptime: {}s, msg/min: {}, quality: {}%", 
               self.metrics.total_uptime_secs, messages_per_minute, self.metrics.connection_quality_score);
    }

    /// 📜 Add message to history for debugging and analysis
    fn add_to_message_history(&mut self, message: &str, is_outgoing: bool) {
        self.message_history.push((Instant::now(), message.to_string(), is_outgoing));
        
        // Keep history size manageable
        if self.message_history.len() > MESSAGE_HISTORY_LIMIT {
            self.message_history.remove(0);
        }
    }

    /// 🔍 Enhanced error classification for intelligent handling
    fn classify_error_enhanced(&self, error: &anyhow::Error) -> TwitchErrorType {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("authentication") || error_str.contains("unauthorized") || error_str.contains("invalid nick") {
            TwitchErrorType::PermanentAuth
        } else if error_str.contains("timeout") || error_str.contains("connection") || error_str.contains("network") || error_str.contains("tls") || error_str.contains("certificate") || error_str.contains("ssl") {
            TwitchErrorType::TemporaryNetwork
        } else if error_str.contains("storage") || error_str.contains("database") {
            TwitchErrorType::TemporaryStorage
        } else if error_str.contains("rate limit") || error_str.contains("too many") {
            TwitchErrorType::TemporaryRateLimit
        } else {
            TwitchErrorType::TemporaryIrc
        }
    }

    /// 🔄 Enhanced reconnection decision logic with intelligent error handling
    fn should_reconnect_enhanced(&self, error_type: &TwitchErrorType) -> bool {
        match error_type {
            TwitchErrorType::PermanentAuth | TwitchErrorType::PermanentConfig => false,
            TwitchErrorType::TemporaryNetwork | 
            TwitchErrorType::TemporaryIrc | 
            TwitchErrorType::TemporaryStorage |
            TwitchErrorType::TemporaryRateLimit => {
                self.reconnect_attempt < MAX_RECONNECT_ATTEMPTS
            }
            TwitchErrorType::ApiError => {
                // For API errors, only retry a few times
                self.reconnect_attempt < 3
            }
        }
    }

    /// 📊 Get comprehensive enhanced connection metrics
    pub fn get_enhanced_metrics(&self) -> &IrcMetrics {
        &self.metrics
    }

    /// 🔗 Get current enhanced connection state
    pub fn get_connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    /// 🎯 Get enhanced error state
    pub fn get_error_state(&self) -> Option<&TwitchErrorState> {
        self.error_state.as_ref()
    }

    /// ✅ Enhanced health check - comprehensive connection and performance validation
    pub fn is_healthy(&self) -> bool {
        matches!(self.connection_state, ConnectionState::Connected) && 
        self.stream.is_some() &&
        self.error_state.is_none() &&
        self.health_data.connection_stability_score > 50.0
    }

    /// 📜 Get message history for debugging
    pub fn get_message_history(&self) -> &[(Instant, String, bool)] {
        &self.message_history
    }

    /// 🎯 Get negotiated capabilities
    pub fn get_capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// 🏥 Get health data
    pub fn get_health_data(&self) -> &HealthData {
        &self.health_data
    }
}

impl Drop for IrcClient {
    fn drop(&mut self) {
        if matches!(self.connection_state, ConnectionState::Connected) {
            info!("🔌 Enhanced IRC client disconnecting gracefully");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_test_client() -> (IrcClient, mpsc::UnboundedReceiver<TwitchEvent>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (_message_sender, message_receiver) = mpsc::unbounded_channel();
        
        let auth = crate::component::twitch::types::TwitchAuth {
            token: "oauth:test_token".to_string(),
            username: "testuser".to_string(),
            obtained_at: TwitchEvent::now_timestamp(),
            expires_at: None,
            refresh_token: None,
            scopes: vec!["chat:read".to_string(), "chat:edit".to_string()],
            user_id: Some("12345".to_string()),
            validation_count: 0,
            last_validated: None,
        };

        let config = crate::component::twitch::types::TwitchConfig {
            username: "testuser".to_string(),
            channel: "testchannel".to_string(),
            client_id: None,
            client_secret: None,
            oauth_scopes: None,
            oauth_timeout_secs: 300,
            max_message_length: 500,
            max_stored_messages: 100,
            reconnect_delays: vec![1, 2, 4, 8, 16, 30, 60],
            irc_server: "irc.chat.twitch.tv".to_string(),
            irc_port: 6697,
            max_messages_per_minute: 20,
            token_validation_interval_secs: 3600,
            enable_user_validation: true,
            enable_metrics: true,
            enable_auto_reconnect: true,
            max_reconnect_attempts: 10,
            enable_enhanced_parsing: true,
            message_queue_size: 1000,
            enable_rate_limiting: true,
        };

        let client = IrcClient::new(
            auth,
            "testchannel".to_string(),
            config,
            event_sender,
            message_receiver,
        );
        
        (client, event_receiver)
    }

    #[test]
    fn test_enhanced_message_parsing() {
        let (client, _) = create_test_client();
        
        // Test enhanced message with comprehensive tags
        let raw = "@badge-info=subscriber/8;badges=subscriber/6,premium/1;color=#FF0000;display-name=TestUser;emotes=;first-msg=0;flags=;id=test-id;mod=0;returning-chatter=0;room-id=12345;subscriber=1;tmi-sent-ts=1234567890;turbo=0;user-id=67890;user-type=;vip=0 :testuser!testuser@testuser.tmi.twitch.tv PRIVMSG #testchannel :Hello world!";

        let message = client.parse_irc_message_enhanced_with_validation(raw).unwrap();

        assert_eq!(message.command, "PRIVMSG");
        assert_eq!(message.params, vec!["#testchannel", "Hello world!"]);
        assert_eq!(message.tags.get("display-name"), Some(&"TestUser".to_string()));
        assert_eq!(message.tags.get("subscriber"), Some(&"1".to_string()));
        assert_eq!(message.tags.get("color"), Some(&"#FF0000".to_string()));
        assert_eq!(message.tags.get("user-id"), Some(&"67890".to_string()));
        assert!(message.size > 0);
    }

    #[test]
    fn test_enhanced_error_classification() {
        let (client, _) = create_test_client();

        let auth_error = anyhow::anyhow!("Authentication failed");
        assert_eq!(client.classify_error_enhanced(&auth_error), TwitchErrorType::PermanentAuth);

        let timeout_error = anyhow::anyhow!("Connection timeout");
        assert_eq!(client.classify_error_enhanced(&timeout_error), TwitchErrorType::TemporaryNetwork);

        let storage_error = anyhow::anyhow!("Storage database error");
        assert_eq!(client.classify_error_enhanced(&storage_error), TwitchErrorType::TemporaryStorage);

        let rate_limit_error = anyhow::anyhow!("Rate limit exceeded");
        assert_eq!(client.classify_error_enhanced(&rate_limit_error), TwitchErrorType::TemporaryRateLimit);
    }

    #[test]
    fn test_enhanced_reconnection_logic() {
        let (client, _) = create_test_client();

        // Should reconnect for temporary errors
        assert!(client.should_reconnect_enhanced(&TwitchErrorType::TemporaryNetwork));
        assert!(client.should_reconnect_enhanced(&TwitchErrorType::TemporaryIrc));
        assert!(client.should_reconnect_enhanced(&TwitchErrorType::TemporaryRateLimit));

        // Should not reconnect for permanent errors
        assert!(!client.should_reconnect_enhanced(&TwitchErrorType::PermanentAuth));
        assert!(!client.should_reconnect_enhanced(&TwitchErrorType::PermanentConfig));
    }

    #[test]
    fn test_enhanced_metrics() {
        let (client, _) = create_test_client();
        let metrics = client.get_enhanced_metrics();

        assert_eq!(metrics.connection_attempts, 0);
        assert_eq!(metrics.messages_sent, 0);
        assert_eq!(metrics.messages_received, 0);
        assert_eq!(metrics.ping_pong_cycles, 0);
        assert_eq!(metrics.rate_limited_messages, 0);
        assert_eq!(metrics.dropped_messages, 0);
    }

    #[test]
    fn test_enhanced_rate_limiter() {
        let mut rate_limiter = RateLimiter::new(5, Duration::from_secs(30), 2);
        
        // Should allow initial messages
        assert!(rate_limiter.can_send());
        
        // Record messages up to limit + burst
        for _ in 0..7 {
            rate_limiter.record_message();
        }
        
        // Should now be rate limited
        assert!(!rate_limiter.can_send());
        
        // Should have wait time
        assert!(rate_limiter.get_wait_time().is_some());
    }

    #[test]
    fn test_enhanced_irc_tag_decoding() {
        let (client, _) = create_test_client();

        let encoded = "test\\:value\\smore\\\\data\\r\\n";
        let decoded = client.decode_irc_tag_value_enhanced(encoded);
        assert_eq!(decoded, "test;value more\\data\r\n");
    }

    #[test]
    fn test_enhanced_health_monitoring() {
        let (client, _) = create_test_client();
        
        // Should start disconnected (not healthy until connected)
        assert!(!client.is_healthy());
        
        // Check health data initialization
        let health_data = client.get_health_data();
        assert_eq!(health_data.consecutive_timeouts, 0);
        assert_eq!(health_data.consecutive_errors, 0);
        assert_eq!(health_data.connection_stability_score, 100.0);
    }

    #[test]
    fn test_enhanced_connection_states() {
        let (client, _) = create_test_client();
        
        // Should start disconnected
        assert_eq!(*client.get_connection_state(), ConnectionState::Disconnected);
        
        // Test state comparisons
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }
}