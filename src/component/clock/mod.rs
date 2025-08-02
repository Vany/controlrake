use crate::component::Component;
use crate::{address::Address, bus::BusHandle, message::Message};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
    subscriber: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_subscribers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    was_new: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    was_removed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduled_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unixtime: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscribers_notified: Option<usize>,
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
            subscriber: None,
            total_subscribers: None,
            was_new: None,
            was_removed: None,
            scheduled_time: None,
            target: None,
            current_time: None,
            delay_seconds: None,
            target_time: None,
            unixtime: None,
            subscribers_notified: None,
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
            subscriber: None,
            total_subscribers: None,
            was_new: None,
            was_removed: None,
            scheduled_time: None,
            target: None,
            current_time: None,
            delay_seconds: None,
            target_time: None,
            unixtime: None,
            subscribers_notified: None,
            message: None,
            error: Some(error.to_string()),
        }
    }

    /// Add timestamp (current UTC)
    fn with_timestamp(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now().to_rfc3339());
        self
    }

    /// Add subscriber info
    fn with_subscriber(mut self, subscriber: &str, total: usize, was_new: bool) -> Self {
        self.subscriber = Some(subscriber.to_string());
        self.total_subscribers = Some(total);
        self.was_new = Some(was_new);
        self
    }

    /// Add unsubscribe info
    fn with_unsubscribe(mut self, subscriber: &str, total: usize, was_removed: bool) -> Self {
        self.subscriber = Some(subscriber.to_string());
        self.total_subscribers = Some(total);
        self.was_removed = Some(was_removed);
        self
    }

    /// Add schedule info
    fn with_schedule(mut self, scheduled_time: u64, target: &str, current_time: u64) -> Self {
        self.scheduled_time = Some(scheduled_time);
        self.target = Some(target.to_string());
        self.current_time = Some(current_time);
        self
    }

    /// Add delayed schedule info
    fn with_delayed_schedule(mut self, delay_seconds: u64, target_time: u64, target: &str) -> Self {
        self.delay_seconds = Some(delay_seconds);
        self.target_time = Some(target_time);
        self.target = Some(target.to_string());
        self
    }

    /// Add tick info
    fn with_tick(mut self, unixtime: u64, subscribers_notified: usize) -> Self {
        self.unixtime = Some(unixtime);
        self.subscribers_notified = Some(subscribers_notified);
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

/// Type alias for scheduled callbacks map
type ScheduledCallbacksMap = Arc<RwLock<HashMap<u64, Vec<(String, Value)>>>>;

/// ⚙️ Clock component configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Automatically start tick generation on initialization
    #[serde(default = "default_auto_start_tick")]
    pub auto_start_tick: bool,
    
    /// Tick interval in seconds
    #[serde(default = "default_tick_interval")]
    pub tick_interval_secs: u64,
}

fn default_auto_start_tick() -> bool {
    true
}

fn default_tick_interval() -> u64 {
    1
}

/// ⏰ Clock Component - Works with macro-generated infrastructure
#[crate::component(template = "clock.hbs", config = "Config")]
pub struct Clock {
    // User fields only - macro handles all framework fields
    /// Set of component addresses subscribed to tick events
    subscribers: Arc<RwLock<HashSet<String>>>,
    /// Scheduled one-shot callbacks: unixtime -> Vec<(target_address, message_payload)>
    scheduled_callbacks: ScheduledCallbacksMap,
    /// Flag to track if tick generation is running
    tick_task_running: Arc<RwLock<bool>>,
    /// Internal bus handle reference (derived from system)
    bus_handle: Option<BusHandle>,
}

impl Clock {
    /// 🔧 Called after component creation - initialize user fields
    /// NOTE: This is called by macro-generated constructor
    pub async fn new_component(&mut self) -> Result<()> {
        // Initialize user fields with defaults
        self.subscribers = Arc::new(RwLock::new(HashSet::new()));
        self.scheduled_callbacks = Arc::new(RwLock::new(HashMap::new()));
        self.tick_task_running = Arc::new(RwLock::new(false));
        
        // Get bus handle from system reference
        self.bus_handle = Some(self.system.get_bus_handle().clone());
        
        // Try to load saved state
        let state_key = format!("clock:{}:state", self.id());
        if let Ok(Some(state)) = self.load_state(&state_key).await {
            self.load_clock_state_from_data(&state).await?;
            debug!("📖 Loaded clock state for '{}'", self.id());
        }
        
        // Start tick generation if configured
        if self.cfg.auto_start_tick {
            if let Err(e) = self.start_tick_generation_internal().await {
                warn!("Failed to start tick generation: {}", e);
            }
        }
        
        info!("⏰ Clock component '{}' initialized", self.id());
        Ok(())
    }

    /// 📝 Subscribe to tick events - routed by macro
    pub async fn subscribe(&mut self, subscriber: &str) -> Result<Value> {
        let mut subs = self.subscribers.write().await;
        let was_new = subs.insert(subscriber.to_string());

        if was_new {
            info!("📝 Clock '{}' added subscriber: {}", self.id(), subscriber);
        }

        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save clock state: {}", e);
        }

        Ok(ComponentResponse::success("subscribe", self.id(), self.current_version())
            .with_subscriber(subscriber, subs.len(), was_new)
            .with_timestamp()
            .to_json())
    }

    /// ❌ Unsubscribe from tick events - routed by macro
    pub async fn unsubscribe(&mut self, subscriber: &str) -> Result<Value> {
        let mut subs = self.subscribers.write().await;
        let was_removed = subs.remove(subscriber);

        if was_removed {
            info!("📝 Clock '{}' removed subscriber: {}", self.id(), subscriber);
        }

        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save clock state: {}", e);
        }

        Ok(ComponentResponse::success("unsubscribe", self.id(), self.current_version())
            .with_unsubscribe(subscriber, subs.len(), was_removed)
            .with_timestamp()
            .to_json())
    }

    /// ⏰ Schedule callback at specific time - routed by macro
    pub async fn schedule_callback(&mut self, time: u64, target: &str, message: Value) -> Result<Value> {
        let mut callbacks = self.scheduled_callbacks.write().await;
        callbacks.entry(time)
            .or_insert_with(Vec::new)
            .push((target.to_string(), message.clone()));

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if time <= current_time && self.bus_handle.is_some() {
            info!("⚡ Clock '{}' firing immediate callback for past time {} to {}", 
                self.id(), time, target);
            
            if let Err(e) = self.bus_handle.as_ref().unwrap().send(Message {
                target: Address::parse(target)?,
                source: Some(Address::parse(self.id())?),
                payload: message.clone(),
                id: None,
                reply: None,
                finish: false,
            }).await {
                warn!("❌ Failed to send immediate callback to {}: {}", target, e);
            }
        } else {
            info!("⏰ Clock '{}' scheduled callback for time {} to {}", 
                self.id(), time, target);
        }

        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save clock state: {}", e);
        }

        Ok(ComponentResponse::success("schedule_callback", self.id(), self.current_version())
            .with_schedule(time, target, current_time)
            .with_timestamp()
            .to_json())
    }

    /// ⏰ Schedule delayed callback - routed by macro
    pub async fn schedule_delayed_callback(&mut self, delay_secs: u64, target: &str, message: Value) -> Result<Value> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let target_time = current_time + delay_secs;

        let mut callbacks = self.scheduled_callbacks.write().await;
        callbacks.entry(target_time)
            .or_insert_with(Vec::new)
            .push((target.to_string(), message.clone()));

        info!("⏰ Clock '{}' scheduled delayed callback for {}s to {}", 
            self.id(), delay_secs, target);

        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save clock state: {}", e);
        }

        Ok(ComponentResponse::success("schedule_delayed_callback", self.id(), self.current_version())
            .with_delayed_schedule(delay_secs, target_time, target)
            .with_timestamp()
            .to_json())
    }

    /// ⏰ Process tick event - routed by macro
    pub async fn tick(&mut self, unixtime: u64) -> Result<Value> {
        debug!("⏰ Clock '{}' processing tick for unixtime: {}", self.id(), unixtime);

        if let Some(bus_handle) = &self.bus_handle {
            // Process scheduled callbacks for this time
            let mut callbacks = self.scheduled_callbacks.write().await;
            if let Some(scheduled) = callbacks.remove(&unixtime) {
                debug!("📡 Broadcasting tick to {} callbacks", scheduled.len());
                for (target, message) in scheduled {
                    match Address::parse(&target) {
                        Ok(addr) => {
                            if let Err(e) = bus_handle.send(Message {
                                target: addr,
                                source: Some(Address::parse(self.id())?),
                                payload: message,
                                id: None,
                                reply: None,
                                finish: false,
                            }).await {
                                warn!("❌ Failed to send callback to {}: {}", target, e);
                            }
                        }
                        Err(e) => {
                            warn!("❌ Invalid callback address '{}': {}", target, e);
                        }
                    }
                }
            }

            // Broadcast tick to all subscribers
            let subs = self.subscribers.read().await;
            for subscriber in subs.iter() {
                match Address::parse(subscriber) {
                    Ok(addr) => {
                        if let Err(e) = bus_handle.send(Message {
                            target: addr,
                            source: Some(Address::parse(self.id())?),
                            payload: json!({"tick": unixtime}),
                            id: None,
                            reply: None,
                            finish: false,
                        }).await {
                            warn!("❌ Failed to send tick to subscriber '{}': {}", subscriber, e);
                        }
                    }
                    Err(e) => {
                        warn!("❌ Invalid subscriber address '{}': {}", subscriber, e);
                    }
                }
            }

            self.increment_version();

            Ok(ComponentResponse::success("tick", self.id(), self.current_version())
                .with_tick(unixtime, subs.len())
                .with_timestamp()
                .to_json())
        } else {
            Ok(ComponentResponse::error("Bus handle not available").to_json())
        }
    }

    /// 📊 Get clock status and statistics - routed by macro
    pub async fn get(&self, key: &str) -> Result<Value> {
        match key {
            "status" => {
                let subs = self.subscribers.read().await;
                let callbacks = self.scheduled_callbacks.read().await;
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                Ok(json!({
                    "id": self.id(),
                    "current_time": current_time,
                    "subscribers": subs.iter().collect::<Vec<_>>(),
                    "subscriber_count": subs.len(),
                    "scheduled_callbacks": callbacks.len(),
                    "version": self.current_version(),
                    "tick_running": *self.tick_task_running.read().await
                }))
            }
            _ => Ok(json!({
                "error": format!("Unknown key: {}", key),
                "available_keys": ["status"]
            }))
        }
    }

    /// 🚀 Start tick generation - routed by macro
    pub async fn start_tick(&mut self) -> Result<Value> {
        if let Err(e) = self.start_tick_generation_internal().await {
            Ok(ComponentResponse::error(&format!("Failed to start tick generation: {e}")).to_json())
        } else {
            Ok(ComponentResponse::success("start_tick", self.id(), self.current_version())
                .with_message("Tick generation started successfully")
                .to_json())
        }
    }

    /// 🛑 Stop tick generation - routed by macro
    pub async fn stop_tick(&mut self) -> Result<Value> {
        if let Err(e) = self.stop_tick_generation_internal().await {
            Ok(ComponentResponse::error(&format!("Failed to stop tick generation: {e}")).to_json())
        } else {
            Ok(ComponentResponse::success("stop_tick", self.id(), self.current_version())
                .with_message("Tick generation stopped successfully")
                .to_json())
        }
    }

    // ================================================================================
    // 🔧 HELPER METHODS
    // ================================================================================

    /// 💾 Save component state using macro-generated storage helpers
    async fn save_component_state(&self) -> Result<()> {
        let subs = self.subscribers.read().await;
        let callbacks = self.scheduled_callbacks.read().await;

        let subscribers_data: Vec<String> = subs.iter().cloned().collect();
        let callbacks_data: HashMap<String, Vec<(String, Value)>> = callbacks
            .iter()
            .map(|(time, cb_list)| (time.to_string(), cb_list.clone()))
            .collect();

        let state = json!({
            "subscribers": subscribers_data,
            "scheduled_callbacks": callbacks_data,
            "version": self.current_version(),
            "last_updated": chrono::Utc::now().to_rfc3339()
        });
        
        let state_key = format!("clock:{}:state", self.id());
        self.save_state(&state_key, state).await
    }

    /// 📂 Load clock state from saved data
    async fn load_clock_state_from_data(&mut self, state: &Value) -> Result<()> {
        if let Some(subs_array) = state.get("subscribers").and_then(|v| v.as_array()) {
            let mut subs = self.subscribers.write().await;
            subs.clear();
            for sub in subs_array {
                if let Some(s) = sub.as_str() {
                    subs.insert(s.to_string());
                }
            }
        }
        
        if let Some(callbacks_obj) = state.get("scheduled_callbacks").and_then(|v| v.as_object()) {
            let mut callbacks = self.scheduled_callbacks.write().await;
            callbacks.clear();
            for (time_str, callback_list) in callbacks_obj {
                if let (Ok(time), Some(list)) = (time_str.parse::<u64>(), callback_list.as_array()) {
                    let mut time_callbacks = Vec::new();
                    for callback in list {
                        if let Some(arr) = callback.as_array() {
                            if arr.len() >= 2 {
                                if let (Some(target), Some(message)) = (arr[0].as_str(), arr.get(1)) {
                                    time_callbacks.push((target.to_string(), message.clone()));
                                }
                            }
                        }
                    }
                    callbacks.insert(time, time_callbacks);
                }
            }
        }
        Ok(())
    }

    /// 🚀 Start background tick generation task (internal implementation)
    async fn start_tick_generation_internal(&mut self) -> Result<()> {
        let mut is_running = self.tick_task_running.write().await;
        if *is_running {
            warn!("🕰️ Tick generation already running for clock '{}'", self.id());
            return Ok(());
        }

        *is_running = true;
        
        if let Some(bus_handle) = &self.bus_handle {
            let subscribers = self.subscribers.clone();
            let scheduled_callbacks = self.scheduled_callbacks.clone();
            let bus_handle = bus_handle.clone();
            let tick_task_running = self.tick_task_running.clone();
            let version = Arc::new(AtomicU64::new(self.current_version()));
            let component_id = self.id().to_string();
            let tick_interval = std::time::Duration::from_secs(self.cfg.tick_interval_secs);

            // Spawn background task for tick generation
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tick_interval);
                
                while *tick_task_running.read().await {
                    interval.tick().await;
                    
                    let current_time = chrono::Utc::now().timestamp() as u64;
                    
                    // Process scheduled callbacks
                    let mut callbacks_to_process = Vec::new();
                    {
                        let mut callbacks = scheduled_callbacks.write().await;
                        if let Some(time_callbacks) = callbacks.remove(&current_time) {
                            callbacks_to_process = time_callbacks;
                        }
                    }
                    
                    // Execute scheduled callbacks
                    for (target, payload) in callbacks_to_process {
                        if let Ok(target_addr) = Address::parse(&target) {
                            let source_addr = if component_id.is_empty() {
                                Address::parse("clock:").unwrap_or_else(|_| Address::parse("clock").unwrap())
                            } else {
                                Address::parse(&format!("clock:{component_id}")).unwrap_or_else(|_| Address::parse("clock").unwrap())
                            };

                            let message = Message {
                                target: target_addr,
                                payload,
                                source: Some(source_addr),
                                reply: None,
                                finish: true,
                                id: None,
                            };
                            
                            if let Err(e) = bus_handle.send(message).await {
                                tracing::error!("❌ Failed to send scheduled callback: {}", e);
                            }
                        }
                    }
                    
                    // Send tick events to subscribers
                    let subs = subscribers.read().await;
                    let tick_payload = json!({
                        "timestamp": current_time,
                        "iso": chrono::Utc::now().to_rfc3339(),
                        "type": "tick"
                    });
                    
                    for subscriber in subs.iter() {
                        if let Ok(subscriber_addr) = Address::parse(subscriber) {
                            let source_addr = if component_id.is_empty() {
                                Address::parse("clock:").unwrap_or_else(|_| Address::parse("clock").unwrap())
                            } else {
                                Address::parse(&format!("clock:{component_id}")).unwrap_or_else(|_| Address::parse("clock").unwrap())
                            };

                            let message = Message {
                                target: subscriber_addr,
                                payload: tick_payload.clone(),
                                source: Some(source_addr),
                                reply: None,
                                finish: true,
                                id: None,
                            };
                            
                            if let Err(e) = bus_handle.send(message).await {
                                tracing::error!("❌ Failed to send tick to subscriber {}: {}", subscriber, e);
                            }
                        }
                    }
                    
                    // Update version for WebSocket notifications
                    version.fetch_add(1, Ordering::SeqCst);
                }
                
                tracing::info!("🛑 Tick generation stopped for clock '{}'", component_id);
            });

            info!("✅ Tick generation started for clock '{}'", self.id());
        }
        
        Ok(())
    }

    /// 🛑 Stop background tick generation task (internal implementation)
    async fn stop_tick_generation_internal(&mut self) -> Result<()> {
        let mut is_running = self.tick_task_running.write().await;
        if !*is_running {
            warn!("🕰️ Tick generation not running for clock '{}'", self.id());
            return Ok(());
        }

        *is_running = false;
        info!("🛑 Stopping tick generation for clock '{}'", self.id());
        Ok(())
    }
}
