#!/usr/bin/env bash
# Cocoon Images Workflow - Direct execution wrapper
# This delegates to the main build script in crates/cocoon/scripts/
set -euo pipefail

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
    # Colors
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' BLUE='\033[0;34m' BOLD='\033[1m' NC='\033[0m'
    # Logging
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
fi

COCOON_DIR="$PROJECT_ROOT/crates/cocoon"
BUILD_SCRIPT="$COCOON_DIR/scripts/build-images.sh"

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Build and release cocoon Docker image variants.

OPTIONS:
    --all               Build all standard variants (default)
    --minimal           Build minimal variants (alpine, debian)
    --dev               Build dev variants (ubuntu, python, node)
    --variant NAME      Build specific variant
    --push              Push images to registry
    --tag TAG           Image tag (default: latest)
    --platform PLAT     Target platform (default: linux/amd64,linux/arm64)
    --no-cache          Build without cache
    --dry-run           Show what would be built
    -h, --help          Show this help

VARIANTS:
    alpine      Minimal (~15MB) - bash, curl, git, jq
    debian      Slim (~100MB) - build-essential, python3, vim
    ubuntu      Standard (~150MB) - nodejs, clang, cmake, sudo
    python      Python-focused (~180MB) - pip, poetry, uv, jupyter
    node        Node.js-focused (~200MB) - npm, yarn, pnpm, bun
    full        Everything (~500MB) - rust, go, docker, kubectl, terraform
    gpu         CUDA-enabled (~2GB) - cuda 12.4, cudnn, pytorch-ready
    custom      User-configurable

EXAMPLES:
    $0                              # Build all variants locally
    $0 --push                       # Build all + push to registry
    $0 --variant ubuntu --push      # Build only ubuntu + push
    $0 --minimal --tag v0.2.1       # Build alpine+debian with tag
    $0 --dev --platform linux/amd64 # Build dev variants for amd64 only
    $0 --dry-run                    # Show what would be built
EOF
    exit 0
}

# Check build script exists
if [[ ! -x "$BUILD_SCRIPT" ]]; then
    error "Build script not found: $BUILD_SCRIPT"
fi

# Pass all args to the build script
if [[ $# -eq 0 ]]; then
    # Interactive mode - show help
    echo ""
    echo -e "${BOLD}Cocoon Docker Images${NC}"
    echo ""
    echo "Run with --help for options, or use:"
    echo ""
    echo -e "  ${CYAN}adi workflow cocoon-images${NC}  # Interactive mode"
    echo -e "  ${CYAN}$0 --all --push${NC}             # Build all + push"
    echo ""
    exit 0
fi

# Delegate to build script
exec "$BUILD_SCRIPT" "$@"
