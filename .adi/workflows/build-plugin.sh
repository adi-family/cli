#!/bin/bash
# Build and install a plugin locally without publishing to registry
# Usage: adi workflow build-plugin
# Example: adi workflow build-plugin

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

# Platform detection
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
    local platform="$1"
    case "$platform" in
        darwin-*) echo "dylib" ;;
        windows-*) echo "dll" ;;
        *) echo "so" ;;
    esac
}

# Plugin installation directory (platform-specific)
get_plugins_dir() {
    if [[ -n "${ADI_PLUGINS_DIR:-}" ]]; then
        echo "$ADI_PLUGINS_DIR"
    elif [[ "$(uname -s)" == "Darwin" ]]; then
        echo "$HOME/Library/Application Support/adi/plugins"
    else
        echo "$HOME/.local/share/adi/plugins"
    fi
}
PLUGINS_DIR="$(get_plugins_dir)"

usage() {
    cat <<EOF
Usage: $0 <plugin-name> [OPTIONS]

Build and optionally install a plugin locally without publishing to registry.

OPTIONS:
    --install           Install to local plugins directory after building
    --force             Force replace existing installation (with --install)
    --skip-lint         Skip linting step (faster build)
    -h, --help          Show this help

PLUGINS:
    cocoon              Containerized worker with PTY support
    agent-loop          Autonomous LLM agent loop
    indexer             Code indexer
    knowledgebase       Knowledge base with graph DB
    tasks               Task management
    workflow            Workflow automation
    coolify             Coolify deployment integration
    linter              Code linter
    api-proxy           LLM API proxy (BYOK/Platform)
    hive                Hive orchestration CLI
    llm-extract         LLM extraction utilities
    llm-uzu             Local LLM inference (Apple Silicon)
    embed               Embedding utilities
    lang-*              Language plugins (rust, python, typescript, etc.)

EXAMPLES:
    $0 adi.hive                     # Build only
    $0 adi.hive --install           # Build and install
    $0 adi.hive --install --force   # Build and force-replace
    $0 adi.cocoon --install         # Build and install cocoon
    $0 adi.agent-loop --install     # Build and install agent-loop

LOCATION:
    Plugins are installed to: $PLUGINS_DIR/<plugin-id>/<version>/
EOF
    exit 0
}

# Ensure manifest-gen binary is available
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
        if [[ ! -f "$manifest_gen" ]]; then
            manifest_gen="$PROJECT_ROOT/target/debug/manifest-gen"
        fi
    fi
    echo "$manifest_gen"
}

# Get crate directory for plugin by searching for Cargo.toml with [package.metadata.plugin]
get_plugin_crate() {
    local name="$1"

    # First, try to find by plugin ID in Cargo.toml [package.metadata.plugin]
    local found_path=""
    local plugin_id=""
    while IFS= read -r f; do
        # Check if this Cargo.toml has [package.metadata.plugin] section
        if ! grep -q 'package\.metadata\.plugin' "$f" 2>/dev/null; then
            continue
        fi
        plugin_id=$(grep -A1 '\[package\.metadata\.plugin\]' "$f" 2>/dev/null | grep '^id = ' | sed 's/id = "//;s/"//' | tr -d '\n')
        if [[ "$plugin_id" == "$name" ]]; then
            found_path=$(dirname "$f" | sed "s|^$PROJECT_ROOT/||")
            break
        fi
    done < <(find "$PROJECT_ROOT/crates" -name 'Cargo.toml' -type f 2>/dev/null)

    if [[ -n "$found_path" ]]; then
        echo "$found_path"
        return
    fi

    # Fallback to legacy short names for backward compatibility
    name="${name%-plugin}"
    case "$name" in
        cocoon) echo "crates/cocoon" ;;
        hive|adi-hive) echo "crates/hive/plugin" ;;
        agent-loop|adi-agent-loop) echo "crates/adi-agent-loop/plugin" ;;
        indexer|adi-indexer) echo "crates/adi-indexer/plugin" ;;
        knowledgebase|adi-knowledgebase) echo "crates/adi-knowledgebase/plugin" ;;
        tasks|adi-tasks) echo "crates/adi-tasks/plugin" ;;
        workflow|adi-workflow) echo "crates/adi-workflow/plugin" ;;
        coolify|adi-coolify) echo "crates/adi-coolify/plugin" ;;
        linter|adi-linter) echo "crates/adi-linter/plugin" ;;
        api-proxy|adi-api-proxy) echo "crates/adi-api-proxy/plugin" ;;
        llm-extract|adi-llm-extract) echo "crates/adi-llm-extract-plugin" ;;
        llm-uzu|adi-llm-uzu) echo "crates/adi-llm-uzu-plugin" ;;
        tsp-gen|typespec) echo "crates/lib/lib-typespec-api/plugin" ;;
        lang-cpp|adi-lang-cpp) echo "crates/adi-lang/cpp/plugin" ;;
        lang-csharp|adi-lang-csharp) echo "crates/adi-lang/csharp/plugin" ;;
        lang-go|adi-lang-go) echo "crates/adi-lang/go/plugin" ;;
        lang-java|adi-lang-java) echo "crates/adi-lang/java/plugin" ;;
        lang-lua|adi-lang-lua) echo "crates/adi-lang/lua/plugin" ;;
        lang-php|adi-lang-php) echo "crates/adi-lang/php/plugin" ;;
        lang-python|adi-lang-python) echo "crates/adi-lang/python/plugin" ;;
        lang-ruby|adi-lang-ruby) echo "crates/adi-lang/ruby/plugin" ;;
        lang-rust|adi-lang-rust) echo "crates/adi-lang/rust/plugin" ;;
        lang-swift|adi-lang-swift) echo "crates/adi-lang/swift/plugin" ;;
        lang-typescript|adi-lang-typescript) echo "crates/adi-lang/typescript/plugin" ;;
        embed|adi-embed) echo "crates/adi-embed-plugin" ;;
        audio|adi-audio) echo "crates/adi-audio" ;;
        cli-lang-en|adi-cli-lang-en) echo "crates/adi-cli-lang-en" ;;
        # Hive plugins
        hive-plugin-abi) echo "crates/hive/plugins/abi" ;;
        hive-runner-docker) echo "crates/hive/plugins/runner-docker" ;;
        hive-runner-compose) echo "crates/hive/plugins/runner-compose" ;;
        hive-runner-podman) echo "crates/hive/plugins/runner-podman" ;;
        hive-obs-stdout) echo "crates/hive/plugins/obs-stdout" ;;
        hive-obs-file) echo "crates/hive/plugins/obs-file" ;;
        hive-obs-loki) echo "crates/hive/plugins/obs-loki" ;;
        hive-obs-prometheus) echo "crates/hive/plugins/obs-prometheus" ;;
        hive-proxy-cors) echo "crates/hive/plugins/proxy-cors" ;;
        hive-proxy-rate-limit) echo "crates/hive/plugins/proxy-rate-limit" ;;
        hive-proxy-ip-filter) echo "crates/hive/plugins/proxy-ip-filter" ;;
        hive-proxy-headers) echo "crates/hive/plugins/proxy-headers" ;;
        hive-proxy-compress) echo "crates/hive/plugins/proxy-compress" ;;
        hive-proxy-cache) echo "crates/hive/plugins/proxy-cache" ;;
        hive-proxy-rewrite) echo "crates/hive/plugins/proxy-rewrite" ;;
        hive-proxy-auth-jwt) echo "crates/hive/plugins/proxy-auth-jwt" ;;
        hive-proxy-auth-basic) echo "crates/hive/plugins/proxy-auth-basic" ;;
        hive-proxy-auth-api-key) echo "crates/hive/plugins/proxy-auth-api-key" ;;
        hive-proxy-auth-oidc) echo "crates/hive/plugins/proxy-auth-oidc" ;;
        hive-health-http) echo "crates/hive/plugins/health-http" ;;
        hive-health-tcp) echo "crates/hive/plugins/health-tcp" ;;
        hive-health-cmd) echo "crates/hive/plugins/health-cmd" ;;
        hive-health-grpc) echo "crates/hive/plugins/health-grpc" ;;
        hive-health-postgres) echo "crates/hive/plugins/health-postgres" ;;
        hive-health-redis) echo "crates/hive/plugins/health-redis" ;;
        hive-health-mysql) echo "crates/hive/plugins/health-mysql" ;;
        hive-env-dotenv) echo "crates/hive/plugins/env-dotenv" ;;
        hive-env-vault) echo "crates/hive/plugins/env-vault" ;;
        hive-env-1password) echo "crates/hive/plugins/env-1password" ;;
        hive-env-aws-secrets) echo "crates/hive/plugins/env-aws-secrets" ;;
        hive-rollout-recreate) echo "crates/hive/plugins/rollout-recreate" ;;
        hive-rollout-blue-green) echo "crates/hive/plugins/rollout-blue-green" ;;
        *) echo "" ;;
    esac
}

# Global variables for plugin metadata (set by build_plugin)
PLUGIN_ID=""
PLUGIN_VERSION=""
PLUGIN_PLATFORM=""
PLUGIN_LIB_PATH=""
PLUGIN_TOML_PATH=""

build_plugin() {
    local plugin_name="$1"
    local crate_dir="$2"
    local skip_lint="$3"

    cd "$PROJECT_ROOT"

    # Find Cargo.toml with plugin metadata
    local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
    require_file "$cargo_toml" "Cargo.toml not found in $crate_dir"

    # Generate plugin.toml from Cargo.toml metadata
    local manifest_gen
    manifest_gen=$(ensure_manifest_gen)
    local generated_toml
    generated_toml=$(mktemp "${TMPDIR:-/tmp}/plugin.XXXXXX.toml")
    "$manifest_gen" --cargo-toml "$cargo_toml" --output "$generated_toml" || error "Failed to generate manifest from $cargo_toml"

    # Parse plugin metadata from generated manifest
    PLUGIN_ID=$(sed -n '/^\[plugin\]/,/^\[/{/^id = /p;}' "$generated_toml" | sed 's/id = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_VERSION=$(sed -n '/^\[plugin\]/,/^\[/{/^version = /p;}' "$generated_toml" | sed 's/version = "\(.*\)"/\1/' | tr -d '\n')

    PLUGIN_ID=$(require_value "$PLUGIN_ID" "Could not read plugin ID from Cargo.toml metadata")
    PLUGIN_VERSION=$(require_value "$PLUGIN_VERSION" "Could not read plugin version from Cargo.toml")
    PLUGIN_TOML_PATH="$generated_toml"

    info "Plugin: $PLUGIN_ID v$PLUGIN_VERSION"

    # Lint plugin unless skipped
    if [ "$skip_lint" != "true" ]; then
        info "Linting plugin..."
        if ! "$WORKFLOWS_DIR/lint-plugin.sh" "$PROJECT_ROOT/$crate_dir" 2>/dev/null; then
            warn "Lint failed - continuing anyway (use --skip-lint to suppress)"
        else
            success "Lint passed"
        fi
    else
        info "Skipping lint (--skip-lint)"
    fi

    info "Building library..."

    # Get package name from Cargo.toml (may differ from plugin ID)
    local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
    if [[ ! -f "$cargo_toml" ]] || grep -q '^\[workspace\]' "$cargo_toml" 2>/dev/null; then
        if [[ -f "$PROJECT_ROOT/$crate_dir/plugin/Cargo.toml" ]]; then
            cargo_toml="$PROJECT_ROOT/$crate_dir/plugin/Cargo.toml"
        fi
    fi
    local package_name
    package_name=$(grep '^name = ' "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/')

    # Build ONLY the library (not the binary)
    local build_dir="$PROJECT_ROOT"
    local target_dir="$PROJECT_ROOT/target"
    if grep -q '^\[workspace\]' "$PROJECT_ROOT/$crate_dir/Cargo.toml" 2>/dev/null; then
        build_dir="$PROJECT_ROOT/$crate_dir"
        target_dir="$PROJECT_ROOT/$crate_dir/target"
    fi
    (cd "$build_dir" && cargo build --release -p "$package_name" --lib)

    # Find the built library
    PLUGIN_PLATFORM=$(get_platform)
    local lib_ext
    lib_ext=$(get_lib_extension "$PLUGIN_PLATFORM")

    local lib_name="lib${package_name//-/_}"
    PLUGIN_LIB_PATH="$target_dir/release/${lib_name}.${lib_ext}"

    require_file "$PLUGIN_LIB_PATH" "Library not found: $PLUGIN_LIB_PATH"

    success "Built: $PLUGIN_LIB_PATH"
}

install_plugin() {
    local force="$1"
    
    local install_dir="$PLUGINS_DIR/$PLUGIN_ID/$PLUGIN_VERSION"
    local version_file="$PLUGINS_DIR/$PLUGIN_ID/.version"
    
    # Check if already installed
    if [[ -d "$install_dir" ]] && [[ "$force" != "true" ]]; then
        warn "Plugin $PLUGIN_ID v$PLUGIN_VERSION already installed at:"
        warn "  $install_dir"
        warn "Use --force to replace"
        return 1
    fi
    
    # Remove existing installation if force
    if [[ -d "$install_dir" ]]; then
        info "Removing existing installation..."
        rm -rf "$install_dir"
    fi
    
    # Create installation directory
    ensure_dir "$install_dir"
    
    # Get library extension
    local lib_ext
    lib_ext=$(get_lib_extension "$PLUGIN_PLATFORM")
    
    # Copy files (rename library to plugin.<ext> as expected by adi-cli)
    info "Installing to: $install_dir"
    cp "$PLUGIN_LIB_PATH" "$install_dir/plugin.$lib_ext"
    cp "$PLUGIN_TOML_PATH" "$install_dir/plugin.toml"
    
    # Sign binary on macOS
    if [[ "$(uname -s)" == "Darwin" ]]; then
        codesign -s - -f "$install_dir/plugin.$lib_ext" 2>/dev/null || true
    fi
    
    # Update version file (tracks current active version)
    echo "$PLUGIN_VERSION" > "$version_file"
    
    success "Installed $PLUGIN_ID v$PLUGIN_VERSION"
    echo ""
    info "Installation directory: $install_dir"
    ls -la "$install_dir"
}

main() {
    local plugin_name=""
    local install=false
    local force=false
    local skip_lint=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --install)
                install=true
                shift
                ;;
            --force)
                force=true
                shift
                ;;
            --skip-lint)
                skip_lint=true
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
        error "Plugin name required. Run with --help to see available plugins."
    fi

    # Get crate directory
    local crate_dir
    crate_dir=$(get_plugin_crate "$plugin_name")

    if [ -z "$crate_dir" ]; then
        error "Unknown plugin: $plugin_name. Run with --help to see available plugins."
    fi

    require_dir "$PROJECT_ROOT/$crate_dir" "Plugin crate not found: $crate_dir"

    # Check prerequisites
    ensure_command "cargo"

    echo ""
    info "Building plugin: $plugin_name"
    echo ""

    # Build plugin (sets global PLUGIN_* variables)
    build_plugin "$plugin_name" "$crate_dir" "$skip_lint"

    echo ""

    if [ "$install" = true ]; then
        info "Installing plugin locally..."
        install_plugin "$force"
        echo ""
        success "Plugin ready to use: adi $PLUGIN_ID <command>"
    else
        # Output to dist directory
        local dist_dir="$PROJECT_ROOT/dist/plugins"
        ensure_dir "$dist_dir"
        
        local lib_ext
        lib_ext=$(get_lib_extension "$PLUGIN_PLATFORM")
        local archive_name="${PLUGIN_ID}-v${PLUGIN_VERSION}-${PLUGIN_PLATFORM}.tar.gz"
        local archive_path="$dist_dir/$archive_name"
        
        # Create package
        local pkg_dir
        pkg_dir=$(mktemp -d)
        cp "$PLUGIN_LIB_PATH" "$pkg_dir/plugin.$lib_ext"
        cp "$PLUGIN_TOML_PATH" "$pkg_dir/plugin.toml"
        
        tar -czf "$archive_path" -C "$pkg_dir" "plugin.$lib_ext" "plugin.toml"
        rm -rf "$pkg_dir"
        
        success "Built: $archive_path"
        ls -lh "$archive_path"
        echo ""
        info "To install locally, run again with --install"
    fi
}

main "$@"
