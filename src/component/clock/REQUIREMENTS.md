# Clock Component Requirements

## 🎯 Functional Requirements

### Core Functionality
- **Real-time Clock Display**: Show current time with configurable formatting
- **Timezone Support**: Display time in different timezones
- **Automatic Updates**: Periodic time updates without manual refresh
- **Format Customization**: Various time format options (12/24 hour, date formats)
- **Multiple Instances**: Support multiple clock instances with different configurations

### Default Instance Support
- **Automatic Creation**: Clock component supports default instances
- **Single Default**: Only one default clock instance can exist per system
- **Auto-Spawn**: Default instance is automatically created at system startup
- **Empty Address**: Default instance uses empty instance ID (`clock:`)
- **Named Instances**: Additional named instances still supported (`clock:wall`, `clock:desk`)

### API Specification
```rust
// Default instance syntax (recommended)
"clock:.get_time"            # Get time from default instance
"clock:.set_timezone"        # Change default instance timezone
"clock:.set_format"          # Update default instance format
"clock:.start_updates"       # Begin default instance updates
"clock:.stop_updates"        # Stop default instance updates
"clock:.status"              # Default instance status

// Named instance syntax (still supported)
"clock:wall.get_time"        # Get time from named instance
"clock:desk.set_timezone"    # Change named instance timezone

// Legacy syntax (deprecated but supported)
"clock:main.get_time"        # Old main instance syntax
```

### Message Handling Methods
```rust
// Message handling methods:
"get_time" => return current time in configured format
"set_timezone" => change timezone for time display
"set_format" => update time format string
"start_updates" => begin periodic time updates
"stop_updates" => stop periodic time updates
"status" => return clock configuration and current time
```

### Time Format Options
```rust
// Supported format patterns (using chrono formatting):
"%H:%M:%S"           // 24-hour format: 14:30:45
"%I:%M:%S %p"        // 12-hour format: 02:30:45 PM
"%Y-%m-%d %H:%M:%S"  // Full datetime: 2025-07-27 14:30:45
"%A, %B %d, %Y"      // Full date: Sunday, July 27, 2025
"%c"                 // Locale default: Sun Jul 27 14:30:45 2025
```

### Timezone Support
```rust
// Supported timezone specifications:
"UTC"                // Coordinated Universal Time
"America/New_York"   // Eastern Time
"Europe/London"      // Greenwich Mean Time / British Summer Time
"Asia/Tokyo"         // Japan Standard Time
"Local"              // System local timezone
```

### Update Intervals
- **Default**: 1 second updates for smooth time display
- **Configurable**: Support custom update intervals (100ms - 60s)
- **Adaptive**: Reduce update frequency when not visible (future enhancement)

## 🔧 Technical Requirements

### Dependencies
- `chrono` for time handling and formatting
- `chrono-tz` for timezone support
- `tokio` for periodic update timers
- `serde` for configuration serialization

### Default Instance Configuration
```toml
[clock]
default_timezone = "Local"
default_format = "%H:%M:%S"
update_interval_ms = 1000
# No instance_id needed - uses default instance
```

### Named Instance Configuration
```toml
[clock.wall]
instance_id = "wall"
timezone = "America/New_York"
format = "%I:%M:%S %p"
update_interval_ms = 1000
```

### State Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClockState {
    timezone: String,          // Timezone identifier
    format: String,            // Time format string
    update_interval: Duration, // Update frequency
    is_running: bool,          // Update timer status
    last_update: DateTime<Utc>, // Last time value
}
```

### Performance Targets
- **Time calculation**: <10μs per update
- **Template rendering**: <20μs
- **WebSocket updates**: <5μs generation
- **Memory usage**: <1KB per clock instance

### Error Handling
- **Invalid timezone**: Fall back to UTC with warning
- **Invalid format**: Fall back to default format
- **Timer errors**: Restart update timer automatically
- **Configuration errors**: Use sensible defaults
- **Default instance duplicate**: Prevent multiple default instances

## 🎨 User Interface Requirements

### Display Elements
- **Time display**: Large, readable time text
- **Timezone indicator**: Show current timezone
- **Format display**: Optional format pattern display
- **Status indicator**: Running/stopped state
- **Instance identifier**: Show if default or named instance

### Interactive Controls
- **Start/Stop**: Manual control of time updates
- **Timezone selector**: Dropdown or input for timezone change
- **Format input**: Text field for custom format patterns
- **Preset buttons**: Quick access to common formats

### Template Structure
```handlebars
<div id="clock-{{#if id}}{{id}}{{else}}default{{/if}}" class="clock-component">
    <div class="time-display">
        <span class="time">{{current_time}}</span>
        <span class="timezone">{{timezone}}</span>
        {{#unless id}}<span class="instance-type">(default)</span>{{/unless}}
    </div>
    <div class="format-info">
        Format: <code>{{format}}</code>
    </div>
    <div class="controls">
        <button onclick="self.send('start_updates')">Start</button>
        <button onclick="self.send('stop_updates')">Stop</button>
        <select onchange="self.send('set_timezone', {timezone: this.value})">
            <option value="UTC">UTC</option>
            <option value="America/New_York">Eastern</option>
            <option value="Europe/London">London</option>
            <option value="Asia/Tokyo">Tokyo</option>
        </select>
    </div>
</div>
```

### Real-time Updates
```rust
// WebSocket update for time change (default instance)
UpdateMessage::SetText {
    selector: "#clock-default .time".to_string(),
    value: formatted_time,
}

// WebSocket update for time change (named instance)
UpdateMessage::SetText {
    selector: format!("#clock-{} .time", self.id),
    value: formatted_time,
}

// Update timezone display
UpdateMessage::SetText {
    selector: self.get_timezone_selector(),
    value: self.state.timezone.clone(),
}
```

## 🧪 Testing Requirements

### Unit Tests
- Time formatting with various patterns
- Timezone conversion accuracy
- Update timer functionality
- Configuration validation
- **Default instance creation**
- **Empty instance ID handling**

### Integration Tests
- WebSocket update delivery
- Browser time display accuracy
- Timezone changes via WebSocket
- Concurrent clock instances
- **Default vs named instance routing**
- **Migration from `main` instance syntax**

### Performance Tests
- Update frequency accuracy
- Memory usage over time
- CPU usage during updates
- Latency of WebSocket updates

### Default Instance Tests
```rust
#[tokio::test]
async fn test_default_instance_creation() {
    let mut system = System::new().await?;
    
    // Default instance should be created automatically
    let result = system.send_message("clock:.get_time", json!({})).await?;
    assert!(result.is_ok());
    
    // Multiple default instances should be prevented
    let spawn_result = system.send_message("system:.spawn", 
        json!({"class": "clock", "id": ""})).await?;
    assert!(spawn_result.is_err());
}
```

### Time Zone Tests
```rust
#[tokio::test]
async fn test_timezone_accuracy() {
    let mut clock = ClockComponent::new_simple("test".to_string(), bus_handle)?;
    
    // Set to a specific timezone
    clock.handle_message("set_timezone", json!({"timezone": "America/New_York"})).await?;
    
    // Verify time is correctly offset from UTC
    let time_response = clock.handle_message("get_time", json!({})).await?;
    let displayed_time = time_response["time"].as_str().unwrap();
    
    // Compare with expected offset
    assert_time_zone_correct(displayed_time, "America/New_York");
}
```

## ⚙️ Configuration Options

### Default Instance Configuration
```rust
// Default clock instance creation (automatic)
{
    "timezone": "Local",
    "format": "%H:%M:%S",
    "update_interval_ms": 1000,
    "auto_start": true
}
```

### Named Instance Configuration
```rust
// Named clock instance creation
{
    "instance_id": "wall",
    "timezone": "Europe/London",
    "format": "%H:%M:%S",
    "update_interval_ms": 500,
    "auto_start": true
}
```

### Global Configuration
```toml
[clock]
# Default settings for new clock instances
default_timezone = "Local"
default_format = "%H:%M:%S"
default_update_interval_ms = 1000
max_instances = 10
```

### Format Presets
```rust
// Predefined format options
const FORMAT_PRESETS: &[(&str, &str)] = &[
    ("time_24", "%H:%M:%S"),
    ("time_12", "%I:%M:%S %p"),
    ("datetime", "%Y-%m-%d %H:%M:%S"),
    ("date_only", "%Y-%m-%d"),
    ("full_date", "%A, %B %d, %Y"),
    ("iso8601", "%Y-%m-%dT%H:%M:%S%z"),
];
```

## 🔄 Update Management

### Timer Implementation
```rust
// Periodic update task
async fn run_update_timer(&mut self) -> Result<()> {
    while self.state.is_running {
        tokio::time::sleep(self.state.update_interval).await;
        
        let now = Utc::now();
        if now.duration_since(self.state.last_update) >= self.state.update_interval {
            self.update_time().await?;
            self.state.last_update = now;
            self.version.fetch_add(1, Ordering::SeqCst);
        }
    }
    Ok(())
}
```

### Efficient Updates
- **Change detection**: Only update when display time changes
- **Batch updates**: Group timezone and format changes
- **Selective rendering**: Only re-render changed elements
- **Timer management**: Clean shutdown of update timers

## 🚀 Performance Considerations

### Memory Management
- **Circular buffers**: Limit stored time history
- **Timer cleanup**: Proper disposal of background tasks
- **String pooling**: Reuse formatted time strings
- **Timezone caching**: Cache timezone objects

### CPU Optimization
- **Lazy evaluation**: Only calculate time when needed
- **Format caching**: Reuse chrono format objects
- **Update batching**: Reduce WebSocket message frequency
- **Background updates**: Offload to background tasks

## 📋 Migration Notes

### From Legacy Syntax
- **Old**: `clock:main` → **New**: `clock:`
- **Old**: `clock:main.method` → **New**: `clock:.method`
- **Automatic**: System creates default instance at startup
- **Backward Compatible**: Named instances still supported
- **Storage**: Existing `main` instance data migrated to default instance

### Client-Side Updates
```javascript
// Old syntax (deprecated)
ws.send("clock:main.get_time {}");

// New syntax (recommended)
ws.send("clock:.get_time {}");

// Named instances (still supported)
ws.send("clock:wall.get_time {}");
```

---

## 🔧 Macro-Based Implementation

The Clock component uses `#[crate::component(template = "clock.hbs", config = "ClockComponentConfig")]` 
with automatic infrastructure generation for time management and subscription handling.

---

**Status**: Production Ready with Default Instance Support - **MACRO SYSTEM** ✅  
**Performance**: <20μs time calculation, real-time updates  
**Usage**: Time display for dashboards and monitoring interfaces  
**Default Instance**: Automatically available at system startup  
**Migration**: Successfully converted to macro-based implementation
