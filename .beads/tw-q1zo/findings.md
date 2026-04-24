# tw-q1zo Findings: Validate branch names against git refname rules

## Vulnerability

**ADR-014/012**: Branch names passed directly to `git checkout`/`git rebase` without validation.

### Root Cause

Two issues found:

1. **Serde bypass vulnerability**: `BranchName` derived `Deserialize` directly, allowing `serde_json::from_str::<BranchName>("\"--upload-pack=evil\"")` to bypass `BranchName::new()` validation entirely. Any deserialized JSON could inject malicious branch names.

2. **Missing `--` separator**: `sync.rs` passed branch names to `git checkout` and `git rebase` without the `--` end-of-options separator. While `BranchName::new()` rejects leading hyphens, a serde-bypassed name could be interpreted as a git flag.

### Fix Applied

**File 1: `crates/vcs/src/vcs/types/branch.rs`**
- Removed `#[derive(Serialize, Deserialize)]` from `BranchName`
- Implemented custom `Serialize` (simple passthrough) and `Deserialize` (calls `BranchName::new()` for validation)
- Added 4 serde bypass rejection tests (`--upload-pack=evil`, `-foo`, empty, `main~1`)

**File 2: `crates/vcs/src/vcs/git/sync.rs`**
- Added `--` separator before branch name arguments in `git checkout` and `git rebase` commands
- Updated error message strings to reflect `--` in commands

### Verification

- `cargo check -p scp-vcs`: PASS
- `cargo nextest run -p scp-vcs`: 323 tests PASS (0 failures)
- `cargo clippy -p scp-vcs -- -D warnings`: PASS (exit 0)
- `cargo fmt -p scp-vcs -- --check`: PASS (no diffs in changed files)

### Defense-in-Depth Layers

| Layer | Protection |
|-------|-----------|
| `BranchName::new()` | Validates at construction (empty, syntax, invisible chars) |
| Custom `Deserialize` | Prevents serde bypass of validation |
| `--` separator | Prevents flag injection even if validation bypassed |
| `Command::args()` | Rust's `Command` does NOT shell-expand (no `$(...)` injection) |
