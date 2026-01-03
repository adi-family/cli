#!/bin/bash
# =============================================================================
# Download Utilities Library
# =============================================================================
# Provides download functionality using curl or wget
#
# Usage:
#   source scripts/lib/download.sh
#
# Functions:
#   has_curl            - Check if curl is available
#   has_wget            - Check if wget is available
#   check_downloader    - Ensure curl or wget is available
#   download <url> <output> - Download file to output path
# =============================================================================

# Prevent double-loading
if [[ -n "${DOWNLOAD_LIB_LOADED}" ]]; then
    return 0
fi
DOWNLOAD_LIB_LOADED=1

# Load logging library
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/log.sh
source "$SCRIPT_LIB_DIR/log.sh"

# -----------------------------------------------------------------------------
# Downloader Detection
# -----------------------------------------------------------------------------

# Check if curl is available
has_curl() {
    command -v curl >/dev/null 2>&1
}

# Check if wget is available
has_wget() {
    command -v wget >/dev/null 2>&1
}

# Ensure at least one downloader is available
check_downloader() {
    if ! has_curl && ! has_wget; then
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# -----------------------------------------------------------------------------
# Download Functions
# -----------------------------------------------------------------------------

# Download file from URL to output path
# Usage: download <url> <output>
download() {
    local url="$1"
    local output="$2"

    if [ -z "$url" ] || [ -z "$output" ]; then
        error "Usage: download <url> <output>"
    fi

    info "Downloading from $url"

    if has_curl; then
        curl -fsSL "$url" -o "$output"
    elif has_wget; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found"
    fi
}

# Download file with progress bar
# Usage: download_with_progress <url> <output>
download_with_progress() {
    local url="$1"
    local output="$2"

    if [ -z "$url" ] || [ -z "$output" ]; then
        error "Usage: download_with_progress <url> <output>"
    fi

    info "Downloading from $url"

    if has_curl; then
        curl -fL --progress-bar "$url" -o "$output"
    elif has_wget; then
        wget --show-progress "$url" -O "$output"
    else
        error "Neither curl nor wget found"
    fi
}

# Fetch content from URL to stdout
# Usage: fetch <url>
fetch() {
    local url="$1"

    if [ -z "$url" ]; then
        error "Usage: fetch <url>"
    fi

    if has_curl; then
        curl -fsSL "$url"
    elif has_wget; then
        wget -qO- "$url"
    else
        error "Neither curl nor wget found"
    fi
}
