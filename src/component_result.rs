/// 🎯 ComponentResult<T> - Serializable result type for macro system
///
/// **Purpose**: Provide a Result type that can be serialized to JSON
/// for communication between components via the message bus.
///
/// **Design**:
/// - Wraps std::Result for ergonomic error handling
/// - Serializes both Ok and Err variants to JSON 
/// - Preserves error information across component boundaries
/// - Used by macro system for method return values
use serde::{Deserialize, Serialize};

/// 📦 Component result wrapper with serializable error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentResult<T> {
    /// Success/failure status
    pub success: bool,
    /// Data payload (present on success)
    pub data: Option<T>,
    /// Error message (present on failure)
    pub error: Option<String>,
    /// Error type classification
    pub error_type: Option<String>,
}

impl<T> ComponentResult<T>
where
    T: Serialize,
{
    /// ✅ Create successful result
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            error_type: None,
        }
    }

    /// ❌ Create error result
    pub fn error<E: std::fmt::Display>(error: E) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            error_type: Some("ComponentError".to_string()),
        }
    }

    /// ❌ Create error result with type classification
    pub fn error_with_type<E: std::fmt::Display>(error: E, error_type: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            error_type: Some(error_type.to_string()),
        }
    }

    /// 🔄 Convert to standard Result
    pub fn into_result(self) -> anyhow::Result<T> {
        if self.success {
            if let Some(data) = self.data {
                Ok(data)
            } else {
                Err(anyhow::anyhow!("ComponentResult marked success but contains no data"))
            }
        } else {
            let error_msg = self.error.unwrap_or_else(|| "Unknown component error".to_string());
            Err(anyhow::anyhow!("{}", error_msg))
        }
    }

    /// ✅ Check if result is successful
    pub fn is_ok(&self) -> bool {
        self.success
    }

    /// ❌ Check if result is an error
    pub fn is_err(&self) -> bool {
        !self.success
    }
}

/// 🔄 Convert from std::Result to ComponentResult
impl<T, E> From<Result<T, E>> for ComponentResult<T>
where
    T: Serialize,
    E: std::fmt::Display,
{
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => ComponentResult::ok(data),
            Err(error) => ComponentResult::error(error),
        }
    }
}

/// 🔄 Convert ComponentResult<T> to serde_json::Value
impl<T> From<ComponentResult<T>> for serde_json::Value
where
    T: Serialize,
{
    fn from(result: ComponentResult<T>) -> Self {
        serde_json::to_value(result).unwrap_or_else(|_| {
            serde_json::json!({
                "success": false,
                "data": null,
                "error": "Failed to serialize ComponentResult",
                "error_type": "SerializationError"
            })
        })
    }
}

/// 🔄 Convert serde_json::Value to ComponentResult<T>
impl<T> TryFrom<serde_json::Value> for ComponentResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

/// 🎯 Type alias for common usage pattern
pub type ComponentJsonResult = ComponentResult<serde_json::Value>;

/// 🎯 Type alias for void operations  
pub type ComponentVoidResult = ComponentResult<()>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_result_success() {
        let result = ComponentResult::ok("test data");
        
        assert!(result.is_ok());
        assert!(!result.is_err());
        assert_eq!(result.data, Some("test data"));
        assert_eq!(result.error, None);
    }

    #[test]
    fn test_component_result_error() {
        let result: ComponentResult<String> = ComponentResult::error("test error");
        
        assert!(!result.is_ok());
        assert!(result.is_err());
        assert_eq!(result.data, None);
        assert_eq!(result.error, Some("test error".to_string()));
    }

    #[test]
    fn test_from_std_result() {
        let ok_result: Result<i32, &str> = Ok(42);
        let component_result: ComponentResult<i32> = ok_result.into();
        
        assert!(component_result.is_ok());
        assert_eq!(component_result.data, Some(42));

        let err_result: Result<i32, &str> = Err("failure");
        let component_result: ComponentResult<i32> = err_result.into();
        
        assert!(component_result.is_err());
        assert_eq!(component_result.error, Some("failure".to_string()));
    }

    #[test]
    fn test_from_anyhow_result() {
        let ok_result: anyhow::Result<i32> = Ok(42);
        let component_result: ComponentResult<i32> = ok_result.into();
        
        assert!(component_result.is_ok());
        assert_eq!(component_result.data, Some(42));

        let err_result: anyhow::Result<i32> = Err(anyhow::anyhow!("anyhow failure"));
        let component_result: ComponentResult<i32> = err_result.into();
        
        assert!(component_result.is_err());
        assert!(component_result.error.unwrap().contains("anyhow failure"));
    }

    #[test]
    fn test_serialization_to_json() {
        let result = ComponentResult::ok(42);
        let json_value: serde_json::Value = result.into();
        
        assert_eq!(json_value["success"], true);
        assert_eq!(json_value["data"], 42);
        assert_eq!(json_value["error"], serde_json::Value::Null);
    }

    #[test]
    fn test_deserialization_from_json() {
        let json = serde_json::json!({
            "success": true,
            "data": "test data",
            "error": null,
            "error_type": null
        });
        
        let result: ComponentResult<String> = json.try_into().unwrap();
        
        assert!(result.is_ok());
        assert_eq!(result.data, Some("test data".to_string()));
    }

    #[test]
    fn test_into_result_conversion() {
        let ok_result = ComponentResult::ok(42);
        let converted: anyhow::Result<i32> = ok_result.into_result();
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), 42);

        let err_result: ComponentResult<i32> = ComponentResult::error("test error");
        let converted: anyhow::Result<i32> = err_result.into_result();
        assert!(converted.is_err());
    }
}