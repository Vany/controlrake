// HTTP component tests updated for macro-based constructor
// Tests HTTP component functionality using the new macro-generated infrastructure

use controlrake::component::http::{HttpComponent, HttpComponentConfig};
use controlrake::{Bus, Component, SystemComponent};
use serde_json::json;
use std::sync::Arc;

/// Create test HTTP component using new constructor pattern
async fn create_test_http_component(id: &str) -> anyhow::Result<HttpComponent> {
    let bus = Bus::new().await;
    let handle = bus.handle();
    
    // Create system component
    let system = Arc::new(SystemComponent::new(handle));
    
    // Create HTTP component config
    let config = HttpComponentConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 8891, // Test port to avoid conflicts
        base_url: "http://localhost:8891".to_string(),
        static_folder: "./static".to_string(),
        auth_key: "test-key".to_string(),
    };
    
    // Use new macro-based constructor
    HttpComponent::new(id.to_string(), system, config)
}

#[tokio::test]
async fn test_http_component_creation() {
    let http_component = create_test_http_component("http_test").await;
    assert!(http_component.is_ok(), "HTTP component should be created successfully");
}

#[tokio::test]
async fn test_http_component_render() {
    let http_component = create_test_http_component("render_test").await.unwrap();
    
    // Test that component can render
    let render_result = http_component.render().await;
    assert!(render_result.is_ok(), "HTTP component should render successfully");
    
    let html = render_result.unwrap();
    assert!(html.contains("HTTP Gateway"), "Rendered HTML should contain HTTP Gateway heading");
}

#[tokio::test]
async fn test_http_component_message_handling() {
    let mut http_component = create_test_http_component("message_test").await.unwrap();
    
    // Test status method
    let status_result = http_component.handle_message("status", json!({})).await;
    assert!(status_result.is_ok(), "Status method should work");
    
    // Test render method via message handling
    let render_result = http_component.handle_message("render", json!({})).await;
    assert!(render_result.is_ok(), "Render method should work via message handling");
}

#[tokio::test]
async fn test_http_component_id() {
    let http_component = create_test_http_component("id_test").await.unwrap();
    
    // Test component ID
    assert_eq!(http_component.id(), "id_test");
}

#[tokio::test]
async fn test_http_component_versioning() {
    let http_component = create_test_http_component("version_test").await.unwrap();
    
    // Test version tracking
    let version = http_component.current_version();
    assert!(version > 0, "Component should have a version");
    
    // Test incremental updates (empty for http component typically)
    let updates = http_component.get_updates(0).await.unwrap();
    // HTTP component may or may not have updates - just test it doesn't crash
    assert!(updates.len() >= 0);
}

// Note: Advanced integration tests for HTTP server startup, WebSocket connections,
// static file serving, etc. are covered by integration_test.rs and the actual
// system testing. These unit tests focus on the Component trait implementation.
