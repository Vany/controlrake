pub mod helpers;
pub mod simple_storage;
pub mod database_recovery;
pub mod process_lock;

use crate::component::Component;
use crate::{bus::BusHandle, config::Config as GlobalConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::info;

/// 📤 Component response builder for system operations
#[derive(Debug, Clone, Serialize)]
struct ComponentResponse {
    success: bool,
    action: String,
    version: u64,
    component_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_loaded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    well_known_instances: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dynamic_instances: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_components: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_stats: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_status: Option<String>,
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
            config_loaded: None,
            well_known_instances: None,
            dynamic_instances: None,
            total_components: None,
            component_class: None,
            instance_id: None,
            component_list: None,
            storage_stats: None,
            health_status: None,
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
            config_loaded: None,
            well_known_instances: None,
            dynamic_instances: None,
            total_components: None,
            component_class: None,
            instance_id: None,
            component_list: None,
            storage_stats: None,
            health_status: None,
            message: None,
            error: Some(error.to_string()),
        }
    }

    /// Add timestamp (current UTC)
    fn with_timestamp(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now().to_rfc3339());
        self
    }

    /// Add system stats
    fn with_stats(mut self, config_loaded: bool, well_known: usize, dynamic: usize) -> Self {
        self.config_loaded = Some(config_loaded);
        self.well_known_instances = Some(well_known);
        self.dynamic_instances = Some(dynamic);
        self.total_components = Some(well_known + dynamic);
        self
    }

    /// Add component spawn info
    fn with_spawn(mut self, class: &str, instance_id: &str) -> Self {
        self.component_class = Some(class.to_string());
        self.instance_id = Some(instance_id.to_string());
        self
    }

    /// Add component list
    fn with_component_list(mut self, components: Vec<String>) -> Self {
        self.component_list = Some(components);
        self
    }

    /// Add storage stats
    fn with_storage_stats(mut self, stats: Value) -> Self {
        self.storage_stats = Some(stats);
        self
    }

    /// Add health status
    fn with_health(mut self, status: &str) -> Self {
        self.health_status = Some(status.to_string());
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

/// ⚙️ System component configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Data directory for storage
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    
    /// Enable automatic component recovery
    #[serde(default = "default_auto_recovery")]
    pub auto_recovery: bool,
    
    /// Maximum components allowed
    #[serde(default = "default_max_components")]
    pub max_components: usize,
}

fn default_data_dir() -> String {
    "data".to_string()
}

fn default_auto_recovery() -> bool {
    true
}

fn default_max_components() -> usize {
    1000
}

/// 🎛️ System Component - Manual Implementation Following Component Rules
/// 
/// **FOLLOWS COMPONENT RULES**: Manual implementation that follows same patterns as macro components
/// **WHY MANUAL**: System component has unique initialization and storage mediation requirements
/// **RESPONSIBILITIES**:
/// - Storage mediation for components  
/// - System lifecycle (start/stop/spawn)
/// - Bus message routing to system methods
///   **NO LONGER**: Component registry management (moved to global ComponentRegistry)
pub struct SystemComponent {
    // Framework fields (same as macro-generated components)
    id: String,
    version: std::sync::atomic::AtomicU64,
    
    // Configuration (same pattern as macro components)
    cfg: Config,
    
    // User fields (same as other components)
    /// System configuration (loaded once during init)
    global_config: Option<GlobalConfig>,

    /// Storage engine for component state persistence
    storage: Arc<simple_storage::SimpleStorage>,

    /// Bus handle for component communication
    bus_handle: BusHandle,

    /// Component registry handle for component management
    registry_handle: crate::component_registry::ComponentRegistryHandle,

    /// Message receiver for system: method calls
    message_receiver: Option<UnboundedReceiver<crate::message::Message>>,
}

impl SystemComponent {
    /// 🔧 Constructor following component pattern - compatible with macro-generated components
    /// 
    /// **Arguments**: 
    /// - `id`: Component instance ID ("system" for default instance, or "system:instance-id")
    /// - `bus_handle`: Bus handle for communication (used instead of system Arc)
    /// - `registry_handle`: ComponentRegistry handle for component management
    /// - `cfg`: Component configuration
    pub fn new(
        id: String, 
        bus_handle: BusHandle, 
        registry_handle: crate::component_registry::ComponentRegistryHandle,
        cfg: Config
    ) -> Result<Self> {
        info!("🎛️ Initializing SystemComponent '{}'", id);
        
        // Register system class channel for method calls
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        bus_handle.register_class("system", tx);
        
        // Initialize storage engine - return errors instead of panicking
        let data_path = std::path::Path::new(&cfg.data_dir).join("controlrake.db");
        let storage = match simple_storage::SimpleStorage::new(&data_path) {
            Ok(s) => {
                info!("🗄️ Storage engine initialized: {:?}", data_path);
                Arc::new(s)
            },
            Err(e) => {
                tracing::error!("🚨 Storage engine init failed: {:#}", e);
                // Return error instead of panicking - allows graceful handling
                anyhow::bail!("Storage engine initialization failed: {e}");
            }
        };

        let mut system = Self {
            id: id.clone(),
            version: std::sync::atomic::AtomicU64::new(1),
            cfg,
            global_config: None,
            storage,
            bus_handle: bus_handle.clone(),
            registry_handle,
            message_receiver: Some(rx),
        };

        // Start message handling task
        system.start_message_handler();
        
        info!("✅ SystemComponent '{}' initialized", id);
        Ok(system)
    }

    /// 🏗️ Create new system component with default data path - backward compatibility
    /// 
    /// **Returns**: Result containing SystemComponent or error if storage initialization fails  
    pub fn new_with_bus(bus: BusHandle, registry: crate::component_registry::ComponentRegistryHandle) -> Result<Self> {
        let cfg = Config::default();
        Self::new("system".to_string(), bus, registry, cfg)
    }

    /// 🧪 Create new system component for testing - handles database lock errors gracefully
    /// 
    /// **Purpose**: For use in tests where database lock conflicts are expected
    /// **Behavior**: Falls back to in-memory database if lock error occurs
    /// **Returns**: SystemComponent that works in test environments
    #[cfg(test)]
    pub fn new_for_test(bus: BusHandle, registry: crate::component_registry::ComponentRegistryHandle) -> Self {
        match Self::new_with_bus(bus.clone(), registry.clone()) {
            Ok(system) => system,
            Err(e) => {
                tracing::warn!("🧪 Test database lock error (expected in parallel tests): {:#}", e);
                tracing::info!("🧪 Creating test system with temporary database");
                
                // Create with temporary directory for tests
                let temp_dir = std::env::temp_dir().join(format!("controlrake_test_{}", std::process::id()));
                let mut cfg = Config::default();
                cfg.data_dir = temp_dir.to_str().unwrap().to_string();
                Self::new("system".to_string(), bus, registry, cfg)
                    .expect("Test system creation should not fail with temp directory")
            }
        }
    }

    /// 🔧 Initialize basic configuration (same pattern as macro components)
    pub fn init_config(&mut self, config: &GlobalConfig) -> Result<()> {
        info!("🔧 Initializing system configuration");
        self.global_config = Some(config.clone());
        info!("✅ System configuration initialized");  
        Ok(())
    }

    /// 🔄 Start message handler task for system: method calls
    fn start_message_handler(&mut self) {
        let mut receiver = self.message_receiver.take()
            .expect("Message receiver already taken");
        
        let storage = Arc::clone(&self.storage);
        let bus_handle = self.bus_handle.clone();
        let registry_handle = self.registry_handle.clone();

        tokio::spawn(async move {
            info!("🎛️ System message handler started");

            while let Some(message) = receiver.recv().await {
                let method = match &message.target.method {
                    Some(m) => m.clone(),
                    None => {
                        tracing::warn!("🤷 System message without method: {:?}", message.target);
                        continue;
                    }
                };

                let result = match method.as_str() {
                    "start" => helpers::handle_start(&bus_handle, &registry_handle, &storage, message.payload).await,
                    "stop" => helpers::handle_stop(message.payload).await,
                    "spawn" => helpers::handle_spawn(&bus_handle, &registry_handle, &storage, message.payload).await,
                    "log" => helpers::handle_log(message.payload).await,
                    "set" => helpers::handle_storage_set(&storage, &bus_handle, message).await,
                    "get" => helpers::handle_storage_get(&storage, &bus_handle, message).await,
                    "delete" => helpers::handle_storage_delete(&storage, &bus_handle, message).await,
                    _ => {
                        tracing::warn!("🤷 Unknown system method: {}", method);
                        continue;
                    }
                };

                if let Err(e) = result {
                    tracing::error!("❌ System method '{}' failed: {}", method, e);
                }
            }

            info!("🛑 System message handler stopped");
        });
    }

    /// 🚌 Get bus handle for component communication
    pub fn get_bus_handle(&self) -> &BusHandle {
        &self.bus_handle
    }

    /// 🔧 Component-style version management (same as macro components)
    pub fn current_version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 🔧 Increment version (same as macro components)
    pub fn increment_version(&self) {
        self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// 💾 Save state helper (same pattern as macro components)
    pub async fn save_state(&self, key: &str, value: Value) -> Result<()> {
        let scoped_key = format!("{}:{}", self.id, key);
        self.storage.save_component(&scoped_key, value, Some("system".to_string())).await
    }

    /// 📖 Load state helper (same pattern as macro components)
    pub async fn load_state(&self, key: &str) -> Result<Option<Value>> {
        let scoped_key = format!("{}:{}", self.id, key);
        self.storage.load_component(&scoped_key).await
    }

    // ================================================================================
    // 🎯 DEFAULT INSTANCE METHODS (system:) - These work without instance ID
    // ================================================================================

    /// 🚀 Start system and all components - default instance method (follows component pattern)
    pub async fn start(&mut self, config_data: Value) -> Result<Value> {
        info!("🚀 System starting with configuration");
        
        // Parse and store global configuration
        let global_config: GlobalConfig = serde_json::from_value(config_data)?;
        self.global_config = Some(global_config.clone());
        
        // Initialize well-known default instances
        self.init_well_known_instances(&global_config).await?;
        
        self.increment_version();
        
        Ok(ComponentResponse::success("start", &self.id, self.current_version())
            .with_message("System started successfully")
            .with_timestamp()
            .to_json())
    }

    /// 🛑 Stop system gracefully - default instance method (follows component pattern)
    pub async fn stop(&mut self) -> Result<Value> {
        info!("🛑 System stopping gracefully");
        
        // TODO: Implement graceful component shutdown
        self.increment_version();
        
        Ok(ComponentResponse::success("stop", &self.id, self.current_version())
            .with_message("System stopped successfully")
            .with_timestamp()
            .to_json())
    }

    /// 📦 Spawn new component instance - default instance method (follows component pattern)
    pub async fn spawn(&mut self, class: &str, instance_id: &str, config: Option<Value>) -> Result<Value> {
        info!("📦 Spawning component: {}:{}", class, instance_id);
        
        // Validate component limit - TODO: Check with ComponentRegistry
        // if registry.component_count() >= self.cfg.max_components {
        //     return Ok(ComponentResponse::error("Maximum component limit reached").to_json());
        // }
        
        // Use helper function for actual spawning
        match helpers::handle_spawn(&self.bus_handle, &self.registry_handle, &self.storage, json!({
            "class": class,
            "id": instance_id,
            "config": config
        })).await {
            Ok(_) => {
                self.increment_version();
                Ok(ComponentResponse::success("spawn", &self.id, self.current_version())
                    .with_spawn(class, instance_id)
                    .with_message(&format!("Component {class}:{instance_id} spawned successfully"))
                    .with_timestamp()
                    .to_json())
            }
            Err(e) => {
                Ok(ComponentResponse::error(&format!("Failed to spawn component: {e}")).to_json())
            }
        }
    }

    /// 📝 System logging - default instance method (follows component pattern)
    pub async fn log(&self, level: &str, message: &str) -> Result<Value> {
        match level {
            "trace" => tracing::trace!("📝 {}", message),
            "debug" => tracing::debug!("📝 {}", message),
            "info" => tracing::info!("📝 {}", message),
            "warn" => tracing::warn!("📝 {}", message),
            "error" => tracing::error!("📝 {}", message),
            _ => tracing::info!("📝 [{}] {}", level, message),
        }
        
        Ok(ComponentResponse::success("log", &self.id, self.current_version())
            .with_message(&format!("Logged {level} message"))
            .to_json())
    }

    /// 💾 Global storage set - default instance method (follows component pattern)
    pub async fn set(&self, key: &str, value: Value) -> Result<Value> {
        match helpers::handle_storage_set(&self.storage, &self.bus_handle, crate::message::Message {
            target: crate::address::Address::parse("system:")?,
            source: None,
            payload: json!({ "key": key, "value": value }),
            id: None,
            reply: None,
            finish: false,
        }).await {
            Ok(_) => {
                Ok(ComponentResponse::success("set", &self.id, self.current_version())
                    .with_message(&format!("Set key '{key}' successfully"))
                    .to_json())
            }
            Err(e) => {
                Ok(ComponentResponse::error(&format!("Failed to set key: {e}")).to_json())
            }
        }
    }

    /// 📖 Global storage get - default instance method (follows component pattern)
    pub async fn get(&self, key: &str) -> Result<Value> {
        match helpers::handle_storage_get(&self.storage, &self.bus_handle, crate::message::Message {
            target: crate::address::Address::parse("system:")?,
            source: None,
            payload: json!({ "key": key }),
            id: None,
            reply: None,
            finish: false,
        }).await {
            Ok(_) => {
                Ok(ComponentResponse::success("get", &self.id, self.current_version())
                    .with_message(&format!("Retrieved key '{key}' successfully"))
                    .to_json())
            }
            Err(e) => {
                Ok(ComponentResponse::error(&format!("Failed to get key: {e}")).to_json())
            }
        }
    }

    /// 🗑️ Global storage delete - default instance method (follows component pattern)
    pub async fn delete(&self, key: &str) -> Result<Value> {
        match helpers::handle_storage_delete(&self.storage, &self.bus_handle, crate::message::Message {
            target: crate::address::Address::parse("system:")?,
            source: None,
            payload: json!({ "key": key }),
            id: None,
            reply: None,
            finish: false,
        }).await {
            Ok(_) => {
                Ok(ComponentResponse::success("delete", &self.id, self.current_version())
                    .with_message(&format!("Deleted key '{key}' successfully"))
                    .to_json())
            }
            Err(e) => {
                Ok(ComponentResponse::error(&format!("Failed to delete key: {e}")).to_json())
            }
        }
    }

    // ================================================================================
    // 🎯 INSTANCE METHODS (system:instance-id) - These require specific instance ID
    // ================================================================================

    /// 📊 Get system statistics - instance method (follows component pattern)
    pub async fn get_stats(&self) -> Result<Value> {
        let stats = self.get_stats_internal();
        
        Ok(ComponentResponse::success("get_stats", &self.id, self.current_version())
            .with_stats(stats.config_loaded, stats.well_known_instances, stats.dynamic_instances)
            .with_timestamp()
            .to_json())
    }

    /// 📋 Get component list - instance method (follows component pattern)
    pub async fn get_component_list(&self) -> Result<Value> {
        // TODO: Get component list from ComponentRegistry when it's integrated
        let components = Vec::<String>::new(); // Will be retrieved from ComponentRegistry
        
        Ok(ComponentResponse::success("get_component_list", &self.id, self.current_version())
            .with_component_list(components)
            .with_timestamp()
            .to_json())
    }

    /// 🗄️ Get storage statistics - instance method (follows component pattern)
    pub async fn get_storage_stats(&self) -> Result<Value> {
        match self.storage.get_stats().await {
            Ok(stats) => {
                Ok(ComponentResponse::success("get_storage_stats", &self.id, self.current_version())
                    .with_storage_stats(stats)
                    .with_timestamp()
                    .to_json())
            }
            Err(e) => {
                Ok(ComponentResponse::error(&format!("Failed to get storage stats: {e}")).to_json())
            }
        }
    }

    /// 💚 Get system health status - instance method (follows component pattern)
    pub async fn get_system_health(&self) -> Result<Value> {
        // TODO: Check component status with ComponentRegistry when integrated
        let health_status = if self.global_config.is_some() {
            "healthy"
        } else {
            "degraded"
        };
        
        Ok(ComponentResponse::success("get_system_health", &self.id, self.current_version())
            .with_health(health_status)
            .with_timestamp()
            .to_json())
    }

    // ================================================================================
    // 🔧 HELPER METHODS - Internal implementation
    // ================================================================================

    /// Initialize well-known default instances
    async fn init_well_known_instances(&mut self, _config: &GlobalConfig) -> Result<()> {
        info!("🔍 Initializing well-known default instances");
        
        // TODO: Create default instances for HTTP, Clock, Twitch components
        // This requires the macro system to be fully integrated
        
        Ok(())
    }

    /// Get system statistics (internal)
    fn get_stats_internal(&self) -> SystemStats {
        // TODO: Get stats from ComponentRegistry when it's integrated
        SystemStats {
            config_loaded: self.global_config.is_some(),
            well_known_instances: 0, // Will be retrieved from ComponentRegistry
            dynamic_instances: 0,    // Will be retrieved from ComponentRegistry
        }
    }

    // ================================================================================
    // 🗄️ STORAGE SERVICE API - Clean interface for components
    // ================================================================================

    /// 💾 Save component state with automatic key scoping
    pub async fn save_component_state(&self, component_id: &str, key: &str, value: serde_json::Value) -> Result<()> {
        let scoped_key = format!("{component_id}:{key}");
        let component_class = component_id.split(':').next().unwrap_or("unknown");
        self.storage.save_component(&scoped_key, value, Some(component_class.to_string())).await
    }

    /// 📖 Load component state with automatic key scoping
    pub async fn load_component_state(&self, component_id: &str, key: &str) -> Result<Option<serde_json::Value>> {
        let scoped_key = format!("{component_id}:{key}");
        self.storage.load_component(&scoped_key).await
    }

    /// 🗑️ Delete component state with automatic key scoping
    pub async fn delete_component_state(&self, component_id: &str, key: &str) -> Result<bool> {
        let scoped_key = format!("{component_id}:{key}");
        self.storage.delete_component(&scoped_key).await
    }

    /// 📋 List all stored component data
    pub async fn list_stored_components(&self) -> Result<Vec<String>> {
        self.storage.list_components().await
    }

    /// 🗄️ Get storage reference (for advanced use cases)
    pub fn get_storage(&self) -> Arc<simple_storage::SimpleStorage> {
        Arc::clone(&self.storage)
    }

}

// ================================================================================
// 🎯 COMPONENT TRAIT IMPLEMENTATION - Manual implementation following component rules
// ================================================================================

impl std::fmt::Debug for SystemComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemComponent")
            .field("id", &self.id)
            .field("version", &self.current_version())
            .field("cfg", &self.cfg)
            .field("config_loaded", &self.global_config.is_some())
            .field("storage_available", &true)
            .finish()
    }
}

// ================================================================================
// 🎯 COMPONENT TRAIT IMPLEMENTATION - Manual implementation following component rules
// ================================================================================

use async_trait::async_trait;

#[async_trait]
impl Component for SystemComponent {
    /// 📨 Handle system component instance messages (follows component pattern)
    async fn handle_message(&mut self, method: &str, _payload: serde_json::Value) -> Result<serde_json::Value> {
        match method {
            "get_stats" => self.get_stats().await,
            "get_component_list" => self.get_component_list().await,
            "get_storage_stats" => self.get_storage_stats().await,
            "get_system_health" => self.get_system_health().await,
            "supports_default_instance" => {
                Ok(json!({
                    "supports_default": true,
                    "message": "System component supports default instance"
                }))
            }
            _ => {
                tracing::warn!("❓ Unknown system instance method: {}", method);
                Ok(json!({
                    "error": format!("Unknown method: {}", method)
                }))
            }
        }
    }

    /// 🎨 Render system dashboard (follows component pattern)
    async fn render(&self) -> Result<String> {
        // Load and render template (same as macro components would do)
        let template = include_str!("templates/system.hbs");
        let stats = self.get_stats_internal();
        
        // Simple template rendering (could use handlebars like macro components)
        let mut rendered = template.to_string();
        rendered = rendered.replace("{{component_id}}", &self.id);
        rendered = rendered.replace("{{version}}", &self.current_version().to_string());
        rendered = rendered.replace("{{config_loaded}}", &stats.config_loaded.to_string());
        rendered = rendered.replace("{{well_known_instances}}", &stats.well_known_instances.to_string());
        rendered = rendered.replace("{{dynamic_instances}}", &stats.dynamic_instances.to_string());
        rendered = rendered.replace("{{total_components}}", &(stats.well_known_instances + stats.dynamic_instances).to_string());
        
        Ok(rendered)
    }

    /// 🆔 Component identifier (follows component pattern)
    fn id(&self) -> &str {
        &self.id
    }

    /// 🎯 System supports default instance (follows component pattern)
    fn supports_default_instance(&self) -> bool {
        true
    }

    /// 🔍 Get component as Any for downcasting (follows component pattern)
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// 🔍 Get mutable component as Any for downcasting (follows component pattern)
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 📊 System statistics for monitoring
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub config_loaded: bool,
    pub well_known_instances: usize,
    pub dynamic_instances: usize,
}

/// 📋 Component instance information for discovery
#[derive(Debug, Clone)]
pub struct ComponentInstanceInfo {
    pub class: String,
    pub instance_id: String,
    pub supports_default: bool,
    pub is_default: bool,
}




