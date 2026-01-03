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

# Convert to bash for library support
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
REPO="adi-family/cocoon"
BINARY_NAME="cocoon"
CONFIG_DIR="/etc/cocoon"
DATA_DIR="/var/lib/cocoon"

# =============================================================================
# Service Creation Functions
# =============================================================================

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
    local signaling_url="$2"
    local secret="$3"
    local setup_token="$4"
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
        <string>${signaling_url}</string>
        <key>COCOON_SECRET</key>
        <string>${secret}</string>
        <key>COCOON_SETUP_TOKEN</key>
        <string>${setup_token}</string>
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

# =============================================================================
# Main Installation Flow
# =============================================================================

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
    local token_parts
    token_parts=$(echo "$setup_token" | tr '.' '\n' | wc -l)
    if [ "$token_parts" -ne 3 ]; then
        error "Invalid setup token format (expected JWT)"
    fi

    info "Setup token validated"

    # Check root
    check_root

    # Detect platform
    local os
    os=$(detect_os)
    local arch
    arch=$(detect_arch)
    local target
    target=$(get_target_musl "$os" "$arch")  # Use musl for static linking

    info "Detected platform: $target"

    # Fetch latest version
    info "Fetching latest version"
    local version
    version=$(fetch_latest_version "$REPO")
    [ -z "$version" ] && error "Failed to fetch latest version"

    info "Installing version: $version"

    # Determine install directory
    local install_dir="${INSTALL_DIR:-/usr/local/bin}"
    ensure_dir "$install_dir"

    info "Install directory: $install_dir"

    # Create config and data directories
    ensure_dir "$CONFIG_DIR"
    ensure_dir "$DATA_DIR"
    chmod 700 "$CONFIG_DIR"
    chmod 700 "$DATA_DIR"

    # Download and install binary
    local temp_dir
    temp_dir=$(create_temp_dir)
    local archive_name="cocoon-${version}-${target}.tar.gz"

    download_github_asset "$REPO" "$version" "$archive_name" "$temp_dir/$archive_name"
    extract_archive "$temp_dir/$archive_name" "$temp_dir"

    local binary_path="$temp_dir/$BINARY_NAME"
    [ ! -f "$binary_path" ] && error "Binary not found in archive"

    chmod +x "$binary_path"
    mv "$binary_path" "$install_dir/$BINARY_NAME"

    success "Installed $BINARY_NAME to $install_dir/$BINARY_NAME"

    # Generate secret
    info "Generating cryptographic secret"
    local secret
    secret=$(generate_secret)

    # Determine configuration
    local cocoon_name="${COCOON_NAME:-$(hostname)}"
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
                    systemctl enable cocoon
                    systemctl start cocoon
                    success "Cocoon service started"
                fi
                ;;
            darwin)
                if create_launchd_plist "$install_dir" "$signaling_url" "$secret" "$setup_token"; then
                    info "Created launchd plist"
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

    # Show service status commands
    case "$os" in
        linux)
            if check_command systemctl; then
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
