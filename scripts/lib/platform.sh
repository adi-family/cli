#!/bin/bash
# =============================================================================
# Platform Detection Library
# =============================================================================
# Detects operating system, architecture, and generates target triples
#
# Usage:
#   source scripts/lib/platform.sh
#
# Functions:
#   detect_os               - Returns: darwin, linux, windows
#   detect_arch             - Returns: x86_64, aarch64
#   get_target <os> <arch>  - Returns target triple (e.g., x86_64-apple-darwin)
#   get_platform            - Returns platform string (e.g., darwin-aarch64)
#   get_lib_extension <platform> - Returns library extension (dylib, so, dll)
# =============================================================================

# Prevent double-loading
if [[ -n "${PLATFORM_LIB_LOADED}" ]]; then
    return 0
fi
PLATFORM_LIB_LOADED=1

# Load logging library
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/log.sh
source "$SCRIPT_LIB_DIR/log.sh"

# -----------------------------------------------------------------------------
# Platform Detection Functions
# -----------------------------------------------------------------------------

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Darwin)
            echo "darwin"
            ;;
        Linux)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
}

# Get Rust target triple from OS and architecture
get_target() {
    local os="$1"
    local arch="$2"

    case "$os" in
        darwin)
            echo "${arch}-apple-darwin"
            ;;
        linux)
            # Default to GNU libc, can be overridden to musl if needed
            echo "${arch}-unknown-linux-gnu"
            ;;
        windows)
            echo "${arch}-pc-windows-msvc"
            ;;
        *)
            error "Unknown OS: $os"
            ;;
    esac
}

# Get Rust target triple for musl (static linking)
get_target_musl() {
    local os="$1"
    local arch="$2"

    case "$os" in
        linux)
            echo "${arch}-unknown-linux-musl"
            ;;
        *)
            # Fall back to regular target for non-Linux
            get_target "$os" "$arch"
            ;;
    esac
}

# Get platform string (e.g., darwin-aarch64, linux-x86_64)
get_platform() {
    local os="${1:-$(detect_os)}"
    local arch="${2:-$(detect_arch)}"
    echo "${os}-${arch}"
}

# Get library extension for platform
get_lib_extension() {
    local platform="$1"

    case "$platform" in
        darwin-*)
            echo "dylib"
            ;;
        linux-*)
            echo "so"
            ;;
        windows-*)
            echo "dll"
            ;;
        *)
            error "Unknown platform: $platform"
            ;;
    esac
}

# Get archive extension for platform
get_archive_extension() {
    local os="${1:-$(detect_os)}"

    case "$os" in
        windows)
            echo "zip"
            ;;
        *)
            echo "tar.gz"
            ;;
    esac
}
