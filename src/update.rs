use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 🎯 Incremental DOM update operations for surgical client-side modifications
///
/// These messages enable granular DOM updates without full innerHTML replacement,
/// preserving client state (form focus, scroll position) and improving performance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum UpdateMessage {
    /// Set text content of an element
    ///
    /// Example: Change counter display without DOM destruction
    /// ```json
    /// {"operation": "set_text", "selector": "#counter-123", "value": "Clicks: 5"}
    /// ```
    #[serde(rename = "set_text")]
    SetText { selector: String, value: String },

    /// Set attribute value on an element
    ///
    /// Example: Update disabled state, href, or data attributes
    /// ```json
    /// {"operation": "set_attribute", "selector": "#btn-123", "attribute": "disabled", "value": "true"}
    /// ```
    #[serde(rename = "set_attribute")]
    SetAttribute {
        selector: String,
        attribute: String,
        value: String,
    },

    /// Set CSS style property on an element
    ///
    /// Example: Update progress bar width, colors, visibility
    /// ```json
    /// {"operation": "set_style", "selector": "#progress-123", "property": "width", "value": "75%"}
    /// ```
    #[serde(rename = "set_style")]
    SetStyle {
        selector: String,
        property: String,
        value: String,
    },

    /// Toggle CSS class on an element
    ///
    /// Example: Add/remove active, error, loading states
    /// ```json
    /// {"operation": "toggle_class", "selector": "#btn-123", "class": "active"}
    /// ```
    #[serde(rename = "toggle_class")]
    ToggleClass { selector: String, class: String },

    /// Add CSS class to an element
    ///
    /// Example: Add visual feedback classes, state indicators
    /// ```json
    /// {"operation": "add_class", "selector": "#btn-123", "class_name": "recently-clicked"}
    /// ```
    #[serde(rename = "add_class")]
    AddClass { selector: String, class_name: String },

    /// Remove CSS class from an element
    ///
    /// Example: Remove temporary states, clear indicators
    /// ```json
    /// {"operation": "remove_class", "selector": "#btn-123", "class_name": "loading"}
    /// ```
    #[serde(rename = "remove_class")]
    RemoveClass { selector: String, class_name: String },

    /// Insert new HTML element into DOM
    ///
    /// Example: Add new list item, notification, or dynamic content
    /// ```json
    /// {"operation": "insert_element", "parent": "#list-123", "html": "<li>New item</li>", "position": "append"}
    /// ```
    #[serde(rename = "insert_element")]
    InsertElement {
        parent: String,
        html: String,
        position: InsertPosition,
    },

    /// Remove element from DOM
    ///
    /// Example: Delete list items, hide notifications, cleanup
    /// ```json
    /// {"operation": "remove_element", "selector": "#item-123"}
    /// ```
    #[serde(rename = "remove_element")]
    RemoveElement { selector: String },

    /// Set JavaScript property on an element
    ///
    /// Example: Update form values, scroll position, or other DOM properties
    /// ```json
    /// {"operation": "set_property", "selector": "#input-123", "property": "value", "value": "new text"}
    /// ```
    #[serde(rename = "set_property")]
    SetProperty {
        selector: String,
        property: String,
        value: serde_json::Value,
    },
}

/// 📍 DOM insertion position for `InsertElement` operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InsertPosition {
    /// Insert as first child
    Prepend,
    /// Insert as last child (default)
    #[default]
    Append,
    /// Insert before the parent element
    Before,
    /// Insert after the parent element
    After,
}

/// 🔒 Security validation for DOM operations
impl UpdateMessage {
    /// Validate update message for security and correctness
    ///
    /// Prevents XSS attacks and ensures safe DOM operations:
    /// - Validates CSS selectors are well-formed
    /// - Sanitizes HTML content in insert operations
    /// - Validates CSS property names
    /// - Prevents script injection
    pub fn validate(&self) -> Result<()> {
        self.validate_with_config(None)
    }

    /// Validate update message with optional configuration for limits
    pub fn validate_with_config(&self, config: Option<&crate::config::UpdateConfig>) -> Result<()> {
        let max_text_length = config.map_or(1000, |c| c.max_text_length);

        match self {
            UpdateMessage::SetText { selector, value } => {
                Self::validate_selector(selector)?;
                Self::validate_text_content(value, max_text_length)?;
            }
            UpdateMessage::SetAttribute {
                selector,
                attribute,
                value,
            } => {
                Self::validate_selector(selector)?;
                Self::validate_attribute_name(attribute)?;
                Self::validate_attribute_value(value, max_text_length)?;
            }
            UpdateMessage::SetStyle {
                selector,
                property,
                value,
            } => {
                Self::validate_selector(selector)?;
                Self::validate_css_property(property)?;
                Self::validate_css_value(value)?;
            }
            UpdateMessage::ToggleClass { selector, class } => {
                Self::validate_selector(selector)?;
                Self::validate_css_class(class)?;
            }
            UpdateMessage::AddClass { selector, class_name } => {
                Self::validate_selector(selector)?;
                Self::validate_css_class(class_name)?;
            }
            UpdateMessage::RemoveClass { selector, class_name } => {
                Self::validate_selector(selector)?;
                Self::validate_css_class(class_name)?;
            }
            UpdateMessage::InsertElement {
                parent,
                html,
                position: _,
            } => {
                Self::validate_selector(parent)?;
                Self::validate_html_content(html, max_text_length)?;
            }
            UpdateMessage::RemoveElement { selector } => {
                Self::validate_selector(selector)?;
            }
            UpdateMessage::SetProperty {
                selector,
                property,
                value,
            } => {
                Self::validate_selector(selector)?;
                Self::validate_property_name(property)?;
                Self::validate_property_value(value)?;
            }
        }
        Ok(())
    }

    /// Validate CSS selector format and safety
    pub fn validate_selector(selector: &str) -> Result<()> {
        if selector.is_empty() {
            bail!("CSS selector cannot be empty");
        }

        // Must start with valid CSS selector characters
        if !selector.starts_with(['#', '.', '['])
            && !selector.chars().next().unwrap().is_ascii_alphabetic()
        {
            bail!("Invalid CSS selector format: {}", selector);
        }

        // Prevent dangerous selectors
        let dangerous_patterns = ["<script", "javascript:", "data:", "vbscript:"];
        for pattern in dangerous_patterns {
            if selector.to_lowercase().contains(pattern) {
                bail!("Potentially dangerous selector: {}", selector);
            }
        }

        // Length limit to prevent DoS
        if selector.len() > 200 {
            bail!("CSS selector too long (max 200 chars): {}", selector.len());
        }

        Ok(())
    }

    /// Validate JavaScript property name
    pub fn validate_property_name(property: &str) -> Result<()> {
        if property.is_empty() {
            bail!("Property name cannot be empty");
        }

        // Dangerous properties that could enable script execution
        let dangerous_props = [
            "innerhtml",
            "outerhtml", 
            "insertadjacenthtml",
            "onload",
            "onerror",
            "onclick",
            "onfocus",
            "onblur",
            "onchange",
            "src", // for script/iframe elements
        ];

        if dangerous_props.contains(&property.to_lowercase().as_str()) {
            bail!("Dangerous property not allowed: {}", property);
        }

        // Must be valid JavaScript property name format
        if !property.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            bail!("Invalid property name format: {}", property);
        }

        if property.len() > 50 {
            bail!("Property name too long (max 50 chars): {}", property.len());
        }

        Ok(())
    }

    /// Validate JavaScript property value
    pub fn validate_property_value(value: &serde_json::Value) -> Result<()> {
        match value {
            serde_json::Value::String(s) => {
                // Check for script injection in string values
                let dangerous_patterns = ["javascript:", "data:", "vbscript:", "<script"];
                for pattern in dangerous_patterns {
                    if s.to_lowercase().contains(pattern) {
                        bail!("Potentially dangerous property value: {}", s);
                    }
                }
                
                if s.len() > 1000 {
                    bail!("Property string value too long (max 1000 chars): {}", s.len());
                }
            }
            serde_json::Value::Number(_) | serde_json::Value::Bool(_) | serde_json::Value::Null => {
                // These are generally safe
            }
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                // Complex values could potentially be dangerous, but we allow them with size limits
                let serialized = serde_json::to_string(value).unwrap_or_default();
                if serialized.len() > 2000 {
                    bail!("Property complex value too large (max 2000 chars when serialized): {}", serialized.len());
                }
            }
        }

        Ok(())
    }

    /// Validate text content is safe (no script injection)
    pub fn validate_text_content(content: &str, max_length: usize) -> Result<()> {
        // Text content is generally safe since it's inserted as textContent, not innerHTML
        // But we still check for excessively long content
        if content.len() > max_length {
            bail!(
                "Text content too long (max {} chars): {}",
                max_length,
                content.len()
            );
        }
        Ok(())
    }

    /// Validate HTML attribute name
    pub fn validate_attribute_name(attr: &str) -> Result<()> {
        if attr.is_empty() {
            bail!("Attribute name cannot be empty");
        }

        // Dangerous attributes that could enable XSS
        let dangerous_attrs = [
            "onclick",
            "onload",
            "onerror",
            "onmouseover",
            "onmouseout",
            "onfocus",
            "onblur",
            "onchange",
            "onsubmit",
            "onkeydown",
            "onkeyup",
            "onkeypress",
            "style", // style should use SetStyle instead
        ];

        if dangerous_attrs.contains(&attr.to_lowercase().as_str()) {
            bail!("Dangerous attribute not allowed: {}", attr);
        }

        // Must be valid HTML attribute name format
        if !attr
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!("Invalid attribute name format: {}", attr);
        }

        if attr.len() > 50 {
            bail!("Attribute name too long (max 50 chars): {}", attr.len());
        }

        Ok(())
    }

    /// Validate HTML attribute value
    pub fn validate_attribute_value(value: &str, max_length: usize) -> Result<()> {
        // Check for script injection in attribute values
        let dangerous_patterns = ["javascript:", "data:", "vbscript:", "<script"];
        for pattern in dangerous_patterns {
            if value.to_lowercase().contains(pattern) {
                bail!("Potentially dangerous attribute value: {}", value);
            }
        }

        if value.len() > max_length {
            bail!(
                "Attribute value too long (max {} chars): {}",
                max_length,
                value.len()
            );
        }

        Ok(())
    }

    /// Validate CSS property name
    pub fn validate_css_property(property: &str) -> Result<()> {
        if property.is_empty() {
            bail!("CSS property cannot be empty");
        }

        // Must be valid CSS property format (kebab-case or camelCase)
        if !property
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            bail!("Invalid CSS property format: {}", property);
        }

        if property.len() > 50 {
            bail!(
                "CSS property name too long (max 50 chars): {}",
                property.len()
            );
        }

        Ok(())
    }

    /// Validate CSS value
    pub fn validate_css_value(value: &str) -> Result<()> {
        // Check for dangerous CSS values that could enable attacks
        let dangerous_patterns = ["javascript:", "expression(", "url(javascript", "@import"];
        for pattern in dangerous_patterns {
            if value.to_lowercase().contains(pattern) {
                bail!("Potentially dangerous CSS value: {}", value);
            }
        }

        if value.len() > 500 {
            bail!("CSS value too long (max 500 chars): {}", value.len());
        }

        Ok(())
    }

    /// Validate CSS class name
    pub fn validate_css_class(class: &str) -> Result<()> {
        if class.is_empty() {
            bail!("CSS class cannot be empty");
        }

        // Must be valid CSS class format
        if !class
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            bail!("Invalid CSS class format: {}", class);
        }

        if class.len() > 50 {
            bail!("CSS class name too long (max 50 chars): {}", class.len());
        }

        Ok(())
    }

    /// Validate HTML content for insert operations
    pub fn validate_html_content(html: &str, max_length: usize) -> Result<()> {
        // Basic HTML validation - prevent script injection
        let dangerous_patterns = [
            "<script",
            "</script>",
            "javascript:",
            "data:",
            "vbscript:",
            "onload=",
            "onerror=",
            "onclick=",
            "onmouseover=",
        ];

        let html_lower = html.to_lowercase();
        for pattern in dangerous_patterns {
            if html_lower.contains(pattern) {
                bail!("Potentially dangerous HTML content: contains {}", pattern);
            }
        }

        // Length limit
        if html.len() > max_length {
            bail!(
                "HTML content too long (max {} chars): {}",
                max_length,
                html.len()
            );
        }

        // Basic tag balance check for common tags
        let tags_to_check = [
            "div", "span", "p", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li",
        ];
        for tag in tags_to_check {
            // Look for opening tags (with > or space after tag name)
            let open_simple = format!("<{tag}>");
            let open_with_attrs = format!("<{tag} ");
            let close_tag = format!("</{tag}>");

            let open_count = html_lower.matches(&open_simple).count()
                + html_lower.matches(&open_with_attrs).count();
            let close_count = html_lower.matches(&close_tag).count();

            // All opening tags must have matching closing tags for these tag types
            if open_count != close_count {
                bail!(
                    "Unbalanced HTML tags for '{}': {} open, {} close",
                    tag,
                    open_count,
                    close_count
                );
            }
        }

        Ok(())
    }

    /// Get the primary selector from an update message
    pub fn get_selector(&self) -> &str {
        match self {
            UpdateMessage::SetText { selector, .. }
            | UpdateMessage::SetAttribute { selector, .. }
            | UpdateMessage::SetStyle { selector, .. }
            | UpdateMessage::ToggleClass { selector, .. }
            | UpdateMessage::AddClass { selector, .. }
            | UpdateMessage::RemoveClass { selector, .. }
            | UpdateMessage::RemoveElement { selector }
            | UpdateMessage::SetProperty { selector, .. } => selector,
            UpdateMessage::InsertElement { parent, .. } => parent,
        }
    }
}

/// 📦 Batch update operations for performance optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBatch {
    /// List of update operations to apply atomically
    pub updates: Vec<UpdateMessage>,
    /// Optional target component address for routing
    pub target: Option<String>,
    /// Batch execution mode
    pub mode: BatchMode,
}

/// 🔄 Batch execution modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BatchMode {
    /// Apply all updates immediately (default)
    #[default]
    Immediate,
    /// Apply updates with small delay for batching optimization
    Delayed,
    /// Apply updates only when explicitly flushed
    Manual,
}

impl UpdateBatch {
    /// Create new update batch
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            target: None,
            mode: BatchMode::default(),
        }
    }

    /// Add update to batch
    pub fn add_update(mut self, update: UpdateMessage) -> Self {
        self.updates.push(update);
        self
    }

    /// Set target component for batch
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    /// Set batch execution mode
    pub fn with_mode(mut self, mode: BatchMode) -> Self {
        self.mode = mode;
        self
    }

    /// Validate entire batch
    pub fn validate(&self) -> Result<()> {
        self.validate_with_config(None)
    }

    /// Validate entire batch with optional configuration for limits
    pub fn validate_with_config(&self, config: Option<&crate::config::UpdateConfig>) -> Result<()> {
        if self.updates.is_empty() {
            bail!("Update batch cannot be empty");
        }

        if self.updates.len() > 50 {
            bail!(
                "Update batch too large (max 50 operations): {}",
                self.updates.len()
            );
        }

        // Validate each update in the batch
        for (index, update) in self.updates.iter().enumerate() {
            update.validate_with_config(config).map_err(|e| {
                anyhow::anyhow!("Validation failed for update {} in batch: {}", index, e)
            })?;
        }

        // Check for duplicate selectors in certain operations to prevent conflicts
        let mut selectors_used = HashSet::new();
        for update in &self.updates {
            let selector = update.get_selector();
            if selectors_used.contains(selector) {
                // Multiple operations on same selector might be intentional
                // (e.g., set text AND toggle class), so just warn in logs
                tracing::debug!("Multiple operations on selector '{}' in same batch", selector);
            }
            selectors_used.insert(selector.to_string());
        }

        Ok(())
    }
}

impl Default for UpdateBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_update_message_serialization() {
        let update = UpdateMessage::SetText {
            selector: "#counter-123".to_string(),
            value: "Clicks: 5".to_string(),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("\"operation\":\"set_text\""));
        assert!(json.contains("\"selector\":\"#counter-123\""));
        assert!(json.contains("\"value\":\"Clicks: 5\""));

        let deserialized: UpdateMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(update, deserialized);
    }

    #[test]
    fn test_all_update_message_variants() {
        let updates = vec![
            UpdateMessage::SetText {
                selector: "#test".to_string(),
                value: "test value".to_string(),
            },
            UpdateMessage::SetAttribute {
                selector: "#test".to_string(),
                attribute: "disabled".to_string(),
                value: "true".to_string(),
            },
            UpdateMessage::SetStyle {
                selector: "#test".to_string(),
                property: "color".to_string(),
                value: "red".to_string(),
            },
            UpdateMessage::ToggleClass {
                selector: "#test".to_string(),
                class: "active".to_string(),
            },
            UpdateMessage::AddClass {
                selector: "#test".to_string(),
                class_name: "recently-clicked".to_string(),
            },
            UpdateMessage::RemoveClass {
                selector: "#test".to_string(),
                class_name: "loading".to_string(),
            },
            UpdateMessage::InsertElement {
                parent: "#list".to_string(),
                html: "<li>New item</li>".to_string(),
                position: InsertPosition::Append,
            },
            UpdateMessage::RemoveElement {
                selector: "#item-123".to_string(),
            },
            UpdateMessage::SetProperty {
                selector: "#input-123".to_string(),
                property: "value".to_string(),
                value: serde_json::json!("test value"),
            },
        ];

        for update in updates {
            // Test serialization roundtrip
            let json = serde_json::to_string(&update).unwrap();
            let deserialized: UpdateMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(update, deserialized);

            // Test validation
            assert!(update.validate().is_ok());
        }
    }

    #[test]
    fn test_insert_position_serialization() {
        let positions = vec![
            InsertPosition::Prepend,
            InsertPosition::Append,
            InsertPosition::Before,
            InsertPosition::After,
        ];

        for pos in positions {
            let json = serde_json::to_string(&pos).unwrap();
            let deserialized: InsertPosition = serde_json::from_str(&json).unwrap();
            assert_eq!(pos, deserialized);
        }

        // Test default
        assert_eq!(InsertPosition::default(), InsertPosition::Append);
    }

    #[test]
    fn test_selector_validation() {
        // Valid selectors
        let valid_selectors = [
            "#test-id",
            ".test-class",
            "div",
            "div.class",
            "#id.class",
            "[data-test]",
            "#complex-id_123",
        ];

        for selector in valid_selectors {
            assert!(
                UpdateMessage::validate_selector(selector).is_ok(),
                "Selector should be valid: {selector}"
            );
        }

        // Invalid selectors
        let long_selector = "a".repeat(300);
        let invalid_selectors = [
            "",                              // empty
            "<script>alert('xss')</script>", // script injection
            "javascript:alert('xss')",       // javascript protocol
            long_selector.as_str(),          // too long
        ];

        for selector in invalid_selectors {
            assert!(
                UpdateMessage::validate_selector(selector).is_err(),
                "Selector should be invalid: {selector}"
            );
        }
    }

    #[test]
    fn test_dangerous_attribute_validation() {
        // Safe attributes
        let safe_attrs = [
            "id",
            "class",
            "data-test",
            "aria-label",
            "disabled",
            "checked",
        ];
        for attr in safe_attrs {
            assert!(
                UpdateMessage::validate_attribute_name(attr).is_ok(),
                "Attribute should be safe: {attr}"
            );
        }

        // Dangerous attributes
        let dangerous_attrs = ["onclick", "onload", "onerror", "style"];
        for attr in dangerous_attrs {
            assert!(
                UpdateMessage::validate_attribute_name(attr).is_err(),
                "Attribute should be blocked: {attr}"
            );
        }
    }

    #[test]
    fn test_html_content_validation() {
        // Safe HTML
        let safe_html = [
            "<div>Safe content</div>",
            "<p>Text content</p>",
            "<ul><li>Item 1</li><li>Item 2</li></ul>",
            "<img src='image.png' alt='safe image'>",
        ];

        for html in safe_html {
            assert!(
                UpdateMessage::validate_html_content(html, 1000).is_ok(),
                "HTML should be safe: {html}"
            );
        }

        // Dangerous HTML
        let dangerous_html = [
            "<script>alert('xss')</script>",
            "<div onclick='alert()'>Bad</div>",
            "<img src='x' onerror='alert()'>",
            "javascript:alert('xss')",
        ];

        for html in dangerous_html {
            assert!(
                UpdateMessage::validate_html_content(html, 1000).is_err(),
                "HTML should be blocked: {html}"
            );
        }
    }

    #[test]
    fn test_unbalanced_html_validation() {
        // Unbalanced tags should fail
        let unbalanced_html = [
            "<div>Content",      // missing closing tag
            "<p>Text</div>",     // wrong closing tag
            "<ul><li>Item</ul>", // missing li close
        ];

        for html in unbalanced_html {
            assert!(
                UpdateMessage::validate_html_content(html, 1000).is_err(),
                "Unbalanced HTML should fail: {html}"
            );
        }

        // Self-closing and properly balanced should pass
        let balanced_html = [
            "<div>Content</div>",
            "<br>",                         // self-closing
            "<img src='test.png'>",         // self-closing
            "<p>Text</p><span>More</span>", // multiple balanced
        ];

        for html in balanced_html {
            assert!(
                UpdateMessage::validate_html_content(html, 1000).is_ok(),
                "Balanced HTML should pass: {html}"
            );
        }
    }

    #[test]
    fn test_update_batch() {
        let mut batch = UpdateBatch::new()
            .with_target("button:test".to_string())
            .with_mode(BatchMode::Delayed);

        batch = batch.add_update(UpdateMessage::SetText {
            selector: "#counter".to_string(),
            value: "5".to_string(),
        });

        batch = batch.add_update(UpdateMessage::ToggleClass {
            selector: "#button".to_string(),
            class: "active".to_string(),
        });

        assert_eq!(batch.updates.len(), 2);
        assert_eq!(batch.target, Some("button:test".to_string()));
        assert_eq!(batch.mode, BatchMode::Delayed);
        assert!(batch.validate().is_ok());

        // Test serialization
        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: UpdateBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(batch.updates.len(), deserialized.updates.len());
    }

    #[test]
    fn test_batch_validation_limits() {
        // Empty batch should fail
        let empty_batch = UpdateBatch::new();
        assert!(empty_batch.validate().is_err());

        // Too many updates should fail
        let mut large_batch = UpdateBatch::new();
        for i in 0..60 {
            // More than 50 limit
            large_batch = large_batch.add_update(UpdateMessage::SetText {
                selector: format!("#item-{i}"),
                value: "test".to_string(),
            });
        }
        assert!(large_batch.validate().is_err());

        // Just under limit should pass
        let mut good_batch = UpdateBatch::new();
        for i in 0..45 {
            // Under 50 limit
            good_batch = good_batch.add_update(UpdateMessage::SetText {
                selector: format!("#item-{i}"),
                value: "test".to_string(),
            });
        }
        assert!(good_batch.validate().is_ok());
    }

    #[test]
    fn test_css_validation() {
        // Valid CSS properties
        let valid_props = ["color", "background-color", "font-size", "margin-top"];
        for prop in valid_props {
            assert!(UpdateMessage::validate_css_property(prop).is_ok());
        }

        // Valid CSS values
        let valid_values = ["red", "#ff0000", "12px", "100%", "1em", "rgba(255,0,0,0.5)"];
        for value in valid_values {
            assert!(UpdateMessage::validate_css_value(value).is_ok());
        }

        // Dangerous CSS values
        let dangerous_values = [
            "javascript:alert()",
            "expression(alert())",
            "url(javascript:alert())",
        ];
        for value in dangerous_values {
            assert!(UpdateMessage::validate_css_value(value).is_err());
        }
    }

    #[test]
    fn test_property_validation() {
        // Test SetProperty validation
        let prop_update = UpdateMessage::SetProperty {
            selector: "#test".to_string(),
            property: "value".to_string(),
            value: serde_json::json!("safe value"),
        };
        assert!(prop_update.validate().is_ok());

        // Test dangerous property names
        let dangerous_prop = UpdateMessage::SetProperty {
            selector: "#test".to_string(),
            property: "innerHTML".to_string(),
            value: serde_json::json!("value"),
        };
        assert!(dangerous_prop.validate().is_err());

        // Test dangerous property values
        let dangerous_value = UpdateMessage::SetProperty {
            selector: "#test".to_string(),
            property: "value".to_string(),
            value: serde_json::json!("javascript:alert('xss')"),
        };
        assert!(dangerous_value.validate().is_err());
    }

    #[test]
    fn test_get_selector() {
        let updates = vec![
            (UpdateMessage::SetText {
                selector: "#test1".to_string(),
                value: "value".to_string(),
            }, "#test1"),
            (UpdateMessage::SetAttribute {
                selector: "#test2".to_string(),
                attribute: "attr".to_string(),
                value: "value".to_string(),
            }, "#test2"),
            (UpdateMessage::InsertElement {
                parent: "#parent".to_string(),
                html: "<div></div>".to_string(),
                position: InsertPosition::Append,
            }, "#parent"),
        ];

        for (update, expected_selector) in updates {
            assert_eq!(update.get_selector(), expected_selector);
        }
    }
}
