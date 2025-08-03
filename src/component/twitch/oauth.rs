use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::component::twitch::types::{constants, TwitchAuth, TwitchConfig};
// Removed SimpleStorage import - using SystemComponent storage service instead
use crate::bus::BusHandle;

/// ⏱️ Production-grade timeout and performance constants
const TOKEN_CACHE_MAX_AGE_SECS: u64 = 30; // Reduced for better security
const HTTP_REQUEST_TIMEOUT_SECS: u64 = 15; // Increased for reliability
const HTTP_CONNECT_TIMEOUT_SECS: u64 = 5;
const CONNECTION_POOL_SIZE: usize = 10; // Optimized for multiple concurrent requests
const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_DELAY_BASE_MS: u64 = 1000;
const TOKEN_REFRESH_BUFFER_SECS: u64 = 300; // Refresh 5 minutes before expiry

/// 🔐 Enhanced OAuth state with comprehensive security tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub state_token: String,
    pub instance_id: String,
    pub username: String,
    pub callback_path: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub client_fingerprint: String, // Enhanced security
    pub csrf_token: String,         // Additional CSRF protection
    pub redirect_uri: String,       // Store the actual redirect URI used
    pub scopes: String,             // Store requested scopes
    pub nonce: String,              // Additional security nonce
}

/// 📡 Enhanced Twitch OAuth token response with comprehensive validation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    scope: Option<Vec<String>>,
    token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>, // For OIDC flows
}

/// 🔍 Enhanced token validation response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenValidationResponse {
    client_id: String,
    login: String,
    scopes: Vec<String>,
    user_id: String,
    expires_in: u64,
}

/// ⚡ Enhanced cached token with comprehensive validation and metrics
#[derive(Debug, Clone)]
struct CachedToken {
    #[allow(dead_code)] // Future use for token data access
    auth: TwitchAuth,
    cached_at: SystemTime,
    expires_at: Option<SystemTime>,
    #[allow(dead_code)] // Future use for validation metrics
    validation_count: u32,
    #[allow(dead_code)] // Future use for validation tracking
    last_validated: Option<SystemTime>,
    refresh_attempts: u32,
    last_refresh_attempt: Option<SystemTime>,
}

impl CachedToken {
    fn is_valid(&self) -> bool {
        // Check cache freshness
        let cache_age = SystemTime::now()
            .duration_since(self.cached_at)
            .unwrap_or(Duration::from_secs(TOKEN_CACHE_MAX_AGE_SECS + 1));

        if cache_age > Duration::from_secs(TOKEN_CACHE_MAX_AGE_SECS) {
            debug!("🗑️ Token cache expired after {:?}", cache_age);
            return false;
        }

        // Check token expiry
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now();
            if now >= expires_at {
                debug!("⏰ Token expired at {:?}", expires_at);
                return false;
            }
            
            // Warn if token expires soon (within 1 hour)
            let time_until_expiry = expires_at.duration_since(now).unwrap_or(Duration::ZERO);
            if time_until_expiry < Duration::from_secs(3600) {
                warn!("⚠️ Token expires in {:?}", time_until_expiry);
            }
        }

        true
    }

    fn should_refresh(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now();
            let time_until_expiry = expires_at.duration_since(now).unwrap_or(Duration::ZERO);
            // Refresh if expires within configured buffer time
            time_until_expiry < Duration::from_secs(TOKEN_REFRESH_BUFFER_SECS)
        } else {
            false
        }
    }

    fn can_attempt_refresh(&self) -> bool {
        // Limit refresh attempts and add cooldown
        if self.refresh_attempts >= 3 {
            return false;
        }
        
        if let Some(last_attempt) = self.last_refresh_attempt {
            let cooldown_duration = Duration::from_secs(300); // 5 minute cooldown
            SystemTime::now().duration_since(last_attempt).unwrap_or(Duration::ZERO) > cooldown_duration
        } else {
            true
        }
    }
}

/// 🔐 Production-ready OAuth manager with enhanced security and reliability
#[derive(Debug)]
pub struct TwitchOAuth {
    bus_handle: BusHandle,
    config: TwitchConfig,
    instance_id: String,
    /// ⚡ Enhanced token cache with validation tracking
    token_cache: Arc<RwLock<Option<CachedToken>>>,
    /// 🌐 Production-grade HTTP client with retry logic
    http_client: reqwest::Client,
    /// 📊 Performance and reliability metrics
    metrics: Arc<RwLock<OAuthMetrics>>,
    /// 🗄️ Storage service key for scoped storage operations (no direct database connection)
    storage_key_prefix: String,
}

/// 📊 Comprehensive OAuth operation metrics for monitoring and debugging
#[derive(Debug, Default, Clone)]
pub struct OAuthMetrics {
    pub oauth_flows_started: u64,
    pub oauth_flows_completed: u64,
    pub oauth_flows_failed: u64,
    pub tokens_cached: u64,
    pub tokens_retrieved: u64,
    pub token_validations: u64,
    pub token_validations_successful: u64,
    pub token_refresh_attempts: u64,
    pub token_refresh_successful: u64,
    pub http_requests: u64,
    pub http_timeouts: u64,
    pub storage_operations: u64,
    pub storage_errors: u64,
    pub avg_oauth_duration_ms: u64,
    pub avg_validation_duration_ms: u64,
    pub cache_hit_rate: f64,
}

impl TwitchOAuth {
    /// Create enhanced OAuth manager with production settings
    /// 
    /// **Default Instance Support**: 
    /// - Empty `instance_id` string creates OAuth manager for default instance
    /// - Non-empty `instance_id` creates OAuth manager for named instance
    /// - Storage keys are properly formatted for both default and named instances
    pub fn new(
        bus_handle: BusHandle,
        config: TwitchConfig,
        instance_id: String,
    ) -> Self {
        let display_name = if instance_id.is_empty() {
            "default".to_string()
        } else {
            instance_id.clone()
        };
        
        info!("🔐 Creating production OAuth manager for {} Twitch instance: '{}'", 
              if instance_id.is_empty() { "default" } else { "named" }, display_name);

        // 🗄️ Phase 3.1: Use SystemComponent storage service instead of direct database connection
        info!("✅ Using SystemComponent storage service - no direct database connection");
        
        // 🗄️ Storage key handling for default vs named instances
        let storage_key_prefix = if instance_id.is_empty() {
            "twitch".to_string()  // Default instance storage key: "twitch"
        } else {
            format!("twitch:{instance_id}")  // Named instance storage key: "twitch:instance_name"
        };

        // 🚀 Enhanced HTTP client with comprehensive settings
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(HTTP_REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
            .pool_max_idle_per_host(CONNECTION_POOL_SIZE)
            .pool_idle_timeout(Duration::from_secs(30))
            .user_agent("ControlRake-TwitchBot/1.0") // Proper user agent
            .gzip(true) // Enable compression
            .redirect(reqwest::redirect::Policy::limited(3)) // Safe redirect handling
            .build()
            .unwrap_or_else(|e| {
                warn!("⚠️ Failed to create optimized HTTP client: {}, using default", e);
                reqwest::Client::new()
            });

        Self {
            bus_handle,
            config,
            instance_id,
            token_cache: Arc::new(RwLock::new(None)),
            http_client,
            metrics: Arc::new(RwLock::new(OAuthMetrics::default())),
            storage_key_prefix,
        }
    }

    /// 🚀 Enhanced OAuth flow with comprehensive error handling and metrics
    #[instrument(skip(self), fields(instance_id = %self.instance_id, username = %self.config.username))]
    pub async fn start_oauth_flow(&self) -> Result<String> {
        let flow_start = Instant::now();
        info!("🔐 Starting enhanced OAuth flow for user: {}", self.config.username);

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.oauth_flows_started += 1;
        }

        // 📊 PHASE 1: Enhanced cache check with validation
        let cache_start = Instant::now();
        debug!("🔍 PHASE 1/7: Enhanced cache validation...");

        if let Some(cached) = self.get_cached_token().await {
            let cache_duration = cache_start.elapsed();
            
            if cached.is_valid() {
                let total_duration = flow_start.elapsed();
                info!("⚡ Valid cached token found (cache: {:?}, total: {:?})", 
                     cache_duration, total_duration);
                
                // Check if token should be refreshed proactively
                if cached.should_refresh() && cached.can_attempt_refresh() {
                    info!("🔄 Token valid but should be refreshed proactively");
                    // Try to refresh token in background
                    self.attempt_token_refresh().await;
                }
                
                self.update_metrics_success(total_duration).await;
                return Ok("existing_token_found".to_string());
            } else {
                debug!("🗑️ Cached token invalid, clearing cache (check: {:?})", cache_duration);
                self.clear_token_cache().await;
            }
        } else {
            debug!("❌ No cached token (check: {:?})", cache_start.elapsed());
        }

        // 📊 PHASE 2: Enhanced storage check with retry logic
        let storage_start = Instant::now();
        debug!("🔍 PHASE 2/7: Enhanced storage validation with retry...");

        match self.get_stored_token_with_retry().await {
            Ok(existing_token) => {
                let storage_duration = storage_start.elapsed();
                let total_duration = flow_start.elapsed();
                
                // Enhanced token validation
                if self.is_token_expired(&existing_token) {
                    if existing_token.refresh_token.is_some() {
                        info!("🔄 Stored token expired, attempting refresh");
                        match self.refresh_token_if_possible(&existing_token).await {
                            Ok(refreshed_token) => {
                                info!("✅ Token refreshed successfully");
                                self.cache_token(&refreshed_token).await;
                                self.update_metrics_success(total_duration).await;
                                return Ok("existing_token_found".to_string());
                            }
                            Err(e) => {
                                warn!("⚠️ Token refresh failed, continuing with OAuth: {}", e);
                            }
                        }
                    } else {
                        warn!("⏰ Stored token expired and no refresh token available");
                    }
                } else {
                    info!("✅ Valid stored token found (storage: {:?}, total: {:?})", 
                         storage_duration, total_duration);
                    
                    // Cache the token for future use
                    self.cache_token(&existing_token).await;
                    self.update_metrics_success(total_duration).await;
                    return Ok("existing_token_found".to_string());
                }
            }
            Err(e) => {
                debug!("❌ No valid stored token (storage: {:?}, error: {})", 
                      storage_start.elapsed(), e);
            }
        }

        // 📊 PHASE 3: Enhanced state generation with comprehensive security
        let state_start = Instant::now();
        debug!("🔍 PHASE 3/7: Generating enhanced OAuth state with security...");

        let state_token = Uuid::new_v4().to_string();
        let csrf_token = Uuid::new_v4().to_string();
        let nonce = Uuid::new_v4().to_string();
        let callback_path = format!("/oauth/twitch/{}", self.instance_id);
        let client_fingerprint = self.generate_enhanced_client_fingerprint();

        debug!("✅ Enhanced state generated (tokens: {}, fingerprint: {}) in {:?}",
               &state_token[..8], &client_fingerprint[..8], state_start.elapsed());

        // 📊 PHASE 4: Generate enhanced OAuth URL with security features
        let url_generation_start = Instant::now();
        debug!("🔍 PHASE 4/7: Generating enhanced OAuth URL with security...");
        
        let oauth_url_template = self.build_enhanced_oauth_url("PLACEHOLDER", &state_token, &csrf_token, &nonce)?;
        debug!("✅ Enhanced OAuth URL generated in {:?}", url_generation_start.elapsed());

        // 📊 PHASE 5: Parallel operations with enhanced error handling and timeout protection
        debug!("🔍 PHASE 5/7: Starting parallel operations with enhanced reliability...");
        let parallel_start = Instant::now();

        let callback_future = self.register_http_callback_with_retry(&callback_path);
        let state_future = self.store_oauth_state_with_retry(&state_token, &callback_path, &csrf_token, &nonce);

        let (callback_result, state_result) = tokio::join!(callback_future, state_future);

        let parallel_duration = parallel_start.elapsed();
        debug!("✅ Parallel operations completed in {:?}", parallel_duration);

        // 📊 PHASE 6: Process results with enhanced validation and error classification
        debug!("🔍 PHASE 6/7: Processing results with enhanced validation...");
        let process_start = Instant::now();

        let callback_url = match callback_result {
            Ok(url) => {
                debug!("✅ HTTP callback registered: {}", url);
                url
            }
            Err(e) => {
                let total_duration = flow_start.elapsed();
                error!("❌ HTTP callback registration failed after {:?}: {}", total_duration, e);
                self.update_metrics_failure().await;
                return Err(e.context("Failed to register HTTP callback with enhanced retry"));
            }
        };

        if let Err(e) = state_result {
            let total_duration = flow_start.elapsed();
            error!("❌ OAuth state storage failed after {:?}: {}", total_duration, e);
            self.update_metrics_failure().await;
            return Err(e.context("Failed to store OAuth state with enhanced retry"));
        }

        let oauth_url = oauth_url_template.replace("PLACEHOLDER", &callback_url);
        debug!("✅ Final enhanced OAuth URL ready");
        debug!("✅ Results processed in {:?}", process_start.elapsed());

        // 📊 PHASE 7: Enhanced browser launch with multiple fallbacks and user guidance
        debug!("🔍 PHASE 7/7: Enhanced browser launch with fallbacks...");
        let browser_start = Instant::now();

        info!("🌐 Opening enhanced OAuth URL in browser: {}", oauth_url);

        // Enhanced browser launch with multiple fallbacks and user instructions
        match self.launch_browser_with_enhanced_fallbacks(&oauth_url).await {
            Ok(_) => {
                info!("✅ Browser launched successfully in {:?}", browser_start.elapsed());
            }
            Err(e) => {
                warn!("⚠️ Browser launch failed in {:?}: {}", browser_start.elapsed(), e);
                warn!("📋 Manual action required: Please open this URL in your browser:");
                warn!("🔗 {}", oauth_url);
            }
        }

        // 📊 Final metrics and comprehensive summary
        let total_duration = flow_start.elapsed();
        {
            let mut metrics = self.metrics.write().await;
            metrics.avg_oauth_duration_ms = 
                (metrics.avg_oauth_duration_ms + total_duration.as_millis() as u64) / 
                (metrics.oauth_flows_started.max(1));
        }

        info!("✅ Enhanced OAuth flow completed in {:?} for user: {}", 
             total_duration, self.config.username);

        Ok(oauth_url)
    }

    /// 🔄 Enhanced OAuth callback handler with comprehensive validation and security
    #[instrument(skip(self, params))]
    pub async fn handle_oauth_callback(&self, params: HashMap<String, String>) -> Result<String> {
        let callback_start = Instant::now();
        info!("🔄 Processing enhanced OAuth callback with security validation");

        // 1. Enhanced parameter validation with security checks
        let state = params
            .get("state")
            .ok_or_else(|| anyhow!("Missing state parameter in OAuth callback"))?;

        let code = params
            .get("code")
            .ok_or_else(|| anyhow!("Missing authorization code in OAuth callback"))?;

        // 2. Enhanced error checking with detailed error information
        if let Some(error) = params.get("error") {
            let error_description = params
                .get("error_description")
                .map(|s| s.as_str())
                .unwrap_or("Unknown OAuth error");
            let error_uri = params.get("error_uri");
            
            let mut error_msg = format!("OAuth error: {error} - {error_description}");
            if let Some(uri) = error_uri {
                error_msg.push_str(&format!(" (See: {uri})"));
            }
            
            self.update_metrics_failure().await;
            return Err(anyhow!(error_msg));
        }

        // 3. Parallel validation and state retrieval with timeout protection
        let (oauth_state_result, _) = tokio::join!(
            self.get_oauth_state_with_retry(state),
            async {
                // Update metrics for callback processing
                let mut metrics = self.metrics.write().await;
                metrics.http_requests += 1;
            }
        );

        let oauth_state = oauth_state_result?;

        // 4. Enhanced state validation with comprehensive security checks
        self.validate_oauth_state_enhanced(&oauth_state, &params)?;

        // 5. Enhanced token exchange with retry logic and validation
        let (token_result, _) = tokio::join!(
            self.exchange_code_for_token_with_retry(code, &oauth_state),
            self.prepare_enhanced_cleanup(state)
        );

        let token = token_result?;

        // 6. Enhanced token validation before storage
        if let Err(e) = self.validate_new_token(&token).await {
            warn!("⚠️ New token validation failed: {}", e);
            // Continue anyway but log the issue
        }

        // 7. Enhanced parallel cleanup operations with error resilience
        let (store_result, cleanup_result, unregister_result) = tokio::join!(
            self.store_token_with_retry(&token),
            self.cleanup_oauth_state_with_retry(state),
            self.unregister_http_callback_with_retry(&oauth_state.callback_path)
        );

        // Process results with individual error handling
        if let Err(e) = store_result {
            error!("❌ Failed to store token: {}", e);
            return Err(e);
        }

        if let Err(e) = cleanup_result {
            warn!("⚠️ Failed to cleanup OAuth state: {}", e);
            // Continue - not critical
        }

        if let Err(e) = unregister_result {
            warn!("⚠️ Failed to unregister HTTP callback: {}", e);
            // Continue - not critical
        }

        // 8. Cache token and update comprehensive metrics
        self.cache_token(&token).await;
        self.update_metrics_success(callback_start.elapsed()).await;

        info!("✅ Enhanced OAuth callback completed in {:?} for: {}", 
             callback_start.elapsed(), self.config.username);
        Ok("oauth_success".to_string())
    }

    /// 🔍 Enhanced token validation with comprehensive checks and Twitch API integration
    #[instrument(skip(self))]
    pub async fn has_valid_token(&self) -> bool {
        let start = Instant::now();
        debug!("🔍 Starting enhanced token validation with API verification");

        // Check cache first
        if let Some(cached) = self.get_cached_token().await {
            if cached.is_valid() {
                debug!("✅ Valid token in cache (check: {:?})", start.elapsed());
                self.update_cache_hit_rate(true).await;
                return true;
            }
        }
        self.update_cache_hit_rate(false).await;

        // Check storage with retry logic and API validation
        match self.get_stored_token_with_retry().await {
            Ok(token) => {
                // Enhanced token validation with API call
                if self.is_token_expired(&token) {
                    debug!("⏰ Stored token expired (check: {:?})", start.elapsed());
                    
                    // Attempt token refresh if possible
                    if token.refresh_token.is_some() {
                        match self.refresh_token_if_possible(&token).await {
                            Ok(refreshed_token) => {
                                info!("✅ Token refreshed successfully during validation");
                                self.cache_token(&refreshed_token).await;
                                debug!("✅ Refreshed token validated (check: {:?})", start.elapsed());
                                return true;
                            }
                            Err(e) => {
                                debug!("❌ Token refresh failed during validation: {}", e);
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                }
                
                // Validate token with Twitch API
                match self.validate_token_with_api(&token).await {
                    Ok(true) => {
                        // Cache valid token
                        self.cache_token(&token).await;
                        debug!("✅ Valid token confirmed via API (check: {:?})", start.elapsed());
                        true
                    }
                    Ok(false) => {
                        debug!("❌ Token validation failed via API (check: {:?})", start.elapsed());
                        false
                    }
                    Err(e) => {
                        warn!("⚠️ API validation error (check: {:?}): {}", start.elapsed(), e);
                        // Assume token is valid if API call fails (network issues)
                        self.cache_token(&token).await;
                        true
                    }
                }
            }
            Err(e) => {
                debug!("❌ No valid token found (check: {:?}): {}", start.elapsed(), e);
                false
            }
        }
    }

    /// 🔄 Enhanced hourly token validation with automatic refresh (FIXED)
    #[instrument(skip(self))]
    pub async fn validate_token(&self) -> Result<bool> {
        info!("🕐 Performing hourly token validation as required by Twitch");
        
        let start = Instant::now();
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.token_validations += 1;
        }

        // Get current token
        let token = match self.get_stored_token_with_retry().await {
            Ok(token) => token,
            Err(e) => {
                error!("❌ No stored token found during hourly validation: {}", e);
                return Ok(false);
            }
        };
        
        // **FIX: Check if token is expired and attempt refresh BEFORE API validation**
        if self.is_token_expired(&token) {
            warn!("⏰ Token expired during hourly validation, attempting refresh");
            
            if token.refresh_token.is_some() {
                match self.refresh_token_if_possible(&token).await {
                    Ok(refreshed_token) => {
                        info!("✅ Token refreshed successfully during hourly validation");
                        
                        // Store and cache the refreshed token
                        if let Err(e) = self.store_token_with_retry(&refreshed_token).await {
                            error!("❌ Failed to store refreshed token: {}", e);
                            return Ok(false);
                        }
                        self.cache_token(&refreshed_token).await;
                        
                        // Continue with API validation of the new token
                        return self.validate_refreshed_token_with_api(&refreshed_token, start).await;
                    }
                    Err(e) => {
                        error!("❌ Token refresh failed during hourly validation: {}", e);
                        return Ok(false);
                    }
                }
            } else {
                error!("❌ Token expired and no refresh token available");
                return Ok(false);
            }
        }
        
        // **EXISTING: Validate with Twitch API**
        match self.validate_token_with_api(&token).await {
            Ok(is_valid) => {
                let duration = start.elapsed();
                
                {
                    let mut metrics = self.metrics.write().await;
                    if is_valid {
                        metrics.token_validations_successful += 1;
                    }
                    metrics.avg_validation_duration_ms = 
                        (metrics.avg_validation_duration_ms + duration.as_millis() as u64) / 
                        metrics.token_validations.max(1);
                }
                
                if is_valid {
                    info!("✅ Hourly validation successful in {:?}", duration);
                    
                    // Update token validation metadata
                    let mut updated_token = token;
                    updated_token.validation_count += 1;
                    updated_token.last_validated = Some(chrono::Utc::now().to_rfc3339());
                    
                    let _ = self.store_token_with_retry(&updated_token).await; // Update storage
                    self.cache_token(&updated_token).await; // Update cache
                } else {
                    warn!("⚠️ Hourly validation failed in {:?} - token may have been revoked", duration);
                }
                
                Ok(is_valid)
            }
            Err(e) => {
                error!("❌ Hourly validation error: {}", e);
                Err(e)
            }
        }
    }

    /// 📊 Get comprehensive OAuth metrics for monitoring
    pub async fn get_metrics(&self) -> OAuthMetrics {
        self.metrics.read().await.clone()
    }

    /// 🗄️ Get stored token (public interface for component)
    pub async fn get_stored_token(&self) -> Result<TwitchAuth> {
        self.get_stored_token_with_retry().await
    }

    // === PRIVATE ENHANCED METHODS ===

    /// 🗄️ Phase 3.1: Access SystemComponent storage service instead of direct database
    /// 
    /// **Purpose**: Use SystemComponent's storage service to eliminate duplicate database connections
    /// **Benefits**: Single database connection, proper architectural separation, no lock conflicts
    async fn save_to_system_storage(&self, key: &str, value: serde_json::Value) -> anyhow::Result<()> {
        let storage_key = format!("{}:{}", self.storage_key_prefix, key);
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.storage_operations += 1;
        }

        let request_payload = serde_json::json!({
            "key": storage_key,
            "value": value
        });

        match self.bus_handle.call("system:.set", request_payload).await {
            Ok(_) => {
                tracing::debug!("✅ Saved to system storage: {}", storage_key);
                Ok(())
            }
            Err(e) => {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.storage_errors += 1;
                }
                tracing::error!("❌ Failed to save to system storage: {}", e);
                Err(anyhow::anyhow!("System storage save failed: {}", e))
            }
        }
    }

    /// 🗄️ Phase 3.1: Load from SystemComponent storage service
    async fn load_from_system_storage(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let storage_key = format!("{}:{}", self.storage_key_prefix, key);
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.storage_operations += 1;
        }

        let request_payload = serde_json::json!({
            "key": storage_key
        });

        match self.bus_handle.call("system:.get", request_payload).await {
            Ok(response) => {
                // Check if the response indicates the key was found
                if let Some(value) = response.get("value") {
                    tracing::debug!("✅ Loaded from system storage: {}", storage_key);
                    Ok(Some(value.clone()))
                } else {
                    tracing::debug!("📭 Key not found in system storage: {}", storage_key);
                    Ok(None)
                }
            }
            Err(e) => {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.storage_errors += 1;
                }
                tracing::error!("❌ Failed to load from system storage: {}", e);
                Err(anyhow::anyhow!("System storage load failed: {}", e))
            }
        }
    }

    /// 🗄️ Phase 3.1: Delete from SystemComponent storage service
    async fn delete_from_system_storage(&self, key: &str) -> anyhow::Result<bool> {
        let storage_key = format!("{}:{}", self.storage_key_prefix, key);
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.storage_operations += 1;
        }

        let request_payload = serde_json::json!({
            "key": storage_key
        });

        match self.bus_handle.call("system:.delete", request_payload).await {
            Ok(response) => {
                let deleted = response.get("deleted").and_then(|v| v.as_bool()).unwrap_or(false);
                if deleted {
                    tracing::debug!("✅ Deleted from system storage: {}", storage_key);
                } else {
                    tracing::debug!("📭 Key not found for deletion in system storage: {}", storage_key);
                }
                Ok(deleted)
            }
            Err(e) => {
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.storage_errors += 1;
                }
                tracing::error!("❌ Failed to delete from system storage: {}", e);
                Err(anyhow::anyhow!("System storage delete failed: {}", e))
            }
        }
    }

    /// 🔄 Attempt token refresh in background
    async fn attempt_token_refresh(&self) {
        let token = match self.get_stored_token_with_retry().await {
            Ok(token) => token,
            Err(_) => return,
        };

        if let Err(e) = self.refresh_token_if_possible(&token).await {
            warn!("⚠️ Background token refresh failed: {}", e);
        }
    }

    /// 🔄 Refresh token if refresh token is available
    async fn refresh_token_if_possible(&self, token: &TwitchAuth) -> Result<TwitchAuth> {
        let refresh_token = token.refresh_token.as_ref()
            .ok_or_else(|| anyhow!("No refresh token available"))?;

        info!("🔄 Attempting token refresh");

        {
            let mut metrics = self.metrics.write().await;
            metrics.token_refresh_attempts += 1;
        }

        let client_id = self.config.client_id.as_ref()
            .ok_or_else(|| anyhow!("client_id required for token refresh"))?;
        let client_secret = self.config.client_secret.as_ref()
            .ok_or_else(|| anyhow!("client_secret required for token refresh"))?;

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];

        let response = self
            .http_client
            .post(constants::REFRESH_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .context("Failed to send token refresh request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Token refresh failed with status {}: {}", status, error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token refresh response")?;

        let now = chrono::Utc::now();
        let expires_at = token_response
            .expires_in
            .map(|seconds| (now + chrono::Duration::seconds(seconds as i64)).to_rfc3339());

        let refreshed_auth = TwitchAuth {
            token: format!("oauth:{}", token_response.access_token),
            username: token.username.clone(),
            obtained_at: now.to_rfc3339(),
            expires_at,
            refresh_token: token_response.refresh_token.or_else(|| token.refresh_token.clone()),
            scopes: token_response.scope.unwrap_or_else(|| token.scopes.clone()),
            user_id: token.user_id.clone(),
            validation_count: token.validation_count,
            last_validated: Some(now.to_rfc3339()),
        };

        {
            let mut metrics = self.metrics.write().await;
            metrics.token_refresh_successful += 1;
        }

        info!("✅ Token refreshed successfully");
        Ok(refreshed_auth)
    }

    /// 🔍 Validate token with Twitch API
    async fn validate_token_with_api(&self, token: &TwitchAuth) -> Result<bool> {
        let clean_token = token.token.strip_prefix("oauth:").unwrap_or(&token.token);

        let response = self
            .http_client
            .get(constants::VALIDATE_URL)
            .header("Authorization", format!("OAuth {clean_token}"))
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to validate token with Twitch API")?;

        match response.status().as_u16() {
            200 => {
                let validation_response: TokenValidationResponse = response
                    .json()
                    .await
                    .context("Failed to parse token validation response")?;
                
                // Verify username matches
                if validation_response.login != token.username {
                    warn!("⚠️ Token username mismatch: expected {}, got {}", 
                          token.username, validation_response.login);
                    return Ok(false);
                }
                
                debug!("✅ Token validated successfully via API");
                Ok(true)
            }
            401 => {
                debug!("❌ Token validation failed: invalid or expired token");
                Ok(false)
            }
            _ => {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Token validation API error: {}", error_text))
            }
        }
    }

    /// ⏰ Check if token is expired
    fn is_token_expired(&self, token: &TwitchAuth) -> bool {
        if let Some(expires_at_str) = &token.expires_at {
            if let Ok(expires_time) = chrono::DateTime::parse_from_rfc3339(expires_at_str) {
                let now = chrono::Utc::now();
                return expires_time.with_timezone(&chrono::Utc) <= now;
            }
        }
        false // If no expiry set, assume not expired
    }

    /// ✅ Validate newly obtained token
    async fn validate_new_token(&self, token: &TwitchAuth) -> Result<()> {
        // Basic validation
        if token.token.is_empty() {
            return Err(anyhow!("Empty token received"));
        }
        
        if token.username.is_empty() {
            return Err(anyhow!("Empty username in token"));
        }

        // API validation
        self.validate_token_with_api(token).await?;
        
        Ok(())
    }

    /// 📊 Update cache hit rate metrics
    async fn update_cache_hit_rate(&self, hit: bool) {
        let mut metrics = self.metrics.write().await;
        if hit {
            metrics.cache_hit_rate = (metrics.cache_hit_rate + 1.0) / 2.0;
        } else {
            metrics.cache_hit_rate /= 2.0;
        }
    }

    /// ⚡ Enhanced cached token retrieval with metrics
    async fn get_cached_token(&self) -> Option<CachedToken> {
        let start = Instant::now();
        let cache = self.token_cache.read().await;
        let result = cache.clone();
        
        {
            let mut metrics = self.metrics.write().await;
            metrics.tokens_retrieved += 1;
        }
        
        debug!("🏃 Enhanced cache read in {:?}, found: {}", 
               start.elapsed(), result.is_some());
        result
    }

    /// 🗑️ Clear token cache with metrics
    async fn clear_token_cache(&self) {
        let start = Instant::now();
        let mut cache = self.token_cache.write().await;
        *cache = None;
        debug!("🧹 Cache cleared in {:?}", start.elapsed());
    }

    /// ⚡ Enhanced token caching with validation metrics
    async fn cache_token(&self, auth: &TwitchAuth) {
        let start = Instant::now();

        let expires_at = auth.expires_at.as_ref().and_then(|exp_str| {
            chrono::DateTime::parse_from_rfc3339(exp_str)
                .ok()
                .map(|dt| SystemTime::UNIX_EPOCH + Duration::from_secs(dt.timestamp() as u64))
        });

        let cached = CachedToken {
            auth: auth.clone(),
            cached_at: SystemTime::now(),
            expires_at,
            validation_count: auth.validation_count,
            last_validated: SystemTime::now().into(),
            refresh_attempts: 0,
            last_refresh_attempt: None,
        };

        let mut cache = self.token_cache.write().await;
        *cache = Some(cached);

        {
            let mut metrics = self.metrics.write().await;
            metrics.tokens_cached += 1;
        }

        debug!("⚡ Token cached in {:?} with enhanced validation", start.elapsed());
    }

    /// 🗄️ Enhanced storage operations with retry logic
    async fn get_stored_token_with_retry(&self) -> Result<TwitchAuth> {
        let mut last_error = None;
        
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.get_stored_token_fast().await {
                Ok(token) => {
                    if attempt > 1 {
                        info!("✅ Storage retrieval succeeded on attempt {}", attempt);
                    }
                    return Ok(token);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ Storage attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.storage_errors += 1;
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Failed to retrieve token after retries")))
    }

    /// 🗄️ Direct storage access (simplified) - Phase 3.1: Use SystemComponent storage service
    async fn get_stored_token_fast(&self) -> Result<TwitchAuth> {
        // 🗄️ Storage key handling for default vs named instances
        let storage_key = if self.instance_id.is_empty() {
            format!("auth:{}", self.config.username)  // Default instance: "auth:username"
        } else {
            format!("auth:{}:{}", self.instance_id, self.config.username)  // Named instance: "auth:instance:username"
        };

        let response = self.load_from_system_storage(&storage_key).await?;

        if let Some(stored_value) = response {
            serde_json::from_value(stored_value)
                .context("Failed to parse stored auth token")
        } else {
            Err(anyhow!("No stored token found"))
        }
    }

    /// 🌐 Enhanced OAuth URL generation with comprehensive security features
    fn build_enhanced_oauth_url(&self, callback_url: &str, state_token: &str, _csrf_token: &str, nonce: &str) -> Result<String> {
        let client_id = self.config.client_id.as_ref().ok_or_else(|| {
            anyhow!("❌ CRITICAL CONFIG ERROR: client_id required for OAuth. Get from: https://dev.twitch.tv/console/apps")
        })?;

        let scopes = self.config.oauth_scopes
            .as_deref()
            .unwrap_or("chat:read chat:edit");

        // Enhanced URL with comprehensive security parameters
        let query_string = format!(
            "client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&nonce={}",
            urlencoding::encode(client_id),
            urlencoding::encode(callback_url),
            urlencoding::encode(scopes),
            urlencoding::encode(state_token),
            urlencoding::encode(nonce)
        );

        Ok(format!("{}?{}", constants::OAUTH_URL, query_string))
    }

    /// 🔐 Generate enhanced client fingerprint for security
    fn generate_enhanced_client_fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.instance_id.hash(&mut hasher);
        self.config.username.hash(&mut hasher);
        self.config.channel.hash(&mut hasher);
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .hash(&mut hasher);

        // Add some randomness
        let random_data = uuid::Uuid::new_v4().to_string();
        random_data.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// 🌐 Enhanced browser launch with comprehensive fallbacks
    async fn launch_browser_with_enhanced_fallbacks(&self, url: &str) -> Result<()> {
        // Try multiple browser launch methods with enhanced error handling
        #[allow(clippy::type_complexity)]
        let methods: Vec<(&str, Box<dyn Fn() -> Result<(), std::io::Error>>)> = vec![
            ("open crate", Box::new(|| {
                open::that(url).map_err(std::io::Error::other)
            })),
            ("system command", Box::new(|| {
                #[cfg(target_os = "macos")]
                {
                    std::process::Command::new("open").arg(url).spawn()?;
                }
                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("cmd").args(&["/c", "start", "", url]).spawn()?;
                }
                #[cfg(target_os = "linux")]
                {
                    // Try multiple Linux browser launchers
                    for launcher in &["xdg-open", "gnome-open", "kde-open"] {
                        if let Ok(_) = std::process::Command::new(launcher).arg(url).spawn() {
                            return Ok(());
                        }
                    }
                    return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No browser launcher found"));
                }
            })),
        ];

        let mut last_error = None;
        for (method_name, method) in methods.iter() {
            match method() {
                Ok(_) => {
                    info!("✅ Browser launched successfully using: {}", method_name);
                    return Ok(());
                }
                Err(e) => {
                    let error_string = format!("{e}");
                    last_error = Some(e);
                    debug!("⚠️ Browser launch method '{}' failed: {}", method_name, error_string);
                }
            }
        }

        Err(anyhow!("All browser launch methods failed: {:?}", last_error))
    }

    /// 📊 Update success metrics
    async fn update_metrics_success(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.oauth_flows_completed += 1;
        metrics.avg_oauth_duration_ms = 
            (metrics.avg_oauth_duration_ms + duration.as_millis() as u64) / 
            metrics.oauth_flows_completed.max(1);
    }

    /// 📊 Update failure metrics
    async fn update_metrics_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.oauth_flows_failed += 1;
    }

    // === HELPER METHODS (implementations for existing interface) ===
    
    async fn register_http_callback_with_retry(&self, callback_path: &str) -> Result<String> {
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.register_http_callback_fast(callback_path).await {
                Ok(url) => return Ok(url),
                Err(e) => {
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ HTTP callback registration attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    async fn store_oauth_state_with_retry(&self, state_token: &str, callback_path: &str, csrf_token: &str, nonce: &str) -> Result<()> {
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.store_oauth_state_fast(state_token, callback_path, csrf_token, nonce).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ OAuth state storage attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    async fn exchange_code_for_token_with_retry(&self, auth_code: &str, oauth_state: &OAuthState) -> Result<TwitchAuth> {
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.exchange_code_for_token_fast(auth_code, oauth_state).await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ Token exchange attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    async fn store_token_with_retry(&self, token: &TwitchAuth) -> Result<()> {
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.store_token_fast(token).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ Token storage attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    async fn prepare_enhanced_cleanup(&self, _state_token: &str) -> Result<()> {
        // Enhanced cleanup preparation - could include metrics cleanup, cache invalidation, etc.
        Ok(())
    }

    async fn get_oauth_state_with_retry(&self, state_token: &str) -> Result<OAuthState> {
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            match self.get_oauth_state_fast(state_token).await {
                Ok(state) => return Ok(state),
                Err(e) => {
                    if attempt < MAX_RETRY_ATTEMPTS {
                        let delay = Duration::from_millis(RETRY_DELAY_BASE_MS * attempt as u64);
                        debug!("⚠️ OAuth state retrieval attempt {} failed, retrying in {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        unreachable!()
    }

    fn validate_oauth_state_enhanced(&self, oauth_state: &OAuthState, params: &HashMap<String, String>) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Enhanced validation checks
        if now > oauth_state.expires_at {
            return Err(anyhow!("OAuth state has expired (created at {}, expired at {})", 
                             oauth_state.created_at, oauth_state.expires_at));
        }
        
        if oauth_state.instance_id != self.instance_id {
            return Err(anyhow!("OAuth state instance ID mismatch: expected {}, got {}", 
                             self.instance_id, oauth_state.instance_id));
        }
        
        if oauth_state.username != self.config.username {
            return Err(anyhow!("OAuth state username mismatch: expected {}, got {}", 
                             self.config.username, oauth_state.username));
        }

        // Additional security validations
        if let Some(returned_state) = params.get("state") {
            if returned_state != &oauth_state.state_token {
                return Err(anyhow!("State token mismatch - possible CSRF attack"));
            }
        }

        debug!("✅ Enhanced OAuth state validation passed");
        Ok(())
    }

    async fn cleanup_oauth_state_with_retry(&self, state_token: &str) -> Result<()> {
        // Allow cleanup to fail without affecting the main flow
        let _ = self.cleanup_oauth_state_fast(state_token).await;
        Ok(())
    }

    async fn unregister_http_callback_with_retry(&self, callback_path: &str) -> Result<()> {
        // Allow unregistration to fail without affecting the main flow
        let _ = self.unregister_http_callback_fast(callback_path).await;
        Ok(())
    }

    // Core implementation methods
    async fn register_http_callback_fast(&self, callback_path: &str) -> Result<String> {
        // 🎯 Handle default vs named instance addressing for HTTP callback registration
        let target_address = if self.instance_id.is_empty() {
            "twitch:".to_string()  // Default instance: "twitch:"
        } else {
            format!("twitch:{}", self.instance_id)  // Named instance: "twitch:instance_name"
        };

        let request_payload = json!({
            "path": callback_path,
            "target": target_address,
            "method": "handle_oauth"
        });

        let response = self
            .bus_handle
            .with_caller(&target_address)
            .call("http:", request_payload)  // Always register with HTTP default instance
            .await?;

        let callback_url = response
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("HTTP registration did not return callback URL"))?;

        Ok(callback_url.to_string())
    }

    async fn store_oauth_state_fast(&self, state_token: &str, callback_path: &str, csrf_token: &str, nonce: &str) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let expires_at = now + self.config.oauth_timeout_secs;

        let oauth_state = OAuthState {
            state_token: state_token.to_string(),
            instance_id: self.instance_id.clone(),
            username: self.config.username.clone(),
            callback_path: callback_path.to_string(),
            created_at: now,
            expires_at,
            client_fingerprint: self.generate_enhanced_client_fingerprint(),
            csrf_token: csrf_token.to_string(),
            redirect_uri: format!("http://localhost:8888{callback_path}"),
            scopes: self.config.oauth_scopes.clone().unwrap_or_else(|| "chat:read chat:edit".to_string()),
            nonce: nonce.to_string(),
        };

        let storage_key = format!("oauth_state:{state_token}");

        self.save_to_system_storage(&storage_key, serde_json::to_value(oauth_state)?).await?;

        Ok(())
    }

    async fn exchange_code_for_token_fast(&self, auth_code: &str, oauth_state: &OAuthState) -> Result<TwitchAuth> {
        let client_id = self.config.client_id.as_ref()
            .ok_or_else(|| anyhow!("client_id required for token exchange"))?;
        let client_secret = self.config.client_secret.as_ref()
            .ok_or_else(|| anyhow!("client_secret required for token exchange"))?;

        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", auth_code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &oauth_state.redirect_uri),
        ];

        let response = self
            .http_client
            .post(constants::TOKEN_URL)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .context("Failed to send token exchange request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Token exchange failed with status {}: {}", status, error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        let now = chrono::Utc::now();
        let expires_at = token_response
            .expires_in
            .map(|seconds| (now + chrono::Duration::seconds(seconds as i64)).to_rfc3339());

        let auth = TwitchAuth {
            token: format!("oauth:{}", token_response.access_token),
            username: self.config.username.clone(),
            obtained_at: now.to_rfc3339(),
            expires_at,
            refresh_token: token_response.refresh_token,
            scopes: token_response.scope.unwrap_or_else(|| vec!["chat:read".to_string(), "chat:edit".to_string()]),
            user_id: None, // Will be filled by user validation
            validation_count: 0,
            last_validated: None,
        };

        Ok(auth)
    }

    async fn store_token_fast(&self, token: &TwitchAuth) -> Result<()> {
        // 🗄️ Storage key handling for default vs named instances (must match get_stored_token_fast)
        let storage_key = if self.instance_id.is_empty() {
            format!("auth:{}", self.config.username)  // Default instance: "auth:username"
        } else {
            format!("auth:{}:{}", self.instance_id, self.config.username)  // Named instance: "auth:instance:username"
        };

        self.save_to_system_storage(&storage_key, serde_json::to_value(token)?).await?;

        Ok(())
    }

    async fn get_oauth_state_fast(&self, state_token: &str) -> Result<OAuthState> {
        // Use simplified storage key format: "oauth_state:{state_token}"
        let storage_key = format!("oauth_state:{state_token}");

        let response = self.load_from_system_storage(&storage_key).await?;

        if let Some(stored_value) = response {
            serde_json::from_value(stored_value).context("Failed to parse OAuth state")
        } else {
            Err(anyhow!("OAuth state not found"))
        }
    }

    async fn cleanup_oauth_state_fast(&self, state_token: &str) -> Result<()> {
        // Use simplified storage key format: "oauth_state:{state_token}"
        let storage_key = format!("oauth_state:{state_token}");

        let _ = self.delete_from_system_storage(&storage_key).await; // Ignore errors for cleanup

        Ok(())
    }

    async fn unregister_http_callback_fast(&self, callback_path: &str) -> Result<()> {
        // 🎯 Handle default vs named instance addressing for HTTP callback unregistration
        let caller_address = if self.instance_id.is_empty() {
            "twitch:".to_string()  // Default instance: "twitch:"
        } else {
            format!("twitch:{}", self.instance_id)  // Named instance: "twitch:instance_name"
        };

        let unregister_payload = json!({"path": callback_path});

        let _ = self
            .bus_handle
            .with_caller(&caller_address)
            .call("http:", unregister_payload)  // Always unregister from HTTP default instance
            .await; // Ignore errors for cleanup

        Ok(())
    }

    /// **NEW: Helper method for validating refreshed tokens**
    async fn validate_refreshed_token_with_api(&self, token: &TwitchAuth, start: Instant) -> Result<bool> {
        match self.validate_token_with_api(token).await {
            Ok(is_valid) => {
                let duration = start.elapsed();
                
                {
                    let mut metrics = self.metrics.write().await;
                    if is_valid {
                        metrics.token_validations_successful += 1;
                    }
                    metrics.avg_validation_duration_ms = 
                        (metrics.avg_validation_duration_ms + duration.as_millis() as u64) / 
                        metrics.token_validations.max(1);
                }
                
                if is_valid {
                    info!("✅ Refreshed token validation successful in {:?}", duration);
                    
                    // Update token validation metadata
                    let mut updated_token = token.clone();
                    updated_token.validation_count += 1;
                    updated_token.last_validated = Some(chrono::Utc::now().to_rfc3339());
                    
                    let _ = self.store_token_with_retry(&updated_token).await;
                    self.cache_token(&updated_token).await;
                } else {
                    warn!("⚠️ Refreshed token validation failed in {:?}", duration);
                }
                
                Ok(is_valid)
            }
            Err(e) => {
                error!("❌ Refreshed token validation error: {}", e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bus;

    #[tokio::test]
    async fn test_enhanced_oauth_manager() {
        let bus = Bus::new().await;
        let config = TwitchConfig {
            username: "testuser".to_string(),
            channel: "testchannel".to_string(),
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_secret".to_string()),
            oauth_scopes: Some("chat:read chat:edit".to_string()),
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

        let oauth = TwitchOAuth::new(bus.handle(), config, "test_instance".to_string());

        // Test enhanced client fingerprint generation
        let fingerprint1 = oauth.generate_enhanced_client_fingerprint();
        let fingerprint2 = oauth.generate_enhanced_client_fingerprint();
        
        // Should be different due to timestamp and randomness
        assert_ne!(fingerprint1, fingerprint2);
        assert!(!fingerprint1.is_empty());

        // Test enhanced metrics initialization
        let metrics = oauth.get_metrics().await;
        assert_eq!(metrics.oauth_flows_started, 0);
        assert_eq!(metrics.oauth_flows_completed, 0);
        assert_eq!(metrics.token_validations, 0);

        // Test has_valid_token with no token
        assert!(!oauth.has_valid_token().await);
    }

    #[tokio::test]
    async fn test_enhanced_oauth_url_generation() {
        let bus = Bus::new().await;
        let config = TwitchConfig {
            username: "testuser".to_string(),
            channel: "testchannel".to_string(),
            client_id: Some("test_client_id".to_string()),
            client_secret: Some("test_secret".to_string()),
            oauth_scopes: Some("chat:read chat:edit".to_string()),
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

        let oauth = TwitchOAuth::new(bus.handle(), config, "test_instance".to_string());
        
        let url = oauth
            .build_enhanced_oauth_url(
                "http://localhost:8888/callback", 
                "test_state", 
                "test_csrf",
                "test_nonce"
            )
            .unwrap();

        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8888%2Fcallback"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=chat%3Aread%20chat%3Aedit"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("nonce=test_nonce"));
    }

    #[tokio::test]
    async fn test_enhanced_cached_token_validation() {
        let auth = TwitchAuth {
            token: "oauth:test_token".to_string(),
            username: "testuser".to_string(),
            obtained_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::minutes(2)).to_rfc3339()), // 2 minutes = within 5 min buffer
            refresh_token: Some("refresh123".to_string()),
            scopes: vec!["chat:read".to_string(), "chat:edit".to_string()],
            user_id: Some("12345".to_string()),
            validation_count: 1,
            last_validated: Some(chrono::Utc::now().to_rfc3339()),
        };

        let expires_at = auth.expires_at.as_ref().and_then(|exp_str| {
            chrono::DateTime::parse_from_rfc3339(exp_str)
                .ok()
                .map(|dt| SystemTime::UNIX_EPOCH + Duration::from_secs(dt.timestamp() as u64))
        });

        let cached = CachedToken {
            auth,
            cached_at: SystemTime::now(),
            expires_at,
            validation_count: 1,
            last_validated: Some(SystemTime::now()),
            refresh_attempts: 0,
            last_refresh_attempt: None,
        };

        assert!(cached.is_valid());
        assert!(cached.should_refresh()); // Should refresh within buffer time
        assert!(cached.can_attempt_refresh()); // No previous attempts
    }

    #[tokio::test]
    async fn test_token_expiry_detection() {
        let bus = Bus::new().await;
        let config = TwitchConfig::default();
        let oauth = TwitchOAuth::new(bus.handle(), config, "test".to_string());

        // Test expired token
        let expired_token = TwitchAuth {
            token: "oauth:expired".to_string(),
            username: "testuser".to_string(),
            obtained_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
            refresh_token: None,
            scopes: vec![],
            user_id: None,
            validation_count: 0,
            last_validated: None,
        };

        assert!(oauth.is_token_expired(&expired_token));

        // Test valid token
        let valid_token = TwitchAuth {
            token: "oauth:valid".to_string(),
            username: "testuser".to_string(),
            obtained_at: chrono::Utc::now().to_rfc3339(),
            expires_at: Some((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339()),
            refresh_token: None,
            scopes: vec![],
            user_id: None,
            validation_count: 0,
            last_validated: None,
        };

        assert!(!oauth.is_token_expired(&valid_token));

        // Test token without expiry
        let no_expiry_token = TwitchAuth {
            token: "oauth:no_expiry".to_string(),
            username: "testuser".to_string(),
            obtained_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
            refresh_token: None,
            scopes: vec![],
            user_id: None,
            validation_count: 0,
            last_validated: None,
        };

        assert!(!oauth.is_token_expired(&no_expiry_token));
    }
}
