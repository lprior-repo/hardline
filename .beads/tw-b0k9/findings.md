# tw-b0k9: Move receipt storage to async with tokio::fs

## Changes Made

**File:** `crates/receipt/src/storage/receipt_store.rs`

### What changed
- `std::fs::create_dir_all` → `tokio::fs::create_dir_all`
- `std::fs::write` → `tokio::fs::write`
- `std::fs::read_to_string` → `tokio::fs::read_to_string`
- `std::fs::read_dir` → `tokio::fs::read_dir` (with async `next_entry()` loop)
- `dir.exists()` sync check replaced with `NotFound` error kind matching on `read_dir`
- All 5 I/O methods made `async`: `save`, `load`, `list_op_ids`, `latest_op_id`, `load_latest`
- Tests converted from `#[test]` to `#[tokio::test]` with `.await` on store calls
- `new()` and `default()` remain sync (no I/O)

### What did NOT change
- `crates/snapshot/src/storage/receipt_store.rs` has its own copy with `SnapshotError` types — out of scope per bead description
- No callers of the receipt crate's `ReceiptStore` were found in other crates (it's only used in tests)

### Verification
- `cargo check -p scp-receipt` passes
- `cargo test -p scp-receipt` — 39 tests pass
