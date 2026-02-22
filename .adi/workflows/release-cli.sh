#!/usr/bin/env bash
# Release adi CLI: Build cross-platform binaries and create GitHub release
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

REPO="adi-family/adi-cli"
CLI_CRATE="$PROJECT_ROOT/crates/cli"
DIST_DIR="$PROJECT_ROOT/dist/cli"

# Targets to build
TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-unknown-linux-musl"
    # "x86_64-pc-windows-gnu"  # Disabled: lib-daemon-core uses Unix-only APIs
)

usage() {
    cat <<EOF
Usage: $0 --version VERSION [OPTIONS]

Build adi CLI for multiple platforms and create a GitHub release.

OPTIONS:
    --version VERSION   Version to release (required, e.g., 1.0.1)
    --title TITLE       Release title (default: "adi v{version}")
    --draft             Create as draft release
    --skip-build        Skip building binaries (use existing in dist/)
    -h, --help          Show this help

TARGETS:
    - aarch64-apple-darwin (macOS ARM)
    - x86_64-unknown-linux-musl (Linux x86_64)
    - x86_64-pc-windows-gnu (Windows x86_64)

EXAMPLES:
    $0 --version 1.0.1
    $0 --version 1.0.1 --title "Bug fixes"
    $0 --version 1.0.1 --draft
EOF
    exit 0
}

# Parse arguments
VERSION=""
TITLE=""
DRAFT=false
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
        --version)
            VERSION="$2"
            shift 2
            ;;
        --title)
            TITLE="$2"
            shift 2
            ;;
        --draft)
            DRAFT=true
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

if [[ -z "$VERSION" ]]; then
    error "Version is required. Use --version X.Y.Z"
fi

if [[ -z "$TITLE" ]]; then
    TITLE="adi v$VERSION"
fi

TAG="v$VERSION"

echo ""
echo -e "${BOLD}=== ADI CLI Release ===${NC}"
echo ""
info "Version: $VERSION"
info "Tag: $TAG"
info "Title: $TITLE"
info "Draft: $DRAFT"
echo ""

# Create dist directory
mkdir -p "$DIST_DIR"

# Step 1: Build binaries for each target
if [[ "$SKIP_BUILD" == "false" ]]; then
    echo -e "${BOLD}Step 1: Building binaries...${NC}"
    
    for target in "${TARGETS[@]}"; do
        info "Building for $target..."
        
        # Check if target is installed
        if ! rustup target list --installed | grep -q "$target"; then
            warn "Target $target not installed, installing..."
            rustup target add "$target"
        fi
        
        # Build
        cargo build --release --target "$target" -p cli
        
        # Determine binary name and archive format
        if [[ "$target" == *"windows"* ]]; then
            binary_name="adi.exe"
            archive_name="adi-v$VERSION-$target.zip"
        else
            binary_name="adi"
            archive_name="adi-v$VERSION-$target.tar.gz"
        fi
        
        # Copy binary to dist
        src_binary="$PROJECT_ROOT/target/$target/release/$binary_name"
        if [[ ! -f "$src_binary" ]]; then
            error "Binary not found: $src_binary"
        fi
        
        # Create archive
        archive_path="$DIST_DIR/$archive_name"
        
        if [[ "$target" == *"windows"* ]]; then
            # Create zip for Windows
            cd "$PROJECT_ROOT/target/$target/release"
            zip -j "$archive_path" "$binary_name"
            cd "$PROJECT_ROOT"
        else
            # Create tar.gz for Unix
            tar -czf "$archive_path" -C "$PROJECT_ROOT/target/$target/release" "$binary_name"
        fi
        
        success "Created $archive_name"
    done
    echo ""
else
    warn "Skipping build (--skip-build)"
    echo ""
fi

# Step 2: Verify archives exist
echo -e "${BOLD}Step 2: Verifying archives...${NC}"
ASSETS=()
for target in "${TARGETS[@]}"; do
    if [[ "$target" == *"windows"* ]]; then
        archive_name="adi-v$VERSION-$target.zip"
    else
        archive_name="adi-v$VERSION-$target.tar.gz"
    fi
    
    archive_path="$DIST_DIR/$archive_name"
    if [[ ! -f "$archive_path" ]]; then
        error "Archive not found: $archive_path"
    fi
    
    ASSETS+=("$archive_path")
    info "Found: $archive_name ($(du -h "$archive_path" | cut -f1))"
done
echo ""

# Step 3: Create GitHub release
echo -e "${BOLD}Step 3: Creating GitHub release...${NC}"

# Check if release already exists
if gh release view "$TAG" -R "$REPO" &>/dev/null; then
    warn "Release $TAG already exists, deleting..."
    gh release delete "$TAG" -R "$REPO" --yes
fi

# Build release command
RELEASE_CMD="gh release create $TAG -R $REPO --title \"$TITLE\""

if [[ "$DRAFT" == "true" ]]; then
    RELEASE_CMD="$RELEASE_CMD --draft"
fi

# Add assets
for asset in "${ASSETS[@]}"; do
    RELEASE_CMD="$RELEASE_CMD \"$asset\""
done

# Create release
info "Creating release $TAG..."
eval "$RELEASE_CMD"

success "Release created!"
echo ""

# Summary
echo -e "${BOLD}=== Release Summary ===${NC}"
echo ""
echo -e "  ${GREEN}✓${NC} Version: $VERSION"
echo -e "  ${GREEN}✓${NC} Tag: $TAG"
echo -e "  ${GREEN}✓${NC} Assets:"
for asset in "${ASSETS[@]}"; do
    echo -e "      - $(basename "$asset")"
done
echo ""
echo -e "  ${CYAN}→${NC} https://github.com/$REPO/releases/tag/$TAG"
echo ""

success "adi CLI v$VERSION released!"
