use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use tracing::info;

/// File lock handle that automatically releases on drop
pub struct ProcessLock {
    #[cfg(unix)]
    _file: File,
    #[cfg(not(unix))]
    _file: File,
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        info!("🔓 Process lock released (automatic on drop)");
    }
}

/// 🔒 Acquire exclusive file lock on database file using Unix flock
/// 
/// **Strategy**: Lock the database file directly instead of using separate PID files
/// **Benefits**: 
/// - Automatically released when process dies (Unix guarantee)
/// - No stale lock files to clean up
/// - More reliable than PID-based detection
/// - Works directly on the resource we're protecting
/// 
/// **Arguments**: 
/// - `db_path`: Path to database file to lock
/// 
/// **Returns**: ProcessLock handle that releases lock on drop
pub fn acquire_process_lock_on_database(db_path: &Path) -> Result<ProcessLock> {
    // Create data directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Open or create the database file for locking
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)  // Don't truncate - we just want to lock the file
        .write(true)
        .read(true)
        .open(db_path)?;
    
    info!("🔒 Attempting to acquire exclusive lock on database: {:?}", db_path);
    
    // Try to acquire exclusive lock with immediate failure (non-blocking)
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        
        let result = unsafe {
            libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
        };
        
        if result == 0 {
            info!("✅ Exclusive database lock acquired");
            Ok(ProcessLock { _file: file })
        } else {
            let error = io::Error::last_os_error();
            match error.kind() {
                io::ErrorKind::WouldBlock => {
                    anyhow::bail!(
                        "Database is locked by another controlrake process. \
                         The lock will be automatically released when that process exits. \
                         If you're sure no other process is running, the previous process \
                         may have crashed - wait a moment and try again."
                    );
                }
                _ => {
                    anyhow::bail!("Failed to acquire database lock: {}", error);
                }
            }
        }
    }
    
    #[cfg(not(unix))]
    {
        // Fallback for non-Unix systems - just return the file handle
        // This won't provide actual locking but allows the code to work
        warn!("⚠️ File locking not implemented on non-Unix systems");
        info!("✅ Pseudo-lock acquired (no actual locking on this platform)");
        Ok(ProcessLock { _file: file })
    }
}

/// 🔒 Acquire process lock using default database path (backward compatibility)
/// 
/// **Note**: This maintains the same API as the old pidfile system but uses flock internally
pub fn acquire_process_lock() -> Result<ProcessLock> {
    let db_path = Path::new("data/controlrake.db");
    acquire_process_lock_on_database(db_path)
}

/// 🔓 Release process lock (backward compatibility - but not needed with flock)
/// 
/// **Note**: With flock, locks are automatically released when the process exits
/// or when the ProcessLock handle is dropped. This function exists for API compatibility.
pub fn release_process_lock() {
    info!("🔓 Process lock release requested (flock releases automatically)");
}
