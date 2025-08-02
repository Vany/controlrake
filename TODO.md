# ControlRake TODO

- [ ] Immediatly ensure all components use this functionality. there is no old code in other components.

*Current tasks and priorities for the macro-based component architecture system.*

## 🎯 Active Tasks

### [ ] Consolidate Database Management into SystemComponent and Eliminate Lock Conflicts

**PROBLEM**: Database is opened multiple times causing "Database already open. Cannot acquire lock." errors. Complex pidfile system when Unix flock would be simpler.

**ERROR SYMPTOMS**:
```
🚨 Storage engine init failed: Database already open. Cannot acquire lock.
🚨 Failed to create temporary system for spawning: Storage engine initialization failed
❌ Failed to spawn HTTP component: Temporary system creation failed during component spawn
❌ System method 'start' failed: HTTP component spawn failed
```

**ROOT CAUSE ANALYSIS**:
1. `init()` calls `acquire_process_lock_with_recovery()`
2. Which calls `DatabaseRecovery::test_database_access()`
3. Which calls `Database::create(DB_PATH)` and leaves connection open
4. Later `SystemComponent::new()` calls `SimpleStorage::new()`
5. Which tries `Database::create()` on same file → "already open" error
6. redb doesn't allow multiple connections to same database file

**TARGET STATE**:
- Single database connection owned by SystemComponent
- All components access database through SystemComponent storage service
- Use Unix flock for process locking instead of pidfiles
- Leverage redb's built-in locking capabilities

#### Phase 1: Database Connection Consolidation

##### [x] 1.1 Remove Duplicate Database Connections
- [x] Remove `Database::create()` call from `DatabaseRecovery::test_database_access()`
- [x] Make database recovery use file system checks only (no database opening)
- [x] Ensure only `SimpleStorage::new()` opens the database
- [x] Add database file accessibility checks without opening connection

##### [x] 1.2 Refactor Database Recovery to File-System-Only Checks
- [x] Replace `test_database_access()` with file permission/lock checks
- [x] Use `fs::metadata()` and file locking tests instead of database opening
- [x] Keep backup/recovery logic but remove database connection testing
- [x] Add Unix file lock testing as alternative to redb connection testing

#### Phase 2: Replace Pidfile System with Unix flock

##### [x] 2.1 Implement Unix flock Process Locking
- [x] Create new `ProcessLock` struct using `libc::flock()`
- [x] Implement process locking on database file directly (not separate .pid file)
- [x] Add automatic lock release on process termination (Unix guarantees this)
- [x] Handle `EWOULDBLOCK` for immediate failure detection

##### [x] 2.2 Remove Pidfile Infrastructure
- [x] Delete `LOCK_FILE` constant and all pidfile references
- [x] Remove all `.pid` file creation and checking logic
- [x] Remove PID-based process detection (`kill -0` commands)
- [x] Clean up any remaining pidfile references in codebase

##### [x] 2.3 Integrate flock with Database Recovery
- [x] Replace `acquire_process_lock_with_recovery()` with `acquire_database_flock()`
- [x] Use flock on database file for process exclusion
- [x] Add database file recovery logic triggered by successful flock acquisition
- [x] Remove all pidfile-based recovery logic

#### Phase 3: Centralize Database Access through SystemComponent

##### [x] 3.1 Make SystemComponent the Single Database Owner
- [x] Ensure SystemComponent creates the ONLY database connection
- [x] Move SimpleStorage initialization earlier in startup process
- [x] Create database connection before any component spawning
- [x] Add database connection health checks in SystemComponent

##### [x] 3.2 Enforce Database Access Only Through SystemComponent
- [x] Remove direct SimpleStorage access from components
- [x] Ensure all components use `self.system.save_component_state()` patterns
- [x] Audit codebase for any direct storage access bypassing SystemComponent
- [x] Add compile-time enforcement if possible (private storage fields)

##### [x] 3.3 Add Database Connection Sharing Architecture
- [x] Consider if multiple components need direct storage access
- [x] If yes: Create shared storage handle with connection pooling
- [x] If no: Keep storage mediated through SystemComponent only
- [x] Document the chosen pattern clearly in REQUIREMENTS.md

#### Phase 4: Leverage redb Built-in Capabilities

##### [x] 4.1 Use redb's Lock Management Features
- [x] Research redb's built-in locking and concurrency features
- [x] Replace custom lock management with redb's native mechanisms
- [x] Add proper error handling for redb lock conflicts
- [x] Document redb's guarantees about process safety

##### [x] 4.2 Optimize Database Access Patterns
- [x] Use redb read/write transactions optimally
- [x] Minimize transaction scope and duration
- [x] Add database connection reuse where beneficial
- [x] Profile database operations for performance bottlenecks

#### Phase 5: Testing and Validation

##### [x] 5.1 Test Database Lock Conflict Resolution
- [x] Create tests that simulate crashed processes (stale locks)
- [x] Test concurrent process startup attempts
- [x] Verify flock-based locking works correctly
- [x] Test database recovery scenarios

##### [x] 5.2 Test SystemComponent Database Integration
- [x] Test component spawning with single database connection
- [x] Test storage operations through SystemComponent mediation
- [x] Verify no database "already open" errors during startup
- [x] Test graceful shutdown with database connection cleanup

##### [x] 5.3 Performance and Reliability Testing
- [x] Measure database operation latency after consolidation
- [x] Test system behavior under database lock contention
- [x] Verify system startup reliability across multiple runs
- [x] Test database file corruption recovery scenarios

#### Phase 6: Documentation and Cleanup

##### [x] 6.1 Update Architecture Documentation
- [x] Update REQUIREMENTS.md with new database architecture
- [x] Document the single-connection pattern in MEMO.md
- [x] Add troubleshooting guide for database issues
- [x] Document flock-based process exclusion

##### [x] 6.2 Code Cleanup and Optimization
- [x] Remove all unused database/lock-related code
- [x] Optimize import statements and dependencies
- [x] Add comprehensive error messages for database failures
- [x] Run clippy and fix any new warnings

**ACCEPTANCE CRITERIA**:
- [x] No "Database already open" errors during startup
- [x] Single database connection owned by SystemComponent
- [x] Unix flock used for process exclusion (no pidfiles)
- [x] All components access storage through SystemComponent
- [x] redb's built-in locking capabilities leveraged
- [x] System startup succeeds consistently
- [x] Database recovery works without connection conflicts

**ESTIMATED EFFORT**: 4-6 hours (architectural refactoring with database management) ✅ **COMPLETED**

**DEPENDENCIES**: ✅ **SATISFIED**
- ✅ Uses `libc` for direct Unix flock system calls
- ✅ Leveraged redb MVCC capabilities for optimal performance
- ✅ No additional crates needed - built with existing dependencies

**RISK MITIGATION**: ✅ **COMPLETED**
- ✅ Comprehensive test coverage with temporary databases
- ✅ Extensive error logging during all operations
- ✅ Database recovery and backup systems implemented
- ✅ No rollback needed - implementation successful

---

## ⚙️ Deferred Tasks

### [ ] Refactor Start/Stop to Use Component Trait Methods Consistently

**PROBLEM**: Component trait has default start()/stop() implementations but system uses custom helper functions instead. This creates architectural inconsistency.

**STATUS**: Deferred until database consolidation is complete - start/stop methods interact with storage

**CURRENT STATE**:
- Component trait: Empty default `start()` and `stop()` methods
- System helpers: `handle_start()` and `handle_stop()` contain actual business logic
- SystemComponent: Has separate `start()` and `stop()` methods that don't use trait defaults
- Inconsistent: Some code paths use trait methods, others use helper functions

**TARGET STATE**: Use Component trait methods everywhere for consistency
- Move business logic from helpers into Component trait implementations
- SystemComponent uses Component trait start/stop methods
- All components follow the same start/stop pattern
- Helpers either call Component trait methods or are eliminated

---

## 📋 Completed Tasks

*Completed tasks will be moved to MEMO.md for reference.*

---

**Instructions for maintaining this file:**
- [ ] Always mark completed tasks with [x]
- [ ] Decompose large tasks and move them to the top
- [ ] Include specific technical details for implementation tasks
- [ ] Reference REQUIREMENTS.md and component-specific requirements
- [ ] Move completed tasks to MEMO.md with implementation details
