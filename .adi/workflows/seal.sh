#!/bin/bash
# Seal - Commit and push all changes including submodules
# Usage: adi workflow seal
# Example: ./seal.sh --message "feat: add new feature" --push yes

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
    require_dir() { [[ -d "$1" ]] || error "${2:-Directory not found: $1}"; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
    ensure_command() { check_command "$1" || error "$1 not found${2:+. Install: $2}"; }
fi

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Seal - Commit and push all changes including submodules.

This workflow:
1. Finds all submodules with uncommitted changes
2. Commits changes in each submodule (with AI-generated messages)
3. Commits all parent repo changes (including submodule references)
4. Pushes everything to remote

OPTIONS:
    --message <msg>     Custom commit message for parent repo (AI-generated if empty)
    --push <yes|no>     Push after commit (default: yes)
    -h, --help          Show this help

EXAMPLES:
    $0                                    # Commit and push all with AI messages
    $0 --push no                          # Commit only, don't push
    $0 --message "chore: sync all"        # Custom parent commit message

EOF
    exit 0
}

# Generate commit message using Claude
generate_commit_message() {
    local git_status="$1"
    local git_diff="$2"
    local context="$3"
    
    local commit_prompt="Generate a git commit message for these changes.

Context: $context

Git status:
$git_status

Git diff (truncated):
$(echo "$git_diff" | head -200)

RULES:
- Use conventional commit format: <type>: <description>
- Types: feat, fix, refactor, docs, chore, perf, test, style
- Start with emoji matching type (feat=✨, fix=🐛, refactor=♻️, docs=📚, chore=🔧, perf=⚡, test=🧪, style=💄)
- Keep under 72 chars
- Be specific about what changed
- Use imperative mood (\"Add\" not \"Added\")

Respond with ONLY the commit message, nothing else. Single line."

    local commit_message
    commit_message=$(echo "$commit_prompt" | claude -p 2>/dev/null) || {
        echo "chore: update files"
        return
    }

    # Clean up message (remove quotes if present)
    echo "$commit_message" | sed 's/^"//;s/"$//' | head -1
}

# Commit changes in a submodule
commit_submodule() {
    local submodule="$1"
    local submodule_path="$PROJECT_ROOT/$submodule"
    
    cd "$submodule_path"
    
    # Check for changes
    if git diff --quiet && git diff --cached --quiet && [[ -z "$(git ls-files --others --exclude-standard)" ]]; then
        return 1  # No changes
    fi
    
    info "Committing changes in $submodule..."
    
    local git_status
    git_status=$(git status --short)
    
    local git_diff
    git_diff=$(git diff HEAD 2>/dev/null || git diff)
    
    local commit_message
    commit_message=$(generate_commit_message "$git_status" "$git_diff" "Submodule: $submodule")
    
    git add -A
    git commit -m "$commit_message"
    
    local short_hash
    short_hash=$(git rev-parse --short HEAD)
    
    success "Committed $submodule: $commit_message ($short_hash)"
    
    cd "$PROJECT_ROOT"
    return 0
}

main() {
    local custom_message=""
    local do_push="yes"
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --message)
                custom_message="$2"
                shift 2
                ;;
            --push)
                do_push="$2"
                shift 2
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done
    
    ensure_command "claude" "Install Claude CLI"
    
    cd "$PROJECT_ROOT"
    
    echo ""
    info "🔒 Sealing all changes..."
    echo ""
    
    # Step 1: Find and commit submodules with changes
    info "Step 1: Checking submodules for uncommitted changes..."
    
    local submodules_committed=()
    local submodules_to_push=()
    
    # Get list of submodules
    while IFS= read -r line; do
        # Parse submodule status line: "+hash path" or " hash path"
        local status_char="${line:0:1}"
        local submodule_path
        submodule_path=$(echo "$line" | awk '{print $2}')
        
        if [[ -z "$submodule_path" ]]; then
            continue
        fi
        
        local submodule_full_path="$PROJECT_ROOT/$submodule_path"
        
        if [[ ! -d "$submodule_full_path/.git" ]] && [[ ! -f "$submodule_full_path/.git" ]]; then
            continue
        fi
        
        cd "$submodule_full_path"
        
        # Check if submodule has uncommitted changes
        if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard 2>/dev/null)" ]]; then
            cd "$PROJECT_ROOT"
            if commit_submodule "$submodule_path"; then
                submodules_committed+=("$submodule_path")
                submodules_to_push+=("$submodule_path")
            fi
        # Check if submodule has commits ahead of remote
        elif git rev-parse --abbrev-ref --symbolic-full-name @{u} &>/dev/null; then
            local ahead
            ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || echo "0")
            if [[ "$ahead" -gt 0 ]]; then
                submodules_to_push+=("$submodule_path")
            fi
        fi
        
        cd "$PROJECT_ROOT"
    done < <(git submodule status 2>/dev/null || true)
    
    if [[ ${#submodules_committed[@]} -eq 0 ]]; then
        info "No submodules with uncommitted changes"
    else
        success "Committed ${#submodules_committed[@]} submodule(s)"
    fi
    echo ""
    
    # Step 2: Commit parent repo changes
    info "Step 2: Committing parent repo changes..."
    
    cd "$PROJECT_ROOT"
    
    # Check for changes in parent
    local parent_has_changes=false
    if ! git diff --quiet || ! git diff --cached --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
        parent_has_changes=true
    fi
    
    if [[ "$parent_has_changes" == "true" ]]; then
        local git_status
        git_status=$(git status --short)
        
        local git_diff
        git_diff=$(git diff HEAD 2>/dev/null || git diff)
        
        local commit_message="$custom_message"
        if [[ -z "$commit_message" ]]; then
            commit_message=$(generate_commit_message "$git_status" "$git_diff" "Parent repo with submodule updates")
        fi
        
        git add -A
        git commit -m "$commit_message"
        
        local short_hash
        short_hash=$(git rev-parse --short HEAD)
        
        success "Committed parent repo: $commit_message ($short_hash)"
    else
        info "No changes in parent repo"
    fi
    echo ""
    
    # Step 3: Push everything
    if [[ "$do_push" == "yes" ]]; then
        info "Step 3: Pushing all changes..."
        
        # Push submodules first
        for submodule in "${submodules_to_push[@]}"; do
            info "Pushing $submodule..."
            cd "$PROJECT_ROOT/$submodule"
            git push 2>/dev/null && success "Pushed $submodule" || warn "Failed to push $submodule (may need upstream set)"
            cd "$PROJECT_ROOT"
        done
        
        # Push parent repo
        cd "$PROJECT_ROOT"
        local ahead
        ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || echo "0")
        
        if [[ "$ahead" -gt 0 ]]; then
            info "Pushing parent repo..."
            git push && success "Pushed parent repo" || warn "Failed to push parent repo"
        else
            info "Parent repo already up to date with remote"
        fi
        
        echo ""
        success "🔒 Sealed and pushed!"
    else
        echo ""
        success "🔒 Sealed! (push skipped)"
        echo ""
        info "To push manually:"
        for submodule in "${submodules_to_push[@]}"; do
            echo "  cd $submodule && git push"
        done
        echo "  git push"
    fi
}

main "$@"
