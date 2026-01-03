#!/bin/bash
# =============================================================================
# Color and TTY Detection Library
# =============================================================================
# Provides color codes and terminal capability detection
#
# Usage:
#   source scripts/lib/colors.sh
#
# Exports:
#   Color codes: RED, GREEN, YELLOW, BLUE, CYAN, BOLD, DIM, NC
#   Functions: has_tty, in_multiplexer, supports_color, setup_colors
# =============================================================================

# Prevent double-loading
if [[ -n "${COLORS_LIB_LOADED}" ]]; then
    return 0
fi
COLORS_LIB_LOADED=1

# -----------------------------------------------------------------------------
# TTY Detection
# -----------------------------------------------------------------------------

# Check if running with a TTY
has_tty() {
    [ -t 0 ] && [ -t 1 ]
}

# Check if running in tmux/screen
in_multiplexer() {
    [ -n "$TMUX" ] || [ "$TERM" = "screen" ] || [[ "$TERM" == screen* ]]
}

# Check if terminal supports color
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

# -----------------------------------------------------------------------------
# Color Setup
# -----------------------------------------------------------------------------

# Initialize color codes based on terminal capabilities
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

    # Export for use in subshells
    export RED GREEN YELLOW BLUE CYAN BOLD DIM NC
}

# Auto-initialize on load
setup_colors
