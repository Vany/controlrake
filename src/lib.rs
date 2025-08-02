pub mod address;
pub mod bus;
pub mod component;
pub mod component_registry;
pub mod component_result;
pub mod config;
pub mod message;
pub mod update;

// Testing modules
#[cfg(test)]
pub mod test_component_response;

// Testing module (temporarily disabled due to compilation issues)
// pub mod testing;

// ============================================================================
// 🚀 EXPORTS
// ============================================================================

// Re-export macros from the controlrake_macros crate
pub use controlrake_macros::{component, component_response, method};

pub use address::Address;
pub use bus::{Bus, BusHandle, BusStats};
pub use component::{
    // Re-export components from their new locations
    button::Button,
    clock::Clock,
    http::{Http, HttpStats},
    system::{
        simple_storage::{ComponentMetadata, SimpleStorage, StoredComponent},
        SystemComponent,
    },
    twitch::Twitch,
    AdvancedSimpleComponentManager,
    // Simplified component system
    Component,
    SimpleRenderer,
};
pub use component_registry::{ComponentRegistry, ComponentRegistryHandle, ComponentRegistryStats};
pub use component_result::{ComponentJsonResult, ComponentResult, ComponentVoidResult};
pub use config::{Config, HttpConfig, SystemConfig, UpdateConfig};
pub use message::{Message, MessageError, MessageId, RawMessage};
pub use update::{BatchMode, InsertPosition, UpdateBatch, UpdateMessage};

// ============================================================================
// 🌐 GLOBAL COMPONENT REGISTRY STORAGE
// ============================================================================

use std::sync::OnceLock;

/// Global storage for Bus instance (prevents routing task from being dropped)
static BUS_STORAGE: OnceLock<Box<Bus>> = OnceLock::new();

/// Global storage for ComponentRegistry instance
static REGISTRY_STORAGE: OnceLock<ComponentRegistryHandle> = OnceLock::new();

/// Global storage for BusHandle instance (for shutdown operations)
static BUS_HANDLE_STORAGE: OnceLock<BusHandle> = OnceLock::new();

/// Global storage for ProcessLock instance (keeps flock alive)
static PROCESS_LOCK_STORAGE: OnceLock<component::system::process_lock::ProcessLock> = OnceLock::new();

/// 🚀 Initialize the control rake system with clean architecture
pub async fn init() -> anyhow::Result<(BusHandle, ComponentRegistryHandle)> {
    // Step 1: Load configuration FIRST (before logging init)
    let config = crate::config::load_config()?;

    // Step 2: Initialize tracing with config log level and custom time format
    let log_level = match config.system.log_level.as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO, // fallback
    };

    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_max_level(log_level)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%d %H:%M:%S".to_string(),
        ))
        .init();

    tracing::info!("🚀 ControlRake initializing with clean architecture");
    tracing::debug!(
        "📋 Configuration loaded with log_level: {}",
        config.system.log_level
    );

    // Step 3: Acquire process lock with database recovery (AFTER logging init)
    let process_lock = component::system::database_recovery::acquire_process_lock_with_recovery()?;

    // Step 4: Create message bus
    tracing::info!("🚌 Creating message bus");
    let bus = Bus::new().await;
    let bus_handle = bus.handle();

    // Step 5: Create component registry alongside bus
    tracing::info!("📦 Creating component registry");
    let component_registry = component_registry::ComponentRegistry::new();
    let registry_handle = component_registry::ComponentRegistryHandle::new(component_registry);

    // CRITICAL: Store the bus, registry, and process lock in statics to prevent them from being dropped
    // The bus contains the routing task that must remain alive
    // The process lock must remain alive to maintain the flock
    // This is a controlled static allocation, not a memory leak
    if BUS_STORAGE.set(Box::new(bus)).is_err() {
        anyhow::bail!("Bus already initialized - init() should only be called once");
    }

    if REGISTRY_STORAGE.set(registry_handle.clone()).is_err() {
        anyhow::bail!("ComponentRegistry already initialized - init() should only be called once");
    }

    if BUS_HANDLE_STORAGE.set(bus_handle.clone()).is_err() {
        anyhow::bail!("BusHandle already initialized - init() should only be called once");
    }

    if PROCESS_LOCK_STORAGE.set(process_lock).is_err() {
        anyhow::bail!("ProcessLock already initialized - init() should only be called once");
    }

    tracing::info!(
        "✅ Message bus, ComponentRegistry, BusHandle, and ProcessLock created and stored in static locations"
    );

    // Step 6: Create system component with clean architecture - handle database lock errors
    tracing::info!("🎛️ Creating system component");
    let mut system = match component::system::SystemComponent::new_with_bus(
        bus_handle.clone(),
        registry_handle.clone(),
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("🚨 Failed to create system component: {:#}", e);
            anyhow::bail!("System component initialization failed: {e}");
        }
    };

    // Step 7: Initialize system configuration
    tracing::info!("🔧 Initializing system configuration");
    system.init_config(&config)?;

    // Step 8: Register SystemComponent in ComponentRegistry like any other component
    tracing::info!("📦 Registering SystemComponent in ComponentRegistry");
    registry_handle
        .register_component("system", "", Box::new(system))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to register SystemComponent: {}", e))?;

    // Step 9: Send system start message
    tracing::info!("🚀 Starting system via message bus");
    let config_value = serde_json::to_value(&config)?;
    bus_handle
        .send_raw(&format!("system:.start {config_value}"))
        .await?;

    tracing::info!("✅ ControlRake initialization complete with clean architecture");
    tracing::info!(
        "🌐 HTTP server should be available on port {}",
        config.http.port
    );
    tracing::info!("📁 Static files served from: {}", config.http.static_folder);

    Ok((bus_handle, registry_handle))
}

/// 🛑 Initiate graceful bus shutdown
///
/// **Purpose**: Shuts down the message bus gracefully through a special shutdown message
/// **Usage**: Used by main.rs shutdown handler to shut down the bus
/// **Returns**: Result indicating success or failure of shutdown initiation
///
/// **Implementation**: We send a special shutdown message that the routing task will recognize
/// and use to break out of its message loop, causing graceful shutdown.
pub async fn shutdown_bus() -> anyhow::Result<()> {
    if let Some(bus_handle) = BUS_HANDLE_STORAGE.get() {
        tracing::info!("🚌 Initiating bus shutdown via shutdown signal...");

        // Send a special message that will be recognized by the routing task as a shutdown signal
        // We'll use a reserved address that the routing task can check for
        match bus_handle.send_raw("__internal__:.shutdown").await {
            Ok(_) => {
                tracing::info!("✅ Bus shutdown signal sent successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ Failed to send bus shutdown signal: {}", e);
                Err(anyhow::anyhow!("Failed to send bus shutdown signal: {}", e))
            }
        }
    } else {
        anyhow::bail!("BusHandle not initialized - cannot shutdown bus")
    }
}

/// 🛑 Initiate graceful system shutdown
///
/// **Purpose**: Directly shuts down the system component without using the message bus
/// **Usage**: Used by main.rs shutdown handler to initiate system shutdown  
/// **Returns**: Result indicating success or failure of shutdown completion
/// 
/// **Implementation**: Since we're in the same server process, we get the 
/// SystemComponent directly from the ComponentRegistry and call its stop() method,
/// which is more efficient than sending a message through the bus and allows us
/// to wait for actual completion rather than just message delivery.
/// 
/// **Benefits**:
/// - More efficient (no message bus overhead)  
/// - Direct method call (no async message routing)
/// - Actual completion waiting (not just message sent)
/// - More reliable (direct call vs bus message)
pub async fn shutdown_system() -> anyhow::Result<()> {
    if let Some(registry) = REGISTRY_STORAGE.get() {
        tracing::info!("🎛️ Directly shutting down system component...");
        
        // Call specialized stop method on SystemComponent
        match registry.stop_system_component().await {
            Some(result) => {
                match result {
                    Ok(_) => {
                        tracing::info!("✅ System component stopped successfully");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("❌ System component stop failed: {}", e);
                        Err(anyhow::anyhow!("System component stop failed: {}", e))
                    }
                }
            }
            None => {
                tracing::error!("❌ System component not found in registry");
                Err(anyhow::anyhow!("System component not found in registry"))
            }
        }
    } else {
        anyhow::bail!("ComponentRegistry not initialized - cannot shutdown system")
    }
}

/// 🔍 Get component by typed reference (global accessor)
///
/// **Arguments**:
/// - `class`: Component class name (e.g., "button", "http", "system")
/// - `instance_id`: Instance identifier (empty string for default instances)
///   🔍 Get component by typed reference (global accessor - async version)
///
/// **Arguments**:
/// - `class`: Component class name (e.g., "button", "http", "system")
/// - `instance_id`: Instance identifier (empty string for default instances)
///
/// **Returns**: Option<T> - Cloned component data if found and type matches
///
/// **Usage**: `let button = controlrake::get_component::<Button>("button", "my-btn")?;`
///
/// **Thread Safety**: Uses async access to ComponentRegistry
/// **Performance**: Direct registry lookup with type checking
pub async fn get_component_async<T: Component + Clone + 'static>(
    class: &str,
    instance_id: &str,
) -> Option<T> {
    if let Some(registry) = REGISTRY_STORAGE.get() {
        registry.get_component::<T>(class, instance_id).await
    } else {
        tracing::error!("🚨 ComponentRegistry not initialized - call controlrake::init() first");
        None
    }
}

/// 🔍 Check if component exists (global accessor)
///
/// **Arguments**:
/// - `class`: Component class name
/// - `instance_id`: Instance identifier (empty string for default instances)
///
/// **Returns**: bool - true if component is registered
///
/// **Usage**: `if controlrake::has_component("button", "my-btn") { ... }`
///
/// **Thread Safety**: Uses async access to ComponentRegistry
/// **Performance**: Fast existence check without data transfer
pub async fn has_component(class: &str, instance_id: &str) -> bool {
    if let Some(registry) = REGISTRY_STORAGE.get() {
        registry.has_component(class, instance_id).await
    } else {
        tracing::error!("🚨 ComponentRegistry not initialized - call controlrake::init() first");
        false
    }
}

/// 📋 List all registered component keys (global accessor)
///
/// **Returns**: Vec<String> - All component keys in format "class:" or "class:id"
///
/// **Usage**: `let components = controlrake::list_components().await;`
///
/// **Thread Safety**: Uses async access to ComponentRegistry
/// **Use Cases**: Debugging, monitoring, component discovery
pub async fn list_components() -> Vec<String> {
    if let Some(registry) = REGISTRY_STORAGE.get() {
        registry.list_components().await
    } else {
        tracing::error!("🚨 ComponentRegistry not initialized - call controlrake::init() first");
        Vec::new()
    }
}
