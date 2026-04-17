Kani Rust Verifier 0.67.0 (cargo plugin)
   Compiling scp-cli v0.5.0 (/home/lewis/src/hl-kb8/crates/cli)
warning: unused import: `Subcommand`
 --> crates/cli/src/cli/main.rs:7:20
  |
7 | use clap::{Parser, Subcommand};
  |                    ^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `super::json_docs::ai_contracts::work`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:12:9
   |
12 | pub use super::json_docs::ai_contracts::work;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::ai_contracts_part2::abort`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:13:9
   |
13 | pub use super::json_docs::ai_contracts_part2::abort;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::response_types::add as add_response`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:18:9
   |
18 | pub use super::json_docs::response_types::add as add_response;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::response_types::done as done_response`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:23:9
   |
23 | pub use super::json_docs::response_types::done as done_response;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::response_types::remove as remove_response`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:27:9
   |
27 | pub use super::json_docs::response_types::remove as remove_response;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::response_types::sync as sync_response`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:29:9
   |
29 | pub use super::json_docs::response_types::sync as sync_response;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `super::json_docs::system_commands::spawn as spawn_system`
  --> crates/cli/src/commands/isolate_json_docs/mod.rs:37:9
   |
37 | pub use super::json_docs::system_commands::spawn as spawn_system;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::commands::isolate_alias_handler as alias_handler`
 --> crates/cli/src/commands/isolate_mod.rs:3:9
  |
3 | pub use crate::commands::isolate_alias_handler as alias_handler;
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::commands::isolate_commands as commands`
 --> crates/cli/src/commands/isolate_mod.rs:4:9
  |
4 | pub use crate::commands::isolate_commands as commands;
  |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `legacy_commands::build_legacy_commands`
  --> crates/cli/src/commands/object_commands/mod.rs:27:9
   |
27 | pub use legacy_commands::build_legacy_commands;
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused imports: `ConfigAction`, `DoctorAction`, `SessionAction`, `StatusAction`, `TaskAction`, and `ZjjObject`
  --> crates/cli/src/commands/object_commands/mod.rs:31:17
   |
31 | pub use types::{ConfigAction, DoctorAction, SessionAction, StatusAction, TaskAction, ZjjObject};
   |                 ^^^^^^^^^^^^  ^^^^^^^^^^^^  ^^^^^^^^^^^^^  ^^^^^^^^^^^^  ^^^^^^^^^^  ^^^^^^^^^

warning: unused import: `crate::commands::workspace as ws`
 --> crates/cli/src/commands/status.rs:4:5
  |
4 | use crate::commands::workspace as ws;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `TaskId`
 --> crates/cli/src/commands/task_validation.rs:5:51
  |
5 | use crate::commands::task_types::{Assignee, Task, TaskId, TaskState};
  |                                                   ^^^^^^

warning: unused import: `error::Error`
 --> crates/cli/src/commands/task_validation.rs:7:5
  |
7 |     error::Error, error_task::TaskErrorKind, lock::LockGuard, lock::LockManager, lock::LockType,
  |     ^^^^^^^^^^^^

warning: unused import: `navigation::*`
  --> crates/cli/src/commands/workspace/mod.rs:18:9
   |
18 | pub use navigation::*;
   |         ^^^^^^^^^^^^^

warning: unused import: `operations::*`
  --> crates/cli/src/commands/workspace/mod.rs:19:9
   |
19 | pub use operations::*;
   |         ^^^^^^^^^^^^^

warning: unused import: `LockState`
 --> crates/cli/src/commands/lock_kani.rs:5:48
  |
5 |     use scp_core::coordination::locks::types::{LockState, LockResponse};
  |                                                ^^^^^^^^^

error[E0277]: the trait bound `std::string::String: kani::Arbitrary` is not satisfied
 --> crates/cli/src/commands/lock_kani.rs:9:31
  |
9 |         let session: String = kani::any();
  |                               ^^^^^^^^^^^ the trait `kani::Arbitrary` is not implemented for `std::string::String`
  |
  = help: the following other types implement trait `kani::Arbitrary`:
            ()
            (A, B)
            (A, B, C)
            (A, B, C, D)
            (A, B, C, D, E)
            (A, B, C, D, E, F)
            (A, B, C, D, E, F, G)
            (A, B, C, D, E, F, G, H)
          and 51 others
note: required by a bound in `kani::any`
 --> /home/runner/work/kani/kani/library/kani/src/lib.rs:57:0
  = note: this error originates in the macro `kani_core::kani_intrinsics` which comes from the expansion of the macro `kani_core::kani_lib` (in Nightly builds, run with -Z macro-backtrace for more info)

error[E0277]: the trait bound `std::string::String: kani::Arbitrary` is not satisfied
  --> crates/cli/src/commands/lock_kani.rs:10:29
   |
10 |         let agent: String = kani::any();
   |                             ^^^^^^^^^^^ the trait `kani::Arbitrary` is not implemented for `std::string::String`
   |
   = help: the following other types implement trait `kani::Arbitrary`:
             ()
             (A, B)
             (A, B, C)
             (A, B, C, D)
             (A, B, C, D, E)
             (A, B, C, D, E, F)
             (A, B, C, D, E, F, G)
             (A, B, C, D, E, F, G, H)
           and 51 others
note: required by a bound in `kani::any`
  --> /home/runner/work/kani/kani/library/kani/src/lib.rs:57:0
   = note: this error originates in the macro `kani_core::kani_intrinsics` which comes from the expansion of the macro `kani_core::kani_lib` (in Nightly builds, run with -Z macro-backtrace for more info)

error[E0063]: missing fields `acquired_at` and `lock_id` in initializer of `scp_core::coordination::LockResponse`
  --> crates/cli/src/commands/lock_kani.rs:17:19
   |
17 |         let res = LockResponse {
   |                   ^^^^^^^^^^^^ missing `acquired_at` and `lock_id`

warning: unused variable: `config_result`
  --> crates/cli/src/commands/doctor.rs:47:9
   |
47 |     let config_result = check_config_result.as_ref().copied().unwrap_or(false);
   |         ^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_config_result`
   |
   = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `workspaces_count`
  --> crates/cli/src/commands/doctor.rs:48:9
   |
48 |     let workspaces_count = check_workspaces_result.as_ref().copied().unwrap_or(0);
   |         ^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_workspaces_count`

warning: unused variable: `force`
   --> crates/cli/src/commands/session.rs:133:27
    |
133 | pub fn remove(name: &str, force: bool, merge: bool) -> Result<()> {
    |                           ^^^^^ help: if this is intentional, prefix it with an underscore: `_force`

warning: unused variable: `name`
  --> crates/cli/src/commands/workspace/lifecycle.rs:89:13
   |
89 | pub fn sync(name: Option<&str>, all: bool) -> Result<(), Error> {
   |             ^^^^ help: if this is intentional, prefix it with an underscore: `_name`

Some errors have detailed explanations: E0063, E0277.
For more information about an error, try `rustc --explain E0063`.
error: could not compile `scp-cli` (bin "scp-cli") due to 3 previous errors; 22 warnings emitted
error: Failed to execute cargo (exit status: 101). Found 3 compilation errors.
