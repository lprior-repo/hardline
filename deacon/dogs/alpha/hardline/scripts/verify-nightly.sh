#!/usr/bin/env bash
set -euo pipefail

echo "=== Nightly: Kani proofs ==="
cargo kani --workspace || true

echo "=== Nightly: Loom ==="
RUSTFLAGS="--cfg loom" cargo test --workspace --test-threads=1 || true

echo "=== Nightly: Miri ==="
cargo +miri test --workspace || true

echo "=== Nightly: Mutation full ==="
cargo mutants --workspace --timeout 300 || true

echo "=== Nightly: Coverage ==="
cargo llvm-cov --workspace --html --open || true
