#!/bin/bash
# =============================================================================
# Example Script Using ADI Bash Libraries
# =============================================================================
# Demonstrates usage of all library functions
#
# Usage: ./scripts/lib/example.sh
# =============================================================================

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load libraries (order doesn't matter due to auto-loading dependencies)
source "$SCRIPT_DIR/log.sh"
source "$SCRIPT_DIR/platform.sh"
source "$SCRIPT_DIR/download.sh"
source "$SCRIPT_DIR/github.sh"
source "$SCRIPT_DIR/common.sh"

# =============================================================================
# Example Usage
# =============================================================================

main() {
    echo ""
    echo -e "${BOLD}ADI Bash Libraries Example${NC}"
    echo ""

    # -------------------------------------------------------------------------
    # Logging Examples
    # -------------------------------------------------------------------------
    log "This is a generic log message (blue)"
    info "This is an info message (cyan)"
    success "This is a success message (green)"
    warn "This is a warning message (yellow)"
    # error "This would exit the script (red)"
    echo ""

    # -------------------------------------------------------------------------
    # Platform Detection Examples
    # -------------------------------------------------------------------------
    info "Detecting platform..."
    local os=$(detect_os)
    local arch=$(detect_arch)
    local target=$(get_target "$os" "$arch")
    local platform=$(get_platform)
    local lib_ext=$(get_lib_extension "$platform")
    local archive_ext=$(get_archive_extension "$os")

    echo "  OS: ${CYAN}$os${NC}"
    echo "  Architecture: ${CYAN}$arch${NC}"
    echo "  Target Triple: ${CYAN}$target${NC}"
    echo "  Platform: ${CYAN}$platform${NC}"
    echo "  Library Extension: ${CYAN}$lib_ext${NC}"
    echo "  Archive Extension: ${CYAN}$archive_ext${NC}"
    echo ""

    # -------------------------------------------------------------------------
    # TTY Detection Examples
    # -------------------------------------------------------------------------
    info "Checking terminal capabilities..."
    if has_tty; then
        echo "  TTY: ${GREEN}available${NC}"
    else
        echo "  TTY: ${YELLOW}not available${NC}"
    fi

    if in_multiplexer; then
        echo "  Multiplexer: ${GREEN}detected (tmux/screen)${NC}"
    else
        echo "  Multiplexer: ${DIM}not detected${NC}"
    fi

    if supports_color; then
        echo "  Colors: ${GREEN}supported${NC}"
    else
        echo "  Colors: not supported"
    fi
    echo ""

    # -------------------------------------------------------------------------
    # Command Checking Examples
    # -------------------------------------------------------------------------
    info "Checking required commands..."
    if check_command "curl"; then
        echo "  curl: ${GREEN}found${NC}"
    else
        echo "  curl: ${RED}not found${NC}"
    fi

    if check_command "wget"; then
        echo "  wget: ${GREEN}found${NC}"
    else
        echo "  wget: ${RED}not found${NC}"
    fi

    if check_command "jq"; then
        echo "  jq: ${GREEN}found${NC}"
    else
        echo "  jq: ${YELLOW}not found (optional)${NC}"
    fi
    echo ""

    # -------------------------------------------------------------------------
    # GitHub API Examples
    # -------------------------------------------------------------------------
    info "Fetching latest ADI CLI version from GitHub..."
    local latest_version=$(fetch_latest_version "adi-family/adi-cli" 2>/dev/null || echo "unknown")
    echo "  Latest version: ${CYAN}$latest_version${NC}"
    echo ""

    # -------------------------------------------------------------------------
    # Version Management Examples
    # -------------------------------------------------------------------------
    info "Version normalization examples..."
    local v1=$(normalize_version "v1.2.3")
    local v2=$(ensure_v_prefix "1.2.3")
    echo "  normalize_version 'v1.2.3': ${CYAN}$v1${NC}"
    echo "  ensure_v_prefix '1.2.3': ${CYAN}$v2${NC}"
    echo ""

    # -------------------------------------------------------------------------
    # Cryptography Examples
    # -------------------------------------------------------------------------
    info "Generating cryptographic secret..."
    local secret=$(generate_secret 32)
    echo "  Secret (first 16 chars): ${CYAN}${secret:0:16}...${NC}"
    echo ""

    # -------------------------------------------------------------------------
    # Directory Examples
    # -------------------------------------------------------------------------
    info "Creating temporary directory..."
    local temp_dir=$(create_temp_dir)
    echo "  Temp dir: ${CYAN}$temp_dir${NC}"
    echo "  (will be auto-cleaned on exit)"
    echo ""

    # -------------------------------------------------------------------------
    # Download Example (commented to avoid actual download)
    # -------------------------------------------------------------------------
    # info "Download example (commented out):"
    # echo "  download 'https://example.com/file.tar.gz' '/tmp/file.tar.gz'"
    # echo ""

    # -------------------------------------------------------------------------
    # Summary
    # -------------------------------------------------------------------------
    success "All library functions demonstrated successfully!"
    echo ""
    echo "See scripts/lib/README.md for detailed documentation"
    echo ""
}

# Run main
main "$@"
