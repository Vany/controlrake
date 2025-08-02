/// 🧪 Twitch IRC Default Instance Test
/// 
/// Tests IRC connection functionality with default instance (empty ID).
/// Verifies that default instance routing and IRC operations work correctly.

use controlrake::{Bus, SystemComponent};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, debug};

#[tokio::test]
async fn test_twitch_irc_default_instance() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing Twitch IRC with default instance");

    // Create system with bus
    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn Twitch default instance (empty ID)
    let spawn_result = bus_handle.call("system:.spawn", json!({
        "class": "twitch",
        "id": "",
        "config": {
        "username": "test_user",
        "channel": "test_channel",
        "client_id": "test_client_id",
        "client_secret": "test_client_secret",
        "oauth_scopes": "chat:read chat:edit",
        "irc_server": "irc.chat.twitch.tv",
        "irc_port": 6697,
        "max_messages_per_minute": 20,
        "enable_auto_reconnect": true,
        "enable_user_validation": true
        }
    })).await;

    debug!("📋 Twitch spawn result: {:?}", spawn_result);
    assert!(spawn_result.is_ok(), "Failed to spawn Twitch default instance: {:?}", spawn_result);

    // Wait for component initialization
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test 1: Check default instance identification
    let status_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.get_status", json!({}))
    ).await;

    debug!("📋 Status result: {:?}", status_result);
    assert!(status_result.is_ok(), "Status request timeout");
    
    let status_response = status_result.unwrap();
    assert!(status_response.is_ok(), "Failed to get status: {:?}", status_response);
    
    let status_data = status_response.unwrap();
    
    // Verify this is the default instance
    assert_eq!(status_data.get("id").and_then(|v| v.as_str()), Some(""));
    assert_eq!(status_data.get("username").and_then(|v| v.as_str()), Some("test_user"));
    assert_eq!(status_data.get("channel").and_then(|v| v.as_str()), Some("test_channel"));

    info!("✅ Default instance identification verified");

    // Test 2: Test IRC connection attempt (without real credentials)
    let connect_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.connect", json!({}))
    ).await;

    debug!("📋 Connect result: {:?}", connect_result);
    assert!(connect_result.is_ok(), "Connect request timeout");
    
    let connect_response = connect_result.unwrap();
    assert!(connect_response.is_ok(), "Failed to initiate connect: {:?}", connect_response);
    
    let connect_data = connect_response.unwrap();
    // Should initiate OAuth flow or indicate missing credentials
    assert!(
        connect_data.get("success").and_then(|v| v.as_bool()).unwrap_or(false) ||
        connect_data.get("action").and_then(|v| v.as_str()) == Some("oauth_started") ||
        connect_data.get("error").is_some()  // Expected with test credentials
    );

    info!("✅ IRC connection attempt handled correctly for default instance");

    // Test 3: Test message sending interface (should fail gracefully without connection)
    let send_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.send_message", json!({
            "text": "Test message for default instance"
        }))
    ).await;

    debug!("📋 Send message result: {:?}", send_result);
    assert!(send_result.is_ok(), "Send message request timeout");
    
    let send_response = send_result.unwrap();
    assert!(send_response.is_ok(), "Failed to handle send message: {:?}", send_response);
    
    let send_data = send_response.unwrap();
    // Should fail gracefully when not connected
    assert!(send_data.get("error").is_some(), "Expected error when not connected to IRC");

    info!("✅ Message sending interface works correctly for default instance");

    // Test 4: Test OAuth start functionality
    let oauth_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.oauth_start", json!({}))
    ).await;

    debug!("📋 OAuth start result: {:?}", oauth_result);
    assert!(oauth_result.is_ok(), "OAuth start request timeout");
    
    let oauth_response = oauth_result.unwrap();
    assert!(oauth_response.is_ok(), "Failed to start OAuth: {:?}", oauth_response);
    
    let oauth_data = oauth_response.unwrap();
    // Should provide OAuth URL or indicate success
    assert!(
        oauth_data.get("success").and_then(|v| v.as_bool()).unwrap_or(false) ||
        oauth_data.get("oauth_url").and_then(|v| v.as_str()).is_some() ||
        oauth_data.get("error").is_some()  // May error with test config
    );

    info!("✅ OAuth start functionality works correctly for default instance");

    // Test 5: Test metrics and status reporting
    let metrics_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.get_metrics", json!({}))
    ).await;

    debug!("📋 Metrics result: {:?}", metrics_result);
    assert!(metrics_result.is_ok(), "Metrics request timeout");
    
    let metrics_response = metrics_result.unwrap();
    assert!(metrics_response.is_ok(), "Failed to get metrics: {:?}", metrics_response);
    
    let metrics_data = metrics_response.unwrap();
    assert!(metrics_data.get("success").and_then(|v| v.as_bool()).unwrap_or(false));
    assert!(metrics_data.get("metrics").is_some(), "Expected metrics object");

    // Verify metrics structure
    let metrics = metrics_data.get("metrics").unwrap();
    assert!(metrics.get("component").is_some(), "Expected component metrics");
    assert!(metrics.get("connection").is_some(), "Expected connection metrics");
    assert!(metrics.get("messages").is_some(), "Expected message metrics");
    assert!(metrics.get("oauth").is_some(), "Expected OAuth metrics");

    info!("✅ Metrics reporting works correctly for default instance");

    // Test 6: Test disconnect functionality
    let disconnect_result = timeout(
        Duration::from_secs(5),
        bus_handle.call("twitch:.disconnect", json!({}))
    ).await;

    debug!("📋 Disconnect result: {:?}", disconnect_result);
    assert!(disconnect_result.is_ok(), "Disconnect request timeout");
    
    let disconnect_response = disconnect_result.unwrap();
    assert!(disconnect_response.is_ok(), "Failed to disconnect: {:?}", disconnect_response);
    
    let disconnect_data = disconnect_response.unwrap();
    assert_eq!(disconnect_data.get("success").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(disconnect_data.get("status").and_then(|v| v.as_str()), Some("disconnected"));

    info!("✅ Disconnect functionality works correctly for default instance");

    info!("🎉 All Twitch IRC default instance tests passed!");
}

#[tokio::test]
async fn test_twitch_irc_default_vs_named_instance_isolation() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing IRC isolation between default and named Twitch instances");

    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn Twitch default instance
    let default_result = bus_handle.call("system:.spawn", json!({
        "class": "twitch",
        "id": "",
        "config": {
        "username": "default_user",
        "channel": "default_channel",
        "client_id": "default_client",
        "client_secret": "default_secret"
        }
    })).await;
    assert!(default_result.is_ok(), "Failed to spawn default instance");

    // Spawn Twitch named instance
    let named_result = bus_handle.call("system:.spawn", json!({
        "class": "twitch",
        "id": "streamer",
        "config": {
        "username": "streamer_user", 
        "channel": "streamer_channel",
        "client_id": "streamer_client",
        "client_secret": "streamer_secret"
        }
    })).await;
    assert!(named_result.is_ok(), "Failed to spawn named instance");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test isolation: verify each instance has its own configuration
    let default_status = bus_handle.call("twitch:.get_status", json!({})).await.unwrap();
    let named_status = bus_handle.call("twitch:streamer.get_status", json!({})).await.unwrap();

    // Verify default instance
    assert_eq!(
        default_status.get("id").and_then(|v| v.as_str()),
        Some("")
    );
    assert_eq!(
        default_status.get("username").and_then(|v| v.as_str()),
        Some("default_user")
    );
    assert_eq!(
        default_status.get("channel").and_then(|v| v.as_str()),
        Some("default_channel")
    );

    // Verify named instance
    assert_eq!(
        named_status.get("id").and_then(|v| v.as_str()),
        Some("streamer")
    );
    assert_eq!(
        named_status.get("username").and_then(|v| v.as_str()),
        Some("streamer_user")
    );
    assert_eq!(
        named_status.get("channel").and_then(|v| v.as_str()),
        Some("streamer_channel")
    );

    info!("✅ IRC instance isolation verified correctly");

    // Test independent connection attempts
    let default_connect = bus_handle.call("twitch:.connect", json!({})).await;
    let named_connect = bus_handle.call("twitch:streamer.connect", json!({})).await;

    // Both should handle connection attempts independently
    assert!(default_connect.is_ok(), "Default instance connect failed");
    assert!(named_connect.is_ok(), "Named instance connect failed");

    // Test independent OAuth flows
    let default_oauth = bus_handle.call("twitch:.oauth_start", json!({})).await;
    let named_oauth = bus_handle.call("twitch:streamer.oauth_start", json!({})).await;

    // Both should handle OAuth independently
    assert!(default_oauth.is_ok(), "Default instance OAuth failed");
    assert!(named_oauth.is_ok(), "Named instance OAuth failed");

    info!("✅ Independent IRC operations verified correctly");
}

#[tokio::test]
async fn test_twitch_default_instance_addressing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init()
        .ok();

    info!("🧪 Testing Twitch default instance addressing patterns");

    let bus = Bus::new().await;
    let bus_handle = bus.handle();
    let mut system = SystemComponent::new(bus_handle);
    let bus_handle = system.get_bus_handle();

    // Spawn Twitch default instance
    let spawn_result = bus_handle.call("system:.spawn", json!({
        "class": "twitch",
        "id": "",
        "config": {
        "username": "address_test_user",
        "channel": "address_test_channel",
        "client_id": "test_client",
        "client_secret": "test_secret"
        }
    })).await;
    assert!(spawn_result.is_ok(), "Failed to spawn default instance");

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test all addressing patterns for default instance
    let addressing_patterns = vec![
        "twitch:",      // Standard default instance addressing
    ];

    for pattern in addressing_patterns {
        info!("Testing addressing pattern: '{}'", pattern);

        let test_result = timeout(
            Duration::from_secs(3),
            bus_handle.call(&format!("{}.get_status", pattern), json!({}))
        ).await;

        assert!(test_result.is_ok(), "Timeout for pattern: {}", pattern);
        let response = test_result.unwrap();
        assert!(response.is_ok(), "Failed response for pattern: {}", pattern);
        
        let data = response.unwrap();
        assert_eq!(
            data.get("id").and_then(|v| v.as_str()),
            Some(""),
            "Wrong ID for pattern: {}",
            pattern
        );
        assert_eq!(
            data.get("username").and_then(|v| v.as_str()),
            Some("address_test_user"),
            "Wrong username for pattern: {}",
            pattern
        );
    }

    info!("✅ All addressing patterns work correctly for default instance");
}
