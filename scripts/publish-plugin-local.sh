#!/bin/bash
# Publish a plugin to the local registry for testing

set -e

# Usage info
usage() {
    echo "Usage: $0 <plugin-dir>"
    echo ""
    echo "Example:"
    echo "  $0 crates/cocoon"
    echo ""
    echo "This script builds and publishes a plugin to the local registry (http://localhost:8019)"
    exit 1
}

if [ $# -ne 1 ]; then
    usage
fi

PLUGIN_DIR="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REGISTRY_URL="${ADI_REGISTRY_URL:-http://localhost:8019}"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}info${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}done${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${NC} %s\n" "$1"
}

error() {
    printf "\033[0;31merror${NC} %s\n" "$1" >&2
    exit 1
}

# Verify plugin directory exists
cd "$PROJECT_DIR"
if [ ! -d "$PLUGIN_DIR" ]; then
    error "Plugin directory not found: $PLUGIN_DIR"
fi

if [ ! -f "$PLUGIN_DIR/plugin.toml" ]; then
    error "No plugin.toml found in $PLUGIN_DIR"
fi

cd "$PLUGIN_DIR"

# Read plugin info from plugin.toml
PLUGIN_ID=$(grep '^id = ' plugin.toml | sed 's/id = "\(.*\)"/\1/')
PLUGIN_VERSION=$(grep '^version = ' plugin.toml | sed 's/version = "\(.*\)"/\1/')

if [ -z "$PLUGIN_ID" ] || [ -z "$PLUGIN_VERSION" ]; then
    error "Could not read plugin ID or version from plugin.toml"
fi

info "Building plugin: $PLUGIN_ID v$PLUGIN_VERSION"

# Build the plugin
cargo build --release --lib 2>&1 | grep -E "Finished|error" || true

# Find the built library
LIB_NAME=$(echo "$PLUGIN_ID" | sed 's/\./-/g')
DYLIB_PATH="../../../target/release/lib${LIB_NAME}.dylib"
SO_PATH="../../../target/release/lib${LIB_NAME}.so"
DLL_PATH="../../../target/release/${LIB_NAME}.dll"

PLUGIN_LIB=""
if [ -f "$DYLIB_PATH" ]; then
    PLUGIN_LIB="$DYLIB_PATH"
    EXT="dylib"
elif [ -f "$SO_PATH" ]; then
    PLUGIN_LIB="$SO_PATH"
    EXT="so"
elif [ -f "$DLL_PATH" ]; then
    PLUGIN_LIB="$DLL_PATH"
    EXT="dll"
else
    error "Could not find built plugin library"
fi

info "Found plugin library: $PLUGIN_LIB"

# Create tarball
TARBALL="/tmp/${PLUGIN_ID}-${PLUGIN_VERSION}.tar.gz"
info "Creating tarball: $TARBALL"

tar czf "$TARBALL" \
    -C "$(dirname "$PLUGIN_LIB")" "$(basename "$PLUGIN_LIB")" \
    -C "$PWD" plugin.toml

# Publish to registry
info "Publishing to registry: $REGISTRY_URL"

RESPONSE=$(curl -s -w "\n%{http_code}" -X POST \
    -F "file=@$TARBALL" \
    -F "id=$PLUGIN_ID" \
    -F "version=$PLUGIN_VERSION" \
    "$REGISTRY_URL/v1/plugins/publish")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n -1)

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
    success "Published $PLUGIN_ID v$PLUGIN_VERSION to local registry"
    echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
else
    error "Failed to publish (HTTP $HTTP_CODE): $BODY"
fi

# Clean up
rm -f "$TARBALL"

success "Done! Install with: ADI_REGISTRY_URL=$REGISTRY_URL adi plugin install $PLUGIN_ID"
