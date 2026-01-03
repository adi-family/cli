#!/bin/bash
# =============================================================================
# Logging Functions Library
# =============================================================================
# Provides consistent logging functions with color support
#
# Usage:
#   source scripts/lib/log.sh
#
# Functions:
#   log <message>       - Info message (blue)
#   info <message>      - Info message (cyan)
#   success <message>   - Success message (green)
#   warn <message>      - Warning message (yellow)
#   error <message>     - Error message (red), exits with code 1
# =============================================================================

# Prevent double-loading
if [[ -n "${LOG_LIB_LOADED}" ]]; then
    return 0
fi
LOG_LIB_LOADED=1

# Load colors library
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/colors.sh
source "$SCRIPT_LIB_DIR/colors.sh"

# -----------------------------------------------------------------------------
# Logging Functions
# -----------------------------------------------------------------------------

# Generic log message (blue prefix)
log() {
    echo -e "${BLUE}[log]${NC} $1"
}

# Info message (cyan prefix)
info() {
    printf "${CYAN}info${NC} %s\n" "$1"
}

# Success message (green prefix)
success() {
    printf "${GREEN}done${NC} %s\n" "$1"
}

# Warning message (yellow prefix)
warn() {
    printf "${YELLOW}warn${NC} %s\n" "$1"
}

# Error message (red prefix), exits script
error() {
    printf "${RED}error${NC} %s\n" "$1" >&2
    exit 1
}
