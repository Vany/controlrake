# Button Component Requirements

## 🎯 Functional Requirements

### Core Functionality
- **Click Counting**: Atomic click counter with persistence
- **Text Customization**: Configurable button text via WebSocket messages
- **State Persistence**: Automatic state saving to database
- **Real-time Updates**: Incremental DOM updates via WebSocket

### Instance Requirements
- **Named Instances Only**: Button component does NOT support default instances
- **Required Instance ID**: All button instances must have explicit, non-empty instance IDs
- **Unique Instances**: Each button instance maintains separate state and identity
- **No Auto-Spawn**: Button instances are never automatically created at startup

### API Specification
```rust
// Button instances require explicit IDs (REQUIRED)
"button:btn1.click"          # Click button instance 'btn1'
"button:my-btn.set_text"     # Update text for 'my-btn' instance
"button:submit.status"       # Get status of 'submit' button
"button:counter.reset"       # Reset 'counter' button

// INVALID: These will be REJECTED
"button:.click"              # ERROR: Empty instance ID not allowed
"button:.set_text"           # ERROR: Default instance not supported
"system:.spawn {\"class\": \"button\", \"id\": \"\"}" # ERROR: Empty ID rejected
```

### Message Handling Methods
```rust
// Message handling methods:
"click" => increment counter, save state, return success response
"set_text" => update button text, validate input, save state
"status" => return current state (count, text, version)
"reset" => reset counter to zero, save state
```

### Spawning Requirements
```rust
// Button spawning REQUIRES instance_id
"system:.spawn {\"class\": \"button\", \"id\": \"btn1\"}"    # ✅ VALID
"system:.spawn {\"class\": \"button\", \"id\": \"my-btn\"}"  # ✅ VALID
"system:.spawn {\"class\": \"button\", \"id\": \"\"}"       # ❌ ERROR
"system:.spawn {\"class\": \"button\"}"                     # ❌ ERROR
```

### State Structure
```rust
struct ButtonState {
    count: u64,           // Click counter
    text: String,         // Button display text
}
```

### Template Requirements
- **Handlebars template**: `templates/button.hbs`
- **Required context**: `id`, `count`, `text`, `version`
- **Interactive elements**: Click handler, text display
- **Instance identification**: All elements must include instance ID

### WebSocket Updates
```rust
// Updates always include instance ID in selectors
UpdateMessage::SetText {
    selector: format!("#button-{} .count", self.id),  // self.id is never empty
    value: self.state.count.to_string(),
}

UpdateMessage::SetText {
    selector: format!("#button-{} .text", self.id),
    value: self.state.text.clone(),
}
```

## 🔧 Technical Requirements

### Dependencies
- `serde_json` for JSON serialization
- `atomic` operations for counter
- `SimpleStorage` for persistence
- `SimpleRenderer` for template rendering

### Instance Validation
- **Empty ID rejection**: Constructor validates non-empty instance IDs
- **Spawn validation**: System rejects spawn requests with empty IDs
- **Storage keys**: Format `"button:id:key"` (never `"button::key"`)
- **Error messages**: Clear feedback when instance ID is missing

### Performance Targets
- **Click handling**: <100μs (excluding database I/O)
- **Template rendering**: <20μs
- **State persistence**: <5000μs (database bottleneck)

### Error Handling
- **Database errors**: Graceful degradation, log warnings
- **Invalid messages**: Return error response, don't crash
- **Template errors**: Return fallback HTML
- **Empty instance ID**: Return clear error, suggest valid ID format

## 🧪 Testing Requirements

### Unit Tests
- Click increment functionality
- Text validation and setting
- State persistence roundtrip
- WebSocket update generation
- **Instance ID validation**
- **Empty ID rejection**

### Integration Tests
- Browser interaction via WebSocket
- Concurrent click handling
- Database state recovery
- Template rendering with various states
- **Spawn with empty ID (should fail)**
- **Multiple button instances with unique IDs**

### Validation Tests
```rust
#[tokio::test]
async fn test_empty_instance_id_rejection() {
    // Button should reject empty instance IDs
    let result = Button::new("".to_string(), system, config);
    assert!(result.is_err());
    
    // System should reject spawn with empty ID
    let spawn_result = system.send_message("system:.spawn", 
        json!({"class": "button", "id": ""})).await;
    assert!(spawn_result.is_err());
}

#[tokio::test]
async fn test_multiple_button_instances() {
    let mut system = System::new().await?;
    
    // Create multiple button instances with different IDs
    system.send_message("system:.spawn", 
        json!({"class": "button", "id": "btn1"})).await?;
    system.send_message("system:.spawn", 
        json!({"class": "button", "id": "btn2"})).await?;
    
    // Each should maintain separate state
    system.send_message("button:btn1.click", json!({})).await?;
    system.send_message("button:btn2.click", json!({})).await?;
    system.send_message("button:btn2.click", json!({})).await?;
    
    let btn1_status = system.send_message("button:btn1.status", json!({})).await?;
    let btn2_status = system.send_message("button:btn2.status", json!({})).await?;
    
    assert_eq!(btn1_status["count"], 1);
    assert_eq!(btn2_status["count"], 2);
}
```

### Performance Tests
- Click throughput measurement
- Memory usage validation
- Database I/O benchmarking

## 📦 Configuration

### Instance Configuration
```rust
// Each button instance is configured independently
"system:.spawn {
    \"class\": \"button\", 
    \"id\": \"submit-btn\",
    \"config\": {
        \"text\": \"Submit Form\",
        \"initial_count\": 0
    }
}"
```

### No Global Default Configuration
- Button component has no default instance support
- All configuration is per-instance
- No automatic creation at system startup

## 🔄 Multi-Instance Usage Patterns

### Dashboard Buttons
```javascript
// Create multiple buttons for dashboard
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"save\"}");
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"cancel\"}");
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"reset\"}");

// Interact with specific buttons
ws.send("button:save.set_text {\"text\": \"Save Changes\"}");
ws.send("button:cancel.set_text {\"text\": \"Cancel\"}");
ws.send("button:reset.click {}");
```

### Form Controls
```javascript
// Different buttons for different forms
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"login-submit\"}");
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"signup-submit\"}");
ws.send("system:.spawn {\"class\": \"button\", \"id\": \"contact-submit\"}");
```

## 📋 Migration Notes

### No Default Instance Migration
- **Button never had default instances**: No migration needed
- **Always required instance IDs**: Behavior unchanged
- **Storage keys unchanged**: Still use `"button:id:key"` format
- **Template unchanged**: Always included instance ID

### Validation Improvements
- **Better error messages**: Clearer feedback for missing instance IDs
- **System validation**: Spawn requests now validated at system level
- **Type safety**: Component trait prevents accidental default instance creation

### Client-Side Patterns
```javascript
// Correct usage (unchanged)
ws.send("button:btn1.click {}");

// Invalid usage (now with better error messages)
ws.send("button:.click {}");  // ERROR: "Button component requires explicit instance ID"
```

## ⚠️ Important Constraints

### Design Rationale
- **State isolation**: Each button needs unique identity for independent state
- **UI clarity**: Multiple buttons need distinct visual identity
- **Data integrity**: Separate storage prevents state collision
- **User experience**: Clear button identification in interfaces

### Error Prevention
- **Compile-time safety**: `SUPPORTS_DEFAULT = false` prevents accidental default instances
- **Runtime validation**: Constructor and spawn validation catch empty IDs
- **Clear messaging**: Error messages guide users to correct usage patterns

---

## 🔧 Macro-Based Implementation

### Component Macro Attributes
```rust
#[crate::component(template = "button.hbs", config = "Config")]
struct Button {
    // Only business fields - macro generates all infrastructure
    text: String,
    click_count: AtomicU32,
}
```

### Generated Infrastructure
The `#[crate::component]` macro automatically generates:
- **Framework fields**: `id`, `system`, `cfg`, `renderer`, `storage`, `version`
- **Component trait**: Complete implementation with method routing
- **Constructor**: `new(id, system, config)` with template embedding
- **Rendering**: Template rendering with embedded `button.hbs`
- **Configuration**: Type-safe `Config` injection

### Method Implementation
```rust
impl Button {
    // Business methods only - no boilerplate required
    pub async fn click(&mut self) {
        self.click_count.fetch_add(1, Ordering::SeqCst);
        // Storage and rendering handled by macro infrastructure
    }
    
    pub async fn reset(&mut self) {
        self.click_count.store(0, Ordering::SeqCst);
    }
    
    pub async fn set_text(&mut self, text: String) {
        self.text = text;
    }
    
    pub async fn get_count(&self) -> u32 {
        self.click_count.load(Ordering::SeqCst)
    }
}
```

### Configuration Structure
```rust
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default = "default_button_text")]
    pub text: String,
}
```

---

**Status**: Production Ready (Named Instances Only) - **MACRO SYSTEM** ✅  
**Performance**: 4556μs message dispatch (includes DB I/O)  
**Usage**: Reference implementation for named-instance-only components  
**Instance Support**: Named instances only - NO default instance support  
**Migration**: Successfully converted to macro-based implementation
