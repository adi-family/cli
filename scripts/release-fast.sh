#!/usr/bin/env bash
# Fast release: Build natively, copy to minimal containers
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${GREEN}==>${NC} $*"; }
warn() { echo -e "${YELLOW}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; }
step() { echo -e "${BLUE}==>${NC} $*"; }

usage() {
    cat <<EOF
Usage: $0 [OPTIONS] [SERVICE...]

Fast release builds using native cross-compilation + minimal containers.

OPTIONS:
    --push              Push images to registry
    --tag TAG           Additional tag (default: latest)
    -h, --help          Show this help

SERVICES:
    adi-auth                    Auth service
    adi-platform-api            Platform API
    adi-analytics-api           Analytics API
    adi-analytics-ingestion     Analytics ingestion
    tarminal-signaling-server   Signaling server
    adi-plugin-registry         Plugin registry
    flowmap-api                 FlowMap API
    cocoon-manager              Cocoon manager
    all                         All services (default)

EXAMPLES:
    $0 adi-auth                 # Build auth service
    $0 all --push               # Build + push all services
    $0 adi-auth --tag v1.0.0    # Build with custom tag

PERFORMANCE:
    This script uses cross-compilation for 10-20x faster builds:
    - Native build on Mac (vs Docker emulation)
    - Minimal Alpine containers (5MB vs 1GB)
    - Persistent Cargo cache
EOF
    exit 0
}

# Get service crate path
get_service_crate() {
    local service=$1
    case "$service" in
        adi-auth) echo "crates/adi-auth" ;;
        adi-platform-api) echo "crates/adi-platform-api" ;;
        adi-analytics-api) echo "crates/adi-analytics-api" ;;
        adi-analytics-ingestion) echo "crates/adi-analytics-ingestion" ;;
        tarminal-signaling-server) echo "crates/tarminal-signaling-server" ;;
        adi-plugin-registry) echo "crates/adi-plugin-registry-http" ;;
        flowmap-api) echo "apps/flowmap-api" ;;
        cocoon-manager) echo "crates/cocoon-manager" ;;
        *) return 1 ;;
    esac
}

# All available services
ALL_SERVICES="adi-auth adi-platform-api adi-analytics-api adi-analytics-ingestion tarminal-signaling-server adi-plugin-registry flowmap-api cocoon-manager"

# Parse arguments
PUSH=false
TAG="latest"
SERVICES_TO_BUILD=()

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        all)
            SERVICES_TO_BUILD=($ALL_SERVICES)
            shift
            ;;
        *)
            if get_service_crate "$1" >/dev/null; then
                SERVICES_TO_BUILD+=("$1")
                shift
            else
                error "Unknown service: $1"
                echo "Available: $ALL_SERVICES all"
                exit 1
            fi
            ;;
    esac
done

# Default to all services
if [[ ${#SERVICES_TO_BUILD[@]} -eq 0 ]]; then
    SERVICES_TO_BUILD=($ALL_SERVICES)
fi

# Step 1: Build Linux binaries
step "Step 1: Building Linux binaries..."
"$SCRIPT_DIR/build-linux.sh" "${SERVICES_TO_BUILD[@]}"

# Step 2: Build Docker images
step "Step 2: Building Docker images..."
for service in "${SERVICES_TO_BUILD[@]}"; do
    release_dir="$PROJECT_ROOT/release/adi.the-ihor.com/$service"

    if [[ ! -d "$release_dir" ]]; then
        warn "No release dir for $service, skipping"
        continue
    fi

    # Use Dockerfile.fast if it exists, otherwise Dockerfile
    dockerfile="Dockerfile.fast"
    if [[ ! -f "$release_dir/$dockerfile" ]]; then
        dockerfile="Dockerfile"
    fi

    info "Building $service ($dockerfile)"

    # Read version from Cargo.toml
    crate_path=$(get_service_crate "$service")
    version=$(grep '^version' "$PROJECT_ROOT/$crate_path/Cargo.toml" | head -1 | cut -d'"' -f2)

    cd "$release_dir"
    docker build -f "$dockerfile" -t "adi-family/$service:$TAG" -t "adi-family/$service:$version" .

    if [[ "$PUSH" == "true" ]]; then
        info "Pushing $service:$TAG and $service:$version"
        docker push "adi-family/$service:$TAG"
        docker push "adi-family/$service:$version"
    fi
done

info "✓ Release complete!"

if [[ "$PUSH" == "true" ]]; then
    info "Images pushed to registry"
else
    info "Run with --push to push images to registry"
fi
