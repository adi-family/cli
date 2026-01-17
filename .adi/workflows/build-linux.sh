#!/usr/bin/env bash
# Build Linux binaries for Docker deployment
set -euo pipefail

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
    CWD="$PWD"
    # Colors
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' BOLD='\033[1m' DIM='\033[2m' NC='\033[0m'
    # Logging
    log() { echo -e "${BLUE:-\033[0;34m}[log]${NC} $1"; }
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
    # TTY
    has_tty() { [[ -t 0 ]] && [[ -t 1 ]]; }
    in_multiplexer() { [[ -n "$TMUX" ]] || [[ "$TERM" == screen* ]]; }
    supports_color() { [[ -t 1 ]]; }
    # Utils
    ensure_dir() { mkdir -p "$1"; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
    ensure_command() { check_command "$1" || error "$1 not found${2:+. Install: $2}"; }
    require_file() { [[ -f "$1" ]] || error "${2:-File not found: $1}"; }
    require_dir() { [[ -d "$1" ]] || error "${2:-Directory not found: $1}"; }
    require_value() { [[ -n "$1" ]] || error "${2:-Value required}"; echo "$1"; }
    require_env() { [[ -n "${!1}" ]] || error "Environment variable $1 not set"; echo "${!1}"; }
fi

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

# Get service configuration (crate-path:binary-names:features)
# Format: crate-path:binary-names[:features]
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
        hive) echo "crates/hive:hive" ;;
        cocoon) echo "crates/cocoon:cocoon:standalone" ;;
        llm-proxy) echo "crates/adi-api-proxy/http:adi-api-proxy,adi-api-proxy-migrate" ;;
        *) return 1 ;;
    esac
}

# All available services
ALL_SERVICES="adi-auth adi-platform-api adi-analytics-api adi-analytics-ingestion tarminal-signaling-server adi-plugin-registry flowmap-api hive cocoon llm-proxy"

build_service() {
    local service=$1
    local config
    if ! config=$(get_service_config "$service"); then
        error "Unknown service: $service"
        return 1
    fi

    # Parse config: crate-path:binary-names[:features]
    local crate_path="${config%%:*}"
    local rest="${config#*:}"
    local binaries="${rest%%:*}"
    local features=""
    if [[ "$rest" == *:* ]]; then
        features="${rest##*:}"
    fi

    info "Building $service (linux/amd64)"

    # Check if crate is standalone (has [workspace] or is excluded from root workspace)
    local crate_dir="$PROJECT_ROOT/$crate_path"
    local is_standalone=false

    # Check if crate has its own [workspace] section
    if [[ -f "$crate_dir/Cargo.toml" ]] && grep -q '^\[workspace\]' "$crate_dir/Cargo.toml"; then
        is_standalone=true
    fi

    # Check if crate is excluded from root workspace
    if [[ -f "$PROJECT_ROOT/Cargo.toml" ]] && grep -A 10 '^\[workspace\]' "$PROJECT_ROOT/Cargo.toml" | grep -q "\"$crate_path\""; then
        if grep -A 10 'exclude = \[' "$PROJECT_ROOT/Cargo.toml" | grep -q "\"$crate_path\""; then
            is_standalone=true
        fi
    fi

    # Build all binaries for this service
    IFS=',' read -ra BINS <<< "$binaries"
    for binary in "${BINS[@]}"; do
        info "  - $binary${features:+ (features: $features)}"

        # Set environment for musl cross-compilation
        export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc
        export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc

        # Build feature flag if specified
        local feature_flag=""
        if [[ -n "$features" ]]; then
            feature_flag="--features=$features"
        fi

        if [[ "$is_standalone" == "true" ]]; then
            # Standalone crate: build from crate directory
            cd "$crate_dir"
            cargo build --release --target x86_64-unknown-linux-musl --bin "$binary" $feature_flag
        else
            # Workspace member: build from project root with -p
            cd "$PROJECT_ROOT"
            # Read package name from Cargo.toml (may differ from directory name)
            local package_name=$(grep '^name = ' "$crate_dir/Cargo.toml" | head -1 | sed 's/name = "\(.*\)"/\1/')
            if [[ -z "$package_name" ]]; then
                error "Could not read package name from $crate_dir/Cargo.toml"
                return 1
            fi
            cargo build --release --target x86_64-unknown-linux-musl -p "$package_name" --bin "$binary" $feature_flag
        fi
    done

    # Copy binaries to release dir
    local release_dir="$PROJECT_ROOT/release/adi.the-ihor.com/$service"
    if [[ -d "$release_dir" ]]; then
        for binary in "${BINS[@]}"; do
            if [[ "$is_standalone" == "true" ]]; then
                # Standalone crate: binary is in crate's target dir
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
