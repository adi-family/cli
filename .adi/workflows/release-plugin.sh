#!/bin/bash
# Release a single plugin to the ADI plugin registry
# Usage: adi workflow release-plugin
# Example: adi workflow release-plugin

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

# Configuration
REGISTRY_URL="${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}"

usage() {
    cat <<EOF
Usage: $0 <plugin-name> [OPTIONS]

Release a single plugin to the ADI plugin registry.

OPTIONS:
    --no-push           Build only, skip publishing
    --local             Push to local registry (localhost:8019)
    -h, --help          Show this help

PLUGINS:
    cocoon              Containerized worker with PTY support

EXAMPLES:
    $0 cocoon           # Build and publish to production
    $0 cocoon --no-push # Build only (dry-run)
    $0 cocoon --local   # Build and publish to local registry

INSTALL:
    After publishing, install with:
    adi plugin install adi.cocoon
EOF
    exit 0
}

# Get crate directory for plugin
get_plugin_crate() {
    local name="$1"
    case "$name" in
        cocoon) echo "crates/cocoon" ;;
        agent-loop|adi-agent-loop) echo "crates/adi-agent-loop/plugin" ;;
        indexer|adi-indexer) echo "crates/adi-indexer/plugin" ;;
        knowledgebase|adi-knowledgebase) echo "crates/adi-knowledgebase/plugin" ;;
        tasks|adi-tasks) echo "crates/adi-tasks/plugin" ;;
        workflow|adi-workflow) echo "crates/adi-workflow/plugin" ;;
        coolify|adi-coolify) echo "crates/adi-coolify/plugin" ;;
        linter|adi-linter) echo "crates/adi-linter/plugin" ;;
        lang-cpp|adi-lang-cpp) echo "crates/adi-lang/cpp" ;;
        lang-csharp|adi-lang-csharp) echo "crates/adi-lang/csharp" ;;
        lang-go|adi-lang-go) echo "crates/adi-lang/go" ;;
        lang-java|adi-lang-java) echo "crates/adi-lang/java" ;;
        lang-lua|adi-lang-lua) echo "crates/adi-lang/lua" ;;
        lang-php|adi-lang-php) echo "crates/adi-lang/php" ;;
        lang-python|adi-lang-python) echo "crates/adi-lang/python" ;;
        lang-ruby|adi-lang-ruby) echo "crates/adi-lang/ruby" ;;
        lang-rust|adi-lang-rust) echo "crates/adi-lang/rust" ;;
        lang-swift|adi-lang-swift) echo "crates/adi-lang/swift" ;;
        lang-typescript|adi-lang-typescript) echo "crates/adi-lang/typescript" ;;
        embed|adi-embed) echo "crates/adi-embed-plugin" ;;
        *) echo "" ;;
    esac
}

# Global variables for plugin metadata (set by build_plugin)
PLUGIN_ID=""
PLUGIN_VERSION=""
PLUGIN_PLATFORM=""
PLUGIN_ARCHIVE=""
PLUGIN_NAME=""
PLUGIN_DESC=""
PLUGIN_AUTHOR=""
PLUGIN_TYPE=""

build_plugin() {
    local plugin_name="$1"
    local crate_dir="$2"
    local dist_dir="$3"

    cd "$PROJECT_ROOT"

    # Read plugin.toml
    local plugin_toml="$PROJECT_ROOT/$crate_dir/plugin.toml"
    require_file "$plugin_toml" "plugin.toml not found in $crate_dir"

    # Parse plugin metadata
    PLUGIN_ID=$(sed -n '/^\[plugin\]/,/^\[/{/^id = /p;}' "$plugin_toml" | sed 's/id = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_VERSION=$(sed -n '/^\[plugin\]/,/^\[/{/^version = /p;}' "$plugin_toml" | sed 's/version = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_NAME=$(sed -n '/^\[plugin\]/,/^\[/{/^name = /p;}' "$plugin_toml" | sed 's/name = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_DESC=$(sed -n '/^\[plugin\]/,/^\[/{/^description = /p;}' "$plugin_toml" | sed 's/description = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_AUTHOR=$(sed -n '/^\[plugin\]/,/^\[/{/^author = /p;}' "$plugin_toml" | sed 's/author = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_TYPE=$(sed -n '/^\[plugin\]/,/^\[/{/^type = /p;}' "$plugin_toml" | sed 's/type = "\(.*\)"/\1/' | tr -d '\n')

    PLUGIN_ID=$(require_value "$PLUGIN_ID" "Could not read plugin ID from plugin.toml")
    PLUGIN_VERSION=$(require_value "$PLUGIN_VERSION" "Could not read plugin version from plugin.toml")

    info "Plugin: $PLUGIN_ID v$PLUGIN_VERSION"
    info "Building library (no standalone binary)..."

    # Get package name from Cargo.toml (may differ from plugin ID)
    local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
    local package_name
    package_name=$(grep '^name = ' "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/')

    # Build ONLY the library (not the binary)
    cargo build --release -p "$package_name" --lib

    # Find the built library
    PLUGIN_PLATFORM=$(get_platform)
    local lib_ext
    lib_ext=$(get_lib_extension "$PLUGIN_PLATFORM")

    local lib_name="lib${package_name//-/_}"
    local lib_path="$PROJECT_ROOT/target/release/${lib_name}.${lib_ext}"

    require_file "$lib_path" "Library not found: $lib_path"

    info "Built: $lib_path"

    # Create package
    local pkg_dir
    pkg_dir=$(mktemp -d)
    cp "$lib_path" "$pkg_dir/plugin.$lib_ext"
    cp "$plugin_toml" "$pkg_dir/plugin.toml"

    local archive_name="${PLUGIN_ID}-v${PLUGIN_VERSION}-${PLUGIN_PLATFORM}.tar.gz"
    PLUGIN_ARCHIVE="$dist_dir/$archive_name"

    tar -czf "$PLUGIN_ARCHIVE" -C "$pkg_dir" "plugin.$lib_ext" "plugin.toml"
    rm -rf "$pkg_dir"

    success "Created: $archive_name"
}

publish_plugin() {
    local plugin_id="$1"
    local version="$2"
    local platform="$3"
    local archive_path="$4"
    local name="$5"
    local desc="$6"
    local author="$7"
    local plugin_type="$8"
    local registry="$9"

    info "Publishing $plugin_id v$version for $platform..."
    info "Registry: $registry"

    # URL encode parameters
    local encoded_name
    encoded_name=$(echo "$name" | sed 's/ /%20/g')
    local encoded_desc
    encoded_desc=$(echo "$desc" | sed 's/ /%20/g')
    local encoded_author
    encoded_author=$(echo "$author" | sed 's/ /%20/g')

    local url="$registry/v1/publish/plugins/$plugin_id/$version/$platform"
    url="$url?name=$encoded_name&description=$encoded_desc&plugin_type=$plugin_type&author=$encoded_author"

    local response
    response=$(curl -s -w "\n%{http_code}" --max-time 300 -X POST "$url" -F "file=@$archive_path")

    local http_code
    http_code=$(echo "$response" | tail -n 1)
    local body
    body=$(echo "$response" | sed '$d')

    if [ "$http_code" = "200" ] || [ "$http_code" = "201" ]; then
        success "Published $plugin_id v$version"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
    else
        error "Failed to publish (HTTP $http_code): $body"
    fi
}

main() {
    local plugin_name=""
    local push=true
    local registry="$REGISTRY_URL"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --no-push)
                push=false
                shift
                ;;
            --local)
                registry="http://localhost:8019"
                shift
                ;;
            *)
                if [ -z "$plugin_name" ]; then
                    plugin_name="$1"
                else
                    error "Unknown argument: $1"
                fi
                shift
                ;;
        esac
    done

    if [ -z "$plugin_name" ]; then
        error "Plugin name required. Available: cocoon"
    fi

    # Get crate directory
    local crate_dir
    crate_dir=$(get_plugin_crate "$plugin_name")

    if [ -z "$crate_dir" ]; then
        error "Unknown plugin: $plugin_name. Available: cocoon"
    fi

    require_dir "$PROJECT_ROOT/$crate_dir" "Plugin crate not found: $crate_dir"

    # Lint plugin before building
    info "Linting plugin..."
    if ! "$WORKFLOWS_DIR/lint-plugin.sh" "$PROJECT_ROOT/$crate_dir"; then
        error "Plugin lint failed. Fix errors before publishing."
        warn "Run with --fix to auto-fix: $WORKFLOWS_DIR/lint-plugin.sh --fix $crate_dir"
    fi
    success "Lint passed"

    # Check prerequisites
    ensure_command "cargo"
    ensure_command "curl"
    ensure_command "jq" "brew install jq"

    # Create dist directory
    local dist_dir="$PROJECT_ROOT/dist/plugins"
    ensure_dir "$dist_dir"

    echo ""
    info "Building plugin: $plugin_name"
    echo ""

    # Build plugin (sets global PLUGIN_* variables)
    build_plugin "$plugin_name" "$crate_dir" "$dist_dir"

    echo ""
    info "Artifact: $PLUGIN_ARCHIVE"
    ls -lh "$PLUGIN_ARCHIVE"
    echo ""

    if [ "$push" = true ]; then
        publish_plugin "$PLUGIN_ID" "$PLUGIN_VERSION" "$PLUGIN_PLATFORM" "$PLUGIN_ARCHIVE" "$PLUGIN_NAME" "$PLUGIN_DESC" "$PLUGIN_AUTHOR" "$PLUGIN_TYPE" "$registry"
        echo ""
        success "Install with: adi plugin install $PLUGIN_ID"
    else
        info "Build complete. Use without --no-push to publish."
    fi
}

main "$@"
