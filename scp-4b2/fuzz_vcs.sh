#!/bin/bash

set -e

# Build the CLI
cargo build -p scp-cli

# Setup fuzzing directory
FUZZ_DIR="/tmp/scp-fuzz-test"
rm -rf "$FUZZ_DIR"
mkdir -p "$FUZZ_DIR"
cd "$FUZZ_DIR"

git init

git config user.email "fuzzer@example.com"
git config user.name "Fuzz Tester"

# Helper to run scp-cli
SCP_CLI="/home/lewis/src/scp/target/debug/scp-cli"

echo "=== Testing standard commit ==="
touch "normal_file.txt"
git add .
git commit -m "Initial commit"
$SCP_CLI status

echo "=== Payload 1: File name with shell metacharacters ==="
touch 'normal_file_$(touch /tmp/scp-pwned-1).txt'
touch '"; touch /tmp/scp-pwned-2; "'
touch '-la'
git add .
git commit -m "Malicious file names"
$SCP_CLI status 2>&1 | tee status_out_1.log

echo "=== Payload 2: Author name with traversal & XSS ==="
git config user.name "../../../../../../../../../../etc/passwd <script>alert(1)</script>"
touch another.txt
git add .
git commit -m "Malicious author"
$SCP_CLI status 2>&1 | tee status_out_2.log

echo "=== Payload 3: Commit message with shell injection & SQLi ==="
touch another2.txt
git add .
git commit -m "Commit \$(touch /tmp/scp-pwned-3) \`; DROP TABLE users; -- <svg/onload=alert(1)>"
$SCP_CLI status 2>&1 | tee status_out_3.log

echo "=== Payload 4: Branch name with metacharacters ==="
# Git prevents some branch names but allows many things. Let's try some weird ones
git checkout -b 'feature/../xss-<script>' 2>/dev/null || echo "Git rejected branch name 1"
git checkout -b 'feature/shell-$()' 2>/dev/null || echo "Git rejected branch name 2"
# legitimate but weird branch name
git checkout -b 'feature/name-with-;' 2>/dev/null || echo "Git rejected branch name 3"
$SCP_CLI status 2>&1 | tee status_out_4.log

echo "=== Checking for executed payloads ==="
ls -la /tmp/scp-pwned-* 2>/dev/null || echo "No shell injections executed (which is good!)"

echo "=== Checking for panics ==="
grep -i "panic" *.log || echo "No panics found."

echo "Done."
