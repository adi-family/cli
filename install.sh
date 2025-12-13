#!/bin/sh
set -e

# ADI CLI Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/adi-family/cli/main/install.sh | sh

REPO_OWNER="adi-family"
REPO_NAME="cli"
BINARY_NAME="adi"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info() {
    printf "${CYAN}→${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}✓${NC} %s\n" "$1"
}

error() {
    printf "${RED}✗${NC} %s\n" "$1" >&2
}

warning() {
    printf "${YELLOW}!${NC} %s\n" "$1"
}

detect_platform() {
    local os arch

    # Detect OS
    case "$(uname -s)" in
        Darwin)
            os="apple-darwin"
            ;;
        Linux)
            os="unknown-linux-gnu"
            ;;
        MINGW* | MSYS* | CYGWIN*)
            os="pc-windows-msvc"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64 | amd64)
            arch="x86_64"
            ;;
        aarch64 | arm64)
            arch="aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${arch}-${os}"
}

get_latest_version() {
    local url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

download_file() {
    local url="$1"
    local dest="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$dest" "$url"
    fi
}

main() {
    info "Installing ADI CLI..."

    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: ${platform}"

    # Get latest version
    local version
    version=$(get_latest_version)

    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
        exit 1
    fi

    info "Latest version: ${version}"

    # Determine archive extension
    local archive_ext
    case "$platform" in
        *windows*)
            archive_ext="zip"
            ;;
        *)
            archive_ext="tar.gz"
            ;;
    esac

    # Build download URL
    local archive_name="${BINARY_NAME}-${version}-${platform}.${archive_ext}"
    local download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${archive_name}"

    # Create temp directory
    local temp_dir
    temp_dir=$(mktemp -d)
    trap 'rm -rf "$temp_dir"' EXIT

    # Download archive
    info "Downloading ${archive_name}..."
    download_file "$download_url" "${temp_dir}/${archive_name}"

    # Extract binary
    info "Extracting binary..."
    case "$archive_ext" in
        tar.gz)
            tar xzf "${temp_dir}/${archive_name}" -C "$temp_dir"
            ;;
        zip)
            unzip -q "${temp_dir}/${archive_name}" -d "$temp_dir"
            ;;
    esac

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Install binary
    local binary_path="${temp_dir}/${BINARY_NAME}"
    if [ ! -f "$binary_path" ]; then
        # Handle .exe extension on Windows
        binary_path="${binary_path}.exe"
    fi

    if [ ! -f "$binary_path" ]; then
        error "Binary not found in archive"
        exit 1
    fi

    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."

    # Move binary
    mv "$binary_path" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    success "ADI CLI installed successfully!"

    # Check if in PATH
    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        echo ""
        warning "Add ${INSTALL_DIR} to your PATH:"
        echo ""
        echo "  ${BLUE}export PATH=\"${INSTALL_DIR}:\$PATH\"${NC}"
        echo ""
        echo "Add this line to your ~/.bashrc, ~/.zshrc, or ~/.profile"
        echo ""
    fi

    # Verify installation
    if command -v "${INSTALL_DIR}/${BINARY_NAME}" >/dev/null 2>&1; then
        echo ""
        success "Verification: $(${INSTALL_DIR}/${BINARY_NAME} --version)"
        echo ""
        info "Run '${BINARY_NAME} --help' to get started"
    else
        echo ""
        warning "Binary installed but not in PATH. Run: ${INSTALL_DIR}/${BINARY_NAME} --help"
    fi
}

main
