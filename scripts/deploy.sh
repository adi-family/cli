#!/bin/bash
# =============================================================================
# ADI Coolify Deployment Helper
# =============================================================================
# Usage: ./scripts/deploy.sh <command> [service]
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Load .env.local if exists (export only valid KEY=value lines)
if [ -f "$PROJECT_DIR/.env.local" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
        # Skip comments and empty lines
        [[ -z "$line" || "$line" =~ ^# ]] && continue
        # Extract key (everything before first =)
        key="${line%%=*}"
        # Extract value (everything after first =)
        value="${line#*=}"
        # Remove surrounding quotes from value
        value="${value%\"}"
        value="${value#\"}"
        value="${value%\'}"
        value="${value#\'}"
        # Export if key looks valid
        if [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
            export "$key=$value"
        fi
    done < "$PROJECT_DIR/.env.local"
fi

# Configuration
COOLIFY_URL="${COOLIFY_URL:-http://in.the-ihor.com}"
API_BASE="$COOLIFY_URL/api/v1"

ALL_SERVICES="auth platform signaling web analytics-ingestion analytics registry"

# -----------------------------------------------------------------------------
# Service Configuration (functions for bash 3.2 compatibility)
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
# Colors
# -----------------------------------------------------------------------------

supports_color() {
    [ -n "$FORCE_COLOR" ] && return 0
    [ -t 1 ] && return 0
    return 1
}

if supports_color; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    DIM='\033[2m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' NC=''
fi

# -----------------------------------------------------------------------------
# Helpers
# -----------------------------------------------------------------------------

check_api_key() {
    if [ -z "$COOLIFY_API_KEY" ]; then
        echo -e "${RED}Error: COOLIFY_API_KEY not set${NC}"
        echo "Set it in your environment or .env.local file"
        exit 1
    fi
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
        uuid=$(service_uuid "$service")
        local name
        name=$(service_name "$service")

        # Get application status
        local app_info
        app_info=$(api_call GET "/applications/$uuid" 2>/dev/null)

        local status
        status=$(echo "$app_info" | jq -r '.status // "unknown"' 2>/dev/null || echo "unknown")

        local color
        color=$(status_color "$status")
        local icon
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
        uuid=$(service_uuid "$svc")
        local name
        name=$(service_name "$svc")

        echo -ne "  ${CYAN}$name${NC}: Triggering deploy... "

        local result
        result=$(api_call GET "/deploy?uuid=$uuid$force_param" 2>/dev/null)

        local deploy_uuid
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

    # Watch deployments
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
            deploy_info=$(api_call GET "/deployments/$deploy_uuid" 2>/dev/null)

            local status
            status=$(echo "$deploy_info" | jq -r '.status // "unknown"' 2>/dev/null)

            local color
            color=$(status_color "$status")
            local icon
            icon=$(status_icon "$status")

            output="$output  $name: $color$icon $status$NC\n"

            case "$status" in
                queued|in_progress|building)
                    all_done=false
                    ;;
            esac
        done

        # Clear and print status
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

        local status commit
        status=$(echo "$deployments" | jq -r '.[0].status // "none"' 2>/dev/null)
        commit=$(echo "$deployments" | jq -r '.[0].commit // "none"' 2>/dev/null | head -c 7)

        local color
        color=$(status_color "$status")
        local icon
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

    # Get latest deployment
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

        # Format timestamp
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
    ./scripts/deploy.sh <command> [args]

COMMANDS:
    status              Show status of all services
    deploy <svc|all>    Deploy a service (use 'all' for all services)
    deploy <svc> -f     Force rebuild (no cache)
    watch <svc>         Watch deployment progress
    logs <svc>          Show deployment logs
    list <svc> [n]      List recent deployments (default: 5)
    help                Show this help

SERVICES:
    auth                Auth API (adi-auth)
    platform            Platform API (adi-platform-api)
    signaling           Signaling Server (tarminal-signaling-server)
    web                 Web UI (infra-service-web)
    analytics-ingestion Analytics Ingestion (adi-analytics-ingestion)
    analytics           Analytics API (adi-analytics-api)
    registry            Plugin Registry (adi-plugin-registry-http)

ENVIRONMENT:
    COOLIFY_URL         Coolify instance URL (default: http://in.the-ihor.com)
    COOLIFY_API_KEY     API token (required)

EXAMPLES:
    ./scripts/deploy.sh status              # Check all services
    ./scripts/deploy.sh deploy web          # Deploy web UI
    ./scripts/deploy.sh deploy all          # Deploy everything
    ./scripts/deploy.sh deploy auth -f      # Force rebuild auth
    ./scripts/deploy.sh watch platform      # Watch platform deploy
    ./scripts/deploy.sh logs signaling      # Show signaling logs
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
        echo "Run './scripts/deploy.sh help' for usage"
        exit 1
        ;;
esac
