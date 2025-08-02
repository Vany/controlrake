use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use thiserror::Error;

/// 📍 Address for component messaging
///
/// Formats supported:
/// - `"class:id"` → class="class", id=Some("id"), method=None
/// - `"class:id.method"` → class="class", id=Some("id"), method=Some("method")  
/// - `"class:"` → class="class", id=None, method=None (default instance)
/// - `"class:.method"` → class="class", id=None, method=Some("method") (default instance method)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    pub class: String,
    pub id: Option<String>,
    pub method: Option<String>,
}

/// ❌ Address parsing and validation errors
#[derive(Debug, Error, PartialEq)]
pub enum AddressError {
    #[error("Address must contain ':' separator")]
    MissingColon,

    #[error("Class name cannot be empty")]
    EmptyClass,

    #[error("Class name must be alphanumeric with underscores only: {0}")]
    InvalidClass(String),

    #[error("Invalid address format: {0}")]
    InvalidFormat(String),
}

impl Address {
    /// 🔍 Parse address from string with validation
    ///
    /// # Errors
    ///
    /// Returns `AddressError` if the input is malformed:
    /// - `MissingColon`: No colon separator found  
    /// - `EmptyClass`: Class part is empty
    /// - `InvalidFormat`: Invalid format or structure
    pub fn parse(input: &str) -> Result<Self, AddressError> {
        // Must contain colon
        if !input.contains(':') {
            return Err(AddressError::MissingColon);
        }

        // Split on first colon only
        let (class_part, rest) = input
            .split_once(':')
            .ok_or_else(|| AddressError::InvalidFormat(input.to_string()))?;

        // Validate class name
        if class_part.is_empty() {
            return Err(AddressError::EmptyClass);
        }

        if !class_part.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AddressError::InvalidClass(class_part.to_string()));
        }

        // Parse id and method part
        let (id, method) = if rest.is_empty() {
            // "class:" → id=None, method=None
            (None, None)
        } else if let Some(method_name) = rest.strip_prefix('.') {
            // "class:.method" → id=None, method=Some("method")
            if method_name.is_empty() {
                return Err(AddressError::InvalidFormat(input.to_string()));
            }
            (None, Some(method_name.to_string()))
        } else if let Some((id_part, method_part)) = rest.split_once('.') {
            // "class:id.method" → id=Some("id"), method=Some("method")
            if id_part.is_empty() || method_part.is_empty() {
                return Err(AddressError::InvalidFormat(input.to_string()));
            }
            (Some(id_part.to_string()), Some(method_part.to_string()))
        } else {
            // "class:id" → id=Some("id"), method=None
            (Some(rest.to_string()), None)
        };

        Ok(Address {
            class: class_part.to_string(),
            id,
            method,
        })
    }

    /// 🎯 Check if this is a default instance address (empty instance ID)
    pub fn is_default_instance(&self) -> bool {
        self.id.is_none()
    }

    /// 🏷️ Get address key for instance registration, handling default instances
    /// - Default instances (empty ID): returns "class:"
    /// - Named instances: returns "class:id"
    #[must_use]
    pub fn instance_key_with_default(&self) -> String {
        match &self.id {
            Some(id) => format!("{}:{}", self.class, id),
            None => format!("{}:", self.class),  // Default instance key
        }
    }

    /// 🎯 Check if this is a default instance method call
    #[must_use]
    pub fn is_default_instance_method(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    /// 🏷️ Get address key for instance registration (class:id)
    #[must_use]
    pub fn instance_key(&self) -> Option<String> {
        self.id.as_ref().map(|id| format!("{}:{}", self.class, id))
    }

    /// 🏷️ Get class key for class registration
    #[must_use]
    pub fn class_key(&self) -> String {
        self.class.clone()
    }

    /// 🎯 Check if this is a class-level method call
    #[must_use]
    pub fn is_class_method(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    /// 🎯 Check if this is an instance method call
    #[must_use]
    pub fn is_instance_method(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    /// 🎯 Check if this is a plain instance address
    #[must_use]
    pub fn is_instance_address(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }
}
impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.class)?;
        write!(f, ":")?;

        if let Some(ref id) = self.id {
            write!(f, "{id}")?;
        }

        if let Some(ref method) = self.method {
            write!(f, ".{method}")?;
        }

        Ok(())
    }
}