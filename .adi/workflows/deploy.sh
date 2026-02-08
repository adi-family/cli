#!/bin/bash
# =============================================================================
# ADI Coolify Deployment Helper
# =============================================================================
# Usage: adi workflow deploy
#
# Commands:
#   status              Show deployment status for all services
#   deploy <service>    Deploy a service (or 'all' for all services)
#   logs <service>      Show deployment logs
#   list                List all deployments
#   watch <service>     Watch deployment progress live
#
# Services: auth, platform, signaling, web, analytics-ingestion, analytics, registry
#
# Environment:
#   COOLIFY_URL         Coolify instance URL (default: http://in.the-ihor.com)
#   COOLIFY_API_KEY     API token (required)
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

# Load .env.local if exists
if [ -f "$PROJECT_ROOT/.env.local" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        [[ -z "$line" || "$line" =~ ^# ]] && continue
        key="${line%%=*}"
        value="${line#*=}"
        value="${value%\"}"
        value="${value#\"}"
        value="${value%\'}"
        value="${value#\'}"
        if [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
            export "$key=$value"
        fi
    done < "$PROJECT_ROOT/.env.local"
fi

# Configuration
COOLIFY_URL="${COOLIFY_URL:-http://in.the-ihor.com}"
API_BASE="$COOLIFY_URL/api/v1"

ALL_SERVICES="auth platform signaling web analytics-ingestion analytics registry"

# -----------------------------------------------------------------------------
# Service Configuration
# -----------------------------------------------------------------------------

service_uuid() {
    case "$1" in
        auth)                echo "ngg488ogoc80c8wogowkckow" ;;
        platform)            echo "cosw4cw0gscso88w8sskgk8g" ;;
        signaling)           echo "t0k0owcw00w00s4w4o0c000w" ;;
        web)                 echo "tkg84kg0o0ok8gkcs8wcggck" ;;
        analytics-ingestion) echo "TODO_COOLIFY_UUID" ;;
        analytics)           echo "TODO_COOLIFY_UUID" ;;
        registry)            echo "TODO_COOLIFY_UUID" ;;
        *)                   echo "" ;;
    esac
}

service_name() {
    case "$1" in
        auth)                echo "Auth API" ;;
        platform)            echo "Platform API" ;;
        signaling)           echo "Signaling Server" ;;
        web)                 echo "Web UI" ;;
        analytics-ingestion) echo "Analytics Ingestion" ;;
        analytics)           echo "Analytics API" ;;
        registry)            echo "Plugin Registry" ;;
        *)                   echo "$1" ;;
    esac
}

# -----------------------------------------------------------------------------
# API Helpers
# -----------------------------------------------------------------------------

check_api_key() {
    require_env "COOLIFY_API_KEY" >/dev/null
}

api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"

    local args=(-s -X "$method" -H "Authorization: Bearer $COOLIFY_API_KEY" -H "Content-Type: application/json")

    if [ -n "$data" ]; then
        args+=(-d "$data")
    fi

    curl "${args[@]}" "$API_BASE$endpoint"
}

status_color() {
    local status="$1"
    case "$status" in
        running:healthy|running|finished|success)  echo "$GREEN" ;;
        running:unhealthy|running:unknown)         echo "$YELLOW" ;;
        queued|in_progress|building)               echo "$YELLOW" ;;
        exited*|failed|error|cancelled|stopped)    echo "$RED" ;;
        *)                                         echo "$NC" ;;
    esac
}

status_icon() {
    local status="$1"
    case "$status" in
        running:healthy|running|finished|success)  echo "●" ;;
        running:unhealthy|running:unknown)         echo "◐" ;;
        queued)                                    echo "○" ;;
        in_progress|building)                      echo "◐" ;;
        exited*|failed|error|cancelled|stopped)    echo "✗" ;;
        *)                                         echo "?" ;;
    esac
}

# -----------------------------------------------------------------------------
# Commands
# -----------------------------------------------------------------------------

cmd_status() {
    check_api_key

    echo -e "${BOLD}ADI Deployment Status${NC}"
    echo -e "${DIM}Coolify: $COOLIFY_URL${NC}"
    echo ""

    printf "%-12s %-20s %-20s\n" "SERVICE" "NAME" "STATUS"
    echo "────────────────────────────────────────────────────────"

    for service in $ALL_SERVICES; do
        local uuid
        local name

        uuid=$(service_uuid "$service")
        name=$(service_name "$service")

        local app_info
        local status

        app_info=$(api_call GET "/applications/$uuid" 2>/dev/null)
        status=$(echo "$app_info" | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")

        local color
        local icon

        color=$(status_color "$status")
        icon=$(status_icon "$status")

        printf "%-12s %-20s ${color}%s %s${NC}\n" "$service" "$name" "$icon" "$status"
    done
}

cmd_deploy() {
    check_api_key

    local service="$1"
    local force="$2"

    if [ -z "$service" ]; then
        echo -e "${RED}Error: Service name required${NC}"
        echo "Usage: deploy.sh deploy <service|all> [--force]"
        exit 1
    fi

    local services_to_deploy
    if [ "$service" = "all" ]; then
        services_to_deploy="$ALL_SERVICES"
    else
        local uuid

        uuid=$(service_uuid "$service")

        if [ -z "$uuid" ]; then
            echo -e "${RED}Error: Unknown service '$service'${NC}"
            echo "Available: $ALL_SERVICES"
            exit 1
        fi

        services_to_deploy="$service"
    fi

    local force_param=""
    if [ "$force" = "--force" ] || [ "$force" = "-f" ]; then
        force_param="&force=true"
    fi

    echo -e "${BOLD}Deploying services...${NC}"
    echo ""

    local deployment_info=""

    for svc in $services_to_deploy; do
        local uuid
        local name

        uuid=$(service_uuid "$svc")
        name=$(service_name "$svc")

        echo -ne "  ${CYAN}$name${NC}: Triggering deploy... "

        local result
        local deploy_uuid

        result=$(api_call GET "/deploy?uuid=$uuid$force_param" 2>/dev/null)
        deploy_uuid=$(echo "$result" | jq -r '.deployments[0].deployment_uuid // empty' 2>/dev/null)

        if [ -n "$deploy_uuid" ]; then
            echo -e "${GREEN}Started${NC} ($deploy_uuid)"
            deployment_info="$deployment_info $svc:$deploy_uuid"
        else
            local error
            error=$(echo "$result" | jq -r '.message // .error // "Unknown error"' 2>/dev/null)
            echo -e "${RED}Failed${NC}: $error"
        fi
    done

    echo ""

    if [ -n "$deployment_info" ]; then
        echo -e "${BOLD}Watching deployment progress...${NC}"
        echo -e "${DIM}Press Ctrl+C to stop watching${NC}"
        echo ""

        watch_deployments $deployment_info
    fi
}

watch_deployments() {
    local all_done=false

    while [ "$all_done" = false ]; do
        all_done=true
        local output=""

        for item in "$@"; do
            local svc="${item%%:*}"
            local deploy_uuid="${item#*:}"
            local name
            name=$(service_name "$svc")

            local deploy_info
            local status

            deploy_info=$(api_call GET "/deployments/$deploy_uuid" 2>/dev/null)
            status=$(echo "$deploy_info" | jq -r '.status // "unknown"' 2>/dev/null)

            local color
            local icon

            color=$(status_color "$status")
            icon=$(status_icon "$status")

            output="$output  $name: $color$icon $status$NC\n"

            case "$status" in
                queued|in_progress|building)
                    all_done=false
                    ;;
            esac
        done

        echo -ne "\033[${#@}A\033[J" 2>/dev/null || true
        echo -e "$output"

        if [ "$all_done" = false ]; then
            sleep 2
        fi
    done

    echo -e "${GREEN}All deployments completed!${NC}"
}

cmd_watch() {
    check_api_key

    local service="$1"

    if [ -z "$service" ]; then
        echo -e "${RED}Error: Service name required${NC}"
        exit 1
    fi

    local uuid
    uuid=$(service_uuid "$service")
    if [ -z "$uuid" ]; then
        echo -e "${RED}Error: Unknown service '$service'${NC}"
        exit 1
    fi

    local name
    name=$(service_name "$service")

    echo -e "${BOLD}Watching $name deployments...${NC}"
    echo -e "${DIM}Press Ctrl+C to stop${NC}"
    echo ""

    while true; do
        local deployments
        deployments=$(api_call GET "/applications/$uuid/deployments?take=1" 2>/dev/null)

        local status
        local commit

        status=$(echo "$deployments" | jq -r '.[0].status // "none"' 2>/dev/null)
        commit=$(echo "$deployments" | jq -r '.[0].commit // "none"' 2>/dev/null | head -c 7)

        local color
        local icon

        color=$(status_color "$status")
        icon=$(status_icon "$status")

        printf "\r  ${color}%s %-15s${NC} commit: %s   " "$icon" "$status" "$commit"

        case "$status" in
            finished|failed|error|cancelled|success)
                echo ""
                break
                ;;
        esac

        sleep 2
    done
}

cmd_logs() {
    check_api_key

    local service="$1"

    if [ -z "$service" ]; then
        echo -e "${RED}Error: Service name required${NC}"
        exit 1
    fi

    local uuid
    uuid=$(service_uuid "$service")
    if [ -z "$uuid" ]; then
        echo -e "${RED}Error: Unknown service '$service'${NC}"
        exit 1
    fi

    local name
    name=$(service_name "$service")

    local deployments
    deployments=$(api_call GET "/applications/$uuid/deployments?take=1" 2>/dev/null)
    local deploy_uuid
    deploy_uuid=$(echo "$deployments" | jq -r '.[0].deployment_uuid // empty' 2>/dev/null)

    if [ -z "$deploy_uuid" ]; then
        echo -e "${RED}No deployments found for $name${NC}"
        exit 1
    fi

    echo -e "${BOLD}Deployment logs for $name${NC}"
    echo -e "${DIM}Deployment: $deploy_uuid${NC}"
    echo ""

    local deploy_info
    deploy_info=$(api_call GET "/deployments/$deploy_uuid" 2>/dev/null)
    echo "$deploy_info" | jq -r '.logs // "No logs available"' 2>/dev/null
}

cmd_list() {
    check_api_key

    local service="$1"
    local take="${2:-5}"

    if [ -z "$service" ]; then
        echo -e "${RED}Error: Service name required${NC}"
        exit 1
    fi

    local uuid
    uuid=$(service_uuid "$service")
    if [ -z "$uuid" ]; then
        echo -e "${RED}Error: Unknown service '$service'${NC}"
        exit 1
    fi

    local name
    name=$(service_name "$service")

    echo -e "${BOLD}Recent deployments for $name${NC}"
    echo ""

    local deployments
    deployments=$(api_call GET "/applications/$uuid/deployments?take=$take" 2>/dev/null)

    printf "%-12s %-15s %s\n" "STATUS" "COMMIT" "CREATED"
    echo "────────────────────────────────────────────────"

    echo "$deployments" | jq -r '.[] | [.status, .commit[0:7], .created_at] | @tsv' 2>/dev/null | while IFS=$'\t' read -r status commit created; do
        local color
        color=$(status_color "$status")
        local icon
        icon=$(status_icon "$status")

        if [ -n "$created" ] && [ "$created" != "null" ]; then
            created=$(date -j -f "%Y-%m-%dT%H:%M:%S" "${created%%.*}" "+%m/%d %H:%M" 2>/dev/null || echo "$created")
        fi

        printf "${color}%-12s${NC} %-15s %s\n" "$icon $status" "$commit" "$created"
    done
}

cmd_help() {
    cat << 'EOF'
ADI Coolify Deployment Helper

USAGE:
    adi workflow deploy
    .adi/workflows/deploy.sh <command> [args]

COMMANDS:
    status              Show status of all services
    deploy <svc|all>    Deploy a service (use 'all' for all services)
    deploy <svc> -f     Force rebuild (no cache)
    watch <svc>         Watch deployment progress
    logs <svc>          Show deployment logs
    list <svc> [n]      List recent deployments (default: 5)
    help                Show this help

SERVICES:
    auth                Auth API (auth)
    platform            Platform API (platform)
    signaling           Signaling Server (signaling-server)
    web                 Web UI (infra-service-web)
    analytics-ingestion Analytics Ingestion (analytics-ingestion)
    analytics           Analytics API (analytics)
    registry            Plugin Registry (plugin-registry)

ENVIRONMENT:
    COOLIFY_URL         Coolify instance URL (default: http://in.the-ihor.com)
    COOLIFY_API_KEY     API token (required)

EXAMPLES:
    adi workflow deploy                     # Interactive mode
    .adi/workflows/deploy.sh status         # Check all services
    .adi/workflows/deploy.sh deploy web     # Deploy web UI
    .adi/workflows/deploy.sh deploy all     # Deploy everything
    .adi/workflows/deploy.sh deploy auth -f # Force rebuild auth
EOF
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

case "${1:-}" in
    status)     cmd_status ;;
    deploy)     cmd_deploy "$2" "$3" ;;
    watch)      cmd_watch "$2" ;;
    logs)       cmd_logs "$2" ;;
    list)       cmd_list "$2" "$3" ;;
    help|--help|-h|"")
                cmd_help ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo "Run 'adi workflow deploy' or '.adi/workflows/deploy.sh help' for usage"
        exit 1
        ;;
esac
