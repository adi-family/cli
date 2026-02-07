//! Bundled shell prelude for workflow scripts
//!
//! This module provides a self-contained bash prelude that is automatically
//! injected into every workflow step. It provides:
//! - Useful variables ($PROJECT_ROOT, $PLATFORM, $GIT_BRANCH, etc.)
//! - Logging functions (info, success, warn, error)
//! - Spinner/progress functions (spinner_start, spinner_stop, etc.)
//! - Prompt functions (prompt_confirm, prompt_input, etc.)
//! - Common utilities (require_file, ensure_dir, etc.)

/// The complete bundled prelude script
pub const PRELUDE: &str = r#"
# =============================================================================
# ADI Workflow Prelude (Bundled)
# =============================================================================
# This prelude is automatically injected into every workflow step.
# All variables and functions are available without any source statements.
# =============================================================================

# Prevent double-loading
if [[ -n "${_ADI_PRELUDE_LOADED}" ]]; then
    : # Already loaded
else
_ADI_PRELUDE_LOADED=1

# =============================================================================
# Variables
# =============================================================================

# Current working directory
CWD="${CWD:-$PWD}"

# Find project root (look for Cargo.toml, package.json, or .git)
_find_project_root() {
    local dir="$1"
    while [[ "$dir" != "/" ]]; do
        if [[ -f "$dir/Cargo.toml" ]] || [[ -f "$dir/package.json" ]] || [[ -d "$dir/.git" ]]; then
            echo "$dir"
            return 0
        fi
        dir="$(dirname "$dir")"
    done
    echo "$PWD"
}

PROJECT_ROOT="${PROJECT_ROOT:-$(_find_project_root "$PWD")}"
WORKFLOWS_DIR="${WORKFLOWS_DIR:-$PROJECT_ROOT/.adi/workflows}"

# Platform detection
_detect_os() {
    case "$(uname -s)" in
        Darwin*) echo "darwin" ;;
        Linux*)  echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)       echo "unknown" ;;
    esac
}

_detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)            echo "unknown" ;;
    esac
}

OS="${OS:-$(_detect_os)}"
ARCH="${ARCH:-$(_detect_arch)}"
PLATFORM="${PLATFORM:-${OS}-${ARCH}}"

# Git info
if command -v git &>/dev/null && git rev-parse --is-inside-work-tree &>/dev/null 2>&1; then
    GIT_ROOT="${GIT_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null)}"
    GIT_BRANCH="${GIT_BRANCH:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null)}"
else
    GIT_ROOT="${GIT_ROOT:-}"
    GIT_BRANCH="${GIT_BRANCH:-}"
fi

# Time
TIMESTAMP="${TIMESTAMP:-$(date '+%Y-%m-%d %H:%M:%S')}"
DATE="${DATE:-$(date '+%Y-%m-%d')}"

# Export all
export CWD PROJECT_ROOT WORKFLOWS_DIR OS ARCH PLATFORM GIT_ROOT GIT_BRANCH TIMESTAMP DATE

# =============================================================================
# Colors
# =============================================================================

_supports_color() {
    [[ -n "${FORCE_COLOR:-}" ]] && return 0
    [[ -t 1 ]] && return 0
    return 1
}

if _supports_color; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    DIM='\033[2m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' NC=''
fi

export RED GREEN YELLOW BLUE CYAN BOLD DIM NC

# =============================================================================
# Logging Functions
# =============================================================================

log()     { echo -e "${BLUE}[log]${NC} $1"; }
info()    { printf "${CYAN}info${NC} %s\n" "$1"; }
success() { printf "${GREEN}done${NC} %s\n" "$1"; }
warn()    { printf "${YELLOW}warn${NC} %s\n" "$1"; }
error()   { printf "${RED}error${NC} %s\n" "$1" >&2; exit 1; }

# =============================================================================
# TTY Detection
# =============================================================================

has_tty() { [[ -t 0 ]] && [[ -t 1 ]]; }

# =============================================================================
# Spinner Functions
# =============================================================================

_SPINNER_PID=""
_SPINNER_MSG=""

spinner_start() {
    _SPINNER_MSG="$1"
    if ! has_tty; then
        printf "%s... " "$_SPINNER_MSG"
        return
    fi
    tput civis 2>/dev/null || true
    (
        local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
        local i=0
        while true; do
            printf "\r${CYAN}%s${NC} %s" "${frames[$i]}" "$_SPINNER_MSG"
            i=$(( (i + 1) % 10 ))
            sleep 0.1
        done
    ) &
    _SPINNER_PID=$!
    disown $_SPINNER_PID 2>/dev/null
}

spinner_stop() {
    local status="${1:-success}"
    [[ -n "$_SPINNER_PID" ]] && { kill $_SPINNER_PID 2>/dev/null; wait $_SPINNER_PID 2>/dev/null; }
    _SPINNER_PID=""
    tput cnorm 2>/dev/null || true
    has_tty && printf "\r\033[K"
    case "$status" in
        success) printf "${GREEN}✓${NC} %s\n" "$_SPINNER_MSG" ;;
        error)   printf "${RED}✗${NC} %s\n" "$_SPINNER_MSG" ;;
        warn)    printf "${YELLOW}!${NC} %s\n" "$_SPINNER_MSG" ;;
        *)       printf "${CYAN}→${NC} %s: %s\n" "$_SPINNER_MSG" "$status" ;;
    esac
}

spinner_update() { _SPINNER_MSG="$1"; }

with_spinner() {
    local msg="$1"; shift
    spinner_start "$msg"
    if "$@" >/dev/null 2>&1; then
        spinner_stop "success"; return 0
    else
        spinner_stop "error"; return 1
    fi
}

# =============================================================================
# Progress Bar Functions
# =============================================================================

_PROGRESS_TOTAL=0
_PROGRESS_MSG=""
_PROGRESS_START=0

progress_bar() {
    local current="$1" total="$2" width="${3:-30}"
    [[ $total -eq 0 ]] && return
    local pct=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))
    local bar=""
    for ((i=0; i<filled; i++)); do bar+="█"; done
    for ((i=0; i<empty; i++)); do bar+="░"; done
    printf "${CYAN}%s${NC} %3d%%" "$bar" "$pct"
}

progress_start() {
    _PROGRESS_TOTAL="$1"
    _PROGRESS_MSG="$2"
    _PROGRESS_START=$(date +%s)
    has_tty && tput civis 2>/dev/null
    progress_update 0
}

progress_update() {
    local current="$1" msg="${2:-$_PROGRESS_MSG}"
    if has_tty; then
        printf "\r\033[K%s " "$msg"
        progress_bar "$current" "$_PROGRESS_TOTAL"
    else
        printf "."
    fi
}

progress_done() {
    local msg="${1:-$_PROGRESS_MSG}"
    has_tty && { tput cnorm 2>/dev/null; printf "\r\033[K"; }
    local elapsed=$(($(date +%s) - _PROGRESS_START))
    printf "${GREEN}✓${NC} %s ${DIM}(%ds)${NC}\n" "$msg" "$elapsed"
}

# =============================================================================
# Status Line Functions
# =============================================================================

_STATUS_ACTIVE=false

status_line() {
    _STATUS_ACTIVE=true
    has_tty && printf "\r\033[K${CYAN}→${NC} %s" "$1" || printf "%s\n" "$1"
}

status_done() {
    has_tty && $_STATUS_ACTIVE && printf "\r\033[K"
    printf "${GREEN}✓${NC} %s\n" "$1"
    _STATUS_ACTIVE=false
}

status_clear() {
    has_tty && $_STATUS_ACTIVE && printf "\r\033[K"
    _STATUS_ACTIVE=false
}

# =============================================================================
# Step Counter
# =============================================================================

_STEP_CURRENT=0
_STEP_TOTAL=0

steps_init() { _STEP_TOTAL="$1"; _STEP_CURRENT=0; }
step() {
    _STEP_CURRENT=$((_STEP_CURRENT + 1))
    printf "${DIM}[%d/%d]${NC} %s\n" "$_STEP_CURRENT" "$_STEP_TOTAL" "$1"
}

# =============================================================================
# Countdown
# =============================================================================

countdown() {
    local secs="$1" msg="${2:-Waiting}"
    has_tty && tput civis 2>/dev/null
    for ((i=secs; i>0; i--)); do
        has_tty && printf "\r\033[K${YELLOW}%s${NC} %s..." "$i" "$msg" || printf "%d... " "$i"
        sleep 1
    done
    has_tty && { printf "\r\033[K"; tput cnorm 2>/dev/null; } || printf "\n"
}

# =============================================================================
# Prompt Functions
# =============================================================================

prompt_confirm() {
    local msg="$1" default="${2:-n}"
    local suffix="[y/N]"; [[ "$default" =~ ^[Yy] ]] && suffix="[Y/n]"
    has_tty || { [[ "$default" =~ ^[Yy] ]]; return $?; }
    printf "${CYAN}?${NC} %s %s " "$msg" "$suffix"
    read -r reply
    [[ -z "$reply" ]] && reply="$default"
    [[ "$reply" =~ ^[Yy] ]]
}

prompt_input() {
    local msg="$1" default="$2"
    has_tty || { echo "$default"; return; }
    [[ -n "$default" ]] && printf "${CYAN}?${NC} %s ${DIM}(%s)${NC}: " "$msg" "$default" || printf "${CYAN}?${NC} %s: " "$msg"
    read -r reply
    echo "${reply:-$default}"
}

prompt_password() {
    local msg="$1"
    has_tty || return 1
    printf "${CYAN}?${NC} %s: " "$msg"
    read -rs reply; echo ""
    echo "$reply"
}

# =============================================================================
# Requirement Functions
# =============================================================================

require_value() {
    local val="$1" msg="${2:-Value is required}"
    [[ -z "$val" ]] && error "$msg"
    echo "$val"
}

require_env() {
    local var="$1"
    [[ -z "${!var}" ]] && error "Environment variable $var is not set"
    echo "${!var}"
}

require_file() {
    local file="$1" msg="${2:-File not found: $file}"
    [[ ! -f "$file" ]] && error "$msg"
}

require_dir() {
    local dir="$1" msg="${2:-Directory not found: $dir}"
    [[ ! -d "$dir" ]] && error "$msg"
}

# =============================================================================
# Command Functions
# =============================================================================

check_command() { command -v "$1" &>/dev/null; }

ensure_command() {
    local cmd="$1" hint="${2:-}"
    check_command "$cmd" && return
    [[ -n "$hint" ]] && error "$cmd not found. Install: $hint" || error "$cmd not found"
}

# =============================================================================
# Directory Functions
# =============================================================================

ensure_dir() { mkdir -p "$1"; }

in_project() { (cd "$PROJECT_ROOT" && "$@"); }

# =============================================================================
# Utility Functions
# =============================================================================

is_ci() {
    [[ -n "${CI:-}" ]] || [[ -n "${GITHUB_ACTIONS:-}" ]] || [[ -n "${GITLAB_CI:-}" ]]
}

is_interactive() { has_tty && ! is_ci; }

rel_path() { echo "${1#$PROJECT_ROOT/}"; }

# Print all available variables
prelude_info() {
    echo -e "${BOLD}ADI Workflow Variables${NC}"
    echo ""
    echo -e "${CYAN}Directories:${NC}"
    echo "  PROJECT_ROOT   = $PROJECT_ROOT"
    echo "  WORKFLOWS_DIR  = $WORKFLOWS_DIR"
    echo "  CWD            = $CWD"
    echo "  HOME           = $HOME"
    echo ""
    echo -e "${CYAN}Platform:${NC}"
    echo "  OS             = $OS"
    echo "  ARCH           = $ARCH"
    echo "  PLATFORM       = $PLATFORM"
    echo ""
    echo -e "${CYAN}Git:${NC}"
    echo "  GIT_ROOT       = $GIT_ROOT"
    echo "  GIT_BRANCH     = $GIT_BRANCH"
    echo ""
    echo -e "${CYAN}Time:${NC}"
    echo "  TIMESTAMP      = $TIMESTAMP"
    echo "  DATE           = $DATE"
}

# =============================================================================
# Cleanup
# =============================================================================

_prelude_cleanup() {
    [[ -n "$_SPINNER_PID" ]] && kill $_SPINNER_PID 2>/dev/null
    tput cnorm 2>/dev/null || true
}
trap _prelude_cleanup EXIT

fi
# End of prelude
"#;

/// Get the prelude script to inject before commands
pub fn get_prelude() -> &'static str {
    PRELUDE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_not_empty() {
        assert!(!PRELUDE.is_empty());
    }

    #[test]
    fn test_prelude_has_key_functions() {
        assert!(PRELUDE.contains("info()"));
        assert!(PRELUDE.contains("success()"));
        assert!(PRELUDE.contains("spinner_start()"));
        assert!(PRELUDE.contains("PROJECT_ROOT"));
    }
}
