#!/bin/bash
# ADI Plugins Release Script
# Usage: ./scripts/release-plugins.sh [version] [plugin-name]
# Example: ./scripts/release-plugins.sh 0.8.4
# Example: ./scripts/release-plugins.sh 0.8.4 adi-tasks-plugin

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Load libraries
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/common.sh"

# Configuration
REGISTRY_URL=$(require_value "${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}" "ADI_REGISTRY_URL not set")

# Core plugins
PLUGINS=(
    "adi-tasks-plugin:adi.tasks:ADI Tasks:core"
    "adi-agent-loop-plugin:adi.agent-loop:ADI Agent Loop:core"
    "adi-indexer-plugin:adi.indexer:ADI Indexer:core"
    "adi-knowledgebase-plugin:adi.knowledgebase:ADI Knowledgebase:core"
    # Language plugins
    "adi-lang-rust:adi.lang.rust:Rust Language Support:language"
    "adi-lang-python:adi.lang.python:Python Language Support:language"
    "adi-lang-typescript:adi.lang.typescript:TypeScript Language Support:language"
    "adi-lang-cpp:adi.lang.cpp:C++ Language Support:language"
    "adi-lang-go:adi.lang.go:Go Language Support:language"
    "adi-lang-java:adi.lang.java:Java Language Support:language"
    "adi-lang-csharp:adi.lang.csharp:C# Language Support:language"
    "adi-lang-ruby:adi.lang.ruby:Ruby Language Support:language"
    "adi-lang-php:adi.lang.php:PHP Language Support:language"
    "adi-lang-swift:adi.lang.swift:Swift Language Support:language"
    "adi-lang-lua:adi.lang.lua:Lua Language Support:language"
)

# =============================================================================
# Main Release Flow
# =============================================================================

main() {
    # Get current version from first plugin's plugin.toml
    local current_version
    current_version=$(grep '^version' "$ROOT_DIR/crates/adi-tasks-plugin/plugin.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    info "Current version: v$current_version"

    # Get version from argument or prompt
    local version="${1:-}"
    local specific_plugin="${2:-}"

    if [ -z "$version" ]; then
        read -p "Enter new version (or press Enter for v$current_version): " version
        version="${version:-$current_version}"
    fi

    # Remove 'v' prefix if present
    version=$(normalize_version "$version")

    # Update plugin.toml files if version changed
    if [ "$version" != "$current_version" ]; then
        info "Updating plugin.toml files to v$version..."
        for plugin_spec in "${PLUGINS[@]}"; do
            IFS=':' read -r crate_name _ _ _ <<< "$plugin_spec"
            local manifest="$ROOT_DIR/crates/$crate_name/plugin.toml"
            # Handle new adi-lang structure
            case "$crate_name" in
                adi-lang-*) manifest="$ROOT_DIR/crates/adi-lang/${crate_name#adi-lang-}/plugin/plugin.toml" ;;
            esac
            if [ -f "$manifest" ]; then
                sed -i '' "s/^version = \"$current_version\"/version = \"$version\"/" "$manifest"
            fi
        done
        success "Updated plugin.toml files"
    fi

    info "Releasing plugins v$version"
    info "Registry: $REGISTRY_URL"
    echo ""

    # Check prerequisites
    ensure_command "cargo"
    ensure_command "curl"
    ensure_command "jq" "brew install jq"

    # Detect platform
    local platform
    platform=$(get_platform)
    local lib_ext
    lib_ext=$(get_lib_extension "$platform")

    info "Platform: $platform"
    echo ""

    # Create dist directory
    local dist_dir="$ROOT_DIR/dist/plugins-v$version"
    rm -rf "$dist_dir"
    ensure_dir "$dist_dir"

    cd "$ROOT_DIR"

    # Build and package plugins
    for plugin_spec in "${PLUGINS[@]}"; do
        IFS=':' read -r crate_name plugin_id plugin_name plugin_type <<< "$plugin_spec"

        # Skip if specific plugin requested and this isn't it
        if [ -n "$specific_plugin" ] && [ "$crate_name" != "$specific_plugin" ] && [ "$plugin_id" != "$specific_plugin" ]; then
            continue
        fi

        # Map crate name to directory path
        local crate_dir="$ROOT_DIR/crates/$crate_name"
        case "$crate_name" in
            adi-lang-*) crate_dir="$ROOT_DIR/crates/adi-lang/${crate_name#adi-lang-}/plugin" ;;
        esac

        # Check if crate exists
        if [ ! -d "$crate_dir" ]; then
            warn "Skipping $crate_name (not found at $crate_dir)"
            continue
        fi

        echo "=== $plugin_name ($plugin_id) ==="

        # Build
        info "Building $crate_name..."
        if ! cargo build --release -p "$crate_name" 2>/dev/null; then
            warn "Failed to build $crate_name"
            continue
        fi

        # Package
        local lib_name="lib${crate_name//-/_}.$lib_ext"
        local lib_path="$ROOT_DIR/target/release/$lib_name"
        local manifest_path="$crate_dir/plugin.toml"

        if [ ! -f "$lib_path" ]; then
            warn "Library not found: $lib_path"
            continue
        fi

        # Create package
        local pkg_dir
        pkg_dir=$(create_temp_dir)
        cp "$lib_path" "$pkg_dir/plugin.$lib_ext"
        cp "$manifest_path" "$pkg_dir/plugin.toml"

        local archive_name="${plugin_id}-v${version}-${platform}.tar.gz"
        create_tarball "$dist_dir/$archive_name" "$pkg_dir" "plugin.$lib_ext" "plugin.toml"

        success "Created $archive_name"
        echo ""
    done

    # Generate checksums
    info "Generating checksums..."
    cd "$dist_dir"
    generate_checksums "SHA256SUMS" *.tar.gz 2>/dev/null || true
    cd - >/dev/null

    # Show artifacts
    echo ""
    info "Release artifacts:"
    ls -lh "$dist_dir"
    echo ""

    # Confirm publish
    read -p "Publish to registry? [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Aborted. Artifacts are in $dist_dir"
        exit 0
    fi

    # Publish to registry
    echo ""
    for plugin_spec in "${PLUGINS[@]}"; do
        IFS=':' read -r crate_name plugin_id plugin_name plugin_type <<< "$plugin_spec"

        if [ -n "$specific_plugin" ] && [ "$crate_name" != "$specific_plugin" ] && [ "$plugin_id" != "$specific_plugin" ]; then
            continue
        fi

        local archive_name="${plugin_id}-v${version}-${platform}.tar.gz"
        local archive_path="$dist_dir/$archive_name"

        if [ ! -f "$archive_path" ]; then
            continue
        fi

        info "Publishing $plugin_id v$version for $platform..."

        local url="$REGISTRY_URL/v1/publish/plugins/$plugin_id/$version/$platform"
        url="$url?name=$(echo "$plugin_name" | sed 's/ /%20/g')&plugin_type=$plugin_type&author=ADI%20Team"

        local response
        response=$(curl -s --max-time 300 -X POST "$url" -F "file=@$archive_path")
        echo "$response" | jq . 2>/dev/null || echo "$response"

        success "Published $plugin_id"
        echo ""
    done

    success "Done! Published plugins v$version"
}

main "$@"
