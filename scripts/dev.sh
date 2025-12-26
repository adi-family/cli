#!/bin/bash
# =============================================================================
# ADI Local Development Helper
# =============================================================================
# Usage: ./scripts/dev.sh <command>
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PID_DIR="$PROJECT_DIR/.dev"
LOG_DIR="$PROJECT_DIR/.dev/logs"

# All services
ALL_SERVICES="signaling auth web cocoon"
# Default services to start (cocoon is optional)
DEFAULT_SERVICES="signaling auth web"

# -----------------------------------------------------------------------------
# Service Configuration (functions for bash 3.2 compatibility)
# -----------------------------------------------------------------------------

service_dir() {
    case "$1" in
        signaling) echo "crates/tarminal-signaling-server" ;;
        auth)      echo "crates/adi-auth" ;;
        web)       echo "apps/infra-service-web" ;;
        cocoon)    echo "crates/cocoon" ;;
        *)         echo "" ;;
    esac
}

service_cmd() {
    case "$1" in
        signaling) echo "cargo run" ;;
        auth)      echo "cargo run -p adi-auth-http" ;;
        web)       echo "npm run dev" ;;
        cocoon)    echo "cargo run" ;;
        *)         echo "" ;;
    esac
}

service_port_name() {
    case "$1" in
        signaling) echo "adi-signaling" ;;
        auth)      echo "adi-auth" ;;
        web)       echo "adi-web" ;;
        cocoon)    echo "adi-cocoon" ;;
        *)         echo "" ;;
    esac
}

service_description() {
    case "$1" in
        signaling) echo "WebSocket relay for sync" ;;
        auth)      echo "Authentication API" ;;
        web)       echo "Next.js frontend" ;;
        cocoon)    echo "Worker container" ;;
        *)         echo "" ;;
    esac
}

# -----------------------------------------------------------------------------
# TTY and Color Detection (tmux/screen compatible)
# -----------------------------------------------------------------------------

has_tty() {
    [ -t 0 ] && [ -t 1 ]
}

in_multiplexer() {
    [ -n "$TMUX" ] || [ "$TERM" = "screen" ] || [[ "$TERM" == screen* ]]
}

supports_color() {
    [ -n "$FORCE_COLOR" ] && return 0
    if [ -t 1 ]; then
        case "$TERM" in
            xterm*|rxvt*|vt100|screen*|tmux*|linux|cygwin|ansi)
                return 0
                ;;
        esac
        if command -v tput &>/dev/null && [ "$(tput colors 2>/dev/null)" -ge 8 ]; then
            return 0
        fi
    fi
    return 1
}

setup_colors() {
    if supports_color; then
        RED='\033[0;31m'
        GREEN='\033[0;32m'
        YELLOW='\033[1;33m'
        BLUE='\033[0;34m'
        CYAN='\033[0;36m'
        BOLD='\033[1m'
        DIM='\033[2m'
        NC='\033[0m'
    else
        RED=''
        GREEN=''
        YELLOW=''
        BLUE=''
        CYAN=''
        BOLD=''
        DIM=''
        NC=''
    fi
}

setup_colors

# -----------------------------------------------------------------------------
# Logging Functions
# -----------------------------------------------------------------------------

log() { echo -e "${BLUE}[dev]${NC} $1"; }
success() { echo -e "${GREEN}[dev]${NC} $1"; }
warn() { echo -e "${YELLOW}[dev]${NC} $1"; }
error() { echo -e "${RED}[dev]${NC} $1"; exit 1; }

# -----------------------------------------------------------------------------
# Directory Setup
# -----------------------------------------------------------------------------

ensure_dirs() {
    mkdir -p "$PID_DIR" "$LOG_DIR"
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
    local cmd
    local port
    dir=$(service_dir "$service")
    cmd=$(service_cmd "$service")
    port=$(get_port "$service")

    if [ -z "$dir" ]; then
        error "Unknown service: $service"
    fi

    if is_running "$service"; then
        warn "$service is already running"
        return 0
    fi

    local service_dir="$PROJECT_DIR/$dir"
    if [ ! -d "$service_dir" ]; then
        error "Service directory not found: $service_dir"
    fi

    local lf pf
    lf=$(log_file "$service")
    pf=$(pid_file "$service")

    # Service-specific environment setup
    local env_cmd="PORT=$port"
    case "$service" in
        cocoon)
            local signaling_port
            signaling_port=$(get_port "signaling")
            env_cmd="$env_cmd SIGNALING_SERVER_URL=ws://localhost:$signaling_port/ws"
            ;;
        auth)
            # Auth service might need DATABASE_URL etc from .env.local
            if [ -f "$PROJECT_DIR/.env.local" ]; then
                set -a
                # shellcheck disable=SC1091
                source "$PROJECT_DIR/.env.local" 2>/dev/null || true
                set +a
            fi
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
    ensure_dirs
    local services="${1:-$DEFAULT_SERVICES}"
    # If empty string was passed, use default
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
        if [ -n "$(service_dir "$service")" ]; then
            stop_service "$service"
        fi
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

    ensure_dirs

    if [ -n "$service" ]; then
        local lf
        lf=$(log_file "$service")
        if [ ! -f "$lf" ]; then
            error "No logs for $service"
        fi
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
            if [ -f "$lf" ]; then
                log_files="$log_files $lf"
            fi
        done

        if [ -z "$log_files" ]; then
            error "No log files found. Start services first."
        fi

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
    ensure_dirs

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

    local signaling_port auth_port web_port cocoon_port
    signaling_port=$(get_port "signaling")
    auth_port=$(get_port "auth")
    web_port=$(get_port "web")
    cocoon_port=$(get_port "cocoon")

    echo -e "  ${CYAN}Web UI:${NC}      http://localhost:$web_port"
    echo -e "  ${CYAN}Auth API:${NC}    http://localhost:$auth_port"
    echo -e "  ${CYAN}Signaling:${NC}   ws://localhost:$signaling_port/ws"
    echo -e "  ${CYAN}Cocoon:${NC}      (internal, port $cocoon_port)"
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
    if [ -z "$service" ]; then
        error "Usage: ./scripts/dev.sh shell <service>"
    fi

    local dir
    dir=$(service_dir "$service")
    if [ -z "$dir" ]; then
        error "Unknown service: $service"
    fi

    local svc_dir="$PROJECT_DIR/$dir"
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
        local port desc
        port=$(get_port "$service")
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
