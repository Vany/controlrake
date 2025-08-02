use crate::{BusHandle, Message, Component};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

/// 🔧 Helper functions for system message handling - Clean Implementation
///
/// **ARCHITECTURE**: Pure functions that take required parameters directly.
/// No global state dependencies. Fully testable and composable.
///
/// Extract storage key and component ID from message for scoped operations
fn extract_storage_scope(message: &Message) -> Result<(String, String, String)> {
    let key = message.payload
        .get("key")
        .and_then(|k| k.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'key' field in storage request"))?;
    
    let component_id = message.source
        .as_ref()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "system:".to_string());
    
    let scoped_key = format!("{component_id}:{key}");
    
    Ok((key.to_string(), component_id, scoped_key))
}

/// Create standardized storage response
fn create_storage_result(success: bool, key: &str, operation: &str, result_data: Option<Value>, error: Option<String>) -> Value {
    let mut response = serde_json::json!({
        "success": success,
        "key": key,
        "operation": operation
    });
    
    if let Some(data) = result_data {
        response["value"] = data;
    }
    
    if let Some(err) = error {
        response["error"] = Value::String(err);
    }
    
    response
}
///
/// Create storage response message
pub fn create_storage_response(
    message: &Message,
    result: Value,
    success: bool,
    operation: &str,
) -> Option<Message> {
    if let (Some(from_addr), Some(reply_id)) = (&message.source, &message.id) {
        Some(Message {
            target: from_addr.clone(),
            payload: if success {
                result
            } else {
                serde_json::json!({
                    "success": false,
                    "error": result.as_str().unwrap_or("Unknown error"),
                    "operation": operation
                })
            },
            source: Some("system:".parse().unwrap()),
            reply: Some(reply_id.clone()),
            finish: true,
            id: None,
        })
    } else {
        None
    }
}

/// Handle system log messages
pub async fn handle_log(payload: Value) -> Result<()> {
    if let Some(messages) = payload.as_array() {
        let log_messages: Vec<String> = messages
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();

        if !log_messages.is_empty() {
            let combined = log_messages.join(" ");
            tracing::info!("📝 {}", combined);
        }
    } else if let Some(message) = payload.as_str() {
        tracing::info!("📝 {}", message);
    } else {
        tracing::warn!("📝 Invalid log payload format: {:?}", payload);
    }
    Ok(())
}

/// Handle system start - Now spawns default component instances
pub async fn handle_start(
    bus_handle: &BusHandle,
    registry_handle: &crate::component_registry::ComponentRegistryHandle,
    storage: &Arc<super::simple_storage::SimpleStorage>,
    payload: Value
) -> Result<()> {
    tracing::info!("🚀 System starting with config: {:?}", payload);
    
    // Parse configuration to get component settings
    let config: crate::config::Config = serde_json::from_value(payload)
        .map_err(|e| anyhow::anyhow!("Failed to parse system config: {}", e))?;
    
    tracing::info!("🏭 Creating default component instances...");
    
    // Spawn HTTP component (required for web server)
    tracing::info!("🌐 Spawning HTTP default instance...");
    let http_spawn_payload = serde_json::json!({
        "class": "http",
        "instance_id": "",  // Empty for default instance
        "config": serde_json::to_value(&config.http)?
    });
    
    if let Err(e) = handle_spawn(bus_handle, registry_handle, storage, http_spawn_payload).await {
        tracing::error!("❌ Failed to spawn HTTP component: {}", e);
        anyhow::bail!("HTTP component spawn failed: {}", e);
    }
    
    // Spawn Clock component (for time-based functionality)
    tracing::info!("🕰️ Spawning Clock default instance...");
    let clock_spawn_payload = serde_json::json!({
        "class": "clock",
        "instance_id": "",  // Empty for default instance
        "config": {
            "tick_interval_seconds": 1
        }
    });
    
    if let Err(e) = handle_spawn(bus_handle, registry_handle, storage, clock_spawn_payload).await {
        tracing::warn!("⚠️ Failed to spawn Clock component: {}", e);
        // Clock is not critical, continue
    }
    
    // Spawn Twitch component if configured
    if config.twitch.is_some() {
        tracing::info!("📺 Spawning Twitch default instance...");
        let twitch_spawn_payload = serde_json::json!({
            "class": "twitch", 
            "instance_id": "",  // Empty for default instance
            "config": serde_json::to_value(&config.twitch)?
        });
        
        if let Err(e) = handle_spawn(bus_handle, registry_handle, storage, twitch_spawn_payload).await {
            tracing::warn!("⚠️ Failed to spawn Twitch component: {}", e);
            // Twitch is optional, continue
        }
    }
    
    tracing::info!("✅ System startup complete with default components");
    Ok(())
}

/// Handle system stop
pub async fn handle_stop(payload: Value) -> Result<()> {
    tracing::info!("🛑 System stopping with reason: {:?}", payload);
    tracing::info!("✅ System shutdown complete");
    Ok(())
}

/// Handle spawn component - Fixed for macro-based component system
pub async fn handle_spawn(
    bus_handle: &BusHandle, 
    registry_handle: &crate::component_registry::ComponentRegistryHandle,
    storage: &Arc<super::simple_storage::SimpleStorage>,
    payload: Value
) -> Result<()> {
    tracing::info!("🏭 System spawning component: {:?}", payload);

    let class = payload
        .get("class")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'class' field in spawn request"))?;

    let instance_id = payload
        .get("instance_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Build component address
    let component_address = if instance_id.is_empty() {
        format!("{class}:")
    } else {
        format!("{class}:{instance_id}")
    };

    tracing::debug!("🎯 Creating component with address: {}", component_address);

    // Load configuration for the component
    let config_value = if let Some(config) = payload.get("config").cloned() {
        config
    } else {
        // Auto-load component configuration from global config file
        match crate::config::load_config() {
            Ok(global_config) => match class {
                "button" => {
                    // Button uses a simple Config struct with text field
                    serde_json::json!({
                        "text": "Button"
                    })
                }
                "clock" => {
                    // Clock uses a simple Config struct with interval
                    serde_json::json!({
                        "tick_interval_seconds": 1
                    })
                }
                "http" => serde_json::to_value(&global_config.http)?,
                "twitch" => {
                    if let Some(twitch_config) = &global_config.twitch {
                        serde_json::to_value(twitch_config)?
                    } else {
                        tracing::warn!("🔧 No Twitch configuration found in config.toml, using defaults");
                        serde_json::json!({
                            "username": "",
                            "channel": "",
                            "client_id": "",
                            "client_secret": "",
                            "oauth_redirect_url": "http://localhost:8080/twitch/oauth/callback"
                        })
                    }
                }
                _ => {
                    tracing::debug!("🔧 No specific configuration for component class '{}'", class);
                    Value::Null
                }
            },
            Err(e) => {
                tracing::warn!("⚠️ Failed to load global config, using defaults: {}", e);
                Value::Null
            }
        }
    };

    // Create a temporary system reference for component creation
    // We'll need to get this from the calling context properly
    let system_arc = create_temporary_system_for_spawn(registry_handle, storage.clone())?;

    // Create component using macro-generated constructors
    let component: Box<dyn Component> = match class {
        "button" => {
            // Parse Button config
            let button_config: crate::component::button::Config = 
                serde_json::from_value(config_value)
                    .unwrap_or_default();
            
            tracing::debug!("🔘 Creating Button with config: {:?}", button_config);
            let mut button = crate::component::button::Button::new(
                component_address.clone(),
                system_arc.clone(),
                button_config
            )?;
            
            // Initialize the component
            button.new_component().await?;
            
            Box::new(button)
        }
        "clock" => {
            // Parse Clock config  
            let clock_config: crate::component::clock::Config = 
                serde_json::from_value(config_value)
                    .unwrap_or_default();
            
            tracing::debug!("🕰️ Creating Clock with config: {:?}", clock_config);
            let mut clock = crate::component::clock::Clock::new(
                component_address.clone(),
                system_arc.clone(),
                clock_config
            )?;
            
            // Initialize the component
            clock.new_component().await?;
            
            Box::new(clock)
        }
        "http" => {
            // Parse HTTP config
            let http_config: crate::component::http::Config = 
                serde_json::from_value(config_value)
                    .unwrap_or_default();
            
            tracing::debug!("🌐 Creating HTTP with config: {:?}", http_config);
            let mut http = crate::component::http::Http::new(
                component_address.clone(),
                system_arc.clone(),
                http_config
            )?;
            
            // Initialize the component
            http.new_component().await?;
            
            Box::new(http)
        }
        "twitch" => {
            // Parse Twitch config
            let twitch_config: crate::component::twitch::Config = 
                serde_json::from_value(config_value)
                    .unwrap_or_default();
            
            tracing::debug!("📺 Creating Twitch with config: {:?}", twitch_config);
            let mut twitch = crate::component::twitch::Twitch::new(
                component_address.clone(),
                system_arc.clone(),
                twitch_config
            )?;
            
            // Initialize the component
            twitch.new_component().await?;
            
            Box::new(twitch)
        }
        _ => {
            anyhow::bail!(
                "Unknown component class '{}'. Supported: button, clock, http, twitch", 
                class
            );
        }
    };

    // Log creation
    if instance_id.is_empty() {
        tracing::info!("🆕 Created default component {}:", class);
    } else {
        tracing::info!("🆕 Created component {}:{}", class, instance_id);
    }

    // Step 1: Register component with ComponentRegistry for component management
    registry_handle.register_component(class, instance_id, component).await
        .map_err(|e| anyhow::anyhow!("Failed to register component {} in ComponentRegistry: {}", component_address, e))?;
    
    // Step 2: Create and register message channel with Bus for message routing
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    
    if instance_id.is_empty() {
        bus_handle.register_default_instance(class, tx);
    } else {
        bus_handle.register_instance(class, instance_id, tx);
    }
    
    // Step 3: Start message handling for the component
    let component_address_for_task = component_address.clone();
    let _registry_for_task = registry_handle.clone();
    
    tokio::spawn(async move {
        tracing::debug!("🎯 Starting message handler for component: {}", component_address_for_task);
        
        while let Some(message) = rx.recv().await {
            // Get the component from registry and handle the message
            let method = match &message.target.method {
                Some(m) => m.clone(),
                None => {
                    tracing::warn!("🤷 Message without method for component {}: {:?}", component_address_for_task, message.target);
                    continue;
                }
            };
            
            // TODO: Handle component message routing through ComponentRegistry
            // This will require a more sophisticated approach to get mutable component access
            tracing::debug!("📨 Component {} received method call: {}", component_address_for_task, method);
        }
        
        tracing::debug!("🛑 Message handler stopped for component: {}", component_address_for_task);
    });

    // Log startup success
    if instance_id.is_empty() {
        tracing::info!("✅ Started and registered default component {}:", class);
    } else {
        tracing::info!("✅ Started and registered component {}:{}", class, instance_id);
    }

    Ok(())
}

/// Create temporary system reference for spawning components
/// This is a workaround for the circular dependency issue during component creation
fn create_temporary_system_for_spawn(
    registry_handle: &crate::component_registry::ComponentRegistryHandle,
    _storage: Arc<super::simple_storage::SimpleStorage>
) -> Result<Arc<super::SystemComponent>> {
    // Create a minimal bus handle for the temporary system
    let temp_bus = crate::bus::BusHandle::new_test();
    
    // Create temporary system component for spawning - handle database lock errors
    let temp_system = match super::SystemComponent::new_with_bus(temp_bus, registry_handle.clone()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("🚨 Failed to create temporary system for spawning: {:#}", e);
            anyhow::bail!("Temporary system creation failed during component spawn: {e}");
        }
    };
    
    Ok(Arc::new(temp_system))
}

/// Handle storage set operation - Clean implementation
pub async fn handle_storage_set(
    storage: &Arc<super::simple_storage::SimpleStorage>,
    bus_handle: &BusHandle,
    message: Message
) -> Result<()> {
    tracing::debug!("🗄️ Storage set operation");

    // Extract storage scope and validate payload
    let (key, component_id, scoped_key) = extract_storage_scope(&message)?;
    
    let value = message.payload
        .get("value")
        .ok_or_else(|| anyhow::anyhow!("Missing 'value' field in storage set request"))?
        .clone();

    // Perform storage operation
    let component_class = component_id.split(':').next().unwrap_or("unknown");
    let result = storage.save_component(&scoped_key, value, Some(component_class.to_string())).await;
    
    // Create standardized response
    let (success, response_value) = match result {
        Ok(()) => (true, create_storage_result(true, &key, "set", None, None)),
        Err(e) => (false, create_storage_result(false, &key, "set", None, Some(e.to_string())))
    };

    // Send response
    if let Some(response_msg) = create_storage_response(&message, response_value, success, "set") {
        bus_handle.send(response_msg).await?;
    }

    Ok(())
}

/// Handle storage get operation - Clean implementation
pub async fn handle_storage_get(
    storage: &Arc<super::simple_storage::SimpleStorage>,
    bus_handle: &BusHandle,
    message: Message
) -> Result<()> {
    tracing::debug!("🗄️ Storage get operation");

    // Extract storage scope
    let (key, _component_id, scoped_key) = extract_storage_scope(&message)?;

    // Perform storage operation
    let result = storage.load_component(&scoped_key).await;
    
    // Create standardized response
    let (success, response_value) = match result {
        Ok(Some(value)) => (true, create_storage_result(true, &key, "get", Some(value), None)),
        Ok(None) => (true, create_storage_result(true, &key, "get", Some(Value::Null), None)),
        Err(e) => (false, create_storage_result(false, &key, "get", None, Some(e.to_string())))
    };

    // Send response
    if let Some(response_msg) = create_storage_response(&message, response_value, success, "get") {
        bus_handle.send(response_msg).await?;
    }

    Ok(())
}

/// Handle storage delete operation - Clean implementation
pub async fn handle_storage_delete(
    storage: &Arc<super::simple_storage::SimpleStorage>,
    bus_handle: &BusHandle,
    message: Message
) -> Result<()> {
    tracing::debug!("🗄️ Storage delete operation");

    // Extract storage scope
    let (key, _component_id, scoped_key) = extract_storage_scope(&message)?;

    // Perform storage operation
    let result = storage.delete_component(&scoped_key).await;
    
    // Create standardized response
    let (success, response_value) = match result {
        Ok(deleted) => (true, create_storage_result(true, &key, "delete", Some(serde_json::json!({"deleted": deleted})), None)),
        Err(e) => (false, create_storage_result(false, &key, "delete", None, Some(e.to_string())))
    };

    // Send response
    if let Some(response_msg) = create_storage_response(&message, response_value, success, "delete") {
        bus_handle.send(response_msg).await?;
    }

    Ok(())
}
