#!/usr/bin/env bash
# Release services: Build Linux binaries + Docker images + push
set -euo pipefail

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
    CWD="$PWD"
    # Colors
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' BLUE='\033[0;34m' BOLD='\033[1m' DIM='\033[2m' NC='\033[0m'
    # Logging
    log() { echo -e "${BLUE}[log]${NC} $1"; }
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
fi

REGISTRY="docker-registry.the-ihor.com"

# Get service configuration (crate-path:binary-names:release-dir)
get_service_config() {
    local service=$1
    case "$service" in
        auth) echo "crates/auth:auth-http,auth-migrate:auth" ;;
        platform) echo "crates/platform:platform-http:platform" ;;
        analytics) echo "crates/analytics:analytics-http:analytics" ;;
        analytics-ingestion) echo "crates/analytics-ingestion:analytics-ingestion:analytics-ingestion" ;;
        signaling-server) echo "crates/signaling-server:signaling-server:signaling-server" ;;
        plugin-registry) echo "crates/plugin-registry:plugin-registry:plugin-registry" ;;
        flowmap-api) echo "apps/flowmap-api:flowmap-api:flowmap-api" ;;
        hive) echo "crates/hive/http:hive:hive" ;;
        cocoon) echo "crates/cocoon:cocoon:cocoon" ;;
        llm-proxy) echo "crates/llm-proxy/http:llm-proxy,llm-proxy-migrate:llm-proxy" ;;
        *) return 1 ;;
    esac
}

# Get Docker image name for service
get_image_name() {
    local service=$1
    case "$service" in
        llm-proxy) echo "llm-proxy" ;;
        *) echo "$service" ;;
    esac
}

# All available services
ALL_SERVICES="auth platform analytics analytics-ingestion signaling-server plugin-registry flowmap-api hive cocoon llm-proxy"

usage() {
    cat <<EOF
Usage: $0 [OPTIONS] [SERVICE...]

Build and release service images using cross-compilation + Docker.

OPTIONS:
    --push              Push images to registry ($REGISTRY)
    --tag TAG           Additional tag (default: latest)
    --skip-build        Skip Linux binary build (use existing)
    -h, --help          Show this help

SERVICES:
    auth                        Auth service
    platform-api                Platform API
    analytics-api               Analytics API
    analytics-ingestion         Analytics ingestion
    signaling-server            Signaling server
    plugin-registry             Plugin registry
    flowmap-api                 FlowMap API
    hive                        Hive (cocoon orchestration)
    cocoon                      Cocoon worker (Docker image)
    llm-proxy                   LLM API Proxy
    all                         All services (default)

EXAMPLES:
    $0 llm-proxy                # Build llm-proxy image
    $0 all --push               # Build + push all services
    $0 auth --tag v1.0.0        # Build with custom tag
    $0 llm-proxy --skip-build   # Docker only (binaries already built)
EOF
    exit 0
}

# Parse arguments
PUSH=false
TAG="latest"
SKIP_BUILD=false
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
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        all)
            SERVICES_TO_BUILD=($ALL_SERVICES)
            shift
            ;;
        *)
            if get_service_config "$1" >/dev/null 2>&1; then
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

echo ""
echo -e "${BOLD}=== ADI Service Release ===${NC}"
echo ""
info "Services: ${SERVICES_TO_BUILD[*]}"
info "Registry: $REGISTRY"
info "Tag: $TAG"
info "Push: $PUSH"
echo ""

# Step 1: Build Linux binaries
if [[ "$SKIP_BUILD" == "false" ]]; then
    echo -e "${BOLD}Step 1: Building Linux binaries...${NC}"
    "$WORKFLOWS_DIR/build-linux.sh" "${SERVICES_TO_BUILD[@]}"
    echo ""
else
    warn "Skipping binary build (--skip-build)"
    echo ""
fi

# Step 2: Build Docker images
echo -e "${BOLD}Step 2: Building Docker images...${NC}"
for service in "${SERVICES_TO_BUILD[@]}"; do
    config=$(get_service_config "$service")
    release_dir_name="${config##*:}"
    release_dir="$PROJECT_ROOT/release/adi.the-ihor.com/$release_dir_name"
    image_name=$(get_image_name "$service")

    if [[ ! -d "$release_dir" ]]; then
        warn "No release dir for $service at $release_dir, skipping"
        continue
    fi

    if [[ ! -f "$release_dir/Dockerfile" ]]; then
        warn "No Dockerfile for $service, skipping"
        continue
    fi

    info "Building $image_name..."

    # Read version from release dir Cargo.toml
    version="latest"
    if [[ -f "$release_dir/Cargo.toml" ]]; then
        version=$(grep '^version' "$release_dir/Cargo.toml" | head -1 | cut -d'"' -f2)
    fi

    cd "$release_dir"

    # Build image (always amd64 since binaries are cross-compiled for linux/amd64)
    docker build \
        --platform linux/amd64 \
        -t "$REGISTRY/$image_name:$TAG" \
        -t "$REGISTRY/$image_name:$version" \
        .

    success "Built $REGISTRY/$image_name:$TAG"

    # Push if requested
    if [[ "$PUSH" == "true" ]]; then
        info "Pushing $image_name..."
        docker push "$REGISTRY/$image_name:$TAG"
        docker push "$REGISTRY/$image_name:$version"
        success "Pushed $REGISTRY/$image_name:$TAG and :$version"
    fi

    echo ""
done

cd "$PROJECT_ROOT"

# Summary
echo -e "${BOLD}=== Release Summary ===${NC}"
echo ""
for service in "${SERVICES_TO_BUILD[@]}"; do
    image_name=$(get_image_name "$service")
    echo -e "  ${GREEN}✓${NC} $REGISTRY/$image_name:$TAG"
done
echo ""

if [[ "$PUSH" == "true" ]]; then
    success "Images pushed to $REGISTRY"
else
    info "Run with --push to push images to registry"
fi
