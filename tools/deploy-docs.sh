#!/usr/bin/env bash
# Deploy Documentation to GitHub Pages
# Only runs when there are changes to docs/
# Follows the moon task pattern with inputs-based conditional execution

set -euo pipefail

# Configuration
REPO_ROOT="${MOON_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel)}"
DOCS_DIR="${REPO_ROOT}/docs"
DEPLOY_BRANCH="gh-pages"
MAIN_BRANCH="main"
REMOTE="origin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}INFO:${NC} $1"; }
log_success() { echo -e "${GREEN}SUCCESS:${NC} $1"; }
log_warn() { echo -e "${YELLOW}WARNING:${NC} $1"; }
log_error() { echo -e "${RED}ERROR:${NC} $1" >&2; }

# Check if docs have changed
has_docs_changes() {
	# Check for any changes to docs/ in the last commit
	local changed_files
	changed_files=$(git diff --name-only HEAD~1 HEAD -- "${DOCS_DIR}" 2>/dev/null || echo "")

	if [[ -n "$changed_files" ]]; then
		log_info "Documentation changes detected in last commit:"
		echo "$changed_files" | head -10
		return 0
	fi

	# Check for staged but uncommitted changes
	if ! git diff --cached --quiet -- "${DOCS_DIR}" 2>/dev/null; then
		log_info "Staged documentation changes detected"
		return 0
	fi

	# Check for unstaged changes
	if ! git diff --quiet -- "${DOCS_DIR}" 2>/dev/null; then
		log_info "Unstaged documentation changes detected"
		return 0
	fi

	return 1 # No changes
}

# Deploy docs to gh-pages branch using a separate clone
deploy_docs() {
	log_info "Starting documentation deployment..."

	# Ensure we're in the repo root
	cd "${REPO_ROOT}"

	# Save current branch and commit
	local current_branch current_commit
	current_branch=$(git rev-parse --abbrev-ref HEAD)
	current_commit=$(git rev-parse HEAD)

	# Create a temporary directory for deployment
	local temp_dir
	temp_dir=$(mktemp -d)
	trap "rm -rf '${temp_dir}'" EXIT

	# Clone the repo to temp directory (shallow clone for speed)
	log_info "Cloning repository for deployment..."
	if ! git clone --depth 1 --branch "${DEPLOY_BRANCH}" "file://${REPO_ROOT}" "${temp_dir}" 2>/dev/null; then
		log_warn "No existing ${DEPLOY_BRANCH} branch, creating one..."
		# Clone main and create orphan branch
		git clone --depth 1 "file://${REPO_ROOT}" "${temp_dir}"
		cd "${temp_dir}"
		git checkout --orphan "${DEPLOY_BRANCH}"
		git rm -rf . 2>/dev/null || true
		git commit --allow-empty -m "docs: initialize gh-pages branch"
		git branch -M "${DEPLOY_BRANCH}"
	fi

	# Set the remote to push to GitHub
	cd "${temp_dir}"
	git remote set-url origin "https://github.com/lprior-repo/isolate.git"

	# Work in temp directory
	cd "${temp_dir}"

	# Clean existing content (preserve .git)
	find . -maxdepth 1 -not -name '.git' -not -name '.' -exec rm -rf {} +

	# Copy docs from source repo
	log_info "Preparing documentation files..."
	cp -r "${DOCS_DIR}"/* .

	# Add a simple index redirect if no index.html exists
	if [[ ! -f "index.html" ]]; then
		cat >"index.html" <<'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta http-equiv="refresh" content="0; url=00_START_HERE.md">
    <title>Isolate Documentation</title>
</head>
<body>
    <p>Redirecting to <a href="00_START_HERE.md">documentation</a>...</p>
</body>
</html>
EOF
	fi

	# Stage all changes
	git add -A

	# Check if there are changes to commit
	if git diff --cached --quiet; then
		log_info "No changes to deploy"
		return 0
	fi

	# Commit and push
	local commit_msg="docs: update documentation $(date '+%Y-%m-%d %H:%M:%S')"
	git commit -m "${commit_msg}"

	log_info "Pushing to ${DEPLOY_BRANCH}..."
	git push "${REMOTE}" "${DEPLOY_BRANCH}" --force

	log_success "Documentation deployed successfully!"
	log_info "View at: https://lprior-repo.github.io/isolate/"
}

# Main entry point
main() {
	log_info "Documentation deployment check"

	# Check if we're in a git repo
	if ! git rev-parse --git-dir >/dev/null 2>&1; then
		log_error "Not in a git repository"
		exit 1
	fi

	# Check if docs directory exists
	if [[ ! -d "${DOCS_DIR}" ]]; then
		log_error "Documentation directory not found: ${DOCS_DIR}"
		exit 1
	fi

	# Check for docs changes
	if ! has_docs_changes; then
		log_info "No documentation changes detected, skipping deployment"
		exit 0
	fi

	# Deploy
	deploy_docs
}

# Run main
main "$@"
