#!/bin/sh
# ADI CLI Installer (Refactored with Libraries)
# Usage: curl -fsSL https://adi.the-ihor.com/install.sh | sh
#
# Environment variables:
#   ADI_INSTALL_DIR  - Installation directory (default: ~/.local/bin)
#   ADI_VERSION      - Specific version to install (default: latest)

set -e

# Note: Using /bin/sh for compatibility with curl | sh
# Libraries use bash, so we convert to bash
if [ -z "$BASH_VERSION" ]; then
    exec bash "$0" "$@"
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load libraries
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/github.sh"
source "$SCRIPT_DIR/lib/common.sh"

# Configuration
REPO="adi-family/adi-cli"
BINARY_NAME="adi"

# =============================================================================
# Main Installation Flow
# =============================================================================

main() {
    echo ""
    printf "${BLUE}ADI CLI Installer${NC}\n"
    echo ""

    # Detect platform
    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)
    local target
    target=$(get_target "$os" "$arch")

    # Windows redirects to winget
    if [ "$os" = "windows" ]; then
        error "Windows detected. Please use: winget install adi-cli"
    fi

    info "Detected platform: $target"

    # Determine version
    local version="${ADI_VERSION:-}"
    if [ -z "$version" ]; then
        info "Fetching latest version"
        version=$(fetch_latest_version "$REPO")
        [ -z "$version" ] && error "Failed to fetch latest version"
    fi

    # Normalize version
    local version_num
    version_num=$(normalize_version "$version")
    version=$(ensure_v_prefix "$version_num")

    info "Installing version: $version"

    # Determine install directory
    local install_dir="${ADI_INSTALL_DIR:-$HOME/.local/bin}"
    ensure_dir "$install_dir"

    info "Install directory: $install_dir"

    # Get archive extension
    local archive_ext
    archive_ext=$(get_archive_extension "$os")

    # Construct download URLs
    local archive_name="adi-${version}-${target}.${archive_ext}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"
    local checksums_url="https://github.com/${REPO}/releases/download/${version}/SHA256SUMS"

    # Create temp directory (auto-cleaned on exit)
    local temp_dir
    temp_dir=$(create_temp_dir)

    # Download archive
    local archive_path="$temp_dir/$archive_name"
    download "$download_url" "$archive_path"

    # Download and verify checksum
    local checksums_path="$temp_dir/SHA256SUMS"
    if download "$checksums_url" "$checksums_path" 2>/dev/null; then
        local expected_checksum
        expected_checksum=$(grep "$archive_name" "$checksums_path" | cut -d' ' -f1)
        verify_checksum "$archive_path" "$expected_checksum"
    else
        warn "Checksums file not available, skipping verification"
    fi

    # Extract
    extract_archive "$archive_path" "$temp_dir"

    # Install binary
    local binary_path="$temp_dir/$BINARY_NAME"
    [ ! -f "$binary_path" ] && error "Binary not found in archive"

    chmod +x "$binary_path"
    mv "$binary_path" "$install_dir/$BINARY_NAME"

    success "Installed $BINARY_NAME to $install_dir/$BINARY_NAME"

    # Setup PATH
    setup_path "$install_dir"

    # Verify installation
    echo ""
    if command -v adi >/dev/null 2>&1 || [ -x "$install_dir/$BINARY_NAME" ]; then
        local installed_version
        installed_version=$("$install_dir/$BINARY_NAME" --version 2>/dev/null || echo "unknown")
        success "ADI CLI installed successfully!"
        echo ""
        printf "  Version: ${CYAN}%s${NC}\n" "$installed_version"
        printf "  Path:    ${CYAN}%s${NC}\n" "$install_dir/$BINARY_NAME"
        echo ""
        echo "Get started:"
        printf "  ${CYAN}adi --help${NC}\n"
        printf "  ${CYAN}adi plugin list${NC}\n"
    else
        warn "Installation completed but binary verification failed"
    fi
}

main "$@"
