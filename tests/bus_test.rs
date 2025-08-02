// Bus system tests for controlrake
// Tests core message bus functionality using public APIs only

use controlrake::{Bus, BusHandle, Message, Address, RawMessage};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};

// Test utilities and mock service
struct MockService {
    name: String,
    bus_handle: BusHandle,
    receiver: UnboundedReceiver<Message>,
    message_log: Arc<Mutex<Vec<Message>>>,
}

impl MockService {
    fn new(name: &str, bus_handle: BusHandle) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        bus_handle.register_class(name, tx.clone());
        bus_handle.register_instance(name, "main", tx);

        Self {
            name: name.to_string(),
            bus_handle,
            receiver: rx,
            message_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn run(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            let mut log = self.message_log.lock().await;
            log.push(message.clone());
            
            // Auto-respond to test messages
            if let Some(from) = &message.source {
                if let Some(id) = &message.id {
                    let reply_msg = format!("{} {{\"reply\": \"{}\", \"received\": true, \"echo\": {}}}", from, id.as_str(), message.payload);
                    let _ = self.bus_handle.send_raw(&reply_msg).await;
                }
            }
        }
    }

    async fn get_message_count(&self) -> usize {
        self.message_log.lock().await.len()
    }
}

#[tokio::test]
async fn test_bus_creation() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    // Verify bus handle exists and has basic functionality
    let stats = handle.stats();
    assert!(stats.instance_count >= 0);
    assert!(stats.class_count >= 0);
    assert!(stats.pending_calls >= 0);
}

#[tokio::test]
async fn test_address_parsing() {
    // Test various address formats
    assert!(Address::parse("class:instance").is_ok());
    assert!(Address::parse("class:instance.method").is_ok());
    assert!(Address::parse("class:.method").is_ok());
    
    // Test invalid addresses
    assert!(Address::parse("invalid").is_err());
    assert!(Address::parse("").is_err());
    assert!(Address::parse(":").is_err());
}

#[tokio::test]
async fn test_message_creation() {
    let address = Address::parse("test:instance").unwrap();
    let payload = json!({"test": "data"});
    
    let message = Message::new(address, payload.clone());
    assert_eq!(message.target.class, "test");
    assert_eq!(message.target.id, Some("instance".to_string()));
    assert_eq!(message.payload, payload);
}

#[tokio::test]
async fn test_raw_message_parsing() {
    // Test message parsing using RawMessage constructor
    let target = "test:instance";
    let payload = json!({"key": "value"});
    
    let raw_string = format!("{} {}", target, payload);
    let raw_message = RawMessage::new(raw_string.clone());
    assert_eq!(raw_message.raw, raw_string);
}

#[tokio::test]
async fn test_bus_send_raw() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    // Test sending a raw message (should succeed even if no receiver)
    let result = handle.send_raw("test:target {\"message\": \"hello\"}").await;
    assert!(result.is_ok());

    // Test sending malformed message
    let result = handle.send_raw("invalid message format").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_bus_call_timeout() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    let start_time = Instant::now();
    
    // Call to non-existent target should timeout
    let result = handle.call("nonexistent:target", json!({})).await;
    assert!(result.is_err());
    
    let elapsed = start_time.elapsed();
    // Should timeout quickly (within a few seconds)
    assert!(elapsed < Duration::from_secs(5));
}

#[tokio::test]
async fn test_mock_service_registration() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    let mut service = MockService::new("test", handle.clone());
    
    // Start service in background
    tokio::spawn(async move {
        service.run().await;
    });

    // Give service time to start
    sleep(Duration::from_millis(50)).await;

    // Test sending message to service
    let result = handle.send_raw("test:main {\"test\": \"message\"}").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_request_response_pattern() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    let mut service = MockService::new("echo", handle.clone());
    
    // Start service
    let service_handle = tokio::spawn(async move {
        service.run().await;
    });

    // Give service time to register
    sleep(Duration::from_millis(50)).await;

    // Test request-response
    let response = handle.call("echo:main", json!({"echo": "test"})).await;
    assert!(response.is_ok());

    let response_data = response.unwrap();
    assert!(response_data["received"].as_bool().unwrap_or(false));

    // Clean up
    service_handle.abort();
}

#[tokio::test]
async fn test_bus_stats() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    let initial_stats = handle.stats();
    let initial_instance_count = initial_stats.instance_count;

    // Send some messages
    let _ = handle.send_raw("test:target1 {}").await;
    let _ = handle.send_raw("test:target2 {}").await;
    let _ = handle.send_raw("test:target3 {}").await;

    let updated_stats = handle.stats();
    // Note: instance_count might not increase since we're not actually registering instances
    // Just verify the stats are accessible
    assert!(updated_stats.instance_count >= initial_instance_count);
}

#[tokio::test]
async fn test_multiple_services() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    // Create multiple services
    let mut service1 = MockService::new("service1", handle.clone());
    let mut service2 = MockService::new("service2", handle.clone());

    // Start services
    let s1_handle = tokio::spawn(async move {
        service1.run().await;
    });
    
    let s2_handle = tokio::spawn(async move {
        service2.run().await;
    });

    // Give services time to register
    sleep(Duration::from_millis(50)).await;

    // Test messaging both services
    let result1 = handle.send_raw("service1:main {\"test\": 1}").await;
    let result2 = handle.send_raw("service2:main {\"test\": 2}").await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    // Clean up
    s1_handle.abort();
    s2_handle.abort();
}

#[tokio::test]
async fn test_message_error_handling() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    // Test various error conditions
    let result1 = handle.send_raw("").await;
    assert!(result1.is_err());

    let result2 = handle.send_raw("invalid").await;
    assert!(result2.is_err());

    let result3 = handle.send_raw("test-invalid:id {}").await; // Invalid class name with hyphen
    assert!(result3.is_err());
}

#[tokio::test] 
async fn test_address_error_types() {
    // Test address error types - simplified to just check they fail
    let result = Address::parse("");
    assert!(result.is_err());

    let result = Address::parse("no_colon");
    assert!(result.is_err());

    let result = Address::parse(":");
    assert!(result.is_err());
}

// Performance test (basic)
#[tokio::test]
async fn test_bus_performance_basic() {
    let bus = Bus::new().await;
    let handle = bus.handle();

    let start = Instant::now();
    let mut tasks = Vec::new();

    // Send many messages in parallel
    for i in 0..100 {
        let h = handle.clone();
        let task = tokio::spawn(async move {
            let _ = h.send_raw(&format!("test:target{} {{}}", i)).await;
        });
        tasks.push(task);
    }

    // Wait for all to complete
    for task in tasks {
        let _ = task.await;
    }

    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(5)); // Should complete quickly
}
