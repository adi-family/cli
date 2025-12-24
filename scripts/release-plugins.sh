#!/bin/bash
# ADI Plugins Release Script
# Usage: ./scripts/release-plugins.sh [version] [plugin-name]
# Example: ./scripts/release-plugins.sh 0.8.4
# Example: ./scripts/release-plugins.sh 0.8.4 adi-tasks-plugin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
REGISTRY_URL="${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info() { printf "${CYAN}[INFO]${NC} %s\n" "$1"; }
success() { printf "${GREEN}[DONE]${NC} %s\n" "$1"; }
warn() { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; exit 1; }

# Core plugins
PLUGINS=(
    "adi-tasks-plugin:adi.tasks:ADI Tasks:core"
    "adi-agent-loop-plugin:adi.agent-loop:ADI Agent Loop:core"
    "adi-indexer-plugin:adi.indexer:ADI Indexer:core"
    "adi-knowledgebase-plugin:adi.knowledgebase:ADI Knowledgebase:core"
)

# Get version from argument or plugin.toml
VERSION="${1:-}"
SPECIFIC_PLUGIN="${2:-}"

if [ -z "$VERSION" ]; then
    # Try to get version from first plugin's plugin.toml
    VERSION=$(grep '^version' "$ROOT_DIR/crates/adi-tasks-plugin/plugin.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"

info "Releasing plugins v$VERSION"
info "Registry: $REGISTRY_URL"
echo ""

# Check prerequisites
command -v cargo >/dev/null 2>&1 || error "cargo not found"
command -v curl >/dev/null 2>&1 || error "curl not found"
command -v jq >/dev/null 2>&1 || error "jq not found. Install: brew install jq"

# Platform detection
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$os" in
        darwin)
            case "$arch" in
                arm64) echo "darwin-aarch64" ;;
                x86_64) echo "darwin-x86_64" ;;
            esac
            ;;
        linux)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                aarch64) echo "linux-aarch64" ;;
            esac
            ;;
    esac
}

get_lib_extension() {
    case "$1" in
        darwin-*) echo "dylib" ;;
        linux-*) echo "so" ;;
        windows-*) echo "dll" ;;
    esac
}

PLATFORM=$(detect_platform)
[ -z "$PLATFORM" ] && error "Unable to detect platform"

info "Platform: $PLATFORM"
echo ""

# Create dist directory
DIST_DIR="$ROOT_DIR/dist/plugins-v$VERSION"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

cd "$ROOT_DIR"

# Build and package plugins
for plugin_spec in "${PLUGINS[@]}"; do
    IFS=':' read -r crate_name plugin_id plugin_name plugin_type <<< "$plugin_spec"

    # Skip if specific plugin requested and this isn't it
    if [ -n "$SPECIFIC_PLUGIN" ] && [ "$crate_name" != "$SPECIFIC_PLUGIN" ] && [ "$plugin_id" != "$SPECIFIC_PLUGIN" ]; then
        continue
    fi

    # Check if crate exists
    if [ ! -d "$ROOT_DIR/crates/$crate_name" ]; then
        warn "Skipping $crate_name (not found)"
        continue
    fi

    echo "=== $plugin_name ($plugin_id) ==="

    # Build
    info "Building $crate_name..."
    cargo build --release -p "$crate_name" 2>/dev/null || {
        warn "Failed to build $crate_name"
        continue
    }

    # Package
    ext=$(get_lib_extension "$PLATFORM")
    lib_name="lib${crate_name//-/_}.$ext"
    lib_path="$ROOT_DIR/target/release/$lib_name"
    manifest_path="$ROOT_DIR/crates/$crate_name/plugin.toml"

    if [ ! -f "$lib_path" ]; then
        warn "Library not found: $lib_path"
        continue
    fi

    # Create package
    pkg_dir=$(mktemp -d)
    cp "$lib_path" "$pkg_dir/plugin.$ext"
    cp "$manifest_path" "$pkg_dir/plugin.toml"

    archive_name="${plugin_id}-v${VERSION}-${PLATFORM}.tar.gz"
    tar -czf "$DIST_DIR/$archive_name" -C "$pkg_dir" .
    rm -rf "$pkg_dir"

    success "Created $archive_name"
    echo ""
done

# Generate checksums
info "Generating checksums..."
cd "$DIST_DIR"
shasum -a 256 *.tar.gz > SHA256SUMS 2>/dev/null || true
cd - >/dev/null

# Show artifacts
echo ""
info "Release artifacts:"
ls -lh "$DIST_DIR"
echo ""

# Confirm publish
read -p "Publish to registry? [y/N] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    info "Aborted. Artifacts are in $DIST_DIR"
    exit 0
fi

# Publish to registry
echo ""
for plugin_spec in "${PLUGINS[@]}"; do
    IFS=':' read -r crate_name plugin_id plugin_name plugin_type <<< "$plugin_spec"

    if [ -n "$SPECIFIC_PLUGIN" ] && [ "$crate_name" != "$SPECIFIC_PLUGIN" ] && [ "$plugin_id" != "$SPECIFIC_PLUGIN" ]; then
        continue
    fi

    archive_name="${plugin_id}-v${VERSION}-${PLATFORM}.tar.gz"
    archive_path="$DIST_DIR/$archive_name"

    if [ ! -f "$archive_path" ]; then
        continue
    fi

    info "Publishing $plugin_id v$VERSION for $PLATFORM..."

    url="$REGISTRY_URL/v1/publish/plugins/$plugin_id/$VERSION/$PLATFORM"
    url="$url?name=$(echo "$plugin_name" | sed 's/ /%20/g')&plugin_type=$plugin_type&author=ADI%20Team"

    response=$(curl -s -X POST "$url" -F "file=@$archive_path")
    echo "$response" | jq . 2>/dev/null || echo "$response"

    success "Published $plugin_id"
    echo ""
done

success "Done! Published plugins v$VERSION"
