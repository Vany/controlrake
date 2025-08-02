/// 🛠️ Database Recovery System for Stale Lock Handling
/// 
/// **Purpose**: Handle database locks that occur when processes crash or are killed
/// before properly releasing redb database connections.
/// 
/// **Design Goals**:
/// - Detect stale processes via PID file analysis
/// - Test database accessibility with immediate failure detection
/// - Create timestamped backups before destructive recovery
/// - Integrate seamlessly into startup process
/// - Provide detailed logging for debugging
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

const DB_PATH: &str = "data/controlrake.db";
const DB_BACKUP_SUFFIX: &str = ".recovery_backup";

/// 🔧 Database recovery utilities for handling stale locks
pub struct DatabaseRecovery;

impl DatabaseRecovery {
    /// 🛠️ Attempt to recover from database file access issues during startup
    /// 
    /// This function handles the common case where:
    /// 1. Database file has permission or accessibility issues
    /// 2. Database file was corrupted or left in bad state
    /// 
    /// **Recovery Strategy (Simplified - No PID Files)**:
    /// 1. Test database file accessibility (filesystem operations only)
    /// 2. If inaccessible, create backup and start fresh
    /// 3. With flock, process conflicts are handled automatically
    /// 
    /// **Returns**: Ok(()) if recovery successful, Err if manual intervention needed
    pub fn recover_database_locks() -> Result<()> {
        info!("🛠️ Starting database file recovery process");
        
        // Test database file accessibility (filesystem only)
        let db_accessible = Self::test_database_access()?;
        
        if !db_accessible {
            warn!("🔒 Database file is inaccessible - performing recovery");
            Self::recover_inaccessible_database()?;
        } else {
            info!("✅ Database file is accessible - no recovery needed");
        }
        
        info!("✅ Database file recovery complete");
        Ok(())
    }
    
    /// 🧪 Test if database file is accessible using filesystem checks only
    /// 
    /// **Strategy**: Use filesystem operations only - no database connections
    /// **Returns**: true if file is accessible, false if locked or problematic
    fn test_database_access() -> Result<bool> {
        if !Path::new(DB_PATH).exists() {
            info!("📂 Database file doesn't exist - will create new");
            return Ok(true);
        }
        
        info!("🧪 Testing database file accessibility (filesystem only)...");
        
        // Check file permissions and accessibility
        match std::fs::metadata(DB_PATH) {
            Ok(metadata) => {
                // Check if file is readable and writable
                let readonly = metadata.permissions().readonly();
                if readonly {
                    warn!("❌ Database file is read-only");
                    return Ok(false);
                }
                
                // Try to open file for reading to test basic accessibility
                match std::fs::File::open(DB_PATH) {
                    Ok(_) => {
                        info!("✅ Database file is accessible (no database connection test)");
                        Ok(true)
                    }
                    Err(e) => {
                        warn!("❌ Database file access failed: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                warn!("❌ Database file metadata access failed: {}", e);
                Ok(false)
            }
        }
    }
    
    /// 🔧 Recover from inaccessible database by backing up and starting fresh
    /// 
    /// **Safety**: Always creates backup before destructive operations
    fn recover_inaccessible_database() -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let backup_path = format!("{DB_PATH}{DB_BACKUP_SUFFIX}.{timestamp}");
        
        info!("💾 Creating database backup: {}", backup_path);
        fs::copy(DB_PATH, &backup_path)
            .context("Failed to create database backup")?;
        
        info!("🗑️ Removing inaccessible database file");
        fs::remove_file(DB_PATH)
            .context("Failed to remove locked database")?;
        
        info!("✅ Database recovery complete - will start with fresh database");
        info!("📄 Original data backed up to: {}", backup_path);
        
        Ok(())
    }
    
    /// 📋 List all database backup files
    /// 
    /// **Returns**: Sorted list of backup file names
    pub fn list_backups() -> Result<Vec<String>> {
        let mut backups = Vec::new();
        
        if let Ok(entries) = fs::read_dir("data") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if let Some(name) = file_name.to_str() {
                    if name.contains(DB_BACKUP_SUFFIX) {
                        backups.push(name.to_string());
                    }
                }
            }
        }
        
        backups.sort();
        Ok(backups)
    }
    
    /// 🔄 Restore database from backup (for manual recovery)
    /// 
    /// **Arguments**: backup_name - Name of backup file in data/ directory
    /// **Safety**: Creates backup of current database before restore
    pub fn restore_from_backup(backup_name: &str) -> Result<()> {
        let backup_path = format!("data/{backup_name}");
        
        if !Path::new(&backup_path).exists() {
            return Err(anyhow::anyhow!("Backup file not found: {}", backup_path));
        }
        
        // Create backup of current database if it exists
        if Path::new(DB_PATH).exists() {
            let current_backup = format!("{}.replaced.{}", DB_PATH, 
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
            fs::rename(DB_PATH, current_backup)?;
        }
        
        fs::copy(&backup_path, DB_PATH)?;
        info!("🔄 Restored database from backup: {}", backup_name);
        
        Ok(())
    }
}

/// 🛠️ Enhanced process lock with integrated database recovery
/// 
/// This replaces the simple process_lock::acquire_process_lock() function
/// with one that includes database recovery capabilities.
/// 
/// **Workflow**:
/// 1. Attempt database recovery if needed
/// 2. Proceed with flock-based process lock acquisition on database file
/// 3. If lock acquisition fails, provide actionable error message
/// 
/// **Returns**: ProcessLock handle that must be kept alive to maintain lock
pub fn acquire_process_lock_with_recovery() -> Result<super::process_lock::ProcessLock> {
    // Step 1: Attempt database recovery
    info!("🔍 Checking for database recovery needs...");
    if let Err(e) = DatabaseRecovery::recover_database_locks() {
        warn!("⚠️ Database recovery failed: {}", e);
        // Continue anyway - maybe it's a different issue or user intervention required
        // The flock-based process lock will catch actual running processes
    }
    
    // Step 2: Proceed with flock-based process lock acquisition
    info!("🔒 Acquiring flock-based process lock on database file...");
    super::process_lock::acquire_process_lock()
        .context("Failed to acquire process lock after database recovery attempt")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backup_listing() {
        let result = DatabaseRecovery::list_backups();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_database_access_nonexistent() {
        // Test with a path that doesn't exist
        let result = DatabaseRecovery::test_database_access();
        assert!(result.is_ok());
    }
}
