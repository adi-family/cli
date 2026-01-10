#!/bin/bash
# =============================================================================
# ADI Local Development Helper
# =============================================================================
# Usage: adi workflow dev
#
# When run through `adi workflow`, all prelude functions and variables
# are automatically available (info, success, spinner_start, $PROJECT_ROOT, etc.)
#
# Commands:
#   up          Start all services
#   down        Stop all services
#   restart     Restart all services
#   logs        View logs (follow mode)
#   status      Show service status
#   ports       Show assigned ports
#   clean       Stop services and clean PID files
#   shell <svc> Open shell to service directory
#
# Works in: terminal, tmux, screen, CI/CD pipelines
# =============================================================================

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

# Local directories
PID_DIR="$PROJECT_ROOT/.dev"
LOG_DIR="$PROJECT_ROOT/.dev/logs"

# All services
ALL_SERVICES="signaling auth platform web flowmap analytics-ingestion analytics cocoon registry cocoon-manager"
# Default services to start (cocoon, registry, cocoon-manager are optional)
DEFAULT_SERVICES="signaling auth platform web flowmap analytics-ingestion analytics"

# -----------------------------------------------------------------------------
# Service Configuration (functions for bash 3.2 compatibility)
# -----------------------------------------------------------------------------

service_dir() {
    case "$1" in
        signaling) echo "crates/tarminal-signaling-server" ;;
        auth)      echo "crates/adi-auth" ;;
        platform)  echo "crates/adi-platform-api" ;;
        web)       echo "apps/infra-service-web" ;;
        flowmap)   echo "apps/flowmap-api" ;;
        analytics-ingestion) echo "crates/adi-analytics-ingestion" ;;
        analytics) echo "crates/adi-analytics-api" ;;
        cocoon)    echo "crates/cocoon" ;;
        registry)  echo "crates/adi-plugin-registry-http" ;;
        cocoon-manager) echo "crates/cocoon-manager" ;;
        *)         echo "" ;;
    esac
}

service_cmd() {
    case "$1" in
        signaling) echo "cargo run" ;;
        auth)      echo "cargo run -p adi-auth-http" ;;
        platform)  echo "cargo run --bin adi-platform-api" ;;
        web)       echo "npm run dev" ;;
        flowmap)   echo "cargo run --release" ;;
        analytics-ingestion) echo "cargo run" ;;
        analytics) echo "cargo run" ;;
        cocoon)    echo "cargo run --features standalone" ;;
        registry)  echo "cargo run" ;;
        cocoon-manager) echo "cargo run" ;;
        *)         echo "" ;;
    esac
}

service_port_name() {
    case "$1" in
        signaling) echo "adi-signaling" ;;
        auth)      echo "adi-auth" ;;
        platform)  echo "adi-platform" ;;
        web)       echo "adi-web" ;;
        flowmap)   echo "adi-flowmap" ;;
        analytics-ingestion) echo "adi-analytics-ingestion" ;;
        analytics) echo "adi-analytics" ;;
        cocoon)    echo "adi-cocoon" ;;
        registry)  echo "adi-registry" ;;
        cocoon-manager) echo "adi-cocoon-manager" ;;
        *)         echo "" ;;
    esac
}

service_description() {
    case "$1" in
        signaling) echo "WebSocket relay for sync" ;;
        auth)      echo "Authentication API" ;;
        platform)  echo "Platform API (tasks, integrations)" ;;
        web)       echo "Next.js frontend" ;;
        flowmap)   echo "Code flow visualization API" ;;
        analytics-ingestion) echo "Analytics event ingestion (writes)" ;;
        analytics) echo "Analytics API (metrics, dashboards)" ;;
        cocoon)    echo "Worker container" ;;
        registry)  echo "Plugin registry (local)" ;;
        cocoon-manager) echo "Cocoon orchestration API" ;;
        *)         echo "" ;;
    esac
}

# -----------------------------------------------------------------------------
# Port Management (via ports-manager)
# -----------------------------------------------------------------------------

get_port() {
    local service="$1"
    local port_name
    port_name=$(service_port_name "$service")
    # ports-manager get outputs the port (auto-assigns if new)
    # Use tail -1 to get just the port number in case of auto-assign message
    ports-manager get "$port_name" 2>/dev/null | tail -1
}

# -----------------------------------------------------------------------------
# Process Management
# -----------------------------------------------------------------------------

pid_file() {
    echo "$PID_DIR/$1.pid"
}

log_file() {
    echo "$LOG_DIR/$1.log"
}

is_running() {
    local service="$1"
    local pf
    pf=$(pid_file "$service")
    if [ -f "$pf" ]; then
        local pid
        pid=$(cat "$pf")
        if kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
        # Stale PID file
        rm -f "$pf"
    fi
    return 1
}

start_service() {
    local service="$1"
    local dir
    dir=$(service_dir "$service")
    local cmd
    cmd=$(service_cmd "$service")
    local port
    port=$(get_port "$service")

    [ -z "$dir" ] && error "Unknown service: $service"

    if is_running "$service"; then
        warn "$service is already running"
        return 0
    fi

    local service_dir="$PROJECT_ROOT/$dir"
    [ ! -d "$service_dir" ] && error "Service directory not found: $service_dir"

    local lf
    lf=$(log_file "$service")
    local pf
    pf=$(pid_file "$service")

    # Service-specific environment setup
    local env_cmd="PORT=$port"
    case "$service" in
        cocoon)
            local signaling_port
            signaling_port=$(get_port "signaling")
            env_cmd="$env_cmd SIGNALING_SERVER_URL=ws://localhost:$signaling_port/ws"
            ;;
        registry)
            # Registry service needs REGISTRY_DATA_DIR from .env.local
            if [ -f "$PROJECT_ROOT/.env.local" ]; then
                # shellcheck disable=SC1091
                source "$PROJECT_ROOT/.env.local" 2>/dev/null || true
            fi
            local data_dir="${REGISTRY_DATA_DIR:-$PROJECT_ROOT/.dev/registry-data}"
            ensure_dir "$data_dir"
            env_cmd="$env_cmd REGISTRY_DATA_DIR=$data_dir"
            ;;
        web)
            local auth_port
            auth_port=$(get_port "auth")
            local platform_port
            platform_port=$(get_port "platform")
            local flowmap_port
            flowmap_port=$(get_port "flowmap")
            env_cmd="$env_cmd AUTH_API_URL=http://localhost:$auth_port"
            env_cmd="$env_cmd NEXT_PUBLIC_PLATFORM_API_URL=http://localhost:$platform_port"
            env_cmd="$env_cmd NEXT_PUBLIC_FLOWMAP_API_URL=http://localhost:$flowmap_port"
            ;;
        platform)
            # Platform service needs DATABASE_URL, JWT_SECRET from .env.local
            if [ -f "$PROJECT_ROOT/.env.local" ]; then
                # shellcheck disable=SC1091
                source "$PROJECT_ROOT/.env.local" 2>/dev/null || true
                [ -n "$JWT_SECRET" ] && env_cmd="$env_cmd JWT_SECRET=$JWT_SECRET"
            fi
            # Use platform-specific database if set, otherwise default
            if [ -n "$PLATFORM_DATABASE_URL" ]; then
                env_cmd="$env_cmd DATABASE_URL=$PLATFORM_DATABASE_URL"
            elif [ -n "$DATABASE_URL" ]; then
                env_cmd="$env_cmd DATABASE_URL=$DATABASE_URL"
            else
                env_cmd="$env_cmd DATABASE_URL=postgres://adi:adi@localhost:5432/adi_platform"
            fi
            ;;
        auth)
            # Auth service needs JWT_SECRET and SMTP config from .env.local
            if [ -f "$PROJECT_ROOT/.env.local" ]; then
                # shellcheck disable=SC1091
                source "$PROJECT_ROOT/.env.local" 2>/dev/null || true
                [ -n "$JWT_SECRET" ] && env_cmd="$env_cmd JWT_SECRET=$JWT_SECRET"
                # Load SMTP configuration
                [ -n "$SMTP_HOST" ] && env_cmd="$env_cmd SMTP_HOST=$SMTP_HOST"
                [ -n "$SMTP_PORT" ] && env_cmd="$env_cmd SMTP_PORT=$SMTP_PORT"
                [ -n "$SMTP_USERNAME" ] && env_cmd="$env_cmd SMTP_USERNAME=$SMTP_USERNAME"
                [ -n "$SMTP_PASSWORD" ] && env_cmd="$env_cmd SMTP_PASSWORD=$SMTP_PASSWORD"
                [ -n "$SMTP_FROM_EMAIL" ] && env_cmd="$env_cmd SMTP_FROM_EMAIL=$SMTP_FROM_EMAIL"
                [ -n "$SMTP_FROM_NAME" ] && env_cmd="$env_cmd SMTP_FROM_NAME='$SMTP_FROM_NAME'"
                [ -n "$SMTP_TLS" ] && env_cmd="$env_cmd SMTP_TLS=$SMTP_TLS"
            fi
            ;;
        cocoon-manager)
            # Cocoon manager needs database, signaling URL, and Docker config
            local signaling_port
            signaling_port=$(get_port "signaling")
            local db_dir="$PROJECT_ROOT/.dev/cocoon-manager-data"
            ensure_dir "$db_dir"
            env_cmd="$env_cmd DATABASE_URL=sqlite:$db_dir/cocoon-manager.db"
            env_cmd="$env_cmd SIGNALING_SERVER_URL=ws://localhost:$signaling_port/ws"
            env_cmd="$env_cmd COCOON_IMAGE=ghcr.io/adi-family/cocoon:latest"
            env_cmd="$env_cmd MAX_COCOONS=100"
            env_cmd="$env_cmd BIND_ADDRESS=0.0.0.0:$port"
            ;;
    esac

    log "Starting $service on port $port..."

    # Start service in background
    (
        cd "$service_dir"
        eval "export $env_cmd"
        eval "exec $cmd" >> "$lf" 2>&1
    ) &
    local pid=$!
    echo "$pid" > "$pf"

    # Wait for port to be listening (handles cargo compile time)
    local timeout=60
    local elapsed=0
    while ! nc -z localhost "$port" 2>/dev/null; do
        if ! kill -0 "$pid" 2>/dev/null; then
            rm -f "$pf"
            error "Failed to start $service. Check logs: $lf"
        fi
        if [ $elapsed -ge $timeout ]; then
            rm -f "$pf"
            kill "$pid" 2>/dev/null || true
            error "$service timed out after ${timeout}s. Check logs: $lf"
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    success "$service started (PID: $pid, port: $port)"
}

stop_service() {
    local service="$1"
    local pf
    pf=$(pid_file "$service")

    if [ ! -f "$pf" ]; then
        warn "$service is not running"
        return 0
    fi

    local pid
    pid=$(cat "$pf")

    if kill -0 "$pid" 2>/dev/null; then
        log "Stopping $service (PID: $pid)..."
        kill "$pid" 2>/dev/null || true

        # Wait for graceful shutdown
        local count=0
        while kill -0 "$pid" 2>/dev/null && [ $count -lt 10 ]; do
            sleep 0.5
            count=$((count + 1))
        done

        # Force kill if still running
        if kill -0 "$pid" 2>/dev/null; then
            warn "Force killing $service..."
            kill -9 "$pid" 2>/dev/null || true
        fi

        success "$service stopped"
    else
        warn "$service was not running (stale PID file)"
    fi

    rm -f "$pf"
}

# -----------------------------------------------------------------------------
# Commands
# -----------------------------------------------------------------------------

cmd_up() {
    ensure_dir "$PID_DIR"
    ensure_dir "$LOG_DIR"

    local services="${1:-$DEFAULT_SERVICES}"
    [ -z "$services" ] && services="$DEFAULT_SERVICES"

    log "Starting services: $services"
    echo ""

    for service in $services; do
        start_service "$service"
    done

    echo ""
    cmd_ports
    echo ""
    log "View logs: ./scripts/dev.sh logs"
    log "Stop services: ./scripts/dev.sh down"
}

cmd_down() {
    local services="${1:-$ALL_SERVICES}"
    [ -z "$services" ] && services="$ALL_SERVICES"

    log "Stopping services..."

    for service in $services; do
        [ -n "$(service_dir "$service")" ] && stop_service "$service"
    done

    success "All services stopped"
}

cmd_restart() {
    local services="${1:-$DEFAULT_SERVICES}"
    [ -z "$services" ] && services="$DEFAULT_SERVICES"

    log "Restarting services..."
    cmd_down "$services"
    sleep 1
    cmd_up "$services"
}

cmd_logs() {
    local service="${1:-}"
    local follow_flag="-f"

    if ! has_tty; then
        warn "No TTY detected, showing last 100 lines"
        follow_flag=""
    fi

    ensure_dir "$PID_DIR"
    ensure_dir "$LOG_DIR"

    if [ -n "$service" ]; then
        local lf
        lf=$(log_file "$service")
        [ ! -f "$lf" ] && error "No logs for $service"
        if [ -n "$follow_flag" ]; then
            tail -f "$lf"
        else
            tail -100 "$lf"
        fi
    else
        # Follow all logs
        local log_files=""
        for svc in $ALL_SERVICES; do
            local lf
            lf=$(log_file "$svc")
            [ -f "$lf" ] && log_files="$log_files $lf"
        done

        [ -z "$log_files" ] && error "No log files found. Start services first."

        if [ -n "$follow_flag" ]; then
            # shellcheck disable=SC2086
            tail -f $log_files
        else
            # shellcheck disable=SC2086
            tail -100 $log_files
        fi
    fi
}

cmd_status() {
    ensure_dir "$PID_DIR"
    ensure_dir "$LOG_DIR"

    echo -e "${BOLD}Service Status${NC}"
    echo ""
    printf "  %-12s %-10s %-8s %s\n" "SERVICE" "STATUS" "PORT" "PID"
    printf "  %-12s %-10s %-8s %s\n" "-------" "------" "----" "---"

    for service in $ALL_SERVICES; do
        local status pid port
        port=$(get_port "$service")

        if is_running "$service"; then
            pid=$(cat "$(pid_file "$service")")
            status="${GREEN}running${NC}"
        else
            pid="-"
            status="${DIM}stopped${NC}"
        fi

        printf "  %-12s %-18b %-8s %s\n" "$service" "$status" "$port" "$pid"
    done
    echo ""
}

cmd_ports() {
    echo -e "${BOLD}Service Ports${NC}"
    echo ""

    local signaling_port
    signaling_port=$(get_port "signaling")
    local auth_port
    auth_port=$(get_port "auth")
    local platform_port
    platform_port=$(get_port "platform")
    local web_port
    web_port=$(get_port "web")
    local flowmap_port
    flowmap_port=$(get_port "flowmap")
    local cocoon_port
    cocoon_port=$(get_port "cocoon")
    local manager_port
    manager_port=$(get_port "cocoon-manager")

    echo -e "  ${CYAN}Web UI:${NC}           http://localhost:$web_port"
    echo -e "  ${CYAN}FlowMap UI:${NC}       http://localhost:$web_port/flowmap"
    echo -e "  ${CYAN}Auth API:${NC}         http://localhost:$auth_port"
    echo -e "  ${CYAN}Platform API:${NC}     http://localhost:$platform_port"
    echo -e "  ${CYAN}FlowMap API:${NC}      http://localhost:$flowmap_port"
    echo -e "  ${CYAN}Cocoon Manager:${NC}   http://localhost:$manager_port"
    echo -e "  ${CYAN}Signaling:${NC}        ws://localhost:$signaling_port/ws"
    echo -e "  ${CYAN}Cocoon:${NC}           (internal, port $cocoon_port)"
    echo ""
}

cmd_clean() {
    log "Stopping all services and cleaning up..."
    cmd_down "$ALL_SERVICES"

    rm -f "$PID_DIR"/*.pid 2>/dev/null || true
    rm -f "$LOG_DIR"/*.log 2>/dev/null || true

    success "Cleaned up PID files and logs"
}

cmd_shell() {
    local service="${1:-}"
    [ -z "$service" ] && error "Usage: ./scripts/dev.sh shell <service>"

    local dir
    dir=$(service_dir "$service")
    [ -z "$dir" ] && error "Unknown service: $service"

    local svc_dir="$PROJECT_ROOT/$dir"
    log "Opening shell in $svc_dir"
    cd "$svc_dir" && exec "${SHELL:-bash}"
}

cmd_help() {
    echo -e "${BOLD}ADI Local Development Helper${NC}"
    echo ""
    echo "Usage: ./scripts/dev.sh <command> [service...]"
    echo ""
    echo -e "${BOLD}Commands:${NC}"
    echo "  up [services]     Start services (default: signaling auth web)"
    echo "  down [services]   Stop services"
    echo "  restart [svcs]    Restart services"
    echo "  logs [service]    View logs (follow mode)"
    echo "  status            Show service status"
    echo "  ports             Show assigned ports"
    echo "  clean             Stop + remove PID files and logs"
    echo "  shell <service>   cd to service directory"
    echo ""
    echo -e "${BOLD}Services:${NC}"
    for service in $ALL_SERVICES; do
        local port
        port=$(get_port "$service")
        local desc
        desc=$(service_description "$service")
        printf "  %-12s port %-5s  %s\n" "$service" "$port" "$desc"
    done
    echo ""
    echo -e "${BOLD}Examples:${NC}"
    echo "  ./scripts/dev.sh up                  # Start default services"
    echo "  ./scripts/dev.sh up cocoon           # Start only cocoon"
    echo "  ./scripts/dev.sh up signaling auth   # Start specific services"
    echo "  ./scripts/dev.sh logs auth           # Follow auth logs"
    echo "  ./scripts/dev.sh status              # Show all service status"
    echo ""
    echo -e "${BOLD}Environment:${NC}"
    if in_multiplexer; then
        echo -e "  Running in: ${GREEN}tmux/screen${NC}"
    else
        echo "  Running in: terminal"
    fi
    if has_tty; then
        echo -e "  TTY: ${GREEN}available${NC}"
    else
        echo -e "  TTY: ${YELLOW}not available${NC}"
    fi
    echo ""
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

case "${1:-}" in
    up)         shift; cmd_up "$*" ;;
    down)       shift; cmd_down "$*" ;;
    restart)    shift; cmd_restart "$*" ;;
    logs)       cmd_logs "$2" ;;
    status)     cmd_status ;;
    ports)      cmd_ports ;;
    clean)      cmd_clean ;;
    shell)      cmd_shell "$2" ;;
    help|--help|-h|"")  cmd_help ;;
    *)          error "Unknown command: $1. Run './scripts/dev.sh help' for usage." ;;
esac
