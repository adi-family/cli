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

# Platform detection
get_platform() {
    local os arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        darwin) os="darwin" ;;  # Keep as darwin to match CLI expectations
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

# Configuration
REGISTRY_URL="${ADI_REGISTRY_URL:-https://adi-plugin-registry.the-ihor.com}"

usage() {
    cat <<EOF
Usage: $0 <plugin-name> [OPTIONS]

Release a single plugin to the ADI plugin registry.

OPTIONS:
    --no-push           Build only, skip publishing
    --local             Push to local registry (localhost:8019)
    --bump <type>       Version bump type: patch, minor, major
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
    llm-extract         LLM extraction utilities
    llm-uzu             Local LLM inference (Apple Silicon)
    embed               Embedding utilities
    lang-*              Language plugins (rust, python, typescript, etc.)
    cli-lang-en         CLI English translations

    Hive Plugins:
    hive-runner-docker      Docker container runner
    hive-runner-compose     Docker Compose delegation
    hive-runner-podman      Podman container runner
    hive-obs-stdout         Stdout logging
    hive-obs-file           File logging
    hive-obs-loki           Grafana Loki integration
    hive-obs-prometheus     Prometheus metrics export
    hive-proxy-cors         CORS middleware
    hive-proxy-rate-limit   Rate limiting
    hive-proxy-ip-filter    IP allow/deny lists
    hive-proxy-headers      Header manipulation
    hive-proxy-compress     Response compression (gzip, brotli)
    hive-proxy-cache        Response caching with TTL
    hive-proxy-rewrite      URL path/query rewriting
    hive-proxy-auth-jwt     JWT authentication
    hive-proxy-auth-basic   HTTP Basic authentication
    hive-proxy-auth-api-key API key authentication
    hive-proxy-auth-oidc    OpenID Connect authentication
    hive-health-http        HTTP health checks
    hive-health-tcp         TCP health checks
    hive-health-cmd         Command health checks
    hive-health-grpc        gRPC health checks
    hive-health-postgres    PostgreSQL health checks
    hive-health-redis       Redis PING health checks
    hive-health-mysql       MySQL health checks
    hive-env-dotenv         .env file loading
    hive-env-vault          HashiCorp Vault secrets
    hive-env-1password      1Password secrets
    hive-env-aws-secrets    AWS Secrets Manager
    hive-rollout-recreate   Stop-start deployment
    hive-rollout-blue-green Zero-downtime deployment

EXAMPLES:
    $0 cocoon                   # Build and publish (current version)
    $0 cocoon --bump patch      # Bump patch version and publish
    $0 cocoon --bump minor      # Bump minor version and publish
    $0 cocoon --bump major      # Bump major version and publish
    $0 cocoon --no-push         # Build only (dry-run)
    $0 cocoon --local           # Build and publish to local registry

INSTALL:
    After publishing, install with:
    adi plugin install adi.cocoon
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
PLUGIN_ARCHIVE=""
PLUGIN_NAME=""
PLUGIN_DESC=""
PLUGIN_AUTHOR=""
PLUGIN_TYPE=""

# Bump semantic version
# Usage: bump_version <version> <bump_type>
# bump_type: patch, minor, major
bump_version() {
    local version="$1"
    local bump_type="$2"
    
    # Parse version components
    local major minor patch
    IFS='.' read -r major minor patch <<< "$version"
    
    # Remove any pre-release suffix for bumping
    patch="${patch%%-*}"
    
    case "$bump_type" in
        patch)
            patch=$((patch + 1))
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        *)
            error "Unknown bump type: $bump_type. Use patch, minor, or major."
            ;;
    esac
    
    echo "${major}.${minor}.${patch}"
}

# Update version in Cargo.toml (single source of truth)
update_plugin_version() {
    local cargo_toml="$1"
    local old_version="$2"
    local new_version="$3"

    # Check if version is workspace-managed
    if grep -q 'version\.workspace\s*=\s*true' "$cargo_toml" 2>/dev/null || \
       grep -q 'version = { workspace = true }' "$cargo_toml" 2>/dev/null; then
        warn "Cargo.toml uses workspace version, cannot bump individually"
        return 1
    fi

    sed -i '' "s/^version = \"$old_version\"/version = \"$new_version\"/" "$cargo_toml"
    success "Updated Cargo.toml: $old_version -> $new_version"
}

build_plugin() {
    local plugin_name="$1"
    local crate_dir="$2"
    local dist_dir="$3"

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
    PLUGIN_NAME=$(sed -n '/^\[plugin\]/,/^\[/{/^name = /p;}' "$generated_toml" | sed 's/name = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_DESC=$(sed -n '/^\[plugin\]/,/^\[/{/^description = /p;}' "$generated_toml" | sed 's/description = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_AUTHOR=$(sed -n '/^\[plugin\]/,/^\[/{/^author = /p;}' "$generated_toml" | sed 's/author = "\(.*\)"/\1/' | tr -d '\n')
    PLUGIN_TYPE=$(sed -n '/^\[plugin\]/,/^\[/{/^type = /p;}' "$generated_toml" | sed 's/type = "\(.*\)"/\1/' | tr -d '\n')

    PLUGIN_ID=$(require_value "$PLUGIN_ID" "Could not read plugin ID from Cargo.toml metadata")
    PLUGIN_VERSION=$(require_value "$PLUGIN_VERSION" "Could not read plugin version from Cargo.toml")

    info "Plugin: $PLUGIN_ID v$PLUGIN_VERSION"
    info "Building library (no standalone binary)..."

    # Get package name from Cargo.toml (may differ from plugin ID)
    # Handle case where plugin.toml is at root but Cargo.toml is in plugin/ subdir
    local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
    if [[ ! -f "$cargo_toml" ]] || grep -q '^\[workspace\]' "$cargo_toml" 2>/dev/null; then
        if [[ -f "$PROJECT_ROOT/$crate_dir/plugin/Cargo.toml" ]]; then
            cargo_toml="$PROJECT_ROOT/$crate_dir/plugin/Cargo.toml"
        fi
    fi
    local package_name
    package_name=$(grep '^name = ' "$cargo_toml" | head -1 | sed 's/name = "\(.*\)"/\1/')

    # Build ONLY the library (not the binary)
    # If crate has its own workspace, build from there
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
    local lib_path="$target_dir/release/${lib_name}.${lib_ext}"

    require_file "$lib_path" "Library not found: $lib_path"

    info "Built: $lib_path"

    # Create package
    local pkg_dir
    pkg_dir=$(mktemp -d)
    cp "$lib_path" "$pkg_dir/plugin.$lib_ext"
    cp "$generated_toml" "$pkg_dir/plugin.toml"

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
    local bump_type=""

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
            --bump)
                bump_type="$2"
                shift 2
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

    # Handle version bump if requested
    if [ -n "$bump_type" ]; then
        local cargo_toml="$PROJECT_ROOT/$crate_dir/Cargo.toml"
        require_file "$cargo_toml" "Cargo.toml not found in $crate_dir"

        local current_version
        current_version=$(grep '^version = ' "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/')

        local new_version
        new_version=$(bump_version "$current_version" "$bump_type")

        info "Bumping version: $current_version -> $new_version ($bump_type)"
        update_plugin_version "$cargo_toml" "$current_version" "$new_version"
    fi

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
