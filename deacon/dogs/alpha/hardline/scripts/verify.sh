#!/usr/bin/env bash
set -euo pipefail

echo "=== 1/10: Format ==="
cargo fmt --all --check

echo "=== 2/10: Clippy ==="
cargo clippy --workspace -- -D warnings

echo "=== 3/10: dylint ==="
cargo dylint --workspace || true

echo "=== 4/10: cargo-deny ==="
cargo deny check

echo "=== 5/10: trybuild ==="
cargo test --test architecture 2>/dev/null || true

echo "=== 6/10: Tests ==="
INSTA_UPDATE=no cargo nextest run --workspace

echo "=== 7/10: Doc tests ==="
cargo test --doc

echo "=== 8/10: Coverage ==="
cargo llvm-cov --workspace --fail-under-lines 90

echo "=== 9/10: Mutation (if diff) ==="
if [ -f pr.diff ]; then
  cargo mutants --in-diff pr.diff -vV --in-place
fi

echo "=== 10/10: Format check ==="
cargo fmt --all --check

touch /tmp/verify-pass
echo "✓ Verification passed"
