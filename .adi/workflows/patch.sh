#!/usr/bin/env bash
# Quick build + replace for CLI binary or plugin (local dev iteration)
# Usage: patch.sh cli | patch.sh plugin <plugin-id>
set -euo pipefail

# Prelude fallback (when NOT run via `adi workflow`)
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
    CWD="$PWD"
    # Colors
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' BLUE='\033[0;34m' BOLD='\033[1m' DIM='\033[2m' NC='\033[0m'
    # Logging
    log() { echo -e "${BLUE}[log]${NC} $1"; }
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
    # Utils
    ensure_dir() { mkdir -p "$1"; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
    ensure_command() { check_command "$1" || error "$1 not found${2:+. Install: $2}"; }
fi

CLI_BIN="$HOME/.local/bin/adi"

usage() {
    cat <<EOF
Usage: $0 <target> [OPTIONS]

Quick build + replace for local development.

TARGETS:
    cli                     Build CLI and replace $CLI_BIN (with macOS codesign)
    plugin <plugin-id>      Build and reinstall plugin locally (force replace, skip lint)

EXAMPLES:
    $0 cli                          # Rebuild adi binary, replace, codesign
    $0 plugin adi.hive              # Rebuild and reinstall hive plugin
    $0 plugin adi.tasks             # Rebuild and reinstall tasks plugin

OPTIONS:
    -h, --help              Show this help
EOF
    exit 0
}

patch_cli() {
    ensure_command "cargo"

    echo ""
    info "Building CLI (release)..."
    (cd "$PROJECT_ROOT" && cargo build --release -p cli)

    local built_bin="$PROJECT_ROOT/target/release/adi"
    if [[ ! -f "$built_bin" ]]; then
        error "Binary not found: $built_bin"
    fi

    local size
    size=$(du -h "$built_bin" | cut -f1)
    success "Built: $built_bin ($size)"

    # Replace installed binary
    ensure_dir "$(dirname "$CLI_BIN")"

    if [[ -f "$CLI_BIN" ]]; then
        info "Replacing $CLI_BIN..."
    else
        info "Installing to $CLI_BIN..."
    fi
    cp -f "$built_bin" "$CLI_BIN"
    chmod +x "$CLI_BIN"

    # Codesign on macOS (ad-hoc signing)
    if [[ "$(uname -s)" == "Darwin" ]]; then
        info "Signing binary for macOS (ad-hoc)..."
        codesign -s - -f "$CLI_BIN" 2>/dev/null && \
            success "Signed: $CLI_BIN" || \
            warn "Codesign failed (non-fatal)"
    fi

    echo ""
    success "CLI patched: $CLI_BIN ($size)"
    "$CLI_BIN" --version 2>/dev/null || true
}

patch_plugin() {
    local plugin_id="$1"
    [[ -z "$plugin_id" ]] && error "Plugin ID required. Example: $0 plugin adi.hive"

    echo ""
    info "Patching plugin: $plugin_id"

    "$WORKFLOWS_DIR/build-plugin.sh" "$plugin_id" --install --force --skip-lint
}

# Parse arguments
[[ $# -eq 0 ]] && usage

case "${1:-}" in
    -h|--help)
        usage
        ;;
    cli)
        patch_cli
        ;;
    plugin)
        patch_plugin "${2:-}"
        ;;
    *)
        error "Unknown target: $1. Use 'cli' or 'plugin <id>'."
        ;;
esac
