# System Component Requirements

## Overview
The SystemComponent is the core component that manages the entire ControlRake system lifecycle, component registry, storage access, and well-known default instances.

## Architecture Role
- **Component Registry Owner**: Maintains the master registry of all component instances
- **Storage Mediator**: Provides storage access to all components via SystemComponent
- **Well-Known Instance Manager**: Manages typed default instances (http:, clock:)
- **Lifecycle Controller**: Handles system start, stop, spawn operations
- **Macro System Bridge**: Provides required methods for macro-generated components

## Component Configuration
```rust
// System component uses global Config struct - no component-specific config needed
```

## Default Instance Support
- **Supports Default Instance**: YES
- **Default Instance ID**: Empty string (`""`)
- **Default Instance Address**: `"system:"`
- **Purpose**: System-wide operations accessible without specific instance ID

## Bus Methods

### Default Instance Methods (system:)
- `start` - Initialize system and all components
- `stop` - Shutdown system gracefully  
- `spawn` - Create new component instances dynamically
- `log` - System logging operations
- `set` - Global storage set operations
- `get` - Global storage get operations
- `gets` - Global storage batch get operations
- `delete` - Global storage delete operations

### Instance Methods (system:instance-id)
- `get_stats` - Get system statistics and status
- `supports_default_instance` - Check default instance support

## Storage Architecture
- **Component-Scoped Keys**: Automatic prefixing with component ID (`component_id:key`)
- **SystemComponent Mediation**: All component storage operations go through SystemComponent
- **No Direct DB Access**: Components cannot access storage directly (architectural enforcement)
- **Thread-Safe**: Safe concurrent access across components

## Macro System Requirements
The SystemComponent provides required methods for macro-generated components:

### Component Access Methods
- `get_component<T>(&self, id: &str) -> Option<&T>` - Get typed component reference
- `get_component_mut<T>(&mut self, id: &str) -> Option<&mut T>` - Get mutable typed component reference
- `get_component_by_key(&self, key: &str) -> Option<&Box<dyn Component>>` - Get component by full key
- `get_component_by_key_mut(&mut self, key: &str) -> Option<&mut Box<dyn Component>>` - Get mutable component by full key

### Component Registry Methods  
- `register_component_instance(&mut self, class: &str, instance_id: &str, component: Box<dyn Component>) -> Result<()>` - Register component instance
- `has_component_instance(&self, class: &str, instance_id: &str) -> bool` - Check component existence

### Storage Access Methods
- `get_storage(&self) -> Option<()>` - Get storage reference (currently disabled during Phase 4.2)

## Well-Known Instance Management
The SystemComponent manages typed default instances for core system components:

### Managed Default Instances
- **HTTP Component** (`http:`): Web server and WebSocket bridge
- **Clock Component** (`clock:`): System time and scheduling

### Typed Access Methods
- `get_http_default(&self) -> Option<&HttpComponent>` - Get HTTP default instance
- `get_http_default_mut(&mut self) -> Option<&mut HttpComponent>` - Get mutable HTTP default instance
- `get_clock_default(&self) -> Option<&ClockComponent>` - Get clock default instance  
- `get_clock_default_mut(&mut self) -> Option<&mut ClockComponent>` - Get mutable clock default instance

## Component Discovery
- `discover_all_instances(&self) -> HashMap<String, ComponentInstanceInfo>` - List all registered components
- `discover_class_instances(&self, component_class: &str) -> Vec<ComponentInstanceInfo>` - List instances of specific class
- `get_instances_by_class(&self, component_class: &str) -> Vec<String>` - Get instance keys by class
- `has_default_instance(&self, component_class: &str) -> bool` - Check if class has default instance

## Error Handling
- **Duplicate Registration**: Returns error for duplicate component registration
- **Invalid Default Instance**: Returns error if component doesn't support default instances
- **Storage Failures**: Logs errors and continues operation where possible
- **Component Access Failures**: Returns None for missing components (non-fatal)

## Performance Characteristics
- **Component Access**: O(1) HashMap lookup by component key
- **Storage Operations**: <5000μs (database I/O bottleneck)
- **Registry Operations**: <1μs (in-memory HashMap operations)
- **Typed Access**: Zero-cost downcast for well-known instances

## Integration Points
- **Bus Integration**: Receives messages via class channel registration
- **Config Integration**: Uses global Config struct for initialization
- **Storage Integration**: Direct access to SimpleStorage for mediation
- **Component Integration**: Implements Component trait for bus compatibility

## Testing Requirements
- Test all bus method routing
- Test component registration and discovery
- Test well-known instance management  
- Test storage mediation functionality
- Test macro system bridge methods
- Test error handling for edge cases
- Performance benchmarks for registry operations
