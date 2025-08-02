/// 🧪 HTTP Route Registration Default Instance Test
/// 
/// Tests HTTP route registration functionality with default instance (empty ID).
/// Verifies that default instance routing works correctly for HTTP callback registration.

use controlrake::{Bus, SystemComponent};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, debug};

#[tokio::test]
async fn test_http_route_registration_default_instance() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing HTTP route registration with default instance");

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
        "port": 8084,
        "bind_address": "127.0.0.1",
        "base_url": "http://localhost",
        "static_folder": "./static",
        "auth_key": "test-key-routes"
        }
    })).await;

    debug!("📋 HTTP spawn result: {:?}", spawn_result);
    assert!(spawn_result.is_ok(), "Failed to spawn HTTP default instance: {:?}", spawn_result);

    // Wait for HTTP server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test 1: Register multiple HTTP routes using default instance
    let routes_to_register = vec![
        ("/api/button/click", "button:test", "handle_click", "POST"),
        ("/api/clock/tick", "clock:", "handle_tick", "POST"), 
        ("/api/twitch/chat", "twitch:", "handle_chat", "POST"),
        ("/webhook/callback", "system:", "handle_webhook", "POST"),
    ];

    for (path, target, method, http_method) in routes_to_register.iter() {
        let register_result = timeout(
            Duration::from_secs(5),
            bus_handle.call("http:.register", json!({
                "path": path,
                "target": target,
                "method": method,
                "http_method": http_method
            }))
        ).await;

        debug!("📋 Register result for {}: {:?}", path, register_result);
        assert!(register_result.is_ok(), "Registration request timeout for {}", path);
        
        let register_response = register_result.unwrap();
        assert!(register_response.is_ok(), "Failed to register route {}: {:?}", path, register_response);
        
        let register_data = register_response.unwrap();
        assert_eq!(register_data.get("success").and_then(|v| v.as_bool()), Some(true)); 
        assert_eq!(register_data.get("path").and_then(|v| v.as_str()), Some(*path));
        assert_eq!(register_data.get("target").and_then(|v| v.as_str()), Some(*target));

        info!("✅ Route {} registered successfully for default instance", path);
    }

    // Test 2: Verify all routes are registered by checking HTTP stats
    let stats_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.websocket_stats", json!({}))
    ).await;

    debug!("📋 Stats result: {:?}", stats_result);
    assert!(stats_result.is_ok(), "Stats request timeout");
    
    let stats_response = stats_result.unwrap();
    assert!(stats_response.is_ok(), "Failed to get HTTP stats: {:?}", stats_response);
    
    let stats_data = stats_response.unwrap();
    let callback_count = stats_data
        .get("http_callbacks")
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert_eq!(callback_count, 4, "Expected 4 registered callbacks, got {}", callback_count);

    let callback_details = stats_data
        .get("http_callbacks")
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .unwrap();

    // Verify each registered route appears in the details
    for (path, target, method, http_method) in routes_to_register.iter() {
        let found_route = callback_details.iter().find(|route| {
            route.get("path").and_then(|v| v.as_str()) == Some(*path)
                && route.get("target").and_then(|v| v.as_str()) == Some(*target)
                && route.get("method").and_then(|v| v.as_str()) == Some(*method)
                && route.get("http_method").and_then(|v| v.as_str()) == Some(*http_method)
        });

        assert!(found_route.is_some(), "Route {} not found in registered callbacks", path);
    }

    info!("✅ All route registrations verified for default instance");

    // Test 3: Test route unregistration
    let unregister_path = "/api/button/click";
    let unregister_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.unregister", json!({
            "path": unregister_path
        }))
    ).await;

    debug!("📋 Unregister result: {:?}", unregister_result);
    assert!(unregister_result.is_ok(), "Unregister request timeout");
    
    let unregister_response = unregister_result.unwrap();
    assert!(unregister_response.is_ok(), "Failed to unregister route: {:?}", unregister_response);
    
    let unregister_data = unregister_response.unwrap();
    assert_eq!(unregister_data.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(unregister_data.get("was_removed").and_then(|v| v.as_bool()), Some(true));

    info!("✅ Route unregistration successful for default instance");

    // Test 4: Verify route count decreased
    let final_stats = bus_handle.call("http:.websocket_stats", json!({})).await.unwrap();
    let final_count = final_stats
        .get("http_callbacks")
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert_eq!(final_count, 3, "Expected 3 callbacks after unregistration, got {}", final_count);

    // Test 5: Test default instance identification
    let status_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("http:.get_status", json!({}))
    ).await;

    assert!(status_result.is_ok(), "Status request timeout");
    let status_response = status_result.unwrap();
    assert!(status_response.is_ok(), "Failed to get HTTP status");
    let status_data = status_response.unwrap();
    
    // Verify this is the default instance
    assert_eq!(status_data.get("id").and_then(|v| v.as_str()), Some(""));
    assert_eq!(status_data.get("port").and_then(|v| v.as_u64()), Some(8084));

    info!("✅ Default instance addressing verified for route registration");

    info!("🎉 All HTTP route registration default instance tests passed!");
}

#[tokio::test]  
async fn test_route_registration_default_vs_named_isolation() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing route registration isolation between default and named instances");

    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn HTTP default instance
    let default_result = bus_handle.call("system:.spawn", json!({
        "class": "http",
        "id": "",
        "config": {
        "port": 8085,
        "bind_address": "127.0.0.1",
        "base_url": "http://localhost",
        "static_folder": "./static",
        "auth_key": "test-key-default-routes"
        }
    })).await;
    assert!(default_result.is_ok(), "Failed to spawn default instance");

    // Spawn HTTP named instance
    let named_result = bus_handle.call("system:.spawn", json!({
        "class": "http",
        "id": "api",
        "config": {
        "port": 8086,
        "bind_address": "127.0.0.1", 
        "base_url": "http://localhost",
        "static_folder": "./static",
        "auth_key": "test-key-api-routes"
        }
    })).await;
    assert!(named_result.is_ok(), "Failed to spawn named instance");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Register routes on both instances
    let default_register = bus_handle.call("http:.register", json!({
        "path": "/default/route",
        "target": "target:default",
        "method": "handle_default",
        "http_method": "GET"
    })).await;
    assert!(default_register.is_ok(), "Failed to register default route");

    let named_register = bus_handle.call("http:api.register", json!({
        "path": "/api/route", 
        "target": "target:api",
        "method": "handle_api",
        "http_method": "POST"
    })).await;
    assert!(named_register.is_ok(), "Failed to register named route");

    // Verify isolation: each instance should have only its own routes
    let default_stats = bus_handle.call("http:.websocket_stats", json!({})).await.unwrap();
    let named_stats = bus_handle.call("http:api.websocket_stats", json!({})).await.unwrap();

    let default_count = default_stats
        .get("http_callbacks")
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let named_count = named_stats
        .get("http_callbacks")
        .and_then(|v| v.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    assert_eq!(default_count, 1, "Default instance should have 1 route");
    assert_eq!(named_count, 1, "Named instance should have 1 route");

    // Verify correct routes are registered on each instance
    let default_routes = default_stats
        .get("http_callbacks")
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .unwrap();

    let named_routes = named_stats
        .get("http_callbacks")
        .and_then(|v| v.get("details"))
        .and_then(|v| v.as_array())
        .unwrap();

    assert_eq!(
        default_routes[0].get("path").and_then(|v| v.as_str()),
        Some("/default/route")
    );
    assert_eq!(
        named_routes[0].get("path").and_then(|v| v.as_str()),
        Some("/api/route")
    );

    info!("✅ Route registration instance isolation verified correctly");
}
