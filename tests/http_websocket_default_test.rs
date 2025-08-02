/// 🧪 HTTP WebSocket Default Instance Test
/// 
/// Tests WebSocket functionality with default instance (empty ID).
/// Verifies that default instance routing works correctly for WebSocket connections.

use controlrake::{Bus, SystemComponent};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, debug};

#[tokio::test]
async fn test_http_websocket_default_instance() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing HTTP WebSocket with default instance");

    // Create system with bus
    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn HTTP default instance (empty ID)
    let spawn_result = bus_handle.call("system:.spawn", json!({
        "class": "http",
        "id": "",
        "config": {
            "port": 8081,
            "bind_address": "127.0.0.1",
            "base_url": "http://localhost",
            "static_folder": "./static",
            "auth_key": "test-key"
        }
    })).await;

    debug!("📋 HTTP spawn result: {:?}", spawn_result);
    assert!(spawn_result.is_ok(), "Failed to spawn HTTP default instance: {:?}", spawn_result);

    // Wait for HTTP server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test 1: Register a WebSocket route using default instance
    let register_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.register", json!({
            "path": "/ws/test",
            "target": "test:receiver",
            "method": "handle_websocket",
            "http_method": "GET"
        }))
    ).await;

    debug!("📋 Register result: {:?}", register_result);
    assert!(register_result.is_ok(), "Registration request timeout");
    
    let register_response = register_result.unwrap();
    assert!(register_response.is_ok(), "Failed to register WebSocket route: {:?}", register_response);
    
    let register_data = register_response.unwrap();
    assert_eq!(register_data.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(register_data.get("path").and_then(|v| v.as_str()), Some("/ws/test"));
    assert_eq!(register_data.get("target").and_then(|v| v.as_str()), Some("test:receiver"));

    info!("✅ WebSocket route registration successful for default instance");

    // Test 2: Check WebSocket stats for default instance
    let stats_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.websocket_stats", json!({}))
    ).await;

    debug!("📋 Stats result: {:?}", stats_result);
    assert!(stats_result.is_ok(), "Stats request timeout");
    
    let stats_response = stats_result.unwrap();
    assert!(stats_response.is_ok(), "Failed to get WebSocket stats: {:?}", stats_response);
    
    let stats_data = stats_response.unwrap();
    let callback_count = stats_data
        .get("http_callbacks")
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert!(callback_count >= 1, "Expected at least 1 registered callback, got {}", callback_count);

    info!("✅ WebSocket stats verification successful for default instance");

    // Test 3: Test WebSocket broadcast functionality
    let broadcast_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.broadcast", json!({
            "type": "event",
            "event_type": "test_websocket",
            "source": "test:source",
            "data": {
                "message": "WebSocket test for default instance",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        }))
    ).await;

    debug!("📋 Broadcast result: {:?}", broadcast_result);
    assert!(broadcast_result.is_ok(), "Broadcast request timeout");
    
    let broadcast_response = broadcast_result.unwrap();
    assert!(broadcast_response.is_ok(), "Failed to broadcast WebSocket event: {:?}", broadcast_response);
    
    let broadcast_data = broadcast_response.unwrap();
    assert_eq!(broadcast_data.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(broadcast_data.get("broadcast_type").and_then(|v| v.as_str()), Some("event"));

    info!("✅ WebSocket broadcast functionality verified for default instance");

    // Test 4: Unregister the WebSocket route
    let unregister_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.unregister", json!({
            "path": "/ws/test"
        }))
    ).await;

    debug!("📋 Unregister result: {:?}", unregister_result);
    assert!(unregister_result.is_ok(), "Unregister request timeout");
    
    let unregister_response = unregister_result.unwrap();
    assert!(unregister_response.is_ok(), "Failed to unregister WebSocket route: {:?}", unregister_response);
    
    let unregister_data = unregister_response.unwrap();
    assert_eq!(unregister_data.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(unregister_data.get("was_removed").and_then(|v| v.as_bool()), Some(true));

    info!("✅ WebSocket route unregistration successful for default instance");

    // Test 5: Verify default instance addressing
    let status_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.get_status", json!({}))
    ).await;

    debug!("📋 Status result: {:?}", status_result);
    assert!(status_result.is_ok(), "Status request timeout");
    
    let status_response = status_result.unwrap();
    assert!(status_response.is_ok(), "Failed to get HTTP status: {:?}", status_response);
    
    let status_data = status_response.unwrap();
    
    // Verify that this is the default instance
    assert_eq!(status_data.get("id").and_then(|v| v.as_str()), Some(""));
    assert_eq!(status_data.get("port").and_then(|v| v.as_u64()), Some(8081));

    info!("✅ Default instance addressing verified correctly");

    info!("🎉 All HTTP WebSocket default instance tests passed!");
}

#[tokio::test]
async fn test_websocket_default_vs_named_instance_isolation() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing WebSocket isolation between default and named instances");

    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn HTTP default instance
    let default_result = bus_handle.call("system:.spawn", json!({
        "class": "http",
        "id": "",
        "config": {
        "port": 8082,
        "bind_address": "127.0.0.1",
        "base_url": "http://localhost",
        "static_folder": "./static",
        "auth_key": "test-key-default"
        }
    })).await;
    assert!(default_result.is_ok(), "Failed to spawn default instance");

    // Spawn HTTP named instance 
    let named_result = bus_handle.call("system:.spawn", json!({
        "class": "http",
        "id": "secondary",
        "config": {
        "port": 8083,
        "bind_address": "127.0.0.1",
        "base_url": "http://localhost",
        "static_folder": "./static",
        "auth_key": "test-key-secondary"
        }
    })).await;
    assert!(named_result.is_ok(), "Failed to spawn named instance");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Register WebSocket routes on both instances
    let default_register = bus_handle.call("http:.register", json!({
        "path": "/ws/default",
        "target": "test:default",
        "method": "handle_websocket"
    })).await;
    assert!(default_register.is_ok());

    let named_register = bus_handle.call("http:secondary.register", json!({
        "path": "/ws/named", 
        "target": "test:named",
        "method": "handle_websocket"
    })).await;
    assert!(named_register.is_ok());

    // Verify each instance has only its own routes
    let default_stats = bus_handle.call("http:.websocket_stats", json!({})).await.unwrap();
    let named_stats = bus_handle.call("http:secondary.websocket_stats", json!({})).await.unwrap();

    let default_callbacks = default_stats
        .get("http_callbacks")
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .unwrap();

    let named_callbacks = named_stats
        .get("http_callbacks")
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .unwrap();

    // Each instance should have exactly 1 callback
    assert_eq!(default_callbacks.len(), 1);
    assert_eq!(named_callbacks.len(), 1);

    // Verify correct paths are registered
    assert_eq!(
        default_callbacks[0].get("path").and_then(|v| v.as_str()),
        Some("/ws/default")
    );
    assert_eq!(
        named_callbacks[0].get("path").and_then(|v| v.as_str()),
        Some("/ws/named")
    );

    info!("✅ WebSocket instance isolation verified correctly");
}
