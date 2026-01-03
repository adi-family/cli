#!/bin/bash
# =============================================================================
# GitHub API Helpers Library
# =============================================================================
# Provides functions for interacting with GitHub API
#
# Usage:
#   source scripts/lib/github.sh
#
# Functions:
#   fetch_latest_version <repo>    - Get latest release tag
#   fetch_release_info <repo> <tag> - Get release information
#   github_api_call <endpoint>      - Call GitHub API endpoint
# =============================================================================

# Prevent double-loading
if [[ -n "${GITHUB_LIB_LOADED}" ]]; then
    return 0
fi
GITHUB_LIB_LOADED=1

# Load dependencies
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/download.sh
source "$SCRIPT_LIB_DIR/download.sh"
# shellcheck source=scripts/lib/log.sh
source "$SCRIPT_LIB_DIR/log.sh"

# -----------------------------------------------------------------------------
# GitHub API Functions
# -----------------------------------------------------------------------------

# Call GitHub API endpoint
# Usage: github_api_call <endpoint>
# Example: github_api_call "/repos/owner/repo/releases/latest"
github_api_call() {
    local endpoint="$1"
    local url="https://api.github.com${endpoint}"

    fetch "$url"
}

# Fetch latest release version from GitHub repo
# Usage: fetch_latest_version <repo>
# Example: fetch_latest_version "adi-family/adi-cli"
# Returns: v0.8.4
fetch_latest_version() {
    local repo="$1"

    if [ -z "$repo" ]; then
        error "Usage: fetch_latest_version <repo>"
    fi

    local url="https://api.github.com/repos/${repo}/releases/latest"

    if has_curl; then
        fetch "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif has_wget; then
        fetch "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found"
    fi
}

# Fetch release information for specific tag
# Usage: fetch_release_info <repo> <tag>
# Example: fetch_release_info "adi-family/adi-cli" "v0.8.4"
fetch_release_info() {
    local repo="$1"
    local tag="$2"

    if [ -z "$repo" ] || [ -z "$tag" ]; then
        error "Usage: fetch_release_info <repo> <tag>"
    fi

    local url="https://api.github.com/repos/${repo}/releases/tags/${tag}"
    fetch "$url"
}

# Check if release exists
# Usage: release_exists <repo> <tag>
# Returns: 0 if exists, 1 if not
release_exists() {
    local repo="$1"
    local tag="$2"

    if [ -z "$repo" ] || [ -z "$tag" ]; then
        error "Usage: release_exists <repo> <tag>"
    fi

    # Check with gh CLI if available
    if command -v gh >/dev/null 2>&1; then
        gh release view "$tag" -R "$repo" >/dev/null 2>&1
        return $?
    fi

    # Fall back to API call
    local url="https://api.github.com/repos/${repo}/releases/tags/${tag}"
    local response=$(fetch "$url" 2>/dev/null)

    # Check if response contains "id" field (release exists)
    if echo "$response" | grep -q '"id"'; then
        return 0
    else
        return 1
    fi
}

# Download asset from GitHub release
# Usage: download_github_asset <repo> <tag> <asset_name> <output>
download_github_asset() {
    local repo="$1"
    local tag="$2"
    local asset_name="$3"
    local output="$4"

    if [ -z "$repo" ] || [ -z "$tag" ] || [ -z "$asset_name" ] || [ -z "$output" ]; then
        error "Usage: download_github_asset <repo> <tag> <asset_name> <output>"
    fi

    local url="https://github.com/${repo}/releases/download/${tag}/${asset_name}"
    download "$url" "$output"
}
