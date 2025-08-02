# HTTP Component Requirements

## 🎯 Functional Requirements

### Core Functionality
- **Static File Serving**: Serve files from `static/` directory
- **WebSocket Server**: Real-time bidirectional communication with browsers
- **Component Rendering**: Render any component via HTTP requests
- **Message Routing**: Route WebSocket messages to appropriate components
- **CORS Support**: Enable cross-origin requests for development

### Default Instance Support
- **Automatic Creation**: HTTP component supports default instances
- **Single Instance**: Only one default HTTP instance can exist per system
- **Auto-Spawn**: Default instance is automatically created at system startup
- **Empty Address**: Default instance uses empty instance ID (`http:`)

### API Specification

#### HTTP Endpoints
```
GET /                           # Serve index.html
GET /static/*                   # Serve static files
GET /render/{component_address} # Render component HTML
GET /health                     # Health check endpoint
```

#### WebSocket Protocol
```javascript
// Default instance syntax (recommended)
"system:.spawn {\"class\": \"button\", \"id\": \"btn1\"}"  # Component creation
"button:btn1 {\"action\": \"click\"}"                     # Component message  
"button:btn1.render {\"id\": \"req-123\"}"               # Component render

// Legacy named instance syntax (still supported)
"system:main.spawn {\"class\": \"button\", \"id\": \"btn1\"}"

// Direct HTTP messages to default instance
"http: {\"action\": \"status\"}"                          # Default instance
"http:.health {}"                                         # Default instance method
```

#### WebSocket Responses
```javascript
// Success response
{"success": true, "data": {...}, "address": "button:btn1"}

// Error response  
{"error": "Component not found", "address": "button:btn1"}

// Render response
{"render": "<html>...</html>", "id": "req-123"}

// Update broadcast
{"update": [...], "address": "button:btn1", "version": 42}
```

### State Requirements
- **Component Registry**: Track active component instances
- **WebSocket Connections**: Manage multiple concurrent connections
- **Update Subscriptions**: Track which clients want which component updates
- **Default Instance**: Automatically registered with empty instance ID

## 🔧 Technical Requirements

### Dependencies
- `warp` web framework for HTTP and WebSocket
- `tokio` for async runtime
- `serde_json` for message serialization
- `tower_http` for CORS and static file serving

### Default Instance Configuration
```toml
[http]
host = "127.0.0.1"
port = 8080
static_path = "./static"
cors_enabled = true
# No instance_id needed - uses default instance
```

### Performance Targets
- **Static file serving**: <1ms response time
- **WebSocket message routing**: <100μs
- **Component rendering**: <50μs (excluding component render time)
- **Concurrent connections**: Support 1000+ WebSocket connections

### Error Handling
- **Component not found**: Return structured error response
- **Invalid message format**: Log and return error
- **WebSocket disconnection**: Clean up subscriptions
- **File not found**: Return 404 with helpful message
- **Default instance duplicate**: Prevent multiple default instances

## 🌐 WebSocket Update System

### Real-time Updates
```rust
// Update message types
enum UpdateMessage {
    SetText { selector: String, value: String },
    SetAttribute { selector: String, attribute: String, value: String },
    AddClass { selector: String, class: String },
    RemoveClass { selector: String, class: String },
}
```

### Subscription Management
- **Client tracking**: Map WebSocket connections to subscribed components
- **Update broadcasting**: Send component updates to subscribed clients
- **Version tracking**: Only send updates newer than client's last version

### Performance Optimization
- **Incremental updates**: Only send changed DOM elements
- **Batch updates**: Group multiple updates in single message
- **Connection pooling**: Reuse WebSocket connections efficiently

## 🧪 Testing Requirements

### Unit Tests
- HTTP endpoint responses
- WebSocket message parsing
- Component rendering integration
- Error handling scenarios
- **Default instance creation**
- **Empty instance ID handling**

### Integration Tests
- Browser JavaScript client interaction
- Multiple concurrent WebSocket connections
- Component lifecycle via WebSocket
- Static file serving with various file types
- **Default vs named instance routing**
- **Migration from `main` instance syntax**

### Performance Tests
- WebSocket message throughput
- Concurrent connection handling
- Memory usage under load
- Response time measurement

## 📁 Static File Requirements

### Directory Structure
```
static/
├── index.html              # Landing page
├── js/controlrake.js       # JavaScript client library
├── html/                   # Test pages
├── css/                    # Stylesheets
└── assets/                 # Images, fonts, etc.
```

### Content Types
- **HTML**: `text/html`
- **JavaScript**: `application/javascript`
- **CSS**: `text/css`
- **JSON**: `application/json`
- **Images**: Appropriate MIME types

## 🔒 Security Requirements

### Input Validation
- **Component addresses**: Validate format `class:id`, `class:`, or `class:id.method`
- **Default instance addresses**: Accept empty instance ID in `class:` format
- **Message payloads**: Validate JSON structure
- **File paths**: Prevent directory traversal attacks
- **WebSocket origins**: Optional origin validation

### Rate Limiting
- **Message frequency**: Prevent WebSocket spam
- **Connection limits**: Limit connections per IP
- **File serving**: Prevent static file abuse

## 📋 Migration Notes

### From Legacy Syntax
- **Old**: `http:main` → **New**: `http:`
- **Old**: `http:main.method` → **New**: `http:.method`
- **Automatic**: System creates default instance at startup
- **Backward Compatible**: Named instances still supported
- **Storage**: Existing `main` instance data migrated to default instance

### Client-Side Updates
```javascript
// Old syntax (deprecated)
ws.send("http:main.health {}");

// New syntax (recommended)
ws.send("http:.health {}");
```

---

## 🔧 Macro-Based Implementation

### Component Attributes
```rust
#[crate::component(template = "http.hbs", config = "HttpComponentConfig")]
struct HttpComponent {
    // Business fields only - all infrastructure generated by macro
    server_handle: Option<ServerHandle>,
    websocket_state: WebSocketState,
}
```

### Generated Features
- **Complete Component trait** with method routing
- **Configuration injection** of `HttpComponentConfig`
- **Template embedding** for HTTP responses
- **Storage integration** for session management
- **Bus method routing** for WebSocket communication

---

**Status**: Production Ready with Default Instance Support - **MACRO SYSTEM** ✅  
**Performance**: 11μs template rendering, <1ms static serving  
**Usage**: Bridge between browser clients and component system  
**Default Instance**: Automatically available at system startup  
**Migration**: Successfully converted to macro-based implementation
