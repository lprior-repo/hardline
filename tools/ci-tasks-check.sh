#!/usr/bin/env bash
# CI Documentation Checker - Moon Task Validation
# Validates that documented moon tasks exist in .moon/tasks.yml
# Returns: 0 if all documented tasks exist, 1 otherwise

set -euo pipefail

cd /home/lewis/isolate

# Check if TASKS_FILE is set, otherwise use default
if [[ -z "${TASKS_FILE:-}" ]]; then
	TASKS_FILE="/home/lewis/isolate/.moon/tasks.yml"
fi

# Track failures
declare -a MISSING_TASKS=()
declare -a TASK_MISMATCHES=()
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

get_actual_tasks() {
	if [[ ! -f "$TASKS_FILE" ]]; then
		log_error "Tasks file not found: $TASKS_FILE"
		exit 1
	fi

	grep -E '^\s{2}[a-z][a-z-]+:$' "$TASKS_FILE" 2>/dev/null |
		sed -E 's/^\s+([a-z][a-z-]+):.*/\1/' |
		sort -u || true
}

validate_task() {
	local task="$1"

	TOTAL_DOCUMENTED=$((TOTAL_DOCUMENTED + 1))

	local actual_tasks
	actual_tasks=$(get_actual_tasks)

	if echo "$actual_tasks" | grep -q "^${task}$"; then
		TOTAL_FOUND=$((TOTAL_FOUND + 1))
		log_success "Task ':$task' exists in .moon/tasks.yml"

		validate_task_config "$task"

		return 0
	else
		log_error "Task ':$task' not found in .moon/tasks.yml"
		MISSING_TASKS+=("$task")
		TOTAL_ERRORS=$((TOTAL_ERRORS + 1))
	fi
}

validate_task_config() {
	local task="$1"

	local has_command
	has_command=$(grep -A 2 "^  $task:" "$TASKS_FILE" | grep -c 'command:' || true)

	if [[ "$has_command" -gt 0 ]]; then
		log_success "  - Has command definition"
	else
		log_warn "  - No command definition found"
	fi

	local has_description
	has_description=$(grep -A 2 "^  $task:" "$TASKS_FILE" | grep -c 'description:' || true)

	if [[ "$has_description" -gt 0 ]]; then
		log_success "  - Has description"
	else
		log_warn "  - Missing description"
	fi

	local in_ci
	in_ci=$(grep -A 20 "^  $task:" "$TASKS_FILE" | grep -c 'runInCI: true' || true)

	if [[ "$in_ci" -gt 0 ]]; then
		log_success "  - Configured for CI"
	else
		log_warn "  - Not configured for CI (runInCI: false or missing)"
		TASK_MISMATCHES+=("$task (no CI)")
	fi
}

get_documented_tasks() {
	local temp_file="/tmp/moon_tasks_$$"

	grep -rhoE 'moon run :[a-z][a-z-]*' /home/lewis/isolate/docs 2>/dev/null |
		sed 's/moon run ://' |
		sort -u >"$temp_file" || true

	echo "Found $(wc -l <"$temp_file") documented tasks" >&2
	cat "$temp_file"
}

validate_all_tasks() {
	local documented
	documented=$(get_documented_tasks)

	if [[ -z "$documented" ]]; then
		log_warn "No moon tasks found in documentation"
		return 0
	fi

	while IFS= read -r task; do
		if [[ -n "$task" ]]; then
			validate_task "$task" || true
		fi
	done <<<"$documented"
}

generate_report() {
	local report_file="${1:-/tmp/ci-tasks-check-report.json}"
	local missing=""
	local mismatches=""

	if [[ ${#MISSING_TASKS[@]} -gt 0 ]]; then
		missing=$(printf '"%s",' "${MISSING_TASKS[@]}" | sed 's/,$//')
	fi
	if [[ ${#TASK_MISMATCHES[@]} -gt 0 ]]; then
		mismatches=$(printf '"%s",' "${TASK_MISMATCHES[@]}" | sed 's/,$//')
	fi

	cat >"$report_file" <<EOF
{
  "timestamp": "$(date -Iseconds)",
  "tasks_file": "$TASKS_FILE",
  "summary": {
    "documented": $TOTAL_DOCUMENTED,
    "found": $TOTAL_FOUND,
    "missing": $((TOTAL_DOCUMENTED - TOTAL_FOUND)),
    "config_issues": ${#TASK_MISMATCHES[@]},
    "total_errors": $TOTAL_ERRORS
  },
  "missing_tasks": [$missing],
  "config_issues": [$mismatches]
}
EOF
	echo "Report written to $report_file"
}

exit_code=0

main() {
	echo "=========================================="
	echo "  Moon Task Documentation Validation"
	echo "=========================================="
	echo

	if [[ ! -f "$TASKS_FILE" ]]; then
		log_error "Tasks file not found: $TASKS_FILE"
		log_error "Expected location: /home/lewis/isolate/.moon/tasks.yml"
		exit 1
	fi

	validate_all_tasks

	echo
	echo "=========================================="
	echo "  Validation Summary"
	echo "=========================================="
	echo "Documented tasks:    $TOTAL_DOCUMENTED"
	echo "Found tasks:         $TOTAL_FOUND"
	echo "Missing tasks:       $((TOTAL_DOCUMENTED - TOTAL_FOUND))"
	echo "Config issues:       ${#TASK_MISMATCHES[@]}"
	echo "Total errors:        $TOTAL_ERRORS"
	echo

	if [[ $TOTAL_ERRORS -gt 0 ]]; then
		log_error "Task documentation validation FAILED"
		if [[ ${#MISSING_TASKS[@]} -gt 0 ]]; then
			echo "Missing tasks:"
			for task in "${MISSING_TASKS[@]}"; do
				echo "  - :$task"
			done
		fi
		if [[ ${#TASK_MISMATCHES[@]} -gt 0 ]]; then
			echo "Configuration issues:"
			for issue in "${TASK_MISMATCHES[@]}"; do
				echo "  - $issue"
			done
		fi
		generate_report
		exit_code=1
	else
		log_success "All documented tasks validated successfully"
		generate_report
		exit_code=0
	fi
}

main "$@"
exit $exit_code
