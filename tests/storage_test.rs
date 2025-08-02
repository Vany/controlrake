// Legacy storage system tests removed as part of Phase 5.1 cleanup
// The hierarchical PathElement-based storage system has been eliminated
// and replaced with SimpleStorage component-focused API.
//
// Storage functionality is now tested in:
// - tests/simplified_architecture_test.rs (SimpleStorage integration)
// - Component-specific tests (ButtonComponent storage usage)
//
// The complex PathElement abstractions and storage_set/get/delete methods
// have been removed as documented in Phase 4.2 completion.

use controlrake::component::system::simple_storage::SimpleStorage;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn test_simple_storage_basic_operations() {
    // Test the current SimpleStorage system that replaced legacy storage
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    let storage = SimpleStorage::new(&db_path).unwrap();
    
    // Test basic save/load operations
    let test_data = json!({"test": "value", "count": 42});
    storage.save_component("test_key", test_data.clone(), Some("test".to_string())).await.unwrap();
    
    let loaded = storage.load_component("test_key").await.unwrap();
    assert_eq!(loaded.unwrap(), test_data);
    
    // Test deletion
    storage.delete_component("test_key").await.unwrap();
    let deleted = storage.load_component("test_key").await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_simple_storage_component_scoping() {
    // Test component-scoped storage operations
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    let storage = SimpleStorage::new(&db_path).unwrap();
    
    // Storage automatically scopes by component
    let component_data = json!({"state": "active", "value": 100});
    storage.save_component("component_state", component_data.clone(), Some("component".to_string())).await.unwrap();
    
    let retrieved = storage.load_component("component_state").await.unwrap();
    assert_eq!(retrieved.unwrap(), component_data);
}
