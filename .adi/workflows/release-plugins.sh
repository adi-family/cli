#!/bin/bash
# ADI Plugins Release Script - Dynamic discovery of all plugins
# Usage: adi workflow release-plugins
# Example: adi workflow release-plugins

set -e

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

# Alias for compatibility
ROOT_DIR="$PROJECT_ROOT"

# Platform detection (fallback if prelude doesn't provide)
if ! type get_platform &>/dev/null; then
    get_platform() {
        local os arch
        os=$(uname -s | tr '[:upper:]' '[:lower:]')
        arch=$(uname -m)
        case "$os" in
            darwin) os="darwin" ;;
            linux) os="linux" ;;
            mingw*|msys*|cygwin*) os="windows" ;;
        esac
        case "$arch" in
            x86_64|amd64) arch="x86_64" ;;
            arm64|aarch64) arch="aarch64" ;;
        esac
        echo "${os}-${arch}"
    }
fi

if ! type get_lib_extension &>/dev/null; then
    get_lib_extension() {
        case "$1" in
            darwin-*) echo "dylib" ;;
            windows-*) echo "dll" ;;
            *) echo "so" ;;
        esac
    }
fi

# Configuration
REGISTRY_URL=$(require_value "${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}" "ADI_REGISTRY_URL not set")

# Bump semantic version
bump_version() {
    local version="$1"
    local bump_type="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$version"
    patch="${patch%%-*}"
    case "$bump_type" in
        patch) patch=$((patch + 1)) ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        major) major=$((major + 1)); minor=0; patch=0 ;;
        *) error "Unknown bump type: $bump_type. Use patch, minor, or major." ;;
    esac
    echo "${major}.${minor}.${patch}"
}

# Ensure manifest-gen binary is available
ensure_manifest_gen() {
    local manifest_gen="$ROOT_DIR/target/release/manifest-gen"
    if [[ ! -f "$manifest_gen" ]]; then
        manifest_gen="$ROOT_DIR/target/debug/manifest-gen"
    fi
    if [[ ! -f "$manifest_gen" ]]; then
        info "Building manifest-gen..."
        (cd "$ROOT_DIR" && cargo build -p lib-plugin-manifest --features generate --release 2>/dev/null) || \
        (cd "$ROOT_DIR" && cargo build -p lib-plugin-manifest --features generate 2>/dev/null)
        manifest_gen="$ROOT_DIR/target/release/manifest-gen"
        if [[ ! -f "$manifest_gen" ]]; then
            manifest_gen="$ROOT_DIR/target/debug/manifest-gen"
        fi
    fi
    require_file "$manifest_gen" "manifest-gen binary not found. Build with: cargo build -p lib-plugin-manifest --features generate"
    echo "$manifest_gen"
}

# Dynamic plugin discovery
# Finds all Cargo.toml files with [package.metadata.plugin] section
# Output format per line: pkg_name|crate_dir (relative to ROOT_DIR)
discover_plugins() {
    find "$ROOT_DIR/crates" -name 'Cargo.toml' -not -path '*/target/*' -type f 2>/dev/null | sort | while read -r cargo_toml; do
        if ! grep -q '\[package\.metadata\.plugin\]' "$cargo_toml" 2>/dev/null; then
            continue
        fi
        local dir
        dir=$(dirname "$cargo_toml")
        local rel_dir="${dir#$ROOT_DIR/}"
        local pkg_name
        pkg_name=$(grep '^name = ' "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/')
        if [[ -n "$pkg_name" ]]; then
            echo "${pkg_name}|${rel_dir}"
        fi
    done
}

# =============================================================================
# Main Release Flow
# =============================================================================

main() {
    local bump_type=""
    local specific_plugin=""
    local auto_yes=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --bump)
                bump_type="$2"
                shift 2
                ;;
            --plugin)
                specific_plugin="$2"
                shift 2
                ;;
            --yes|-y)
                auto_yes=true
                shift
                ;;
            -h|--help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "OPTIONS:"
                echo "    --bump <type>      Version bump type: patch, minor, major"
                echo "    --plugin <name>    Release specific plugin only (by package name or plugin ID)"
                echo "    --yes, -y          Skip confirmation prompt"
                echo "    -h, --help         Show this help"
                exit 0
                ;;
            *)
                if [ -z "$specific_plugin" ] && [ "$1" != "" ]; then
                    specific_plugin="$1"
                fi
                shift
                ;;
        esac
    done

    # Check prerequisites
    ensure_command "cargo"
    ensure_command "curl"
    ensure_command "jq" "brew install jq"

    # Ensure manifest-gen
    local manifest_gen
    manifest_gen=$(ensure_manifest_gen)

    # Detect platform
    local platform
    platform=$(get_platform)
    local lib_ext
    lib_ext=$(get_lib_extension "$platform")

    info "Platform: $platform"
    info "Registry: $REGISTRY_URL"
    echo ""

    # Discover all plugins
    info "Discovering plugins..."
    local PLUGIN_ENTRIES=()
    while IFS= read -r line; do
        [[ -n "$line" ]] && PLUGIN_ENTRIES+=("$line")
    done < <(discover_plugins)

    if [[ ${#PLUGIN_ENTRIES[@]} -eq 0 ]]; then
        error "No plugins found in crates/"
    fi

    info "Found ${#PLUGIN_ENTRIES[@]} plugins"
    echo ""

    # Create dist directory
    local dist_dir="$ROOT_DIR/dist/plugins-release"
    rm -rf "$dist_dir"
    ensure_dir "$dist_dir"

    cd "$ROOT_DIR"

    # Track built plugins for the publish phase
    # Format: plugin_id|plugin_version|plugin_name|plugin_type|archive_name
    local built_plugins=()
    local skipped=0
    local failed=0

    for entry in "${PLUGIN_ENTRIES[@]}"; do
        local pkg_name crate_dir
        IFS='|' read -r pkg_name crate_dir <<< "$entry"

        # Skip if specific plugin requested and doesn't match
        if [[ -n "$specific_plugin" ]]; then
            local match=false
            if [[ "$pkg_name" == "$specific_plugin" ]]; then
                match=true
            else
                # Check plugin ID from Cargo.toml
                local check_id
                check_id=$(sed -n '/\[package\.metadata\.plugin\]/,/^\[/{/^id = /p;}' "$ROOT_DIR/$crate_dir/Cargo.toml" 2>/dev/null | sed 's/id = "\(.*\)"/\1/' | head -1 | tr -d ' ')
                [[ "$check_id" == "$specific_plugin" ]] && match=true
            fi
            if [[ "$match" == "false" ]]; then
                continue
            fi
        fi

        # Check if crate directory exists
        if [[ ! -d "$ROOT_DIR/$crate_dir" ]]; then
            warn "Skipping $pkg_name (directory not found: $crate_dir)"
            skipped=$((skipped + 1))
            continue
        fi

        echo "=== $pkg_name ($crate_dir) ==="

        # Apply version bump if requested
        if [[ -n "$bump_type" ]]; then
            local cargo_file="$ROOT_DIR/$crate_dir/Cargo.toml"
            if grep -q 'version\.workspace\|version = { workspace' "$cargo_file" 2>/dev/null; then
                info "Skipping version bump (workspace-managed)"
            else
                local current_ver
                current_ver=$(grep '^version = ' "$cargo_file" | head -1 | sed 's/.*"\(.*\)".*/\1/')
                if [[ -n "$current_ver" ]]; then
                    local new_ver
                    new_ver=$(bump_version "$current_ver" "$bump_type")
                    sed -i '' "s/^version = \"$current_ver\"/version = \"$new_ver\"/" "$cargo_file"
                    info "Bumped version: $current_ver -> $new_ver"
                fi
            fi
        fi

        # Build
        info "Building $pkg_name..."
        if ! cargo build --release -p "$pkg_name" --lib 2>&1; then
            warn "Failed to build $pkg_name, skipping"
            failed=$((failed + 1))
            echo ""
            continue
        fi

        # Generate plugin.toml from Cargo.toml metadata
        local generated_toml
        generated_toml=$(mktemp "${TMPDIR:-/tmp}/plugin-XXXXXX")
        mv "$generated_toml" "${generated_toml}.toml"
        generated_toml="${generated_toml}.toml"
        if ! "$manifest_gen" --cargo-toml "$ROOT_DIR/$crate_dir/Cargo.toml" --output "$generated_toml" 2>/dev/null; then
            warn "Failed to generate manifest for $pkg_name, skipping"
            failed=$((failed + 1))
            echo ""
            continue
        fi

        # Parse plugin metadata from generated manifest
        local plugin_id plugin_version plugin_name plugin_type
        plugin_id=$(sed -n '/^\[plugin\]/,/^\[/{/^id = /p;}' "$generated_toml" | sed 's/id = "\(.*\)"/\1/' | tr -d '\n')
        plugin_version=$(sed -n '/^\[plugin\]/,/^\[/{/^version = /p;}' "$generated_toml" | sed 's/version = "\(.*\)"/\1/' | tr -d '\n')
        plugin_name=$(sed -n '/^\[plugin\]/,/^\[/{/^name = /p;}' "$generated_toml" | sed 's/name = "\(.*\)"/\1/' | tr -d '\n')
        plugin_type=$(sed -n '/^\[plugin\]/,/^\[/{/^type = /p;}' "$generated_toml" | sed 's/type = "\(.*\)"/\1/' | tr -d '\n')

        if [[ -z "$plugin_id" ]]; then
            warn "No plugin ID found for $pkg_name, skipping"
            failed=$((failed + 1))
            echo ""
            continue
        fi

        # Find the built library
        local lib_name="lib${pkg_name//-/_}.${lib_ext}"
        local lib_path="$ROOT_DIR/target/release/$lib_name"

        if [[ ! -f "$lib_path" ]]; then
            warn "Library not found: $lib_path, skipping"
            failed=$((failed + 1))
            echo ""
            continue
        fi

        # Build web UI if present (sibling web/ directory with package.json)
        local web_js=""
        local parent_dir
        parent_dir=$(dirname "$ROOT_DIR/$crate_dir")
        if [[ -f "$parent_dir/web/package.json" ]]; then
            info "Building web UI..."
            if (cd "$parent_dir/web" && npm install --silent && npm run build) 2>/dev/null; then
                [[ -f "$parent_dir/web/dist/web.js" ]] && web_js="$parent_dir/web/dist/web.js"
            fi
        fi

        # Create package
        local pkg_dir
        pkg_dir=$(mktemp -d)
        cp "$lib_path" "$pkg_dir/plugin.$lib_ext"
        cp "$generated_toml" "$pkg_dir/plugin.toml"

        local pkg_files=("plugin.$lib_ext" "plugin.toml")
        if [[ -n "$web_js" ]]; then
            cp "$web_js" "$pkg_dir/web.js"
            pkg_files+=("web.js")
            cp "$web_js" "$dist_dir/${plugin_id}-web.js"
        fi

        local archive_name="${plugin_id}-v${plugin_version}-${platform}.tar.gz"
        tar -czf "$dist_dir/$archive_name" -C "$pkg_dir" "${pkg_files[@]}"
        rm -rf "$pkg_dir"

        built_plugins+=("${plugin_id}|${plugin_version}|${plugin_name}|${plugin_type}|${archive_name}")
        success "Created $archive_name"
        echo ""
    done

    if [[ ${#built_plugins[@]} -eq 0 ]]; then
        warn "No plugins built successfully"
        exit 1
    fi

    # Generate checksums
    info "Generating checksums..."
    (cd "$dist_dir" && shasum -a 256 *.tar.gz > SHA256SUMS 2>/dev/null || true)

    # Show summary
    echo ""
    info "Build summary:"
    info "  Built: ${#built_plugins[@]}"
    [[ $skipped -gt 0 ]] && info "  Skipped: $skipped"
    [[ $failed -gt 0 ]] && info "  Failed: $failed"
    echo ""
    info "Release artifacts:"
    ls -lh "$dist_dir"/*.tar.gz 2>/dev/null
    echo ""

    # Confirm publish
    if [[ "$auto_yes" != "true" ]]; then
        read -p "Publish ${#built_plugins[@]} plugins to registry? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            info "Aborted. Artifacts are in $dist_dir"
            exit 0
        fi
    fi

    # Publish to registry
    echo ""
    local published=0
    local pub_failed=0

    for bp in "${built_plugins[@]}"; do
        local plugin_id plugin_version plugin_name plugin_type archive_name
        IFS='|' read -r plugin_id plugin_version plugin_name plugin_type archive_name <<< "$bp"

        local archive_path="$dist_dir/$archive_name"
        if [[ ! -f "$archive_path" ]]; then
            continue
        fi

        info "Publishing $plugin_id v$plugin_version for $platform..."

        local url="$REGISTRY_URL/v1/publish/plugins/$plugin_id/$plugin_version/$platform"
        url="$url?name=$(echo "$plugin_name" | sed 's/ /%20/g')&plugin_type=${plugin_type:-extension}&author=ADI%20Team"

        local response
        response=$(curl -s --max-time 300 -X POST "$url" -F "file=@$archive_path" 2>&1) || true

        if echo "$response" | jq . 2>/dev/null; then
            :
        else
            echo "$response"
        fi

        # Publish web UI if present
        local web_js_path="$dist_dir/${plugin_id}-web.js"
        if [[ -f "$web_js_path" ]]; then
            info "Publishing web UI for $plugin_id..."
            curl -s --max-time 120 -X POST \
                "$REGISTRY_URL/v1/publish/plugins/$plugin_id/$plugin_version/web" \
                -H "Content-Type: application/javascript" \
                --data-binary "@$web_js_path" | jq . 2>/dev/null || true
        fi

        success "Published $plugin_id"
        published=$((published + 1))
        echo ""
    done

    echo ""
    success "Done! Published $published/${#built_plugins[@]} plugins"
}

main "$@"
