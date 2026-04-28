#!/usr/bin/env bash
#
# Isolate Development Environment Setup Script
# =========================================
#
# This script automates the initial setup process for new Isolate developers.
# It checks prerequisites, installs missing dependencies, sets up the
# development database, runs initial build and tests, and provides
# clear next steps.
#
# Usage:
#   ./scripts/dev-setup.sh           # Automated setup with prompts
#   ./scripts/dev-setup.sh --yes     # Non-interactive mode
#   ./scripts/dev-setup.sh --check   # Only check prerequisites
#
# Exit codes:
#   0 - Success
#   1 - Prerequisite check failed
#   2 - Installation failed
#   3 - Build failed
#   4 - Tests failed
#   5 - Database setup failed
#
# See CONTRIBUTING.md for manual setup instructions.

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MIN_RUST_VERSION="1.80"
NON_INTERACTIVE=false
CHECK_ONLY=false

# Color codes for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly BOLD='\033[1m'
readonly NC='\033[0m' # No Color

# ============================================================================
# Helper Functions
# ============================================================================

# Print colored output
print_info() {
	echo -e "${BLUE}ℹ${NC} $*"
}

print_success() {
	echo -e "${GREEN}✓${NC} $*"
}

print_warning() {
	echo -e "${YELLOW}⚠${NC} $*"
}

print_error() {
	echo -e "${RED}✗${NC} $*"
}

print_header() {
	echo ""
	echo -e "${BOLD}$*${NC}"
	echo -e "${BOLD}$(printf '=%.0s' {1..80})${NC}"
}

# Prompt user for confirmation (respects --yes flag)
prompt_yes_no() {
	local prompt="$1"
	local default="${2:-n}"

	if [[ "$NON_INTERACTIVE" == "true" ]]; then
		return 0
	fi

	local prompt_str
	if [[ "$default" == "y" ]]; then
		prompt_str="$prompt [Y/n]: "
	else
		prompt_str="$prompt [y/N]: "
	fi

	while true; do
		read -rp "$prompt_str" response
		response="${response:-$default}"

		case "$response" in
		[Yy] | [Yy][Ee][Ss]) return 0 ;;
		[Nn] | [Nn][Oo]) return 1 ;;
		*) echo "Please answer yes or no." ;;
		esac
	done
}

# Execute command with output
run_cmd() {
	local cmd="$1"
	print_info "Running: $cmd"

	if eval "$cmd"; then
		return 0
	else
		local exit_code=$?
		print_error "Command failed with exit code $exit_code: $cmd"
		return "$exit_code"
	fi
}

# ============================================================================
# Prerequisite Checks
# ============================================================================

# Check if a command exists
command_exists() {
	command -v "$1" &>/dev/null
}

# Check Rust version
check_rust_version() {
	if ! command_exists rustc; then
		print_error "Rust not found"
		return 1
	fi

	local version
	version=$(rustc --version | awk '{print $2}')
	print_info "Found Rust version: $version"

	# Compare versions (simple comparison)
	local required_major required_minor
	required_major=$(echo "$MIN_RUST_VERSION" | cut -d. -f1)
	required_minor=$(echo "$MIN_RUST_VERSION" | cut -d. -f2)

	local current_major current_minor
	current_major=$(echo "$version" | cut -d. -f1)
	current_minor=$(echo "$version" | cut -d. -f2)

	if [[ "$current_major" -lt "$required_major" ]] ||
		[[ "$current_major" -eq "$required_major" && "$current_minor" -lt "$required_minor" ]]; then
		print_error "Rust version $version is too old (minimum: $MIN_RUST_VERSION)"
		return 1
	fi

	return 0
}

# Check if moon is installed and working
check_moon() {
	if ! command_exists moon; then
		print_error "Moon not found"
		return 1
	fi

	print_info "Found Moon version: $(moon --version | head -1)"
	return 0
}

# Check if jj is installed
check_jj() {
	if ! command_exists jj; then
		print_error "JJ (Jujutsu) not found"
		return 1
	fi

	print_info "Found JJ version: $(jj --version | head -1)"
	return 0
}

# Check if zellij is installed
check_zellij() {
	if ! command_exists zellij; then
		print_warning "Zellij not found (optional but recommended)"
		return 1
	fi

	print_info "Found Zellij version: $(zellij --version)"
	return 0
}

# Main prerequisite check
check_prerequisites() {
	print_header "Checking Prerequisites"

	local missing=0

	# Check Rust
	if check_rust_version; then
		print_success "Rust $MIN_RUST_VERSION+ installed"
	else
		print_error "Rust $MIN_RUST_VERSION+ required"
		((missing++))
	fi

	# Check Moon
	if check_moon; then
		print_success "Moon installed"
	else
		print_error "Moon required (https://moonrepo.dev/docs/install)"
		((missing++))
	fi

	# Check JJ
	if check_jj; then
		print_success "JJ installed"
	else
		print_error "JJ required (https://github.com/martinvonz/jj#installation)"
		((missing++))
	fi

	# Check Zellij (optional)
	if check_zellij; then
		print_success "Zellij installed"
	else
		print_warning "Zellij not found (optional but recommended)"
		print_warning "  Install from https://zellij.dev/download"
	fi

	if [[ $missing -gt 0 ]]; then
		print_error "$missing required prerequisite(s) missing"
		return 1
	fi

	print_success "All required prerequisites installed"
	return 0
}

# ============================================================================
# Installation Functions
# ============================================================================

install_rust() {
	print_header "Installing Rust"

	if prompt_yes_no "Install Rust via rustup?" "y"; then
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

		# Source rust environment
		# shellcheck source=/dev/null
		source "$HOME/.cargo/env"

		if check_rust_version; then
			print_success "Rust installed successfully"
			return 0
		else
			print_error "Rust installation failed"
			return 2
		fi
	else
		print_warning "Skipping Rust installation"
		return 1
	fi
}

install_moon() {
	print_header "Installing Moon"

	if prompt_yes_no "Install Moon?" "y"; then
		curl -fsSL https://moonrepo.dev/install/moon.sh | bash

		if check_moon; then
			print_success "Moon installed successfully"
			return 0
		else
			print_error "Moon installation failed"
			print_warning "  You may need to restart your shell or update PATH"
			return 2
		fi
	else
		print_warning "Skipping Moon installation"
		return 1
	fi
}

install_jj() {
	print_header "Installing JJ (Jujutsu)"

	if prompt_yes_no "Install JJ?" "y"; then
		# Detect platform
		local platform
		platform=$(uname -s)

		case "$platform" in
		Linux)
			curl -L https://github.com/martinvonz/jj/releases/latest/download/jj-linux-x86_64-musl -o /tmp/jj
			sudo mv /tmp/jj /usr/local/bin/
			sudo chmod +x /usr/local/bin/jj
			;;
		Darwin)
			if command_exists brew; then
				brew install jj
			else
				print_error "Homebrew not found. Please install manually"
				return 2
			fi
			;;
		*)
			print_error "Unsupported platform: $platform"
			return 2
			;;
		esac

		if check_jj; then
			print_success "JJ installed successfully"
			return 0
		else
			print_error "JJ installation failed"
			return 2
		fi
	else
		print_warning "Skipping JJ installation"
		return 1
	fi
}

install_zellij() {
	print_header "Installing Zellij (optional)"

	if prompt_yes_no "Install Zellij?" "y"; then
		# Detect platform
		local platform
		platform=$(uname -s)

		case "$platform" in
		Linux)
			if command_exists cargo; then
				cargo install zellij
			else
				print_error "Cargo not found"
				return 2
			fi
			;;
		Darwin)
			if command_exists brew; then
				brew install zellij
			else
				print_error "Homebrew not found"
				return 2
			fi
			;;
		*)
			print_error "Unsupported platform: $platform"
			return 2
			;;
		esac

		if check_zellij; then
			print_success "Zellij installed successfully"
			return 0
		else
			print_error "Zellij installation failed"
			return 2
		fi
	else
		print_warning "Skipping Zellij installation"
		return 1
	fi
}

# ============================================================================
# Database Setup
# ============================================================================

setup_database() {
	print_header "Setting Up Development Database"

	# Check if database already exists
	local db_path="${PROJECT_ROOT}/.beads/beads.db"

	if [[ -f "$db_path" ]]; then
		print_info "Database already exists at: $db_path"
		if prompt_yes_no "Recreate database?" "n"; then
			rm -f "$db_path"
		else
			print_success "Using existing database"
			return 0
		fi
	fi

	# Create .beads directory if needed
	local beads_dir="${PROJECT_ROOT}/.beads"
	if [[ ! -d "$beads_dir" ]]; then
		mkdir -p "$beads_dir"
		print_info "Created directory: $beads_dir"
	fi

	# The database is created automatically on first run
	print_info "Database will be created automatically on first use"
	print_success "Database setup complete"
	return 0
}

# ============================================================================
# Build and Test
# ============================================================================

run_build() {
	print_header "Running Initial Build"

	cd "$PROJECT_ROOT"

	if run_cmd "moon run :build"; then
		print_success "Build completed successfully"
		return 0
	else
		print_error "Build failed"
		return 3
	fi
}

run_tests() {
	print_header "Running Initial Tests"

	cd "$PROJECT_ROOT"

	if run_cmd "moon run :test"; then
		print_success "All tests passed"
		return 0
	else
		print_error "Tests failed"
		return 4
	fi
}

run_quick_check() {
	print_header "Running Quick Check (fmt + check)"

	cd "$PROJECT_ROOT"

	if run_cmd "moon run :quick"; then
		print_success "Quick check passed"
		return 0
	else
		print_error "Quick check failed"
		return 3
	fi
}

# ============================================================================
# Next Steps
# ============================================================================

print_next_steps() {
	print_header "Setup Complete!"

	cat <<'EOF'

Your Isolate development environment is ready. Here's what to do next:

QUICK START:
  moon run :quick              # Fast format + type check (6-7ms cached)
  moon run :test               # Run full test suite
  moon run :build              # Create release build
  moon run :ci                 # Full CI pipeline

DEVELOPMENT WORKFLOW:
  1. Make code changes
  2. Run: moon run :quick      # Verify formatting and types
  3. Run: moon run :test       # Run tests
  4. Run: ./target/release/isolate --help    # Test manually

COMMON TASKS:
  moon run :fmt-fix            # Auto-fix formatting issues
  moon run :test -- <name>     # Run specific test
  moon run :check              # Type check only

IMPORTANT REMINDERS:
  • ALWAYS use 'moon run' instead of 'cargo' commands
  • No unwrap(), expect(), panic! allowed (compiler-enforced)
  • Follow DDD patterns: domain logic in core, I/O in shell
  • See AGENTS.md and CONTRIBUTING.md for guidelines

DOCUMENTATION:
  • README.md - Project overview
  • CONTRIBUTING.md - Development guide
  • ARCHITECTURE.md - System design
  • DOMAIN_TYPES_GUIDE.md - Domain types reference

GETTING HELP:
  • GitHub Issues: https://github.com/lprior-repo/isolate/issues
  • Discussions: https://github.com/lprior-repo/isolate/discussions
  • Full docs: https://lprior-repo.github.io/isolate/

EOF
}

# ============================================================================
# Argument Parsing
# ============================================================================

parse_args() {
	while [[ $# -gt 0 ]]; do
		case "$1" in
		--yes | -y)
			NON_INTERACTIVE=true
			shift
			;;
		--check | -c)
			CHECK_ONLY=true
			shift
			;;
		--help | -h)
			cat <<'EOF'
Isolate Development Environment Setup Script

Usage:
  ./scripts/dev-setup.sh           # Automated setup with prompts
  ./scripts/dev-setup.sh --yes     # Non-interactive mode
  ./scripts/dev-setup.sh --check   # Only check prerequisites

Options:
  -y, --yes        Non-interactive mode (accept all prompts)
  -c, --check      Only check prerequisites, don't install
  -h, --help       Show this help message

This script will:
  1. Check for Rust 1.80+, Moon, JJ, and Zellij
  2. Offer to install missing dependencies
  3. Set up the development database
  4. Run initial build and tests
  5. Print next steps

Exit codes:
  0 - Success
  1 - Prerequisite check failed
  2 - Installation failed
  3 - Build failed
  4 - Tests failed
  5 - Database setup failed

See CONTRIBUTING.md for manual setup instructions.
EOF
			exit 0
			;;
		*)
			print_error "Unknown option: $1"
			echo "Use --help for usage information"
			exit 1
			;;
		esac
	done
}

# ============================================================================
# Main Script
# ============================================================================

main() {
	print_header "Isolate Development Environment Setup"

	# Parse arguments
	parse_args "$@"

	# Change to project root
	cd "$PROJECT_ROOT"

	# Check prerequisites
	if ! check_prerequisites; then
		echo ""

		if [[ "$CHECK_ONLY" == "true" ]]; then
			print_error "Prerequisite check failed"
			exit 1
		fi

		# Offer to install missing dependencies
		echo ""

		if ! check_rust_version 2>/dev/null; then
			if ! install_rust; then
				print_error "Cannot proceed without Rust"
				exit 2
			fi
		fi

		if ! check_moon 2>/dev/null; then
			if ! install_moon; then
				print_error "Cannot proceed without Moon"
				exit 2
			fi
		fi

		if ! check_jj 2>/dev/null; then
			if ! install_jj; then
				print_error "Cannot proceed without JJ"
				exit 2
			fi
		fi

		# Zellij is optional
		if ! check_zellij 2>/dev/null; then
			install_zellij || true
		fi

		echo ""
	fi

	if [[ "$CHECK_ONLY" == "true" ]]; then
		print_success "Prerequisite check complete"
		exit 0
	fi

	# Setup database
	if ! setup_database; then
		exit 5
	fi

	echo ""

	# Run quick check first (faster feedback)
	if ! run_quick_check; then
		print_warning "Quick check failed, but this might be fixable with fmt-fix"
		if prompt_yes_no "Run fmt-fix to auto-fix formatting?" "y"; then
			run_cmd "moon run :fmt-fix" || true
			run_cmd "moon run :quick" || print_warning "Quick check still has issues"
		fi
	fi

	echo ""

	# Run full build
	if ! run_build; then
		print_error "Build failed - please fix errors before continuing"
		exit 3
	fi

	echo ""

	# Run tests
	if ! run_tests; then
		print_error "Tests failed - please fix failures before contributing"
		exit 4
	fi

	# Print next steps
	print_next_steps

	exit 0
}

# Run main function
main "$@"
