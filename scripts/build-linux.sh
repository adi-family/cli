#!/usr/bin/env bash
# Build Linux binaries for Docker deployment
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}==>${NC} $*"; }
warn() { echo -e "${YELLOW}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; }

# Check if musl target is installed
if ! rustup target list --installed | grep -q x86_64-unknown-linux-musl; then
    warn "Installing x86_64-unknown-linux-musl target..."
    rustup target add x86_64-unknown-linux-musl
fi

# Check if musl-cross toolchain is installed (needed for C dependencies)
if ! command -v x86_64-linux-musl-gcc &> /dev/null; then
    error "musl-cross toolchain not found"
    error "Install with: brew install filosottile/musl-cross/musl-cross"
    exit 1
fi

# Get service configuration (crate-path:binary-names)
get_service_config() {
    local service=$1
    case "$service" in
        adi-auth) echo "crates/adi-auth:adi-auth-http,adi-auth-migrate" ;;
        adi-platform-api) echo "crates/adi-platform-api:adi-platform-api" ;;
        adi-analytics-api) echo "crates/adi-analytics-api:adi-analytics-api" ;;
        adi-analytics-ingestion) echo "crates/adi-analytics-ingestion:adi-analytics-ingestion" ;;
        tarminal-signaling-server) echo "crates/tarminal-signaling-server:tarminal-signaling" ;;
        adi-plugin-registry) echo "crates/adi-plugin-registry-http:adi-plugin-registry" ;;
        flowmap-api) echo "apps/flowmap-api:flowmap-api" ;;
        cocoon-manager) echo "crates/cocoon-manager:cocoon-manager" ;;
        *) return 1 ;;
    esac
}

# All available services
ALL_SERVICES="adi-auth adi-platform-api adi-analytics-api adi-analytics-ingestion tarminal-signaling-server adi-plugin-registry flowmap-api cocoon-manager"

build_service() {
    local service=$1
    local config
    if ! config=$(get_service_config "$service"); then
        error "Unknown service: $service"
        return 1
    fi

    local crate_path="${config%%:*}"
    local binaries="${config##*:}"

    info "Building $service (linux/amd64)"

    # Check if crate has its own workspace (has Cargo.toml with [workspace])
    local crate_dir="$PROJECT_ROOT/$crate_path"
    local has_workspace=false
    if [[ -f "$crate_dir/Cargo.toml" ]] && grep -q '^\[workspace\]' "$crate_dir/Cargo.toml"; then
        has_workspace=true
    fi

    # Build all binaries for this service
    IFS=',' read -ra BINS <<< "$binaries"
    for binary in "${BINS[@]}"; do
        info "  - $binary"

        # Set environment for musl cross-compilation
        export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc
        export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc

        if [[ "$has_workspace" == "true" ]]; then
            # Standalone workspace: build from crate directory
            cd "$crate_dir"
            cargo build --release --target x86_64-unknown-linux-musl --bin "$binary"
        else
            # Workspace member: build from project root with -p
            cd "$PROJECT_ROOT"
            # Read package name from Cargo.toml (may differ from directory name)
            local package_name=$(grep '^name = ' "$crate_dir/Cargo.toml" | head -1 | sed 's/name = "\(.*\)"/\1/')
            if [[ -z "$package_name" ]]; then
                error "Could not read package name from $crate_dir/Cargo.toml"
                return 1
            fi
            cargo build --release --target x86_64-unknown-linux-musl -p "$package_name" --bin "$binary"
        fi
    done

    # Copy binaries to release dir
    local release_dir="$PROJECT_ROOT/release/adi.the-ihor.com/$service"
    if [[ -d "$release_dir" ]]; then
        for binary in "${BINS[@]}"; do
            if [[ "$has_workspace" == "true" ]]; then
                # Standalone workspace: binary is in crate's target dir
                cp "$crate_dir/target/x86_64-unknown-linux-musl/release/$binary" "$release_dir/"
            else
                # Workspace member: binary is in workspace root target dir
                cp "$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/$binary" "$release_dir/"
            fi
            info "  ✓ Copied $binary to $release_dir/"
        done
    fi

    cd "$PROJECT_ROOT"
}

# Parse arguments
if [[ $# -eq 0 ]]; then
    # Build all services
    for service in $ALL_SERVICES; do
        build_service "$service"
    done
else
    # Build specific services
    for service in "$@"; do
        if ! get_service_config "$service" >/dev/null; then
            error "Unknown service: $service"
            error "Available: $ALL_SERVICES"
            exit 1
        fi
        build_service "$service"
    done
fi

info "✓ Build complete"
