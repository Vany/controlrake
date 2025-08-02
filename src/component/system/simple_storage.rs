/// 🗄️ Simplified Storage Interface
/// 
/// **Purpose**: Replace complex hierarchical path system with simple component-focused storage.
/// 
/// **Design Goals**:
/// - Simple `save_component(id, state)` and `load_component(id)` interface
/// - Eliminate PathElement abstractions and complex path parsing
/// - Keep redb backend for persistence but simplify the interface
/// - Maintain backwards compatibility during migration
/// - Focus on component state management rather than arbitrary document storage
use anyhow::Result;
use redb::{Database, TableDefinition, ReadableTable};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 🗄️ Simplified Storage Engine
/// 
/// Provides a clean, component-focused storage interface that eliminates the complexity
/// of hierarchical paths while maintaining all the persistence functionality.
/// 
/// **MVCC-Optimized**: Leverages redb's built-in Multi-Version Concurrency Control
/// for thread-safe operations without external locking overhead.
/// 
/// **Process Exclusivity**: redb prevents multiple processes from opening the same 
/// database file concurrently, eliminating the need for external process locks.
#[derive(Clone)]
pub struct SimpleStorage {
    /// 💾 Underlying redb database with built-in MVCC thread-safety
    db: Arc<Database>,
    
    /// 📋 Table definition for component state storage
    table: TableDefinition<'static, &'static str, &'static [u8]>,
}

/// 📊 Component metadata for enhanced storage operations
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ComponentMetadata {
    /// 🆔 Component identifier
    pub id: String,
    
    /// 📅 Last updated timestamp
    pub last_updated: u64,
    
    /// 📊 Version number for change tracking
    pub version: u64,
    
    /// 📏 Size of stored data in bytes
    pub size_bytes: usize,
    
    /// 🏷️ Component type/class
    pub component_type: Option<String>,
}

/// 💾 Stored component data with metadata
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StoredComponent {
    /// 📋 Component metadata
    pub metadata: ComponentMetadata,
    
    /// 📦 Component state data
    pub state: Value,
}

impl SimpleStorage {
    /// 🏗️ Create new simplified storage engine
    /// 
    /// **Arguments**:
    /// - `data_path`: Path to the database file
    /// 
    /// **Returns**: New storage engine instance
    pub fn new(data_path: &Path) -> Result<Self> {
        tracing::info!("🗄️ Initializing SimpleStorage at {:?}", data_path);
        
        // Create data directory if needed
        if let Some(parent) = data_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Open database
        let db = Database::create(data_path)?;
        
        // Define table for component storage
        // Key: component_id (string)
        // Value: serialized StoredComponent (bytes)
        const COMPONENT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("components");
        
        // Initialize table to ensure it exists
        {
            let write_txn = db.begin_write()?;
            {
                let _table = write_txn.open_table(COMPONENT_TABLE)?;
                // Table is now created
            }
            write_txn.commit()?;
        }
        
        tracing::info!("✅ SimpleStorage initialized successfully");
        
        Ok(Self {
            db: Arc::new(db),
            table: COMPONENT_TABLE,
        })
    }

    /// 💾 Save component state
    /// 
    /// **Arguments**:
    /// - `component_id`: Unique identifier for the component (e.g., "button:my-btn")
    /// - `state`: Component state as JSON value
    /// - `component_type`: Optional component type for organization
    /// 
    /// **Returns**: Result indicating success/failure
    /// 
    /// **Performance**: Single optimized write transaction with redb's MVCC
    pub async fn save_component(
        &self,
        component_id: &str,
        state: Value,
        component_type: Option<String>,
    ) -> Result<()> {
        tracing::debug!("💾 Saving component state: {}", component_id);
        
        // Create metadata timestamp once
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        // Single write transaction - redb optimization
        let write_txn = self.db.begin_write()?;
        let result: Result<(u64, usize)> = {
            let mut table = write_txn.open_table(self.table)?;
            
            // Get current version within same transaction - much more efficient
            let version = if let Some(existing_data) = table.get(component_id)? {
                let existing_component: StoredComponent = serde_json::from_slice(existing_data.value())?;
                existing_component.metadata.version + 1
            } else {
                1 // New component starts at version 1
            };
            
            // Create metadata with proper size calculation
            let serialized_state = serde_json::to_vec(&state)?;
            let state_size = serialized_state.len();
            let metadata = ComponentMetadata {
                id: component_id.to_string(),
                last_updated: timestamp,
                version,
                size_bytes: state_size,
                component_type,
            };
            
            // Create stored component
            let stored_component = StoredComponent {
                metadata,
                state,
            };
            
            // Single serialization - redb optimization
            let serialized = serde_json::to_vec(&stored_component)?;
            table.insert(component_id, serialized.as_slice())?;
            
            Ok((version, state_size))
        };
        
        // Single commit for atomic operation
        write_txn.commit()?;
        
        let (version, state_size) = result?;
        
        tracing::debug!(
            "✅ Saved component {} (version {}, {} bytes) in single transaction",
            component_id,
            version,
            state_size
        );
        
        Ok(())
    }

    /// 🚀 Optimized batch save operations leveraging redb's MVCC
    /// 
    /// **Arguments**:
    /// - `components`: List of (component_id, state, component_type) tuples to save
    /// 
    /// **Returns**: Result indicating success/failure
    /// 
    /// **Performance**: Single write transaction for multiple components - significantly more efficient
    pub async fn save_components_batch(
        &self,
        components: Vec<(&str, Value, Option<String>)>,
    ) -> Result<()> {
        if components.is_empty() {
            return Ok(());
        }
        
        tracing::debug!("💾 Batch saving {} components", components.len());
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        // Single write transaction for all components - redb MVCC optimization
        let write_txn = self.db.begin_write()?;
        let results: Result<Vec<u64>> = {
            let mut table = write_txn.open_table(self.table)?;
            let mut versions = Vec::with_capacity(components.len());
            
            for (component_id, state, component_type) in &components {
                // Get current version within same transaction
                let version = if let Some(existing_data) = table.get(component_id)? {
                    let existing_component: StoredComponent = serde_json::from_slice(existing_data.value())?;
                    existing_component.metadata.version + 1
                } else {
                    1
                };
                
                // Create metadata with proper size calculation  
                let serialized_state = serde_json::to_vec(state)?;
                let metadata = ComponentMetadata {
                    id: component_id.to_string(),
                    last_updated: timestamp,
                    version,
                    size_bytes: serialized_state.len(),
                    component_type: component_type.clone(),
                };
                
                let stored_component = StoredComponent {
                    metadata,
                    state: state.clone(),
                };
                
                let serialized = serde_json::to_vec(&stored_component)?;
                table.insert(component_id, serialized.as_slice())?;
                
                versions.push(version);
            }
            
            Ok(versions)
        };
        
        // Single commit for all operations - leverages redb's efficient MVCC
        write_txn.commit()?;
        
        let versions = results?;
        tracing::debug!(
            "✅ Batch saved {} components efficiently in single transaction",
            components.len()
        );
        
        // Log details for debugging
        for ((component_id, state, _), version) in components.iter().zip(versions.iter()) {
            tracing::trace!(
                "  - {} (version {}, {} bytes)",
                component_id,
                version,
                serde_json::to_vec(state)?.len()
            );
        }
        
        Ok(())
    }

    /// 📥 Load component state
    /// 
    /// **Arguments**:
    /// - `component_id`: Component identifier to load
    /// 
    /// **Returns**: Component state as JSON value, or None if not found
    /// 
    /// **Performance**: Single database lookup, no path traversal
    pub async fn load_component(&self, component_id: &str) -> Result<Option<Value>> {
        tracing::debug!("📥 Loading component state: {}", component_id);
        
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        
        if let Some(data) = table.get(component_id)? {
            let stored_component: StoredComponent = serde_json::from_slice(data.value())?;
            
            tracing::debug!(
                "✅ Loaded component {} (version {}, {} bytes)",
                component_id,
                stored_component.metadata.version,
                stored_component.metadata.size_bytes
            );
            
            Ok(Some(stored_component.state))
        } else {
            tracing::debug!("📭 Component {} not found in storage", component_id);
            Ok(None)
        }
    }

    /// 📋 Load component with metadata
    /// 
    /// **Arguments**:
    /// - `component_id`: Component identifier to load
    /// 
    /// **Returns**: Full stored component data including metadata
    pub async fn load_component_with_metadata(
        &self,
        component_id: &str,
    ) -> Result<Option<StoredComponent>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        
        if let Some(data) = table.get(component_id)? {
            let stored_component: StoredComponent = serde_json::from_slice(data.value())?;
            Ok(Some(stored_component))
        } else {
            Ok(None)
        }
    }

    /// 🗑️ Delete component state
    /// 
    /// **Arguments**:
    /// - `component_id`: Component to delete
    /// 
    /// **Returns**: Result indicating success/failure
    pub async fn delete_component(&self, component_id: &str) -> Result<bool> {
        tracing::debug!("🗑️ Deleting component: {}", component_id);
        
        let write_txn = self.db.begin_write()?;
        let removed = {
            let mut table = write_txn.open_table(self.table)?;
            let result = table.remove(component_id)?;
            result.is_some()
        };
        write_txn.commit()?;
        
        if removed {
            tracing::debug!("✅ Deleted component: {}", component_id);
        } else {
            tracing::debug!("📭 Component {} not found for deletion", component_id);
        }
        
        Ok(removed)
    }

    /// 📋 List all stored components
    /// 
    /// **Returns**: List of component IDs currently in storage
    pub async fn list_components(&self) -> Result<Vec<String>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        
        let mut components = Vec::new();
        for item in table.iter()? {
            let (key, _) = item?;
            components.push(key.value().to_string());
        }
        
        tracing::debug!("📋 Found {} stored components", components.len());
        Ok(components)
    }

    /// 📊 Get storage statistics
    /// 
    /// **Returns**: Storage statistics and metrics
    pub async fn get_stats(&self) -> Result<Value> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        
        let mut total_components = 0;
        let mut total_size_bytes = 0;
        let mut component_types = std::collections::HashMap::new();
        
        for item in table.iter()? {
            let (_, value) = item?;
            if let Ok(stored_component) = serde_json::from_slice::<StoredComponent>(value.value()) {
                total_components += 1;
                total_size_bytes += stored_component.metadata.size_bytes;
                
                let component_type = stored_component.metadata.component_type
                    .unwrap_or_else(|| "unknown".to_string());
                *component_types.entry(component_type).or_insert(0) += 1;
            }
        }
        
        Ok(serde_json::json!({
            "total_components": total_components,
            "total_size_bytes": total_size_bytes,
            "component_types": component_types,
            "database_path": format!("{:?}", self.db),
        }))
    }

    /// 🧹 Clean up old component versions
    /// 
    /// **Arguments**:
    /// - `max_age_seconds`: Delete components older than this many seconds
    /// 
    /// **Returns**: Number of components deleted
    pub async fn cleanup_old_components(&self, max_age_seconds: u64) -> Result<usize> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let cutoff_time = current_time.saturating_sub(max_age_seconds);
        
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(self.table)?;
        
        let mut to_delete = Vec::new();
        
        for item in table.iter()? {
            let (key, value) = item?;
            if let Ok(stored_component) = serde_json::from_slice::<StoredComponent>(value.value()) {
                if stored_component.metadata.last_updated < cutoff_time {
                    to_delete.push(key.value().to_string());
                }
            }
        }
        
        drop(table);
        drop(read_txn);
        
        // Delete old components
        if !to_delete.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut table = write_txn.open_table(self.table)?;
                for component_id in &to_delete {
                    table.remove(component_id.as_str())?;
                }
            }
            write_txn.commit()?;
            
            tracing::info!("🧹 Cleaned up {} old components", to_delete.len());
        }
        
        Ok(to_delete.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> (SimpleStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = SimpleStorage::new(&db_path).unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_load_component() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let component_id = "button:test-btn";
        let state = serde_json::json!({
            "click_count": 42,
            "text": "Test Button"
        });
        
        // Save component
        storage
            .save_component(component_id, state.clone(), Some("button".to_string()))
            .await
            .unwrap();
        
        // Load component
        let loaded_state = storage.load_component(component_id).await.unwrap();
        assert!(loaded_state.is_some());
        assert_eq!(loaded_state.unwrap(), state);
    }

    #[tokio::test]
    async fn test_component_versioning() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let component_id = "button:versioned-btn";
        
        // Save component multiple times
        for i in 1..=3 {
            let state = serde_json::json!({ "count": i });
            storage
                .save_component(component_id, state, Some("button".to_string()))
                .await
                .unwrap();
        }
        
        // Check version
        let stored = storage
            .load_component_with_metadata(component_id)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(stored.metadata.version, 3);
        assert_eq!(stored.state["count"], 3);
    }

    #[tokio::test]
    async fn test_delete_component() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        let component_id = "button:delete-test";
        let state = serde_json::json!({ "test": true });
        
        // Save component
        storage
            .save_component(component_id, state, None)
            .await
            .unwrap();
        
        // Verify it exists
        assert!(storage.load_component(component_id).await.unwrap().is_some());
        
        // Delete component
        let deleted = storage.delete_component(component_id).await.unwrap();
        assert!(deleted);
        
        // Verify it's gone
        assert!(storage.load_component(component_id).await.unwrap().is_none());
        
        // Try to delete again
        let deleted_again = storage.delete_component(component_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_list_components() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        // Save multiple components
        let components = vec!["button:btn1", "clock:clock1", "twitch:chat1"];
        
        for component_id in &components {
            let state = serde_json::json!({ "id": component_id });
            storage
                .save_component(component_id, state, None)
                .await
                .unwrap();
        }
        
        // List components
        let listed = storage.list_components().await.unwrap();
        
        assert_eq!(listed.len(), 3);
        for component_id in &components {
            assert!(listed.contains(&component_id.to_string()));
        }
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let (storage, _temp_dir) = create_test_storage().await;
        
        // Save components of different types
        storage
            .save_component(
                "button:btn1",
                serde_json::json!({ "clicks": 1 }),
                Some("button".to_string()),
            )
            .await
            .unwrap();
        
        storage
            .save_component(
                "button:btn2",
                serde_json::json!({ "clicks": 2 }),
                Some("button".to_string()),
            )
            .await
            .unwrap();
        
        storage
            .save_component(
                "clock:clock1",
                serde_json::json!({ "time": "12:00" }),
                Some("clock".to_string()),
            )
            .await
            .unwrap();
        
        // Get stats
        let stats = storage.get_stats().await.unwrap();
        
        assert_eq!(stats["total_components"], 3);
        assert_eq!(stats["component_types"]["button"], 2);
        assert_eq!(stats["component_types"]["clock"], 1);
    }
}

impl std::fmt::Debug for SimpleStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleStorage")
            .field("db", &"Arc<Database>")
            .field("table", &"TableDefinition")
            .finish()
    }
}