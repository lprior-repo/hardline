# COMPLETE

Bead scp-4t5: cli: Add task management commands

## Summary
- Implemented 6 task management commands: list, show, claim, yield, start, done
- Added TTL locking support via LockType::Task
- Created in-memory task store for demo purposes
- All preconditions (P1-P5) and postconditions (Q1-Q7) enforced

## Files Created
- .beads/scp-4t5/contract.md
- .beads/scp-4t5/martin-fowler-tests.md
- .beads/scp-4t5/implementation.md
- crates/cli/src/commands/task.rs

## Files Modified
- crates/cli/src/main.rs (TaskCommands enum)
- crates/cli/src/commands/mod.rs (task module)
- crates/cli/Cargo.toml (dependencies)
- crates/core/src/error.rs (error variants)
- crates/core/src/lock.rs (LockType::Task)

## Testing
- CLI compiles successfully
- Task commands work correctly:
  - `scp task list` - lists tasks
  - `scp task show <id>` - shows task details
  - `scp task claim <id>` - claims task
  - `scp task yield <id>` - yields task
  - `scp task start <id>` - starts task
  - `scp task done <id>` - completes task
