/// 🧪 Test ComponentResponse macro functionality
/// 
/// This integration test validates that the #[component_response] macro
/// generates the expected struct, methods, and JSON serialization.

use controlrake_macros::component_response;

/// 📤 Test response struct with custom fields
#[component_response]
struct TestResponse {
    click_count: u32,
    text: String,
}

/// 📤 Test response with no custom fields
#[component_response]
struct SimpleTestResponse {
    // No custom fields - only common ComponentResponse fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_struct_compilation() {
        // Test that the macro generates a working struct
        let response = TestResponse::success("click", "test-component", 42);
        
        // Verify the struct compiles and has the expected methods
        let _json = response.to_json();
    }

    #[test]
    fn test_success_response() {
        let response = TestResponse::success("test_action", "comp-123", 100)
            .with_click_count(5)
            .with_text("Hello World".to_string())
            .with_message("Test completed");
        
        let json = response.to_json();
        
        // Verify common fields
        assert_eq!(json["success"], true);
        assert_eq!(json["action"], "test_action");
        assert_eq!(json["component_id"], "comp-123");
        assert_eq!(json["version"], 100);
        assert_eq!(json["message"], "Test completed");
        
        // Verify custom fields
        assert_eq!(json["click_count"], 5);
        assert_eq!(json["text"], "Hello World");
    }

    #[test]
    fn test_error_response() {
        let response = TestResponse::error("Something went wrong");
        let json = response.to_json();
        
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Something went wrong");
        assert_eq!(json["action"], "");
        assert_eq!(json["component_id"], "");
    }

    #[test]
    fn test_simple_response() {
        let response = SimpleTestResponse::success("simple", "simple-comp", 1)
            .with_message("Simple test");
        
        let json = response.to_json();
        
        assert_eq!(json["success"], true);
        assert_eq!(json["action"], "simple");
        assert_eq!(json["component_id"], "simple-comp");
        assert_eq!(json["version"], 1);
        assert_eq!(json["message"], "Simple test");
    }

    #[test]
    fn test_optional_fields_skipped_when_none() {
        let response = TestResponse::success("test", "comp", 1);
        let json = response.to_json();
        
        // Optional fields should not be present when None
        assert!(json.get("click_count").is_none() || json["click_count"].is_null());
        assert!(json.get("text").is_none() || json["text"].is_null());
        assert!(json.get("message").is_none() || json["message"].is_null());
        assert!(json.get("timestamp").is_none() || json["timestamp"].is_null());
    }

    #[test]
    fn test_builder_pattern() {
        // Test that builder methods return Self for chaining
        let response = TestResponse::success("chain_test", "comp", 1)
            .with_click_count(42)
            .with_text("Chained".to_string())
            .with_message("Builder pattern works");
        
        let json = response.to_json();
        
        assert_eq!(json["click_count"], 42);
        assert_eq!(json["text"], "Chained");
        assert_eq!(json["message"], "Builder pattern works");
    }
}
