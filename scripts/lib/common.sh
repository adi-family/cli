#!/bin/bash
# =============================================================================
# Common Utilities Library
# =============================================================================
# Provides common utilities for bash scripts
#
# Usage:
#   source scripts/lib/common.sh
#
# Functions:
#   verify_checksum <file> <expected>   - Verify SHA256 checksum
#   extract_archive <archive> <dest>    - Extract tar.gz or zip archive
#   setup_path <install_dir>            - Add directory to PATH
#   generate_secret                     - Generate cryptographic secret
#   check_command <cmd>                 - Check if command exists
#   ensure_command <cmd> [install_hint] - Ensure command exists or exit
# =============================================================================

# Prevent double-loading
if [[ -n "${COMMON_LIB_LOADED}" ]]; then
    return 0
fi
COMMON_LIB_LOADED=1

# Load logging library
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/log.sh
source "$SCRIPT_LIB_DIR/log.sh"

# -----------------------------------------------------------------------------
# Command Checking
# -----------------------------------------------------------------------------

# Check if command exists
# Usage: check_command <cmd>
# Returns: 0 if exists, 1 if not
check_command() {
    command -v "$1" >/dev/null 2>&1
}

# Ensure command exists or exit
# Usage: ensure_command <cmd> [install_hint]
ensure_command() {
    local cmd="$1"
    local hint="${2:-}"

    if ! check_command "$cmd"; then
        if [ -n "$hint" ]; then
            error "$cmd not found. Install: $hint"
        else
            error "$cmd not found"
        fi
    fi
}

# -----------------------------------------------------------------------------
# Checksum Verification
# -----------------------------------------------------------------------------

# Verify SHA256 checksum
# Usage: verify_checksum <file> <expected>
verify_checksum() {
    local file="$1"
    local expected="$2"

    if [ -z "$expected" ]; then
        warn "Skipping checksum verification (checksum not available)"
        return 0
    fi

    local actual=""
    if check_command sha256sum; then
        actual=$(sha256sum "$file" | cut -d' ' -f1)
    elif check_command shasum; then
        actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        warn "Skipping checksum verification (sha256sum/shasum not found)"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed!\nExpected: $expected\nActual: $actual"
    fi

    success "Checksum verified"
}

# Generate SHA256 checksums for files
# Usage: generate_checksums <output_file> <files...>
generate_checksums() {
    local output="$1"
    shift

    if check_command sha256sum; then
        sha256sum "$@" > "$output"
    elif check_command shasum; then
        shasum -a 256 "$@" > "$output"
    else
        error "sha256sum or shasum not found"
    fi
}

# -----------------------------------------------------------------------------
# Archive Extraction
# -----------------------------------------------------------------------------

# Extract archive (tar.gz or zip)
# Usage: extract_archive <archive> <dest>
extract_archive() {
    local archive="$1"
    local dest="$2"

    if [ -z "$archive" ] || [ -z "$dest" ]; then
        error "Usage: extract_archive <archive> <dest>"
    fi

    info "Extracting archive"

    case "$archive" in
        *.tar.gz|*.tgz)
            tar -xzf "$archive" -C "$dest"
            ;;
        *.tar.bz2|*.tbz2)
            tar -xjf "$archive" -C "$dest"
            ;;
        *.tar.xz|*.txz)
            tar -xJf "$archive" -C "$dest"
            ;;
        *.zip)
            if check_command unzip; then
                unzip -q "$archive" -d "$dest"
            else
                error "unzip not found, required to extract .zip archives"
            fi
            ;;
        *)
            error "Unknown archive format: $archive"
            ;;
    esac
}

# Create tar.gz archive
# Usage: create_tarball <output> <source_dir> <files...>
create_tarball() {
    local output="$1"
    local source_dir="$2"
    shift 2

    tar -czf "$output" -C "$source_dir" "$@"
}

# -----------------------------------------------------------------------------
# Path Management
# -----------------------------------------------------------------------------

# Add directory to PATH in shell RC file
# Usage: setup_path <install_dir>
setup_path() {
    local install_dir="$1"
    local shell_name=""
    local rc_file=""

    # Detect shell
    if [ -n "$SHELL" ]; then
        shell_name=$(basename "$SHELL")
    fi

    case "$shell_name" in
        zsh)
            rc_file="$HOME/.zshrc"
            ;;
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                rc_file="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                rc_file="$HOME/.bash_profile"
            fi
            ;;
        fish)
            rc_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            rc_file="$HOME/.profile"
            ;;
    esac

    # Check if already in PATH
    case ":$PATH:" in
        *":$install_dir:"*)
            return 0
            ;;
    esac

    echo ""
    warn "$install_dir is not in your PATH"
    echo ""
    echo "Add it by running:"
    echo ""

    case "$shell_name" in
        fish)
            printf "  ${CYAN}fish_add_path %s${NC}\n" "$install_dir"
            ;;
        *)
            printf "  ${CYAN}echo 'export PATH=\"%s:\$PATH\"' >> %s${NC}\n" "$install_dir" "$rc_file"
            ;;
    esac

    echo ""
    echo "Then restart your shell or run:"
    printf "  ${CYAN}source %s${NC}\n" "$rc_file"
}

# -----------------------------------------------------------------------------
# Cryptography
# -----------------------------------------------------------------------------

# Generate strong cryptographic secret
# Usage: generate_secret [length]
generate_secret() {
    local length="${1:-36}"

    if check_command openssl; then
        openssl rand -base64 "$length"
    elif [ -r /dev/urandom ]; then
        head -c "$length" /dev/urandom | base64 | tr -d '\n'
    else
        error "Cannot generate secret: openssl or /dev/urandom required"
    fi
}

# -----------------------------------------------------------------------------
# Version Management
# -----------------------------------------------------------------------------

# Normalize version (remove 'v' prefix)
# Usage: normalize_version <version>
normalize_version() {
    local version="$1"
    echo "${version#v}"
}

# Add 'v' prefix to version if not present
# Usage: ensure_v_prefix <version>
ensure_v_prefix() {
    local version="$1"
    if [[ "$version" != v* ]]; then
        echo "v${version}"
    else
        echo "$version"
    fi
}

# -----------------------------------------------------------------------------
# Directory Management
# -----------------------------------------------------------------------------

# Create directory if it doesn't exist
# Usage: ensure_dir <dir>
ensure_dir() {
    local dir="$1"
    mkdir -p "$dir"
}

# Create temporary directory with cleanup trap
# Usage: create_temp_dir
# Returns: path to temp directory
create_temp_dir() {
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf '$temp_dir'" EXIT
    echo "$temp_dir"
}

# -----------------------------------------------------------------------------
# Root/Sudo Checking
# -----------------------------------------------------------------------------

# Check if running as root
# Usage: check_root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        error "This script must be run as root. Try: sudo $0"
    fi
}

# Check if NOT running as root
# Usage: check_not_root
check_not_root() {
    if [ "$(id -u)" -eq 0 ]; then
        error "This script should not be run as root"
    fi
}
