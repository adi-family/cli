#!/bin/sh
# Cocoon Installer
# Usage: curl -fsSL https://adi.the-ihor.com/family/cocoon/install.sh | sh -s -- <setup_token>
#
# Environment variables:
#   SIGNALING_URL    - Signaling server URL (default: wss://signal.adi.the-ihor.com/ws)
#   COCOON_NAME      - Display name for this cocoon (default: hostname)
#   INSTALL_DIR      - Installation directory (default: /usr/local/bin)
#   NO_SYSTEMD       - Set to 1 to skip systemd service creation

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

REPO="adi-family/cocoon"
BINARY_NAME="cocoon"
CONFIG_DIR="/etc/cocoon"
DATA_DIR="/var/lib/cocoon"

info() {
    printf "${CYAN}info${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}done${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${NC} %s\n" "$1"
}

error() {
    printf "${RED}error${NC} %s\n" "$1" >&2
    exit 1
}

# Check if running as root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        error "This script must be run as root. Try: sudo sh -s -- <token>"
    fi
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin)
            echo "darwin"
            ;;
        Linux)
            echo "linux"
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

# Get target triple
get_target() {
    local os="$1"
    local arch="$2"

    case "$os" in
        darwin)
            echo "${arch}-apple-darwin"
            ;;
        linux)
            echo "${arch}-unknown-linux-musl"
            ;;
    esac
}

# Fetch latest version from GitHub API
fetch_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$url" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download file
download() {
    local url="$1"
    local output="$2"

    info "Downloading from $url"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found"
    fi
}

# Generate strong secret
generate_secret() {
    if command -v openssl >/dev/null 2>&1; then
        openssl rand -base64 36
    elif [ -r /dev/urandom ]; then
        head -c 36 /dev/urandom | base64 | tr -d '\n'
    else
        error "Cannot generate secret: openssl or /dev/urandom required"
    fi
}

# Create systemd service
create_systemd_service() {
    local install_dir="$1"

    if [ ! -d /etc/systemd/system ]; then
        warn "systemd not found, skipping service creation"
        return 1
    fi

    cat > /etc/systemd/system/cocoon.service << EOF
[Unit]
Description=Cocoon - Remote Execution Worker
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=${CONFIG_DIR}/config.env
ExecStart=${install_dir}/${BINARY_NAME}
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=${DATA_DIR}
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    return 0
}

# Create launchd plist (macOS)
create_launchd_plist() {
    local install_dir="$1"
    local plist_path="/Library/LaunchDaemons/com.adi.cocoon.plist"

    cat > "$plist_path" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.adi.cocoon</string>
    <key>ProgramArguments</key>
    <array>
        <string>${install_dir}/${BINARY_NAME}</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>SIGNALING_SERVER_URL</key>
        <string>\${SIGNALING_URL}</string>
        <key>COCOON_SECRET</key>
        <string>\${COCOON_SECRET}</string>
        <key>COCOON_SETUP_TOKEN</key>
        <string>\${SETUP_TOKEN}</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/cocoon.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/cocoon.error.log</string>
</dict>
</plist>
EOF

    return 0
}

main() {
    local setup_token="$1"

    echo ""
    printf "${BLUE}Cocoon Installer${NC}\n"
    echo ""

    # Validate setup token
    if [ -z "$setup_token" ]; then
        echo "Usage: curl -fsSL https://adi.the-ihor.com/family/cocoon/install.sh | sh -s -- <setup_token>"
        echo ""
        echo "Get your setup token from: https://app.adi.the-ihor.com/servers/new"
        echo ""
        error "Setup token is required"
    fi

    # Basic JWT structure check
    token_parts=$(echo "$setup_token" | tr '.' '\n' | wc -l)
    if [ "$token_parts" -ne 3 ]; then
        error "Invalid setup token format (expected JWT)"
    fi

    info "Setup token validated"

    # Check root
    check_root

    # Detect platform
    local os=$(detect_os)
    local arch=$(detect_arch)
    local target=$(get_target "$os" "$arch")

    info "Detected platform: $target"

    # Determine version
    info "Fetching latest version"
    local version=$(fetch_latest_version)
    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
    fi

    info "Installing version: $version"

    # Determine install directory
    local install_dir="${INSTALL_DIR:-/usr/local/bin}"
    mkdir -p "$install_dir"

    info "Install directory: $install_dir"

    # Create config and data directories
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$DATA_DIR"
    chmod 700 "$CONFIG_DIR"
    chmod 700 "$DATA_DIR"

    # Construct download URL
    local archive_name="cocoon-${version}-${target}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

    # Create temp directory
    local temp_dir=$(mktemp -d)
    trap "rm -rf '$temp_dir'" EXIT

    # Download archive
    local archive_path="$temp_dir/$archive_name"
    download "$download_url" "$archive_path"

    # Extract
    info "Extracting archive"
    tar -xzf "$archive_path" -C "$temp_dir"

    # Install binary
    local binary_path="$temp_dir/$BINARY_NAME"
    if [ ! -f "$binary_path" ]; then
        error "Binary not found in archive"
    fi

    chmod +x "$binary_path"
    mv "$binary_path" "$install_dir/$BINARY_NAME"

    success "Installed $BINARY_NAME to $install_dir/$BINARY_NAME"

    # Generate secret
    info "Generating cryptographic secret"
    local secret=$(generate_secret)

    # Determine cocoon name
    local cocoon_name="${COCOON_NAME:-$(hostname)}"

    # Determine signaling URL
    local signaling_url="${SIGNALING_URL:-wss://signal.adi.the-ihor.com/ws}"

    # Create config file
    info "Creating configuration"
    cat > "$CONFIG_DIR/config.env" << EOF
# Cocoon Configuration
# Generated by install-cocoon.sh

# Signaling server URL
SIGNALING_SERVER_URL=${signaling_url}

# Cryptographic secret for persistent device ID
# DO NOT SHARE THIS - it grants access to this cocoon
COCOON_SECRET=${secret}

# Setup token for auto-claiming ownership
COCOON_SETUP_TOKEN=${setup_token}

# Display name for this cocoon
COCOON_NAME=${cocoon_name}
EOF

    chmod 600 "$CONFIG_DIR/config.env"
    success "Configuration saved to $CONFIG_DIR/config.env"

    # Create service
    if [ "$NO_SYSTEMD" != "1" ]; then
        case "$os" in
            linux)
                if create_systemd_service "$install_dir"; then
                    info "Created systemd service"

                    # Enable and start
                    systemctl enable cocoon
                    systemctl start cocoon

                    success "Cocoon service started"
                fi
                ;;
            darwin)
                if create_launchd_plist "$install_dir"; then
                    info "Created launchd plist"

                    # Load and start
                    launchctl load /Library/LaunchDaemons/com.adi.cocoon.plist

                    success "Cocoon service started"
                fi
                ;;
        esac
    else
        warn "Skipping service creation (NO_SYSTEMD=1)"
    fi

    # Show status
    echo ""
    success "Cocoon installed successfully!"
    echo ""
    printf "  Name:     ${CYAN}%s${NC}\n" "$cocoon_name"
    printf "  Binary:   ${CYAN}%s${NC}\n" "$install_dir/$BINARY_NAME"
    printf "  Config:   ${CYAN}%s${NC}\n" "$CONFIG_DIR/config.env"
    printf "  Server:   ${CYAN}%s${NC}\n" "$signaling_url"
    echo ""

    # Show service status
    case "$os" in
        linux)
            if command -v systemctl >/dev/null 2>&1; then
                echo "Check status:"
                printf "  ${CYAN}systemctl status cocoon${NC}\n"
                printf "  ${CYAN}journalctl -u cocoon -f${NC}\n"
            fi
            ;;
        darwin)
            echo "Check status:"
            printf "  ${CYAN}launchctl list | grep cocoon${NC}\n"
            printf "  ${CYAN}tail -f /var/log/cocoon.log${NC}\n"
            ;;
    esac

    echo ""
    echo "Your cocoon should appear in your dashboard shortly."
    echo ""
}

main "$@"
