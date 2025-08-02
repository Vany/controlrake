// Component modules
pub mod button;
pub mod clock;
pub mod http;
pub mod system;
pub mod twitch;

use system::simple_storage::SimpleStorage as DatabaseStorage;
use async_trait::async_trait;
use std::sync::Arc;



// Note: Component messaging utilities are now handled by:
// - bus.call() for request-response patterns (implemented in bus.rs)
// - ComponentFramework for component boilerplate reduction (component_framework.rs)
// - Individual component implementations for specialized patterns

// =====================================================================================
// 🎯 SIMPLIFIED COMPONENT SYSTEM
// =====================================================================================
//
// **Design Philosophy:**
// - Direct message handling without framework abstractions
// - Simple state management without generic complexity
// - Minimal boilerplate, maximum clarity
// - Preserve all functionality while eliminating layers

use std::collections::HashMap;
use tokio::sync::RwLock;

/// 🚀 Simple Component Interface - Zero Framework Overhead
///
/// Replaces the complex ComponentClass/ComponentInstance/MethodHandler chain
/// with a single, clear trait that components implement directly.
#[async_trait]
pub trait Component: Send + Sync {
    /// 📨 Handle incoming messages directly
    /// - method: The method being called (e.g., "click", "set_text")
    /// - payload: Message data as JSON
    /// - Returns response data or error
    async fn handle_message(&mut self, method: &str, payload: serde_json::Value) -> anyhow::Result<serde_json::Value>;

    /// 🎯 Component supports default instance (no instance ID required)
    /// - Default returns false (instance ID required)
    /// - Components like HTTP, Clock, Twitch should return true
    /// - Components like Button must return false (require unique instance IDs)
    fn supports_default_instance(&self) -> bool {
        false
    }

    /// 🎨 Render component HTML
    /// - Returns complete HTML representation
    async fn render(&self) -> anyhow::Result<String>;

    /// 🆔 Component identifier
    /// - Used for addressing and storage
    fn id(&self) -> &str;

    /// 🔄 Get incremental updates since version (optional optimization)
    /// - Default implementation returns empty (no incremental updates)
    /// - Components can override for performance
    async fn get_updates(&self, _last_version: u64) -> anyhow::Result<Vec<crate::update::UpdateMessage>> {
        Ok(vec![])
    }

    /// 📊 Current version for update tracking (optional)
    /// - Default returns 1 (no versioning)
    /// - Components can override for incremental updates
    fn current_version(&self) -> u64 {
        1
    }

    /// 🚀 Initialize component with configuration
    /// - Default implementation does nothing
    /// - Components can override for setup logic
    async fn start(&mut self, _config: serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }

    /// 🛑 Cleanup component resources
    /// - Default implementation does nothing  
    /// - Components can override for cleanup logic
    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    // ================================================================================
    // 🔧 DOWNCASTING SUPPORT - Required by macro system
    // ================================================================================

    /// 🔍 Get component as Any for downcasting (required by macro system)
    ///
    /// **Purpose**: Enable type-safe downcasting from Component trait objects
    /// **Usage**: `component.as_any().downcast_ref::<Button>()`
    /// **Required by macro system** for get_component<T>() functionality
    fn as_any(&self) -> &dyn std::any::Any;

    /// 🔍 Get mutable component as Any for downcasting (required by macro system)
    ///
    /// **Purpose**: Enable type-safe mutable downcasting from Component trait objects
    /// **Usage**: `component.as_any_mut().downcast_mut::<Button>()`
    /// **Required by macro system** for get_component_mut<T>() functionality
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// 📝 Component-focused storage adapter
/// 
/// Provides simple component storage interface backed by the real database storage.
/// Components get the simple API they expect while using proper persistence.
#[derive(Debug)]
pub struct SimpleStorage {
    database: Arc<DatabaseStorage>,
    component_id: String,
}

impl SimpleStorage {
    /// Create component storage adapter
    pub fn new(database: Arc<DatabaseStorage>, component_id: String) -> Self {
        Self {
            database,
            component_id,
        }
    }

    /// 💾 Save component state with automatic prefixing
    pub async fn save(&self, key: &str, value: serde_json::Value) -> anyhow::Result<()> {
        let storage_key = format!("{}:{}", self.component_id, key);
        self.database
            .save_component(&storage_key, value, Some(self.component_type()))
            .await
    }

    /// 📖 Load component state with automatic prefixing
    pub async fn load(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let storage_key = format!("{}:{}", self.component_id, key);
        self.database.load_component(&storage_key).await
    }

    /// 🗑️ Delete component state
    pub async fn delete(&self, key: &str) -> anyhow::Result<bool> {
        let storage_key = format!("{}:{}", self.component_id, key);
        self.database.delete_component(&storage_key).await
    }

    /// 🏷️ Get component type from ID
    fn component_type(&self) -> String {
        // Extract component type from ID (e.g., "button:my-btn" -> "button")
        self.component_id
            .split(':')
            .next()
            .unwrap_or("unknown")
            .to_string()
    }
}

/// 🎨 Simple template renderer - no framework needed
///
/// Components can render templates directly without complex
/// template management systems and registration patterns.
#[derive(Debug)]
pub struct SimpleRenderer {
    templates: handlebars::Handlebars<'static>,
}

impl SimpleRenderer {
    pub fn new() -> Self {
        Self {
            templates: handlebars::Handlebars::new(),
        }
    }

    /// 📝 Register template from string
    pub fn register_template(&mut self, name: &str, template: &str) -> anyhow::Result<()> {
        self.templates
            .register_template_string(name, template)
            .map_err(|e| anyhow::anyhow!("Template registration failed: {}", e))
    }

    /// 🎨 Render template with data
    pub fn render(&self, name: &str, data: &serde_json::Value) -> anyhow::Result<String> {
        self.templates
            .render(name, data)
            .map_err(|e| anyhow::anyhow!("Template rendering failed: {}", e))
    }
}

impl Default for SimpleRenderer {
    fn default() -> Self {
        Self::new()
    }
}



/// 🚀 Advanced Simplified Component Manager
/// 
/// **Purpose**: Replace complex MPSC-based component messaging with direct function calls
/// for local components, while preserving WebSocket messaging for browser communication.
///
/// **Design Goals**:
/// - Direct function calls eliminate MPSC overhead for same-process communication
/// - Maintain WebSocket bridge for browser-server messaging  
/// - Preserve all existing functionality with simpler architecture
/// - Enable gradual migration from complex bus system
///
///   📦 Advanced Simplified Component Manager
/// 
/// Manages component instances with direct method calls instead of MPSC messaging.
/// This eliminates the bus routing overhead for local component interactions.
pub struct AdvancedSimpleComponentManager {
    /// 🗂️ Registry of active component instances
    components: Arc<RwLock<HashMap<String, Box<dyn Component>>>>,
    
    /// 🌐 Optional WebSocket bridge for browser communication
    /// When a message comes from a WebSocket, we still need to route responses back
    websocket_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

impl AdvancedSimpleComponentManager {
    /// 🏗️ Create new simplified component manager
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
            websocket_sender: None,
        }
    }

    /// 🌐 Configure WebSocket bridge for browser communication
    /// 
    /// When components need to send updates to browsers, they use this channel.
    /// The HTTP component handles the actual WebSocket routing.
    pub fn with_websocket_bridge(
        mut self,
        sender: tokio::sync::mpsc::UnboundedSender<String>,
    ) -> Self {
        self.websocket_sender = Some(sender);
        self
    }

    /// 📋 Register a component instance
    /// 
    /// **Arguments**:
    /// - `address`: Component address like "button:my-btn" 
    /// - `component`: Component instance implementing the simple Component trait
    /// 
    /// **Returns**: Result indicating success/failure
    pub async fn register_component(
        &self,
        address: String,
        component: Box<dyn Component>,
    ) -> anyhow::Result<()> {
        let mut components = self.components.write().await;
        
        tracing::info!("📋 Registering simplified component: {}", address);
        
        if components.contains_key(&address) {
            tracing::warn!("⚠️ Component {} already registered, replacing", address);
        }
        
        components.insert(address, component);
        Ok(())
    }

    /// 📤 Unregister a component instance
    pub async fn unregister_component(&self, address: &str) -> anyhow::Result<()> {
        let mut components = self.components.write().await;
        
        if components.remove(address).is_some() {
            tracing::info!("📤 Unregistered component: {}", address);
            Ok(())
        } else {
            tracing::warn!("⚠️ Attempted to unregister non-existent component: {}", address);
            Err(anyhow::anyhow!("Component {} not found", address))
        }
    }

    /// 📨 Send message directly to component (no MPSC overhead)
    /// 
    /// **Arguments**:
    /// - `address`: Target component like "button:my-btn"
    /// - `method`: Method to call like "click" or "render"
    /// - `payload`: Message data as JSON
    /// 
    /// **Returns**: Component response or error
    /// 
    /// **Performance**: Direct function call - no serialization/deserialization overhead
    pub async fn send_message(
        &self,
        address: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let mut components = self.components.write().await;
        
        if let Some(component) = components.get_mut(address) {
            tracing::debug!("📨 Calling {}.{} directly", address, method);
            
            // 🚀 DIRECT FUNCTION CALL - No MPSC, no serialization overhead
            let result = component.handle_message(method, payload).await;
            
            match &result {
                Ok(response) => {
                    tracing::debug!("✅ {}.{} succeeded", address, method);
                    
                    // 🌐 If this affects browser state, notify WebSocket clients
                    if self.should_notify_browsers(method) {
                        self.notify_browsers_of_update(address, method, response).await?;
                    }
                },
                Err(e) => {
                    tracing::error!("❌ {}.{} failed: {}", address, method, e);
                }
            }
            
            result
        } else {
            let error_msg = format!("Component {address} not found");
            tracing::error!("❌ {}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    /// 🎨 Render component HTML directly
    /// 
    /// **Arguments**:
    /// - `address`: Component to render like "button:my-btn"
    /// 
    /// **Returns**: HTML string or error
    pub async fn render_component(&self, address: &str) -> anyhow::Result<String> {
        let components = self.components.read().await;
        
        if let Some(component) = components.get(address) {
            tracing::debug!("🎨 Rendering {} directly", address);
            component.render().await
        } else {
            let error_msg = format!("Component {address} not found for rendering");
            tracing::error!("❌ {}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    }

    /// 📊 Get component status and metrics
    /// 
    /// **Returns**: Status information for all registered components
    pub async fn get_status(&self) -> serde_json::Value {
        let components = self.components.read().await;
        
        let mut status = serde_json::Map::new();
        status.insert("total_components".to_string(), serde_json::Value::Number(components.len().into()));
        
        let mut component_list = Vec::new();
        for (address, component) in components.iter() {
            component_list.push(serde_json::json!({
                "address": address,
                "id": component.id(),
                "version": component.current_version(),
            }));
        }
        
        status.insert("components".to_string(), serde_json::Value::Array(component_list));
        serde_json::Value::Object(status)
    }

    /// 🔍 List all registered components
    pub async fn list_components(&self) -> Vec<String> {
        let components = self.components.read().await;
        components.keys().cloned().collect()
    }

    /// 🧹 Helper: Determine if method affects browser state
    /// 
    /// Methods like "click", "set_text", etc. should notify browsers of changes.
    /// Methods like "status" are read-only and don't need browser notifications.
    fn should_notify_browsers(&self, method: &str) -> bool {
        match method {
            // 📖 Read-only methods - no browser notification needed
            "render" | "status" | "get_updates" | "get" => false,
            
            // ✏️ State-changing methods - notify browsers
            "click" | "set_text" | "reset" | "increment" | "set" | "delete" => true,
            
            // 🔧 Default: notify for unknown methods (safer)
            _ => {
                tracing::debug!("🔧 Unknown method '{}', defaulting to browser notification", method);
                true
            }
        }
    }

    /// 🌐 Helper: Notify WebSocket clients of component updates
    /// 
    /// When a component's state changes, browsers need to be updated.
    /// This preserves the real-time update functionality.
    async fn notify_browsers_of_update(
        &self,
        address: &str,
        method: &str,
        _response: &serde_json::Value,
    ) -> anyhow::Result<()> {
        if let Some(ref websocket_sender) = self.websocket_sender {
            // 📡 Create update notification for browsers
            let update_message = serde_json::json!({
                "type": "component_update",
                "address": address,
                "method": method,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            
            if let Err(e) = websocket_sender.send(update_message.to_string()) {
                tracing::error!("❌ Failed to notify browsers of update: {}", e);
                return Err(anyhow::anyhow!("WebSocket notification failed: {}", e));
            } else {
                tracing::debug!("📡 Notified browsers of {}.{} update", address, method);
            }
        }
        
        Ok(())
    }
}

impl Default for AdvancedSimpleComponentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // unused import removed


    // create_test_bus_handle removed - was unused test code

    // Mock component removed - was unused test code

    // impl Component for MockComponent removed - was orphaned after struct deletion
    // All methods removed - MockComponent struct was deleted



    // =====================================================================================
    // 🧪 SIMPLIFIED COMPONENT SYSTEM TESTS  
    // =====================================================================================

    /// 🧪 Test component implementation for simplified system
    struct TestSimpleComponent {
        id: String,
        counter: u64,
    }

    #[async_trait]
    impl Component for TestSimpleComponent {
        async fn handle_message(&mut self, method: &str, payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
            match method {
                "click" => {
                    self.counter += 1;
                    Ok(serde_json::json!({"counter": self.counter}))
                }
                "reset" => {
                    self.counter = 0;
                    Ok(serde_json::json!({"counter": self.counter}))
                }
                "get_counter" => {
                    Ok(serde_json::json!({"counter": self.counter}))
                }
                "set_counter" => {
                    if let Some(value) = payload.get("value").and_then(|v| v.as_u64()) {
                        self.counter = value;
                        Ok(serde_json::json!({"counter": self.counter}))
                    } else {
                        Err(anyhow::anyhow!("Invalid counter value"))
                    }
                }
                _ => Err(anyhow::anyhow!("Unknown method: {}", method))
            }
        }

        async fn render(&self) -> anyhow::Result<String> {
            Ok(format!("<button>Counter: {}</button>", self.counter))
        }

        fn id(&self) -> &str {
            &self.id
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_simple_component_registration() {
        let manager = AdvancedSimpleComponentManager::new();
        let component = TestSimpleComponent {
            id: "test-btn".to_string(),
            counter: 0,
        };

        // Register component
        manager
            .register_component("button:test-btn".to_string(), Box::new(component))
            .await
            .unwrap();

        // Verify registration
        let components = manager.list_components().await;
        assert!(components.contains(&"button:test-btn".to_string()));
    }

    #[tokio::test]
    async fn test_simple_direct_message_handling() {
        let manager = AdvancedSimpleComponentManager::new();
        let component = TestSimpleComponent {
            id: "test-btn".to_string(),
            counter: 0,
        };

        manager
            .register_component("button:test-btn".to_string(), Box::new(component))
            .await
            .unwrap();

        // Test click method
        let response = manager
            .send_message("button:test-btn", "click", serde_json::Value::Null)
            .await
            .unwrap();

        assert_eq!(response["counter"], 1);

        // Test reset method
        let response = manager
            .send_message("button:test-btn", "reset", serde_json::Value::Null)
            .await
            .unwrap();

        assert_eq!(response["counter"], 0);
    }

    #[tokio::test]
    async fn test_simple_component_rendering() {
        let manager = AdvancedSimpleComponentManager::new();
        let component = TestSimpleComponent {
            id: "test-btn".to_string(),
            counter: 42,
        };

        manager
            .register_component("button:test-btn".to_string(), Box::new(component))
            .await
            .unwrap();

        // Test rendering
        let html = manager.render_component("button:test-btn").await.unwrap();
        assert!(html.contains("Counter: 42"));
    }

    #[tokio::test]
    async fn test_simple_status_and_metrics() {
        let manager = AdvancedSimpleComponentManager::new();
        let component1 = TestSimpleComponent {
            id: "btn1".to_string(),
            counter: 0,
        };
        let component2 = TestSimpleComponent {
            id: "btn2".to_string(),
            counter: 5,
        };

        manager
            .register_component("button:btn1".to_string(), Box::new(component1))
            .await
            .unwrap();
        manager
            .register_component("button:btn2".to_string(), Box::new(component2))
            .await
            .unwrap();

        // Test status
        let status = manager.get_status().await;
        assert_eq!(status["total_components"], 2);
        assert_eq!(status["components"].as_array().unwrap().len(), 2);
    }
}