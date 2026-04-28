#!/bin/bash
#
# validate-schema.sh - Validate YAML spec files against the JSON schema
#
# This script validates YAML specification files against the spec.schema.json
# It supports multiple validators: check-jsonschema, ajv-cli, or yamllint + basic JSON validation
#
# Usage:
#   ./validate-schema.sh <yaml-file>        # Validate a single file
#   ./validate-schema.sh <directory>        # Validate all YAML files in directory
#   ./validate-schema.sh --check            # Check available validators
#
# Exit codes:
#   0 - Validation passed
#   1 - Validation failed
#   2 - No validator available or other error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCHEMA_DIR="$(dirname "$SCRIPT_DIR")/schema"
SCHEMA_FILE="$SCHEMA_DIR/spec.schema.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print error message
error() {
	echo -e "${RED}ERROR:${NC} $*" >&2
}

# Print success message
success() {
	echo -e "${GREEN}SUCCESS:${NC} $*"
}

# Print warning message
warning() {
	echo -e "${YELLOW}WARNING:${NC} $*"
}

# Check if a command exists
command_exists() {
	command -v "$1" >/dev/null 2>&1
}

# Check available validators
check_validators() {
	echo "Checking available validators..."

	if command_exists check-jsonschema; then
		echo "  - check-jsonschema: available"
		echo "VALIDATOR=check-jsonschema"
		return 0
	fi

	if command_exists ajv; then
		echo "  - ajv: available"
		echo "VALIDATOR=ajv"
		return 0
	fi

	if command_exists yq && command_exists python3; then
		echo "  - yq + python3: available (fallback)"
		echo "VALIDATOR=fallback"
		return 0
	fi

	warning "No suitable validator found. Install one of:"
	error "  - check-jsonschema: pip install check-jsonschema"
	error "  - ajv-cli: npm install -g ajv-cli"
	return 1
}

# Validate using check-jsonschema
validate_check_jsonschema() {
	local yaml_file="$1"
	check-jsonschema --schema-path "$SCHEMA_FILE" "$yaml_file"
}

# Validate using ajv-cli
validate_ajv() {
	local yaml_file="$1"
	local json_file
	json_file=$(mktemp)
	trap "rm -f $json_file" EXIT

	# Convert YAML to JSON using yq
	if command_exists yq; then
		yq eval -o json "$yaml_file" >"$json_file"
	else
		error "yq is required for ajv validation"
		return 1
	fi

	# Validate JSON against schema
	ajv validate -s "$SCHEMA_FILE" -d "$json_file"
}

# Validate using fallback (yq + python3)
validate_fallback() {
	local yaml_file="$1"
	local json_file
	json_file=$(mktemp)
	trap "rm -f $json_file" EXIT

	# Convert YAML to JSON using yq
	if command_exists yq; then
		yq eval -o json "$yaml_file" >"$json_file"
	else
		error "yq is required for fallback validation"
		return 1
	fi

	# Validate using Python's jsonschema
	python3 -c "
import json
import sys
try:
    import jsonschema
except ImportError:
    print('ERROR: Python jsonschema module not installed. Install with: pip install jsonschema', file=sys.stderr)
    sys.exit(1)

with open('$SCHEMA_FILE', 'r') as f:
    schema = json.load(f)

with open('$json_file', 'r') as f:
    spec = json.load(f)

try:
    jsonschema.validate(spec, schema)
except jsonschema.ValidationError as e:
    print(f'Validation error: {e.message}', file=sys.stderr)
    print(f'Path: {list(e.absolute_path)}', file=sys.stderr)
    sys.exit(1)
except Exception as e:
    print(f'Error: {e}', file=sys.stderr)
    sys.exit(1)

# Check for duplicate behavior IDs
if 'behaviors' in spec and spec['behaviors']:
    behavior_ids = [b.get('id') for b in spec['behaviors'] if 'id' in b]
    if len(behavior_ids) != len(set(behavior_ids)):
        seen = set()
        for b in spec['behaviors']:
            if 'id' in b:
                if b['id'] in seen:
                    print(f\"Validation error: Duplicate behavior ID '{b['id']}' found\", file=sys.stderr)
                    sys.exit(1)
                seen.add(b['id'])
    print('Validation passed')
"
}

# Determine which validator to use
get_validator() {
	if command_exists check-jsonschema; then
		echo "check-jsonschema"
	elif command_exists ajv; then
		echo "ajv"
	elif command_exists yq && command_exists python3; then
		echo "fallback"
	else
		echo ""
	fi
}

# Run quality scoring (optional)
run_quality_score() {
	local yaml_file="$1"
	local scorer_script="$SCRIPT_DIR/quality-scorer.py"
	local rules_file="$SCRIPT_DIR/rules.yaml"

	if [[ ! -f "$scorer_script" ]]; then
		warning "Quality scorer not found: $scorer_script"
		return 0
	fi

	if [[ ! -f "$rules_file" ]]; then
		warning "Rules file not found: $rules_file"
		return 0
	fi

	if ! command_exists python3; then
		warning "Python3 not available for quality scoring"
		return 0
	fi

	echo ""
	echo "Running quality scoring..."
	python3 "$scorer_script" "$yaml_file" --rules "$rules_file"
	return $?
}

# Main validation function
validate_file() {
	local yaml_file="$1"
	local validator
	validator=$(get_validator)

	if [[ -z "$validator" ]]; then
		error "No validator available. Please install check-jsonschema or ajv-cli"
		return 2
	fi

	if [[ ! -f "$yaml_file" ]]; then
		error "File not found: $yaml_file"
		return 2
	fi

	if [[ ! -f "$SCHEMA_FILE" ]]; then
		error "Schema file not found: $SCHEMA_FILE"
		return 2
	fi

	case "$validator" in
	check-jsonschema)
		validate_check_jsonschema "$yaml_file"
		;;
	ajv)
		validate_ajv "$yaml_file"
		;;
	fallback)
		validate_fallback "$yaml_file"
		;;
	esac
}

# Validate all YAML files in a directory
validate_directory() {
	local dir="$1"
	local validator
	validator=$(get_validator)

	if [[ -z "$validator" ]]; then
		error "No validator available. Please install check-jsonschema or ajv-cli"
		return 2
	fi

	if [[ ! -d "$dir" ]]; then
		error "Directory not found: $dir"
		return 2
	fi

	local yaml_files
	yaml_files=$(find "$dir" -name "*.yaml" -o -name "*.yml" 2>/dev/null | sort)

	if [[ -z "$yaml_files" ]]; then
		warning "No YAML files found in $dir"
		return 0
	fi

	local failed=0
	local total=0

	for yaml_file in $yaml_files; do
		total=$((total + 1))
		echo "Validating: $yaml_file"

		if validate_file "$yaml_file"; then
			success "Valid: $yaml_file"
		else
			error "Invalid: $yaml_file"
			failed=$((failed + 1))
		fi
	done

	echo ""
	echo "Results: $total files validated, $failed failures"

	if [[ $failed -gt 0 ]]; then
		return 1
	fi

	return 0
}

# Main entry point
main() {
	local arg="${1:-}"

	# Check if schema file exists
	if [[ ! -f "$SCHEMA_FILE" ]]; then
		error "Schema file not found: $SCHEMA_FILE"
		error "This script should be run from the specs/linter directory"
		exit 2
	fi

	# Handle special arguments
	case "$arg" in
	--check | -c)
		check_validators
		exit $?
		;;
	--quality | -q)
		# Run quality scoring only (skip schema validation)
		local arg2="${2:-}"
		if [[ -z "$arg2" ]]; then
			error "No spec file provided for quality scoring"
			exit 2
		fi
		run_quality_score "$arg2"
		exit $?
		;;
	--help | -h)
		echo "Usage: $0 <yaml-file|directory> [options]"
		echo ""
		echo "Validate YAML spec files against the JSON schema"
		echo ""
		echo "Arguments:"
		echo "  <yaml-file>    Path to a YAML file to validate"
		echo "  <directory>    Path to directory containing YAML files"
		echo ""
		echo "Options:"
		echo "  --check        Check available validators"
		echo "  --quality      Run quality scoring only (skip schema validation)"
		echo "  --score        Run full validation with quality scoring"
		echo "  --help         Show this help message"
		echo ""
		echo "Exit codes:"
		echo "  0 - Validation passed"
		echo "  1 - Validation failed or quality score below threshold"
		echo "  2 - Security rules failed or no validator available"
		exit 0
		;;
	--score | -s)
		# Run full validation with quality scoring
		local arg2="${2:-}"
		if [[ -z "$arg2" ]]; then
			error "No spec file provided"
			exit 2
		fi

		# First run schema validation
		if validate_file "$arg2"; then
			success "Schema validation passed"
		else
			error "Schema validation failed"
			exit 1
		fi

		# Then run quality scoring
		run_quality_score "$arg2"
		exit $?
		;;
	"")
		error "No argument provided. Use --help for usage information"
		exit 2
		;;
	esac

	# Check if argument is a file or directory
	if [[ -f "$arg" ]]; then
		validate_file "$arg"
		exit $?
	elif [[ -d "$arg" ]]; then
		validate_directory "$arg"
		exit $?
	else
		error "Invalid argument: $arg (must be a file or directory)"
		exit 2
	fi
}

main "$@"
