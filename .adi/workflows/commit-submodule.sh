#!/bin/bash
# =============================================================================
# Submodule Commit Script
# =============================================================================
# Commits changes in a submodule with AI-generated commit messages,
# then updates the parent repo reference.
#
# Usage: adi workflow commit-submodule
#
# Features:
#   - AI-generated commit messages via Claude
#   - Safety check to verify no unexpected files are staged
#   - Auto-updates parent repo submodule reference
#   - Supports --push flag to push both repos
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

# Alias for compatibility
ROOT_DIR="$PROJECT_ROOT"

# =============================================================================
# Functions
# =============================================================================

# Get git changes summary for a directory
get_changes() {
    local dir="$1"
    cd "$dir"

    local staged_diff
    staged_diff=$(git diff --cached --stat 2>/dev/null || echo "")

    local staged_files
    staged_files=$(git diff --cached --name-only 2>/dev/null || echo "")

    local status
    status=$(git status --short 2>/dev/null || echo "")

    echo "=== Staged Files ==="
    echo "$staged_files"
    echo ""
    echo "=== Staged Diff Stats ==="
    echo "$staged_diff"
    echo ""
    echo "=== Full Status ==="
    echo "$status"
}

# Verify nothing unexpected will be committed using Claude
verify_commit_safety() {
    local dir="$1"
    local changes="$2"

    info "Verifying commit safety..."

    local prompt="You are reviewing git changes before commit. Analyze these changes and respond with EXACTLY one word:
- 'true' if only expected code/config changes will be committed (no secrets, credentials, .env files, node_modules, build artifacts, or other unexpected files)
- 'false' followed by a brief explanation if something unexpected or dangerous would be committed

Changes to review:
$changes"

    local result
    result=$(claude -p "$prompt" 2>/dev/null || echo "error")

    # Extract first word
    local first_word
    first_word=$(echo "$result" | head -1 | awk '{print $1}' | tr '[:upper:]' '[:lower:]')

    if [[ "$first_word" == "true" ]]; then
        success "Commit safety verified"
        return 0
    elif [[ "$first_word" == "false" ]]; then
        echo ""
        warn "Safety check failed:"
        echo "$result" | tail -n +1
        return 1
    else
        warn "Could not verify commit safety (Claude response: $result)"
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            error "Aborted by user"
        fi
        return 0
    fi
}

# Generate commit message using Claude
generate_commit_message() {
    local changes="$1"
    local context="$2"

    info "Generating commit message..."

    local prompt="Generate a concise git commit message for these changes. Follow these rules:
- Start with an emoji that reflects the change type (✨ feature, 🐛 fix, ♻️ refactor, 📝 docs, 🔧 config, etc.)
- One line summary (max 72 chars)
- If needed, add blank line then bullet points for details
- Be specific about what changed, not generic
- Do NOT include 'Co-Authored-By' or similar attributions

$context

Changes:
$changes"

    local message
    message=$(claude -p "$prompt" 2>/dev/null || echo "")

    if [ -z "$message" ]; then
        error "Failed to generate commit message"
    fi

    echo "$message"
}

# Show usage
usage() {
    echo "Usage: $0 <submodule-path> [--push] [--no-verify]"
    echo ""
    echo "Arguments:"
    echo "  submodule-path  Path to the submodule (e.g., crates/adi-cli)"
    echo ""
    echo "Options:"
    echo "  --push          Push both submodule and parent repo after commit"
    echo "  --no-verify     Skip safety verification"
    echo ""
    echo "Examples:"
    echo "  $0 crates/adi-cli"
    echo "  $0 apps/infra-service-web --push"
    exit 1
}

# =============================================================================
# Main
# =============================================================================

main() {
    local submodule_path=""
    local do_push=false
    local skip_verify=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --push)
                do_push=true
                shift
                ;;
            --no-verify)
                skip_verify=true
                shift
                ;;
            -h|--help)
                usage
                ;;
            -*)
                error "Unknown option: $1"
                ;;
            *)
                if [ -z "$submodule_path" ]; then
                    submodule_path="$1"
                else
                    error "Too many arguments"
                fi
                shift
                ;;
        esac
    done

    # Validate arguments
    if [ -z "$submodule_path" ]; then
        usage
    fi

    # Resolve submodule path
    local full_path
    if [[ "$submodule_path" = /* ]]; then
        full_path="$submodule_path"
    else
        full_path="$ROOT_DIR/$submodule_path"
    fi

    # Validate submodule exists
    require_dir "$full_path" "Submodule not found: $submodule_path"

    # Check it's a git repo
    if [ ! -d "$full_path/.git" ] && [ ! -f "$full_path/.git" ]; then
        error "Not a git repository: $submodule_path"
    fi

    # Check for changes in submodule
    cd "$full_path"

    local has_staged
    has_staged=$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')

    local has_unstaged
    has_unstaged=$(git status --short 2>/dev/null | wc -l | tr -d ' ')

    if [ "$has_staged" -eq 0 ] && [ "$has_unstaged" -eq 0 ]; then
        info "No changes in $submodule_path"
        exit 0
    fi

    # If nothing staged but there are changes, stage all
    if [ "$has_staged" -eq 0 ]; then
        info "Staging all changes..."
        git add -A
    fi

    # Get changes for analysis
    local changes
    changes=$(get_changes "$full_path")

    echo ""
    echo "=== Changes in $submodule_path ==="
    echo "$changes"
    echo ""

    # Verify commit safety
    if [ "$skip_verify" = false ]; then
        if ! verify_commit_safety "$full_path" "$changes"; then
            error "Commit safety verification failed"
        fi
    fi

    # Generate commit message for submodule
    local submodule_name
    submodule_name=$(basename "$submodule_path")

    local commit_msg
    commit_msg=$(generate_commit_message "$changes" "This is for the $submodule_name submodule.")

    echo ""
    echo "=== Proposed Commit Message ==="
    echo "$commit_msg"
    echo ""

    read -p "Accept this commit message? [Y/n/e(dit)] " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Ee]$ ]]; then
        # Let user edit the message
        local temp_file
        temp_file=$(mktemp)
        echo "$commit_msg" > "$temp_file"
        ${EDITOR:-vim} "$temp_file"
        commit_msg=$(cat "$temp_file")
        rm -f "$temp_file"
    elif [[ $REPLY =~ ^[Nn]$ ]]; then
        error "Aborted by user"
    fi

    # Ask user if they want to proceed with commit
    read -p "Proceed with commit? [Y/n] " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Nn]$ ]]; then
        info "Commit aborted by user"
        exit 0
    fi

    # Commit in submodule
    cd "$full_path"
    info "Committing in $submodule_path..."

    git commit --no-verify -m "$commit_msg"

    success "Committed in submodule"

    # Push submodule if requested or ask user
    if [ "$do_push" = true ]; then
        info "Pushing submodule..."
        git push
        success "Pushed submodule"
    fi

    # Update parent repo
    cd "$ROOT_DIR"

    # Check if submodule is tracked
    if ! git ls-files --stage "$submodule_path" | grep -q .; then
        warn "Submodule not tracked in parent repo, skipping parent update"
        exit 0
    fi

    # Stage the submodule update
    git add "$submodule_path"

    # Check if there's actually a change to commit
    if ! git diff --cached --quiet "$submodule_path"; then
        # Generate commit message for parent repo update
        local parent_changes
        parent_changes=$(git diff --cached --stat)

        # Get short description from submodule commit
        local short_desc
        short_desc=$(cd "$full_path" && git log -1 --format='%s' | head -c 60)

        local parent_msg
        parent_msg="🔗 Update $submodule_name: ${short_desc#* }"

        # Commit parent repo
        info "Updating parent repo reference..."
        git commit --no-verify -m "$parent_msg"

        success "Updated parent repo"

        # Push parent if requested
        if [ "$do_push" = true ]; then
            info "Pushing parent repo..."
            git push
            success "Pushed parent repo"
        fi
    else
        info "No submodule reference change to commit"
    fi

    echo ""
    success "Done!"

    # Ask user if they want to push (only if --push wasn't already provided)
    if [ "$do_push" = false ]; then
        echo ""
        read -p "Push changes to remote? [y/N] " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            # Push submodule
            cd "$full_path"
            info "Pushing submodule..."
            git push
            success "Pushed submodule"

            # Push parent if there was a parent commit
            cd "$ROOT_DIR"
            if git log -1 --format='%s' | grep -q "🔗 Update $submodule_name:"; then
                info "Pushing parent repo..."
                git push
                success "Pushed parent repo"
            fi
        fi
    fi
}

main "$@"
