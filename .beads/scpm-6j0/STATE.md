STATE 8: Landing

Compilation: PASSED (cargo build --package scp-cli)
Tests: PASSED (cargo test --package scp-cli: 73 tests)
Unit tests for batch: 8 tests passing

Pre-existing clippy issues in scp-core (104 errors) - not related to this implementation.

Artifacts created:
- contract.md: Contract specification
- martin-fowler-tests.md: Test plan
- implementation.md: Implementation summary

Files changed:
- crates/cli/src/commands/handlers/batch.rs (new implementation)
- crates/cli/src/commands/batch.rs (new command module)
- crates/cli/src/commands/mod.rs (added batch)
- crates/cli/src/main.rs (added Batch command)
- crates/cli/Cargo.toml (added dependencies)

Next: jj rebase, jj push, bd close
