#!/bin/bash
# Publish core plugins to the registry
# Usage: ./scripts/publish-plugins.sh [--dry-run] [plugin-name]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Configuration
REGISTRY_URL="${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}"
VERSION="${PLUGIN_VERSION:-0.8.3}"

# Core plugins to publish
PLUGINS=(
    "adi-indexer-plugin:adi.indexer:ADI Indexer:core"
    "adi-tasks-plugin:adi.tasks:ADI Tasks:core"
    "adi-knowledgebase-plugin:adi.knowledgebase:ADI Knowledgebase:core"
    "adi-agent-loop-plugin:adi.agent-loop:ADI Agent Loop:core"
)

# Platform mapping
get_rust_target() {
    case "$1" in
        darwin-aarch64) echo "aarch64-apple-darwin" ;;
        darwin-x86_64) echo "x86_64-apple-darwin" ;;
        linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
        windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
        *) echo "" ;;
    esac
}

get_lib_extension() {
    case "$1" in
        darwin-*) echo "dylib" ;;
        linux-*) echo "so" ;;
        windows-*) echo "dll" ;;
        *) echo "" ;;
    esac
}

get_lib_prefix() {
    case "$1" in
        windows-*) echo "" ;;
        *) echo "lib" ;;
    esac
}

# Detect current platform
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
            esac
            ;;
        msys*|mingw*|cygwin*)
            echo "windows-x86_64"
            ;;
    esac
}

# Build a plugin for a target
build_plugin() {
    local crate_name=$1
    local rust_target=$2
    local platform=$3

    echo "Building $crate_name for $platform..."

    if [ "$rust_target" = "$(rustc -vV | grep host | cut -d' ' -f2)" ]; then
        cargo build --release -p "$crate_name"
    else
        cargo build --release -p "$crate_name" --target "$rust_target"
    fi
}

# Package a plugin into tar.gz
package_plugin() {
    local crate_name=$1
    local plugin_id=$2
    local platform=$3
    local rust_target=$4

    local ext=$(get_lib_extension "$platform")
    local prefix=$(get_lib_prefix "$platform")
    local lib_name="${prefix}${crate_name//-/_}.$ext"

    local target_dir="$ROOT_DIR/target"
    if [ "$rust_target" != "$(rustc -vV | grep host | cut -d' ' -f2)" ]; then
        target_dir="$target_dir/$rust_target"
    fi

    local lib_path="$target_dir/release/$lib_name"
    local manifest_path="$ROOT_DIR/crates/$crate_name/plugin.toml"
    local output_dir="$ROOT_DIR/target/plugins/$plugin_id/$VERSION"
    local output_file="$output_dir/$platform.tar.gz"

    if [ ! -f "$lib_path" ]; then
        echo "Error: Library not found: $lib_path"
        return 1
    fi

    mkdir -p "$output_dir"

    # Create temporary directory for packaging
    local tmp_dir=$(mktemp -d)
    cp "$lib_path" "$tmp_dir/plugin.$ext"
    cp "$manifest_path" "$tmp_dir/plugin.toml"

    # Create tar.gz
    tar -czf "$output_file" -C "$tmp_dir" .
    rm -rf "$tmp_dir"

    echo "Created: $output_file"
    echo "$output_file"
}

# Publish a plugin to registry
publish_plugin() {
    local plugin_id=$1
    local plugin_name=$2
    local plugin_type=$3
    local platform=$4
    local archive_path=$5

    if [ "$DRY_RUN" = "1" ]; then
        echo "[DRY-RUN] Would publish $plugin_id v$VERSION for $platform"
        return 0
    fi

    echo "Publishing $plugin_id v$VERSION for $platform..."

    local url="$REGISTRY_URL/v1/publish/plugins/$plugin_id/$VERSION/$platform"
    url="$url?name=$(echo "$plugin_name" | sed 's/ /%20/g')&plugin_type=$plugin_type&author=ADI%20Team"

    curl -s -X POST "$url" \
        -F "file=@$archive_path" \
        | jq .

    echo "Published $plugin_id v$VERSION for $platform"
}

# Main
DRY_RUN=0
SPECIFIC_PLUGIN=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        *)
            SPECIFIC_PLUGIN=$1
            shift
            ;;
    esac
done

CURRENT_PLATFORM=$(detect_platform)
echo "Current platform: $CURRENT_PLATFORM"
echo "Registry URL: $REGISTRY_URL"
echo "Version: $VERSION"
echo ""

if [ -z "$CURRENT_PLATFORM" ]; then
    echo "Error: Unable to detect current platform"
    exit 1
fi

cd "$ROOT_DIR"

for plugin_spec in "${PLUGINS[@]}"; do
    IFS=':' read -r crate_name plugin_id plugin_name plugin_type <<< "$plugin_spec"

    if [ -n "$SPECIFIC_PLUGIN" ] && [ "$crate_name" != "$SPECIFIC_PLUGIN" ] && [ "$plugin_id" != "$SPECIFIC_PLUGIN" ]; then
        continue
    fi

    echo "=== Processing $plugin_name ($plugin_id) ==="

    rust_target=$(get_rust_target "$CURRENT_PLATFORM")

    # Build
    build_plugin "$crate_name" "$rust_target" "$CURRENT_PLATFORM"

    # Package
    archive_path=$(package_plugin "$crate_name" "$plugin_id" "$CURRENT_PLATFORM" "$rust_target")

    # Publish
    publish_plugin "$plugin_id" "$plugin_name" "$plugin_type" "$CURRENT_PLATFORM" "$archive_path"

    echo ""
done

echo "Done!"
