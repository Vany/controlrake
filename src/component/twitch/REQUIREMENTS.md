# Twitch Component Requirements

## 🎯 Functional Requirements

### Core Functionality
- **OAuth Authentication**: Automatic browser-based OAuth flow with Twitch
- **IRC Connection**: Connect to Twitch IRC for real-time chat events
- **Chat Event Processing**: Handle messages, joins, parts, and moderation events
- **User Validation**: Verify user permissions and broadcaster status
- **Error Recovery**: Automatic reconnection with exponential backoff
- **Modern UI**: Clean, responsive chat interface

### Default Instance Support
- **Automatic Creation**: Twitch component supports default instances
- **Single Default**: Only one default twitch instance can exist per system
- **Auto-Spawn**: Default instance is automatically created at system startup
- **Empty Address**: Default instance uses empty instance ID (`twitch:`)
- **Named Instances**: Additional named instances for multi-channel support (`twitch:stream1`, `twitch:chat`)

### API Specification
```rust
// Default instance syntax (recommended)
"twitch:.connect"            # Connect default instance to Twitch
"twitch:.disconnect"         # Disconnect default instance
"twitch:.send_message"       # Send message via default instance
"twitch:.get_status"         # Get default instance status
"twitch:.validate_user"      # Validate user via default instance

// Named instance syntax (multi-channel support)
"twitch:stream1.connect"     # Connect named instance to specific channel
"twitch:chat.send_message"   # Send message via named chat instance

// Legacy syntax (deprecated but supported)
"twitch:main.connect"        # Old main instance syntax
```

### Message Handling Methods
```rust
// Message handling methods:
"connect" => initiate OAuth flow and IRC connection
"disconnect" => gracefully disconnect from IRC
"send_message" => send chat message to channel
"get_status" => return connection status and user info
"validate_user" => verify user permissions
```

### OAuth Flow Requirements
- **Client credentials**: Support `client_id` and `client_secret` from config
- **Browser launch**: Automatic browser opening for user authorization
- **Token handling**: Secure storage and refresh of access tokens
- **Scope management**: Request appropriate Twitch API scopes
- **Error handling**: Clear feedback for OAuth failures

### IRC Integration
- **Connection management**: Reliable TCP connection to Twitch IRC
- **Message parsing**: Handle PRIVMSG, JOIN, PART, MODE events
- **Rate limiting**: Respect Twitch rate limits for messages
- **Reconnection logic**: Automatic retry with backoff on disconnection

### Chat Event Types
```rust
enum TwitchEvent {
    Message { user: String, text: String, timestamp: DateTime<Utc> },
    Join { user: String },
    Part { user: String },
    Subscription { user: String, plan: String },
    Follow { user: String },
    Moderation { action: String, target: String, moderator: String },
}
```

## 🔧 Technical Requirements

### Dependencies
- `oauth2` for OAuth flow implementation
- `tokio-tungstenite` for WebSocket IRC connection
- `serde_json` for API response parsing
- `chrono` for timestamp handling
- `url` for OAuth URL construction

### Default Instance Configuration
```toml
[twitch]
username = "your_username"          # Twitch username
channel = "target_channel"          # Channel to connect to
client_id = "your_client_id"        # Required for OAuth
client_secret = "your_client_secret" # Required for OAuth
redirect_uri = "http://localhost:3000/auth/callback"
scopes = ["chat:read", "chat:edit", "channel:read:subscriptions"]
# No instance_id needed - uses default instance
```

### Named Instance Configuration
```toml
[twitch.stream1]
instance_id = "stream1"
username = "streamer_account"
channel = "streamer_channel"
client_id = "stream_client_id"
client_secret = "stream_client_secret"
redirect_uri = "http://localhost:3001/auth/callback"
scopes = ["chat:read", "chat:edit"]
```

### Performance Targets
- **OAuth completion**: <30 seconds (user-dependent)
- **IRC connection**: <5 seconds
- **Message processing**: <10μs per message
- **UI updates**: Real-time (<100ms latency)

### Error Handling
- **OAuth failures**: Clear error messages, retry options
- **IRC disconnection**: Automatic reconnection with exponential backoff
- **API rate limits**: Graceful handling, queue messages if needed
- **Invalid credentials**: Clear feedback to user
- **Default instance duplicate**: Prevent multiple default instances

## 🎨 User Interface Requirements

### Chat Display
- **Message list**: Scrollable chat message history
- **User highlighting**: Different colors for mods, subscribers, etc.
- **Timestamp display**: Optional message timestamps
- **Emote rendering**: Support for Twitch emotes (future enhancement)
- **Instance indicator**: Show if default or named instance

### Connection Status
- **Status indicator**: Visual connection state (connected/disconnected/connecting)
- **User info**: Display authenticated username and channel
- **Error messages**: Clear feedback for connection issues
- **Reconnect button**: Manual reconnection trigger

### Template Structure
```handlebars
<div id="twitch-{{#if id}}{{id}}{{else}}default{{/if}}" class="twitch-component">
    <div class="status-bar">
        <span class="connection-status">{{status}}</span>
        <span class="user-info">{{username}} @ {{channel}}</span>
        {{#unless id}}<span class="instance-type">(default)</span>{{/unless}}
    </div>
    <div class="chat-messages">
        {{#each messages}}
        <div class="message">
            <span class="timestamp">{{timestamp}}</span>
            <span class="username">{{user}}</span>
            <span class="text">{{text}}</span>
        </div>
        {{/each}}
    </div>
    <div class="controls">
        <button onclick="self.send('connect')">Connect</button>
        <button onclick="self.send('disconnect')">Disconnect</button>
    </div>
</div>
```

## 🧪 Testing Requirements

### Unit Tests
- OAuth URL generation and validation
- IRC message parsing for various event types
- Token storage and refresh logic
- Error handling scenarios
- **Default instance creation**
- **Empty instance ID handling**

### Integration Tests
- OAuth flow with mock Twitch server
- IRC connection with test channels
- WebSocket update generation for chat events
- Browser UI interaction via test harness
- **Default vs named instance routing**
- **Migration from `main` instance syntax**

### Default Instance Tests
```rust
#[tokio::test]
async fn test_default_instance_creation() {
    let mut system = System::new().await?;
    
    // Default instance should be created automatically
    let result = system.send_message("twitch:.get_status", json!({})).await?;
    assert!(result.is_ok());
    
    // Multiple default instances should be prevented
    let spawn_result = system.send_message("system:.spawn", 
        json!({"class": "twitch", "id": ""})).await?;
    assert!(spawn_result.is_err());
}
```

### Manual Testing
- **Real Twitch integration**: Test with actual Twitch account
- **Channel switching**: Verify channel change functionality
- **Rate limiting**: Test message sending limits
- **Error recovery**: Test network disconnection scenarios

## 🔒 Security Requirements

### Token Management
- **Secure storage**: Encrypt access tokens in database
- **Token refresh**: Automatic refresh before expiration
- **Scope validation**: Verify requested scopes are granted
- **Revocation handling**: Handle token revocation gracefully
- **Instance isolation**: Separate token storage for default vs named instances

### Input Validation
- **Channel names**: Validate Twitch channel name format
- **Message content**: Sanitize chat messages for display
- **User data**: Validate user information from API
- **Configuration**: Validate client credentials format
- **Instance addresses**: Validate empty vs non-empty instance IDs

### Privacy Considerations
- **Log sanitization**: Don't log sensitive tokens or user data
- **Local storage**: Keep tokens local, don't transmit unnecessarily
- **User consent**: Clear indication of data usage

## 📊 Monitoring Requirements

### Connection Health
- **Uptime tracking**: Monitor IRC connection stability
- **Reconnection frequency**: Track automatic reconnections
- **Message throughput**: Monitor messages per second
- **Error rates**: Track OAuth and IRC error frequencies

### Performance Metrics
- **OAuth completion time**: Measure user authorization flow
- **IRC latency**: Measure message receive latency
- **UI responsiveness**: Track WebSocket update delivery
- **Memory usage**: Monitor chat history memory consumption

### Multi-Instance Support
- **Instance tracking**: Monitor default and named instances separately
- **Resource usage**: Track memory/CPU per instance
- **Connection limits**: Monitor total IRC connections
- **Channel distribution**: Track which channels are being monitored

## 📋 Migration Notes

### From Legacy Syntax
- **Old**: `twitch:main` → **New**: `twitch:`
- **Old**: `twitch:main.method` → **New**: `twitch:.method`
- **Automatic**: System creates default instance at startup
- **Backward Compatible**: Named instances still supported
- **Storage**: Existing `main` instance data migrated to default instance

### Client-Side Updates
```javascript
// Old syntax (deprecated)
ws.send("twitch:main.connect {}");

// New syntax (recommended)
ws.send("twitch:.connect {}");

// Named instances (multi-channel support)
ws.send("twitch:stream1.connect {}");
```

### Multi-Channel Migration
```rust
// Old single-channel setup
[twitch]
username = "streamer"
channel = "main_channel"

// New multi-instance setup
[twitch]              # Default instance
username = "streamer"
channel = "main_channel"

[twitch.stream1]      # Additional channel
instance_id = "stream1"
username = "alt_account"
channel = "second_channel"
```

---

## 🔧 Macro-Based Implementation

The Twitch component uses `#[crate::component(template = "twitch.hbs", config = "TwitchConfig")]` 
with comprehensive OAuth, IRC, and event management infrastructure generated automatically.

---

**Status**: Production Ready with Default Instance Support - **MACRO SYSTEM** ✅  
**Integration**: Full OAuth + IRC implementation with multi-instance support  
**Usage**: Real-time Twitch chat integration for content creators  
**Default Instance**: Automatically available at system startup  
**Migration**: Successfully converted to macro-based implementation  
**Multi-Channel**: Support for multiple simultaneous channel connections
