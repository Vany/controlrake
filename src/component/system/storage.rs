use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use crate::SimpleStorage;

static STORAGE_CACHE: Lazy<Mutex<HashMap<String, Arc<SimpleStorage>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn get_or_create_storage(path: &std::path::Path) -> anyhow::Result<Arc<SimpleStorage>> {
    let path_str = path.to_string_lossy().to_string();

    let mut cache = STORAGE_CACHE.lock().unwrap();
    if let Some(existing) = cache.get(&path_str) {
        return Ok(existing.clone());
    }

    let storage = SimpleStorage::new(path)
        .map_err(|e| anyhow::anyhow!("🚨 Storage init failed: {:#}", e))?;

    let arc = Arc::new(storage);
    cache.insert(path_str, arc.clone());
    Ok(arc)
}
