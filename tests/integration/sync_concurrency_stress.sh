#!/bin/bash
set -euo pipefail

# Reproduce JJ repository corruption (object not found)
# Scenario: Heavy concurrent sync/commit load

# Find project root
PROJECT_ROOT="$(git rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"

TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "=== Setting up test repo in $TEMP_DIR ==="

# Define isolate binary location
ISOLATE_CMD="cargo run --manifest-path ${PROJECT_ROOT}/crates/isolate/Cargo.toml --quiet --"

# 1. Initialize git/jj/isolate
git init .
jj git init --colocate
mkdir -p .isolate
$ISOLATE_CMD init

# 2. Create some history on main
touch README.md
git add README.md
git commit -m "Initial commit"
jj git import

# 3. Create two workspaces
$ISOLATE_CMD add ws-a --no-open --no-zellij
$ISOLATE_CMD add ws-b --no-open --no-zellij

# 4. Stress test: Concurrent modifications and syncs
echo "=== Starting stress test ==="

pids=""

# Worker A: Commits to main continuously
(
	for i in {1..20}; do
		echo "main commit $i" >>README.md
		git add README.md
		git commit -m "main update $i"
		jj git import
		sleep 0.1
	done
) &
pids="$pids $!"

# Worker B: Syncs ws-a continuously
(
	REPO_NAME=$(basename "$TEMP_DIR")
	cd "../${REPO_NAME}__workspaces/ws-a"
	for i in {1..20}; do
		echo "ws-a change $i" >file-a
		jj new
		jj commit -m "ws-a commit $i"
		# The critical operation: sync
		if ! $ISOLATE_CMD sync; then
			echo "FAIL: isolate sync failed in ws-a"
			exit 1
		fi
		sleep 0.2
	done
) &
pids="$pids $!"

# Worker C: Syncs ws-b continuously
(
	REPO_NAME=$(basename "$TEMP_DIR")
	cd "../${REPO_NAME}__workspaces/ws-b"
	for i in {1..20}; do
		echo "ws-b change $i" >file-b
		jj new
		jj commit -m "ws-b commit $i"
		# The critical operation: sync
		if ! $ISOLATE_CMD sync; then
			echo "FAIL: isolate sync failed in ws-b"
			exit 1
		fi
		sleep 0.3
	done
) &
pids="$pids $!"

# Worker B: Syncs ws-a continuously
(
	REPO_NAME=$(basename "$TEMP_DIR")
	cd "../${REPO_NAME}__workspaces/ws-a"
	for i in {1..20}; do
		echo "ws-a change $i" >file-a
		jj new
		jj commit -m "ws-a commit $i"
		# The critical operation: sync
		if ! isolate sync; then
			echo "FAIL: isolate sync failed in ws-a"
			exit 1
		fi
		sleep 0.2
	done
) &
pids="$pids $!"

# Worker C: Syncs ws-b continuously
(
	REPO_NAME=$(basename "$TEMP_DIR")
	cd "../${REPO_NAME}__workspaces/ws-b"
	for i in {1..20}; do
		echo "ws-b change $i" >file-b
		jj new
		jj commit -m "ws-b commit $i"
		# The critical operation: sync
		if ! isolate sync; then
			echo "FAIL: isolate sync failed in ws-b"
			exit 1
		fi
		sleep 0.3
	done
) &
pids="$pids $!"

# Wait for all
for pid in $pids; do
	wait $pid || {
		echo "Test FAILED"
		exit 1
	}
done

echo "=== Test PASSED ==="
