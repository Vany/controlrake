/// 🧪 Test file for ComponentResponse macro
///
/// This file demonstrates and tests the new #[component_response] macro.

use controlrake_macros::component_response;

/// 📤 Test ComponentResponse for Button component
#[component_response]
struct ButtonResponse {
    // Custom fields specific to button operations
    click_count: u32,
    text: String,
    amount: u32,
}

/// 📤 Test ComponentResponse with custom options
#[component_response(name = "CustomButtonResponse", include_timestamp = true)]
struct CustomResponse {
    // User fields
    user_id: u64,
    status: String,
}

/// 📤 Simple response with no custom fields
#[component_response]
struct SimpleResponse {
    // No custom fields - should just have the common ComponentResponse fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_response_success() {
        let response = ButtonResponse::success("click", "button-1", 42)
            .with_click_count(5)
            .with_text("Test Button".to_string())
            .with_amount(10)
            .with_message("Button clicked successfully");
        
        let json = response.to_json();
        
        assert_eq!(json["success"], true);
        assert_eq!(json["action"], "click");
        assert_eq!(json["component_id"], "button-1");
        assert_eq!(json["version"], 42);
        assert_eq!(json["click_count"], 5);
        assert_eq!(json["text"], "Test Button");
        assert_eq!(json["amount"], 10);
        assert_eq!(json["message"], "Button clicked successfully");
    }

    #[test]
    fn test_button_response_error() {
        let response = ButtonResponse::error("Something went wrong");
        let json = response.to_json();
        
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Something went wrong");
        assert_eq!(json["action"], "");
        assert_eq!(json["component_id"], "");
    }

    #[test]
    fn test_custom_response_with_timestamp() {
        let response = CustomButtonResponse::success("login", "user-123", 1)
            .with_user_id(42)
            .with_status("authenticated".to_string())
            .with_timestamp();
        
        let json = response.to_json();
        
        assert_eq!(json["success"], true);
        assert_eq!(json["action"], "login");
        assert_eq!(json["component_id"], "user-123");
        assert_eq!(json["version"], 1);
        assert_eq!(json["user_id"], 42);
        assert_eq!(json["status"], "authenticated");
        assert!(json["timestamp"].is_string());
    }

    #[test]
    fn test_simple_response() {
        let response = SimpleResponse::success("test", "simple-1", 100)
            .with_message("Simple test");
        
        let json = response.to_json();
        
        assert_eq!(json["success"], true);
        assert_eq!(json["action"], "test");
        assert_eq!(json["component_id"], "simple-1");
        assert_eq!(json["version"], 100);
        assert_eq!(json["message"], "Simple test");
    }
}
