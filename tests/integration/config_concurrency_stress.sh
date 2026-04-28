#!/bin/bash
# Config Concurrency Stress Test
# Tests for concurrent write data loss bug (isolate-16ks)

set -euo pipefail

echo "=== Config Concurrency Stress Test ==="

# Get the project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

# Set up temporary directory
TEMP_DIR=$(mktemp -d)
export HOME="$TEMP_DIR"
CONFIG="$TEMP_DIR/.isolate/config.toml"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

echo "Test 1: 20 parallel writes (reproduces reported bug)"
for i in {1..20}; do
  cargo run --quiet -- config test_key_$i "value_$i" >/dev/null 2>&1 &
done
wait

# Count keys in the config file
if [ -f "$CONFIG" ]; then
    # Extract unique key names from the config file
    KEY_COUNT=$(grep -o '^test_key_[0-9]\+' "$CONFIG" 2>/dev/null | wc -l || echo "0")
else
    KEY_COUNT=0
fi

if [ "$KEY_COUNT" -eq 20 ]; then
    echo "PASS: All 20 keys preserved (0% data loss)"
else
    echo "FAIL: Expected 20 keys, got $KEY_COUNT"
    echo "Data loss: $((100 - KEY_COUNT * 100 / 20))%"
    cat "$CONFIG" 2>/dev/null || true
    exit 1
fi

echo ""
echo "Test 2: 100 parallel writes (stress test)"
rm -f "$CONFIG"

for i in {1..100}; do
  cargo run --quiet -- config stress_key_$i "value_$i" >/dev/null 2>&1 &
done
wait

if [ -f "$CONFIG" ]; then
    KEY_COUNT=$(grep -o '^stress_key_[0-9]\+' "$CONFIG" 2>/dev/null | wc -l || echo "0")
else
    KEY_COUNT=0
fi

if [ "$KEY_COUNT" -eq 100 ]; then
    echo "PASS: All 100 keys preserved (0% data loss)"
else
    echo "FAIL: Expected 100 keys, got $KEY_COUNT"
    DATA_LOSS=$((100 - KEY_COUNT))
    echo "Data loss: $DATA_LOSS%"
    exit 1
fi

echo ""
echo "=== All concurrency tests passed ==="
