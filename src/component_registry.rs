use crate::component::Component;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 📦 Component Registry - Manages all component instances at global level
///
/// **PURPOSE**: Extract component registry from SystemComponent to fix inception problem
/// **ARCHITECTURE**: Operates at same level as Bus, separate from business logic components
/// **THREAD SAFETY**: Uses Arc<RwLock<>> for safe concurrent access across async tasks
/// **ADDRESSING**: Unified format - "class:" for default instances, "class:id" for named instances
pub struct ComponentRegistry {
    /// Component storage - unified format for all instances
    /// Format: "class:" for default instances, "class:id" for named instances
    components: HashMap<String, Box<dyn Component>>,
}

impl ComponentRegistry {
    /// 🏗️ Create new empty component registry
    pub fn new() -> Self {
        tracing::info!("📦 Creating new ComponentRegistry");
        Self {
            components: HashMap::new(),
        }
    }

    /// 📦 Register component instance with validation
    ///
    /// **Arguments**:
    /// - `class`: Component class name (e.g., "button", "http", "system")
    /// - `instance_id`: Instance identifier (empty string for default instances)
    /// - `component`: Boxed component implementing Component trait
    ///
    /// **Returns**: Result<()> - Ok if registered successfully, Err if validation fails
    ///
    /// **Validation**:
    /// - Checks if component supports default instances when instance_id is empty
    /// - Prevents duplicate registration of same component key
    /// - Logs registration for debugging
    pub fn register_component(
        &mut self,
        class: &str,
        instance_id: &str,
        component: Box<dyn Component>,
    ) -> Result<()> {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };

        // Validate default instance support
        if instance_id.is_empty() && !component.supports_default_instance() {
            anyhow::bail!(
                "Component class '{}' does not support default instances",
                class
            );
        }

        // Check for duplicates
        if self.components.contains_key(&key) {
            anyhow::bail!("Component '{}' already registered", key);
        }

        self.components.insert(key.clone(), component);
        tracing::info!("📦 Registered component: {}", key);
        Ok(())
    }

    /// 🔍 Get component by typed reference
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty string for default instances)
    ///
    /// **Returns**: Option<&T> - Typed reference to component if found and type matches
    ///
    /// **Type Safety**: Uses downcast_ref for safe type conversion
    /// **Performance**: Direct HashMap lookup with O(1) complexity
    pub fn get_component<T: Component + 'static>(
        &self,
        class: &str,
        instance_id: &str,
    ) -> Option<&T> {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };

        self.components
            .get(&key)
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }

    /// 🔍 Get mutable component by typed reference
    ///
    /// **Arguments**:
    /// - `class`: Component class name  
    /// - `instance_id`: Instance identifier (empty string for default instances)
    ///
    /// **Returns**: Option<&mut T> - Mutable typed reference to component if found and type matches
    ///
    /// **Type Safety**: Uses downcast_mut for safe mutable type conversion
    /// **Mutability**: Allows modification of component state
    pub fn get_component_mut<T: Component + 'static>(
        &mut self,
        class: &str,
        instance_id: &str,
    ) -> Option<&mut T> {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };

        self.components
            .get_mut(&key)
            .and_then(|c| c.as_any_mut().downcast_mut::<T>())
    }

    /// 🔍 Check if component exists
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty string for default instances)
    ///
    /// **Returns**: bool - true if component is registered, false otherwise
    ///
    /// **Performance**: Fast HashMap key existence check
    /// **Use Case**: Validation before attempting to access components
    pub fn has_component(&self, class: &str, instance_id: &str) -> bool {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };
        self.components.contains_key(&key)
    }

    /// 📋 List all registered component keys
    ///
    /// **Returns**: Vec<String> - All component keys in format "class:" or "class:id"
    ///
    /// **Use Cases**:
    /// - Debugging and monitoring
    /// - Component discovery
    /// - System health checks
    /// - Administrative interfaces
    pub fn list_components(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }

    /// 🗑️ Unregister component instance
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty string for default instances)
    ///
    /// **Returns**: bool - true if component was removed, false if it didn't exist
    ///
    /// **Cleanup**: Completely removes component from registry
    /// **Memory Management**: Drops the component, freeing resources
    pub fn unregister_component(&mut self, class: &str, instance_id: &str) -> bool {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };

        let was_removed = self.components.remove(&key).is_some();
        if was_removed {
            tracing::info!("🗑️ Unregistered component: {}", key);
        }
        was_removed
    }

    /// 📊 Get component registry statistics
    ///
    /// **Returns**: ComponentRegistryStats - Aggregate statistics for monitoring
    ///
    /// **Statistics Included**:
    /// - Total component count
    /// - Default instance count (those ending with ":")
    /// - Named instance count
    /// - Breakdown by component class
    pub fn get_stats(&self) -> ComponentRegistryStats {
        let mut default_instances = 0;
        let mut named_instances = 0;
        let mut class_counts: HashMap<String, usize> = HashMap::new();

        for key in self.components.keys() {
            if key.ends_with(':') {
                default_instances += 1;
                // Extract class name for default instances
                let class = key.trim_end_matches(':');
                *class_counts.entry(class.to_string()).or_insert(0) += 1;
            } else {
                named_instances += 1;
                // Extract class name for named instances
                if let Some(colon_pos) = key.find(':') {
                    let class = &key[..colon_pos];
                    *class_counts.entry(class.to_string()).or_insert(0) += 1;
                }
            }
        }

        ComponentRegistryStats {
            total_components: self.components.len(),
            default_instances,
            named_instances,
            class_counts,
        }
    }

    /// 🔍 List components by class
    ///
    /// **Arguments**:
    /// - `class`: Component class name to filter by
    ///
    /// **Returns**: Vec<String> - All component keys for the specified class
    ///
    /// **Use Cases**:
    /// - Finding all instances of a specific component type
    /// - Class-specific monitoring and management
    /// - Bulk operations on component classes
    pub fn list_components_by_class(&self, class: &str) -> Vec<String> {
        let prefix = format!("{class}:");
        self.components
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect()
    }

}

impl std::fmt::Debug for ComponentRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stats = self.get_stats();
        f.debug_struct("ComponentRegistry")
            .field("total_components", &stats.total_components)
            .field("default_instances", &stats.default_instances)
            .field("named_instances", &stats.named_instances)
            .field("class_counts", &stats.class_counts)
            .finish()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 📊 Component Registry Statistics
///
/// **PURPOSE**: Aggregate statistics for monitoring and debugging
/// **THREAD SAFETY**: Immutable snapshot safe to share across threads
#[derive(Debug, Clone)]
pub struct ComponentRegistryStats {
    /// Total number of registered components
    pub total_components: usize,
    
    /// Number of default instances (keys ending with ":")
    pub default_instances: usize,
    
    /// Number of named instances (keys with "class:id" format)
    pub named_instances: usize,
    
    /// Count of instances per component class
    pub class_counts: HashMap<String, usize>,
}

/// 📡 Component Registry Handle - Thread-safe access wrapper
///
/// **PURPOSE**: Provide thread-safe access to ComponentRegistry across async tasks
/// **THREAD SAFETY**: Uses Arc<RwLock<>> for concurrent read/write access
/// **CLONING**: Cheap to clone - only clones Arc, not the underlying data
/// **PATTERN**: Same pattern as BusHandle for consistency
#[derive(Debug, Clone)]
pub struct ComponentRegistryHandle {
    /// Inner registry protected by RwLock for thread safety
    inner: Arc<RwLock<ComponentRegistry>>,
}

impl ComponentRegistryHandle {
    /// 🏗️ Create new handle wrapping a ComponentRegistry
    ///
    /// **Arguments**:
    /// - `registry`: ComponentRegistry to wrap
    ///
    /// **Returns**: ComponentRegistryHandle for thread-safe access
    ///
    /// **Thread Safety**: Wraps registry in Arc<RwLock<>> for safe sharing
    pub fn new(registry: ComponentRegistry) -> Self {
        Self {
            inner: Arc::new(RwLock::new(registry)),
        }
    }

    /// 📦 Register component instance (async thread-safe)
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty for default instances)
    /// - `component`: Boxed component implementing Component trait
    ///
    /// **Returns**: Result<()> - Ok if registered successfully
    ///
    /// **Concurrency**: Uses write lock for exclusive access during registration
    /// **Blocking**: Will wait for other operations to complete
    pub async fn register_component(
        &self,
        class: &str,
        instance_id: &str,
        component: Box<dyn Component>,
    ) -> Result<()> {
        let mut registry = self.inner.write().await;
        registry.register_component(class, instance_id, component)
    }

    /// 🔍 Get component by typed reference (async thread-safe)
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty for default instances)
    ///
    /// **Returns**: Option<T> - Cloned component data if found and type matches
    ///
    /// **Concurrency**: Uses read lock for shared access
    /// **Performance**: Multiple readers can access simultaneously
    /// **Note**: Returns owned value due to async constraints
    pub async fn get_component<T: Component + Clone + 'static>(
        &self,
        class: &str,
        instance_id: &str,
    ) -> Option<T> {
        let registry = self.inner.read().await;
        registry.get_component::<T>(class, instance_id).cloned()
    }

    /// 🔍 Check if component exists (async thread-safe)
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty for default instances)
    ///
    /// **Returns**: bool - true if component is registered
    ///
    /// **Concurrency**: Uses read lock for shared access
    /// **Performance**: Fast check without data transfer
    pub async fn has_component(&self, class: &str, instance_id: &str) -> bool {
        let registry = self.inner.read().await;
        registry.has_component(class, instance_id)
    }

    /// 🔧 Execute method on component without cloning (async thread-safe)
    ///
    /// **Purpose**: Allows calling methods on components that don't implement Clone
    /// **Arguments**:
    /// - `class`: Component class name  
    /// - `instance_id`: Instance identifier (empty for default instances)
    /// - `f`: Async closure to execute on the component
    ///
    /// **Returns**: Option<R> - Result of the closure execution if component found
    ///
    /// **Use Case**: For components like SystemComponent that can't be cloned
    /// **Concurrency**: Uses write lock for exclusive access during method execution
    /// **Example**: `registry.with_component_mut("system", "", |sys| sys.stop()).await`
    pub async fn with_component_mut<T, R, F, Fut>(
        &self,
        class: &str,
        instance_id: &str,
        f: F,
    ) -> Option<R>
    where
        T: Component + 'static,
        F: FnOnce(&mut T) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let mut registry = self.inner.write().await;
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };

        if let Some(component_box) = registry.components.get_mut(&key) {
            // Downcast to the specific component type
            if let Some(component) = component_box.as_any_mut().downcast_mut::<T>() {
                Some(f(component).await)
            } else {
                tracing::warn!("🔄 Component type mismatch for key: {}", key);
                None
            }
        } else {
            None
        }
    }

    /// 🛑 Stop system component directly (specialized method for shutdown)
    ///
    /// **Purpose**: Specific method to call stop() on SystemComponent for shutdown
    /// **Returns**: Option<Result<serde_json::Value>> - Result of the stop operation
    ///
    /// **Use Case**: Avoids lifetime issues with generic with_component_mut method
    /// **Implementation**: Direct downcast and method call for SystemComponent
    pub async fn stop_system_component(&self) -> Option<anyhow::Result<serde_json::Value>> {
        let mut registry = self.inner.write().await;
        let key = "system:".to_string(); // Default system instance
        
        if let Some(component_box) = registry.components.get_mut(&key) {
            // Downcast to SystemComponent specifically
            if let Some(system) = component_box.as_any_mut().downcast_mut::<crate::component::system::SystemComponent>() {
                Some(system.stop().await)
            } else {
                tracing::error!("🔄 Component at 'system:' is not a SystemComponent");
                Some(Err(anyhow::anyhow!("Component type mismatch - not a SystemComponent")))
            }
        } else {
            tracing::error!("🔍 SystemComponent not found at 'system:'");
            None
        }
    }

    /// 📋 List all registered component keys (async thread-safe)
    ///
    /// **Returns**: Vec<String> - All component keys
    ///
    /// **Concurrency**: Uses read lock for shared access
    /// **Data Transfer**: Returns owned vector for thread safety
    pub async fn list_components(&self) -> Vec<String> {
        let registry = self.inner.read().await;
        registry.list_components()
    }

    /// 🗑️ Unregister component instance (async thread-safe)
    ///
    /// **Arguments**:
    /// - `class`: Component class name
    /// - `instance_id`: Instance identifier (empty for default instances)
    ///
    /// **Returns**: bool - true if component was removed
    ///
    /// **Concurrency**: Uses write lock for exclusive access during removal
    /// **Cleanup**: Completely removes component and frees resources
    pub async fn unregister_component(&self, class: &str, instance_id: &str) -> bool {
        let mut registry = self.inner.write().await;
        registry.unregister_component(class, instance_id)
    }

    /// 📊 Get component registry statistics (async thread-safe)
    ///
    /// **Returns**: ComponentRegistryStats - Aggregate statistics
    ///
    /// **Concurrency**: Uses read lock for shared access
    /// **Monitoring**: Safe to call frequently for health checks
    pub async fn get_stats(&self) -> ComponentRegistryStats {
        let registry = self.inner.read().await;
        registry.get_stats()
    }

    /// 🔍 List components by class (async thread-safe)
    ///
    /// **Arguments**:
    /// - `class`: Component class name to filter by
    ///
    /// **Returns**: Vec<String> - Component keys for specified class
    ///
    /// **Concurrency**: Uses read lock for shared access
    /// **Use Case**: Class-specific operations and monitoring
    pub async fn list_components_by_class(&self, class: &str) -> Vec<String> {
        let registry = self.inner.read().await;
        registry.list_components_by_class(class)
    }

    /// 🔄 Execute operation with read access
    ///
    /// **Arguments**:
    /// - `operation`: Closure that receives &ComponentRegistry
    ///
    /// **Returns**: Result of operation
    ///
    /// **Use Case**: Complex read operations that need direct registry access
    /// **Performance**: Minimizes lock duration for batch operations
    pub async fn with_read<F, R>(&self, operation: F) -> R
    where
        F: FnOnce(&ComponentRegistry) -> R,
    {
        let registry = self.inner.read().await;
        operation(&registry)
    }

    /// 🔄 Execute operation with write access
    ///
    /// **Arguments**:
    /// - `operation`: Closure that receives &mut ComponentRegistry
    ///
    /// **Returns**: Result of operation
    ///
    /// **Use Case**: Complex write operations that need direct registry access
    /// **Atomicity**: Ensures operation completes before other access
    pub async fn with_write<F, R>(&self, operation: F) -> R
    where
        F: FnOnce(&mut ComponentRegistry) -> R,
    {
        let mut registry = self.inner.write().await;
        operation(&mut registry)
    }
}

/// 🧪 Testing utilities for ComponentRegistry
#[cfg(test)]
impl ComponentRegistry {
    /// Create test registry with mock components
    pub fn new_for_test() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl ComponentRegistryHandle {
    /// Create test handle with empty registry
    pub fn new_for_test() -> Self {
        Self::new(ComponentRegistry::new())
    }
}
