#!/bin/bash
# Publish a plugin to the local registry for testing

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Load libraries
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/common.sh"

# Configuration
REGISTRY_URL="${ADI_REGISTRY_URL:-http://localhost:8019}"

# =============================================================================
# Main Flow
# =============================================================================

usage() {
    echo "Usage: $0 <plugin-dir>"
    echo ""
    echo "Example:"
    echo "  $0 crates/cocoon"
    echo ""
    echo "This script builds and publishes a plugin to the local registry (http://localhost:8019)"
    exit 1
}

main() {
    local plugin_dir="$1"

    if [ -z "$plugin_dir" ]; then
        usage
    fi

    # Verify plugin directory exists
    cd "$PROJECT_DIR"
    if [ ! -d "$plugin_dir" ]; then
        error "Plugin directory not found: $plugin_dir"
    fi

    if [ ! -f "$plugin_dir/plugin.toml" ]; then
        error "No plugin.toml found in $plugin_dir"
    fi

    cd "$plugin_dir"

    # Read plugin info from plugin.toml (from [plugin] section only)
    local plugin_id=$(sed -n '/^\[plugin\]/,/^\[/{/^id = /p;}' plugin.toml | sed 's/id = "\(.*\)"/\1/' | tr -d '\n')
    local plugin_version=$(sed -n '/^\[plugin\]/,/^\[/{/^version = /p;}' plugin.toml | sed 's/version = "\(.*\)"/\1/' | tr -d '\n')

    if [ -z "$plugin_id" ] || [ -z "$plugin_version" ]; then
        error "Could not read plugin ID or version from plugin.toml"
    fi

    info "Building plugin: $plugin_id v$plugin_version"

    # Build the plugin
    cargo build --release --lib 2>&1 | grep -E "Finished|error" || true

    # Find the built library - use Cargo.toml package name, not plugin ID
    local package_name=$(grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/')
    local target_dir="$PROJECT_DIR/target/release"

    # Detect platform and library extension
    local platform=$(get_platform)
    local lib_ext=$(get_lib_extension "$platform")

    # Find plugin library
    local lib_name="lib${package_name//-/_}"
    local plugin_lib="$target_dir/${lib_name}.${lib_ext}"

    # Special case for Windows DLL (no lib prefix)
    if [ "$lib_ext" = "dll" ]; then
        plugin_lib="$target_dir/${package_name}.dll"
    fi

    if [ ! -f "$plugin_lib" ]; then
        error "Could not find built plugin library: $plugin_lib"
    fi

    info "Found plugin library: $plugin_lib"

    # Create tarball
    local temp_dir=$(create_temp_dir)
    local tarball="$temp_dir/${plugin_id}-${plugin_version}.tar.gz"

    info "Creating tarball: $tarball"

    tar czf "$tarball" \
        -C "$(dirname "$plugin_lib")" "$(basename "$plugin_lib")" \
        -C "$PWD" plugin.toml

    # Read plugin metadata from plugin.toml
    local plugin_name=$(grep '^name = ' plugin.toml | sed 's/name = "\(.*\)"/\1/')
    local plugin_desc=$(grep '^description = ' plugin.toml | sed 's/description = "\(.*\)"/\1/')
    local plugin_author=$(grep '^author = ' plugin.toml | sed 's/author = "\(.*\)"/\1/')
    local plugin_type=$(grep '^type = ' plugin.toml | sed 's/type = "\(.*\)"/\1/' || echo "extension")

    # Publish to registry
    info "Publishing to registry: $REGISTRY_URL"
    info "Platform: $platform"

    local response=$(curl -s -w "\n%{http_code}" -X POST \
        "$REGISTRY_URL/v1/publish/plugins/$plugin_id/$plugin_version/$platform?name=$plugin_name&description=$plugin_desc&plugin_type=$plugin_type&author=$plugin_author" \
        -F "file=@$tarball")

    local http_code=$(echo "$response" | tail -n 1)
    local body=$(echo "$response" | sed '$d')

    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        success "Published $plugin_id v$plugin_version to local registry"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        error "Failed to publish (HTTP $http_code): $body"
    fi

    success "Done! Install with: ADI_REGISTRY_URL=$REGISTRY_URL adi plugin install $plugin_id"
}

main "$@"
