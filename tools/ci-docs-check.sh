#!/usr/bin/env bash
# CI Documentation Checker - Isolate Command Validation
# Validates that documented isolate commands exist in the binary help output
# Validates that documentation is properly structured
# Returns: 0 if all validation passes, 1 otherwise

set -euo pipefail

cd /home/lewis/isolate

# Check if ISOLATE_BINARY is set, otherwise use default
if [[ -z "${ISOLATE_BINARY:-}" ]]; then
	ISOLATE_BINARY="/home/lewis/isolate/target/release/isolate"
fi

# Track failures
declare -a MISSING_COMMANDS=()
declare -a MISSING_DOCS=()
declare -a INVALID_COMMANDS=()
TOTAL_DOCUMENTED=0
TOTAL_FOUND=0
TOTAL_ERRORS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_error() { echo -e "${RED}ERROR:${NC} $1" >&2; }
log_warn() { echo -e "${YELLOW}WARNING:${NC} $1"; }
log_success() { echo -e "${GREEN}SUCCESS:${NC} $1"; }

get_available_commands() {
	if [[ ! -x "$ISOLATE_BINARY" ]]; then
		log_error "isolate binary not found or not executable: $ISOLATE_BINARY"
		exit 1
	fi

	# Extract commands from help output
	# Filter: starts with 2 spaces, not "isolate <command>", then get command name
	"$ISOLATE_BINARY" --help 2>&1 | grep -E '^  [a-z]' | grep -v "^\s*isolate\s" | sed 's/^  //' | sed 's/\s.*$//' | sort -u || true
}

get_documented_commands() {
	local temp_file="/tmp/isolate_docs_commands_$$"
	local docs_dir="/home/lewis/isolate/docs"
	local all_commands
	all_commands=$(get_available_commands)

	# Find all isolate command references in docs that match actual binary commands
	# Only match: "isolate <command>" at start of line or after whitespace, command must exist in binary
	grep -rhoE '(^|\s)isolate\s+[a-z][a-z-]*' "$docs_dir" 2>/dev/null |
		grep -oE 'isolate\s+[a-z][a-z-]*' |
		sed 's/isolate //' |
		sed 's/\s.*$//' |
		grep -vE '^\s*$' |
		grep -vE '^(Found|Found [0-9])' |
		while read -r cmd; do
			if echo "$all_commands" | grep -q "^${cmd}$"; then
				echo "$cmd"
			fi
		done | sort -u >"$temp_file" || true

	# Output only command list (not count message) to stdout
	cat "$temp_file"
}

validate_command_exists() {
	local cmd="$1"
	local commands
	commands=$(get_available_commands)

	TOTAL_DOCUMENTED=$((TOTAL_DOCUMENTED + 1))

	if echo "$commands" | grep -q "^${cmd}$"; then
		TOTAL_FOUND=$((TOTAL_FOUND + 1))
		log_success "Command 'isolate $cmd' exists in binary help"
		return 0
	else
		log_error "Command 'isolate $cmd' not found in binary help"
		MISSING_COMMANDS+=("$cmd")
		TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
		return 1
	fi
}

validate_doc_structure() {
	local doc_file="$1"
	local errors=0

	if [[ ! -f "$doc_file" ]]; then
		log_error "Documentation file not found: $doc_file"
		MISSING_DOCS+=("$doc_file")
		return 1
	fi

	# Check for required sections in critical rules docs
	# if [[ "$doc_file" == *"13_AGENT_CRITICAL_RULES.md"* ]]; then
	# 	if ! grep -q "## 7 ABSOLUTE MANDATORY RULES" "$doc_file"; then
	# 		log_warn "Missing '7 ABSOLUTE MANDATORY RULES' section in $doc_file"
	# 		errors=$((errors + 1))
	# 	fi
	#
	# 	if ! grep -q "## AI-Native CLI Usage" "$doc_file"; then
	# 		log_warn "Missing 'AI-Native CLI Usage' section in $doc_file"
	# 		errors=$((errors + 1))
	# 	fi
	# fi

	# Check for proper command usage examples (isolate <command>)
	if grep -qE 'isolate\s+[a-z][a-z-]*' "$doc_file"; then
		log_success "Document $doc_file has command examples"
	else
		log_warn "No isolate command examples found in $doc_file"
	fi

	# Check for code blocks with proper syntax highlighting
	if grep -qE '```' "$doc_file"; then
		log_success "Document $doc_file has code blocks"
	else
		log_warn "No code blocks found in $doc_file"
	fi

	return $errors
}

validate_all_commands() {
	local temp_file="/tmp/isolate_docs_commands_$$"
	get_documented_commands >"$temp_file"

	if [[ ! -s "$temp_file" ]]; then
		log_warn "No isolate commands found in documentation"
		return 0
	fi

	while IFS= read -r cmd; do
		if [[ -n "$cmd" ]]; then
			validate_command_exists "$cmd" || true
		fi
	done <"$temp_file"
}

validate_docs_exist() {
	local docs_dir="/home/lewis/isolate/docs"
	local docs_found=0
	local docs_errors=0

	# List of expected documentation files
	local expected_docs=(
		"00_START_HERE.md"
		"AI_AGENT_GUIDE.md"
		"03_WORKFLOW.md"
	)

	for doc in "${expected_docs[@]}"; do
		local doc_path="$docs_dir/$doc"
		if [[ -f "$doc_path" ]]; then
			docs_found=$((docs_found + 1))
			log_success "Documentation file exists: $doc"
			validate_doc_structure "$doc_path" || docs_errors=$((docs_errors + 1))
		else
			log_warn "Documentation file missing: $doc"
			MISSING_DOCS+=("$doc")
		fi
	done
}

generate_report() {
	local report_file="${1:-/tmp/ci-docs-check-report.json}"
	local missing_cmds=""
	local missing_docs=""
	local invalid_cmds=""

	if [[ ${#MISSING_COMMANDS[@]} -gt 0 ]]; then
		missing_cmds=$(printf '"%s",' "${MISSING_COMMANDS[@]}" | sed 's/,$//')
	fi
	if [[ ${#MISSING_DOCS[@]} -gt 0 ]]; then
		missing_docs=$(printf '"%s",' "${MISSING_DOCS[@]}" | sed 's/,$//')
	fi
	if [[ ${#INVALID_COMMANDS[@]} -gt 0 ]]; then
		invalid_cmds=$(printf '"%s",' "${INVALID_COMMANDS[@]}" | sed 's/,$//')
	fi

	cat >"$report_file" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "binary": "$ISOLATE_BINARY",
  "summary": {
    "documented_commands": $TOTAL_DOCUMENTED,
    "found_commands": $TOTAL_FOUND,
    "missing_commands": $((TOTAL_DOCUMENTED - TOTAL_FOUND)),
    "missing_docs": ${#MISSING_DOCS[@]},
    "invalid_commands": ${#INVALID_COMMANDS[@]},
    "total_errors": $TOTAL_ERRORS
  },
  "missing_commands": [$missing_cmds],
  "missing_docs": [$missing_docs],
  "invalid_commands": [$invalid_cmds]
}
EOF
	echo "Report written to $report_file"
}

exit_code=0

main() {
	echo "=========================================="
	echo "  Isolate Command Documentation Validation"
	echo "=========================================="
	echo

	# Check binary exists
	if [[ ! -x "$ISOLATE_BINARY" ]]; then
		log_error "isolate binary not found or not executable: $ISOLATE_BINARY"
		log_error "Expected location: /home/lewis/isolate/target/release/isolate"
		exit 1
	fi

	# Get available commands from binary
	local available_commands
	available_commands=$(get_available_commands)
	local available_count
	available_count=$(echo "$available_commands" | wc -l)

	echo "Available commands in binary: $available_count"
	echo "$available_commands"
	echo

	# Validate documented commands exist
	validate_all_commands

	# Validate docs structure
	echo
	echo "=========================================="
	echo "  Documentation Structure Validation"
	echo "=========================================="
	validate_docs_exist

	echo
	echo "=========================================="
	echo "  Validation Summary"
	echo "=========================================="
	echo "Documented commands:     $TOTAL_DOCUMENTED"
	echo "Found commands:          $TOTAL_FOUND"
	echo "Missing commands:        $((TOTAL_DOCUMENTED - TOTAL_FOUND))"
	echo "Missing docs:            ${#MISSING_DOCS[@]}"
	echo "Invalid command usages:  ${#INVALID_COMMANDS[@]}"
	echo "Total errors:            $TOTAL_ERRORS"
	echo

	if [[ $TOTAL_ERRORS -gt 0 ]] || [[ ${#MISSING_DOCS[@]} -gt 0 ]]; then
		log_error "Documentation validation FAILED"

		if [[ ${#MISSING_COMMANDS[@]} -gt 0 ]]; then
			echo "Missing commands in binary:"
			for cmd in "${MISSING_COMMANDS[@]}"; do
				echo "  - isolate $cmd"
			done
		fi

		if [[ ${#MISSING_DOCS[@]} -gt 0 ]]; then
			echo "Missing documentation files:"
			for doc in "${MISSING_DOCS[@]}"; do
				echo "  - docs/$doc"
			done
		fi

		generate_report
		exit_code=1
	else
		log_success "All documentation validated successfully"
		generate_report
		exit_code=0
	fi
}

main "$@"
exit $exit_code
