#!/bin/bash
# Reset ADI installation - remove local data for clean reinstall
# Usage: adi workflow reset
# Example: ./reset.sh --scope all

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
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
fi

# Platform-specific paths
get_data_dir() {
    if [[ "$(uname -s)" == "Darwin" ]]; then
        echo "$HOME/Library/Application Support"
    else
        echo "${XDG_DATA_HOME:-$HOME/.local/share}"
    fi
}

get_cache_dir() {
    if [[ "$(uname -s)" == "Darwin" ]]; then
        echo "$HOME/Library/Caches"
    else
        echo "${XDG_CACHE_HOME:-$HOME/.cache}"
    fi
}

get_config_dir() {
    echo "${ADI_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/adi}"
}

DATA_DIR="$(get_data_dir)"
CACHE_DIR="$(get_cache_dir)"
CONFIG_DIR="$(get_config_dir)"

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Reset ADI installation by removing local data, plugins, caches, and configuration.

OPTIONS:
    --scope <scope>     What to reset (default: all)
    --yes               Skip confirmation prompt
    -h, --help          Show this help

SCOPES:
    all         Remove everything (full clean slate)
    plugins     Remove only plugins and plugin data
    cache       Remove only caches (registry, models)
    config      Remove only configuration files
    hive        Remove only hive daemon state

EXAMPLES:
    $0                          # Interactive (prompts for confirmation)
    $0 --scope all --yes        # Full reset, no prompt
    $0 --scope plugins          # Remove only plugins
    $0 --scope cache            # Clear caches only

WHAT GETS REMOVED (scope=all):
    Binaries:
        ~/.local/bin/adi
        ~/.local/bin/cocoon

    Plugins + Data (macOS / Linux):
        ~/Library/Application Support/adi/     (~/.local/share/adi/)
        ~/Library/Application Support/com.adi.adi/

    Cache (macOS / Linux):
        ~/Library/Caches/adi/                  (~/.cache/adi/)
        ~/Library/Caches/com.adi.adi/

    Configuration:
        ~/.config/adi/
        ~/.config/cocoon/

    Hive + Global State:
        ~/.adi/hive/
        ~/.adi/workflows/
        ~/.adi/daemon.*
        ~/.adi/plugins/
        ~/.adi/tree/
        ~/.adi/cache/

    Shell Completions:
        ~/.zfunc/_adi
        ~/.local/share/bash-completion/completions/adi.bash
        ~/.bash_completion.d/adi.bash
        ~/.config/fish/completions/adi.fish
        ~/.elvish/lib/adi.elv

    Service Files (macOS / Linux):
        ~/Library/LaunchAgents/com.adi.cocoon.plist
        ~/.config/systemd/user/cocoon.service

    Temp Files:
        /tmp/adi-hive-*
        \$TMPDIR/adi-update/

NOTE: Per-project .adi/ directories are NOT removed.
      Shell config modifications (~/.zshrc completions block) are NOT removed.
EOF
    exit 0
}

# Track what was removed for summary
REMOVED=()
SKIPPED=()

safe_rm() {
    local path="$1"
    local label="$2"
    if [[ -e "$path" ]] || [[ -L "$path" ]]; then
        rm -rf "$path"
        REMOVED+=("$label ($path)")
    else
        SKIPPED+=("$label")
    fi
}

# --- Scope: stop services ---

stop_services() {
    info "Stopping running services..."

    # Stop hive daemon
    if [[ -f "$HOME/.adi/hive/hive.pid" ]]; then
        local pid
        pid=$(cat "$HOME/.adi/hive/hive.pid" 2>/dev/null || true)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            success "Stopped hive daemon (PID $pid)"
        fi
    fi

    # Unload cocoon launchd agent (macOS)
    if [[ "$(uname -s)" == "Darwin" ]]; then
        if [[ -f "$HOME/Library/LaunchAgents/com.adi.cocoon.plist" ]]; then
            launchctl unload "$HOME/Library/LaunchAgents/com.adi.cocoon.plist" 2>/dev/null || true
            success "Unloaded cocoon launchd agent"
        fi
    fi

    # Stop cocoon systemd service (Linux)
    if [[ "$(uname -s)" == "Linux" ]]; then
        if systemctl --user is-active cocoon.service &>/dev/null; then
            systemctl --user stop cocoon.service 2>/dev/null || true
            systemctl --user disable cocoon.service 2>/dev/null || true
            success "Stopped cocoon systemd service"
        fi
    fi
}

# --- Scope: binaries ---

reset_binaries() {
    info "Removing binaries..."
    safe_rm "$HOME/.local/bin/adi" "adi binary"
    safe_rm "$HOME/.local/bin/cocoon" "cocoon binary"
}

# --- Scope: plugins ---

reset_plugins() {
    info "Removing plugins and plugin data..."

    # Main plugins directory
    safe_rm "$DATA_DIR/adi/plugins" "plugins directory"
    safe_rm "$DATA_DIR/adi/tools" "tools directory"
    safe_rm "$DATA_DIR/adi/tools.db" "tools database"
    safe_rm "$DATA_DIR/adi/tasks" "tasks directory"
    safe_rm "$DATA_DIR/adi/knowledgebase" "knowledgebase directory"

    # Plugin data directories (adi/<plugin-id>/ pattern)
    if [[ -d "$DATA_DIR/adi" ]]; then
        for dir in "$DATA_DIR/adi"/*/; do
            [[ -d "$dir" ]] || continue
            local name
            name=$(basename "$dir")
            # Skip the top-level dirs we handle explicitly
            case "$name" in
                plugins|tools|tasks|knowledgebase) continue ;;
            esac
            safe_rm "$dir" "plugin data: $name"
        done
    fi

    # com.adi.adi data (models, embeddings)
    if [[ "$(uname -s)" == "Darwin" ]]; then
        safe_rm "$DATA_DIR/com.adi.adi" "models + embeddings (com.adi.adi)"
    fi

    # ~/.adi/plugins (legacy/alternate location)
    safe_rm "$HOME/.adi/plugins" "~/.adi/plugins"
}

# --- Scope: cache ---

reset_cache() {
    info "Removing caches..."
    safe_rm "$CACHE_DIR/adi" "adi cache"
    if [[ "$(uname -s)" == "Darwin" ]]; then
        safe_rm "$CACHE_DIR/com.adi.adi" "com.adi.adi cache"
    fi
    safe_rm "$HOME/.adi/cache" "~/.adi/cache"

    # Temp files
    rm -rf /tmp/adi-hive-*.conf 2>/dev/null || true
    rm -rf "${TMPDIR:-/tmp}/adi-update" 2>/dev/null || true
}

# --- Scope: config ---

reset_config() {
    info "Removing configuration..."
    safe_rm "$CONFIG_DIR" "adi config"
    safe_rm "$HOME/.config/cocoon" "cocoon config"
    safe_rm "$HOME/.adi/daemon.config.json" "daemon config"
    safe_rm "$HOME/.adi/daemon.pid" "daemon pid"
    safe_rm "$HOME/.adi/config.toml" "global config"
}

# --- Scope: hive ---

reset_hive() {
    info "Removing hive state..."
    safe_rm "$HOME/.adi/hive" "hive directory"
    safe_rm "$HOME/.adi/workflows" "global workflows"
    safe_rm "$HOME/.adi/tree" "tree index"
}

# --- Scope: completions + service files ---

reset_completions() {
    info "Removing shell completions..."
    safe_rm "$HOME/.zfunc/_adi" "zsh completions"
    safe_rm "$HOME/.local/share/bash-completion/completions/adi.bash" "bash completions (XDG)"
    safe_rm "$HOME/.bash_completion.d/adi.bash" "bash completions (fallback)"
    safe_rm "$HOME/.config/fish/completions/adi.fish" "fish completions"
    safe_rm "$HOME/.elvish/lib/adi.elv" "elvish completions"
}

reset_service_files() {
    info "Removing service files..."
    if [[ "$(uname -s)" == "Darwin" ]]; then
        safe_rm "$HOME/Library/LaunchAgents/com.adi.cocoon.plist" "cocoon launchd plist"
    else
        safe_rm "$HOME/.config/systemd/user/cocoon.service" "cocoon systemd service"
    fi
}

# --- Cleanup empty parent directories ---

cleanup_empty_dirs() {
    # Remove adi data dir if empty
    if [[ -d "$DATA_DIR/adi" ]]; then
        rmdir "$DATA_DIR/adi" 2>/dev/null || true
    fi
    # Remove ~/.adi if empty
    if [[ -d "$HOME/.adi" ]]; then
        # Remove .DS_Store so rmdir can succeed
        rm -f "$HOME/.adi/.DS_Store" 2>/dev/null || true
        rmdir "$HOME/.adi" 2>/dev/null || true
    fi
}

# --- Print summary ---

print_summary() {
    echo ""
    if [[ ${#REMOVED[@]} -gt 0 ]]; then
        success "Removed ${#REMOVED[@]} item(s):"
        for item in "${REMOVED[@]}"; do
            printf "  ${GREEN}✓${NC} %s\n" "$item"
        done
    fi

    if [[ ${#SKIPPED[@]} -gt 0 ]]; then
        echo ""
        info "Already clean (${#SKIPPED[@]} item(s) not found):"
        for item in "${SKIPPED[@]}"; do
            printf "  ${DIM}· %s${NC}\n" "$item"
        done
    fi

    echo ""
    if [[ ${#REMOVED[@]} -gt 0 ]]; then
        success "ADI reset complete. Ready for fresh install."
    else
        info "Nothing to remove — ADI is already clean."
    fi
}

main() {
    local scope="all"
    local skip_confirm=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --scope)
                scope="$2"
                shift 2
                ;;
            --yes|-y)
                skip_confirm=true
                shift
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done

    # Validate scope
    case "$scope" in
        all|plugins|cache|config|hive) ;;
        *) error "Unknown scope: $scope. Valid: all, plugins, cache, config, hive" ;;
    esac

    echo ""
    info "ADI Reset (scope: $scope)"
    echo ""

    # Confirmation (when run directly, not via workflow which has its own confirm)
    if [[ "$skip_confirm" != "true" ]] && [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
        printf "${YELLOW}warn${NC} This will permanently delete ADI data (scope: %s). Continue? [y/N] " "$scope"
        read -r answer
        if [[ "$answer" != "y" ]] && [[ "$answer" != "Y" ]]; then
            warn "Reset cancelled"
            exit 0
        fi
    fi

    # Always stop services first
    stop_services

    case "$scope" in
        all)
            reset_binaries
            reset_plugins
            reset_cache
            reset_config
            reset_hive
            reset_completions
            reset_service_files
            ;;
        plugins)
            reset_plugins
            ;;
        cache)
            reset_cache
            ;;
        config)
            reset_config
            ;;
        hive)
            reset_hive
            ;;
    esac

    cleanup_empty_dirs
    print_summary
}

main "$@"
