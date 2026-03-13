# Architecture Refactor Report

## Summary
Refactored `/home/lewis/src/scp/crates/core/src/infrastructure/database.rs` to enforce:
1. **<300 line limit per file**: Split into two focused modules
2. **Scott Wlaschin DDD**: Applied newtypes to eliminate primitive obsession

## Changes Made

### 1. File Split
- **database.rs**: 248 lines (was 216, added DDD types + query method)
- **database_types.rs**: 92 lines (NEW - extracted domain types)

### 2. DDD Refactoring Applied

#### Newtypes Added (database_types.rs)
| Type | Validation | Makes Illegal States Unrepresentable |
|------|------------|--------------------------------------|
| `DatabasePath` | Rejects empty strings | Empty path cannot be constructed |
| `MaxConnections` | Rejects zero | Zero connections cannot be constructed |

#### Updated Types (database.rs)
- `DatabaseConfig.path`: Now `DatabasePath` (was `String`)
- `DatabaseConfig.max_connections`: Now `MaxConnections` (was `u32`)
- Added `DatabaseService::query()` method for row retrieval

### 3. DDD Principles Applied
- **Parse, don't validate**: Invalid configs fail at construction time
- **Types as spec**: Function signatures document constraints
- **No primitive obsession**: Domain concepts have semantic wrappers

## Files Modified
- `crates/core/src/infrastructure/database.rs` - Main implementation
- `crates/core/src/infrastructure/database_types.rs` - NEW: Domain types
- `crates/core/src/infrastructure/mod.rs` - Updated exports

## Tests
All 8 database tests pass.
