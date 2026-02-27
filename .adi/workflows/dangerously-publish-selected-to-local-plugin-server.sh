#!/bin/bash
# Dangerously publish selected plugins to local plugin server
# Skips lint and confirmation — fire and forget
# Edit PLUGINS array below to choose what gets published

set -e

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
    CWD="$PWD"
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' BOLD='\033[1m' DIM='\033[2m' NC='\033[0m'
    log() { echo -e "${BLUE:-\033[0;34m}[log]${NC} $1"; }
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
    has_tty() { [[ -t 0 ]] && [[ -t 1 ]]; }
    ensure_dir() { mkdir -p "$1"; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
    ensure_command() { check_command "$1" || error "$1 not found${2:+. Install: $2}"; }
    require_file() { [[ -f "$1" ]] || error "${2:-File not found: $1}"; }
    require_dir() { [[ -d "$1" ]] || error "${2:-Directory not found: $1}"; }
    require_value() { [[ -n "$1" ]] || error "${2:-Value required}"; echo "$1"; }
fi

REGISTRY_URL="http://adi.test/registry"

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

get_lib_extension() {
    case "$1" in
        darwin-*) echo "dylib" ;;
        windows-*) echo "dll" ;;
        *) echo "so" ;;
    esac
}

ensure_manifest_gen() {
    local manifest_gen="$PROJECT_ROOT/target/release/manifest-gen"
    if [[ ! -f "$manifest_gen" ]]; then
        manifest_gen="$PROJECT_ROOT/target/debug/manifest-gen"
    fi
    if [[ ! -f "$manifest_gen" ]]; then
        info "Building manifest-gen..."
        (cd "$PROJECT_ROOT" && cargo build -p lib-plugin-manifest --features generate --release 2>/dev/null) || \
        (cd "$PROJECT_ROOT" && cargo build -p lib-plugin-manifest --features generate 2>/dev/null)
        manifest_gen="$PROJECT_ROOT/target/release/manifest-gen"
        [[ ! -f "$manifest_gen" ]] && manifest_gen="$PROJECT_ROOT/target/debug/manifest-gen"
    fi
    require_file "$manifest_gen" "manifest-gen not found. Build: cargo build -p lib-plugin-manifest --features generate"
    echo "$manifest_gen"
}

# Find crate directory by plugin ID
find_crate_dir() {
    local target_id="$1"
    find "$PROJECT_ROOT/crates" -name 'Cargo.toml' -not -path '*/target/*' -type f 2>/dev/null | while read -r cargo_toml; do
        if ! grep -q '\[package\.metadata\.plugin\]' "$cargo_toml" 2>/dev/null; then
            continue
        fi
        local plugin_id
        plugin_id=$(sed -n '/\[package\.metadata\.plugin\]/,/^\[/{/^id = /p;}' "$cargo_toml" | sed 's/id = "\(.*\)"/\1/' | head -1 | tr -d ' \n')
        if [[ "$plugin_id" == "$target_id" ]]; then
            dirname "$cargo_toml" | sed "s|^$PROJECT_ROOT/||"
            return 0
        fi
    done
}

build_and_publish() {
    local plugin_id_arg="$1"
    local platform="$2"
    local lib_ext="$3"
    local manifest_gen="$4"
    local dist_dir="$5"

    local crate_dir
    crate_dir=$(find_crate_dir "$plugin_id_arg")
    if [[ -z "$crate_dir" ]]; then
        warn "Plugin not found: $plugin_id_arg"
        return 1
    fi

    local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
    require_file "$cargo_toml"

    # Generate manifest
    local generated_toml
    generated_toml=$(mktemp "${TMPDIR:-/tmp}/plugin-XXXXXX").toml
    "$manifest_gen" --cargo-toml "$cargo_toml" --output "$generated_toml" || { warn "Manifest generation failed for $plugin_id_arg"; return 1; }

    # Parse metadata
    local plugin_id plugin_version plugin_name plugin_desc plugin_author plugin_type
    plugin_id=$(sed -n '/^\[plugin\]/,/^\[/{/^id = /p;}' "$generated_toml" | sed 's/id = "\(.*\)"/\1/' | tr -d '\n')
    plugin_version=$(sed -n '/^\[plugin\]/,/^\[/{/^version = /p;}' "$generated_toml" | sed 's/version = "\(.*\)"/\1/' | tr -d '\n')
    plugin_name=$(sed -n '/^\[plugin\]/,/^\[/{/^name = /p;}' "$generated_toml" | sed 's/name = "\(.*\)"/\1/' | tr -d '\n')
    plugin_desc=$(sed -n '/^\[plugin\]/,/^\[/{/^description = /p;}' "$generated_toml" | sed 's/description = "\(.*\)"/\1/' | tr -d '\n')
    plugin_author=$(sed -n '/^\[plugin\]/,/^\[/{/^author = /p;}' "$generated_toml" | sed 's/author = "\(.*\)"/\1/' | tr -d '\n')
    plugin_type=$(sed -n '/^\[plugin\]/,/^\[/{/^type = /p;}' "$generated_toml" | sed 's/type = "\(.*\)"/\1/' | tr -d '\n')

    plugin_id=$(require_value "$plugin_id" "No plugin ID for $plugin_id_arg")
    plugin_version=$(require_value "$plugin_version" "No version for $plugin_id_arg")

    # Get package name
    local package_name
    package_name=$(grep '^name = ' "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/')

    # Build library
    info "Building $plugin_id v$plugin_version..."
    (cd "$PROJECT_ROOT" && cargo build --release -p "$package_name" --lib) || { warn "Build failed for $plugin_id"; return 1; }

    local lib_name="lib${package_name//-/_}"
    local lib_path="$PROJECT_ROOT/target/release/${lib_name}.${lib_ext}"
    require_file "$lib_path" "Library not found: $lib_path"

    # Build web UI if present
    local web_js=""
    local parent_dir
    parent_dir=$(dirname "$PROJECT_ROOT/$crate_dir")
    if [[ -f "$parent_dir/web/package.json" ]]; then
        info "Building web UI..."
        if (cd "$parent_dir/web" && npm install --silent && npm run build) 2>/dev/null; then
            [[ -f "$parent_dir/web/dist/web.js" ]] && web_js="$parent_dir/web/dist/web.js"
        fi
    elif [[ -f "$PROJECT_ROOT/$crate_dir/web/package.json" ]]; then
        info "Building web UI..."
        if (cd "$PROJECT_ROOT/$crate_dir/web" && npm install --silent && npm run build) 2>/dev/null; then
            [[ -f "$PROJECT_ROOT/$crate_dir/web/dist/web.js" ]] && web_js="$PROJECT_ROOT/$crate_dir/web/dist/web.js"
        fi
    fi

    # Package
    local pkg_dir
    pkg_dir=$(mktemp -d)
    cp "$lib_path" "$pkg_dir/plugin.$lib_ext"
    cp "$generated_toml" "$pkg_dir/plugin.toml"

    local pkg_files=("plugin.$lib_ext" "plugin.toml")
    if [[ -n "$web_js" ]]; then
        cp "$web_js" "$pkg_dir/web.js"
        pkg_files+=("web.js")
    fi

    local archive_name="${plugin_id}-v${plugin_version}-${platform}.tar.gz"
    local archive_path="$dist_dir/$archive_name"
    tar -czf "$archive_path" -C "$pkg_dir" "${pkg_files[@]}"
    rm -rf "$pkg_dir"

    success "Built $archive_name"

    # Publish
    info "Publishing $plugin_id v$plugin_version to $REGISTRY_URL..."

    local encoded_name encoded_desc encoded_author
    encoded_name=$(echo "$plugin_name" | sed 's/ /%20/g')
    encoded_desc=$(echo "$plugin_desc" | sed 's/ /%20/g')
    encoded_author=$(echo "$plugin_author" | sed 's/ /%20/g')

    local url="$REGISTRY_URL/v1/publish/plugins/$plugin_id/$plugin_version/$platform"
    url="$url?name=$encoded_name&description=$encoded_desc&pluginType=${plugin_type:-extension}&author=$encoded_author"

    local response http_code body
    response=$(curl -s -w "\n%{http_code}" --max-time 300 -X POST "$url" \
        -H "Content-Type: application/gzip" \
        --data-binary "@$archive_path")

    http_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | sed '$d')

    if [[ "$http_code" == "200" ]] || [[ "$http_code" == "201" ]]; then
        success "Published $plugin_id v$plugin_version"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        warn "Failed to publish $plugin_id (HTTP $http_code): $body"
        return 1
    fi

    # Publish web UI separately if present
    if [[ -n "$web_js" ]]; then
        info "Publishing web UI for $plugin_id..."
        curl -s --max-time 120 -X POST \
            "$REGISTRY_URL/v1/publish/plugins/$plugin_id/$plugin_version/web" \
            -H "Content-Type: application/javascript" \
            --data-binary "@$web_js" | jq . 2>/dev/null || true
    fi
}

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Edit this list to choose what gets published
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PLUGINS=(
    "adi.auth"
    "adi.knowledgebase"
    "adi.video"
)
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

main() {
    ensure_command "cargo"
    ensure_command "curl"

    local manifest_gen
    manifest_gen=$(ensure_manifest_gen)

    local platform
    platform=$(get_platform)
    local lib_ext
    lib_ext=$(get_lib_extension "$platform")

    local dist_dir="$PROJECT_ROOT/dist/plugins-local"
    rm -rf "$dist_dir"
    ensure_dir "$dist_dir"

    warn "Publishing to LOCAL registry: $REGISTRY_URL"
    info "Plugins: ${PLUGINS[*]}"
    echo ""

    # Build all plugins in parallel
    local pids=()
    local log_dir
    log_dir=$(mktemp -d)

    for plugin_id in "${PLUGINS[@]}"; do
        (
            build_and_publish "$plugin_id" "$platform" "$lib_ext" "$manifest_gen" "$dist_dir"
        ) > "$log_dir/$plugin_id.log" 2>&1 &
        pids+=($!)
    done

    # Wait for all and collect results
    local failed=0 published=0
    for i in "${!PLUGINS[@]}"; do
        local plugin_id="${PLUGINS[$i]}"
        local pid="${pids[$i]}"

        if wait "$pid"; then
            published=$((published + 1))
        else
            failed=$((failed + 1))
        fi

        echo "━━━ $plugin_id ━━━"
        cat "$log_dir/$plugin_id.log"
        echo ""
    done

    rm -rf "$log_dir"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    info "Total: ${#PLUGINS[@]} | Published: $published | Failed: $failed"

    if [[ $failed -gt 0 ]]; then
        warn "$failed plugin(s) failed"
        exit 1
    fi

    success "All $published plugin(s) published to local registry"
}

main
