#!/bin/bash
# Commit changes in a submodule using Claude for validation and message generation
# Usage: adi workflow commit-submodule
# Example: ./commit-submodule.sh apps/infra-service-web --update-parent yes

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
Usage: $0 <submodule-path> [OPTIONS]

Commit changes in a submodule using Claude AI for validation and message generation.

OPTIONS:
    --update-parent     Update parent repo with new submodule reference (yes/no)
    -h, --help          Show this help

EXAMPLES:
    $0 apps/infra-service-web
    $0 crates/cli --update-parent no

EOF
    exit 0
}

main() {
    local submodule=""
    local update_parent="yes"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --update-parent)
                update_parent="$2"
                shift 2
                ;;
            *)
                if [[ -z "$submodule" ]]; then
                    submodule="$1"
                else
                    error "Unknown argument: $1"
                fi
                shift
                ;;
        esac
    done

    if [[ -z "$submodule" ]]; then
        error "Submodule path required. Run with --help for usage."
    fi

    ensure_command "claude" "Install Claude CLI"

    local submodule_path="$PROJECT_ROOT/$submodule"
    require_dir "$submodule_path" "Submodule not found: $submodule"

    # Check if it's actually a submodule
    if ! git -C "$PROJECT_ROOT" submodule status "$submodule" &>/dev/null; then
        error "$submodule is not a git submodule"
    fi

    cd "$submodule_path"

    # Check for changes
    if git diff --quiet && git diff --cached --quiet && [[ -z "$(git ls-files --others --exclude-standard)" ]]; then
        warn "No changes to commit in $submodule"
        exit 0
    fi

    info "Analyzing changes in $submodule..."
    echo ""

    # Get git status for Claude
    local git_status
    git_status=$(git status --short)
    
    local git_diff
    git_diff=$(git diff HEAD 2>/dev/null || git diff)

    # Step 1: Validate staged files with Claude
    info "Step 1: Validating files to commit..."
    
    local validation_prompt="You are validating git changes before commit. Check if any files should NOT be committed.

Git status:
$git_status

RULES - Files that should NEVER be committed:
- target/ directory (Rust build artifacts)
- node_modules/ directory
- .env files (secrets)
- *.log files
- .DS_Store
- Cargo.lock in libraries (ok in binaries/apps)
- Any files containing API keys, passwords, tokens

Respond with ONLY one of:
1. If all files are OK to commit: \"OK\"
2. If there are problematic files: \"REJECT: <reason>\" followed by list of files to exclude

Be strict. If in doubt, reject."

    local validation_result
    validation_result=$(echo "$validation_prompt" | claude -p 2>/dev/null) || {
        error "Claude validation failed"
    }

    echo "$validation_result"
    echo ""

    if [[ "$validation_result" == REJECT* ]]; then
        error "Validation failed. Please fix the issues above and try again."
    fi

    success "Validation passed"
    echo ""

    # Step 2: Generate commit message with Claude
    info "Step 2: Generating commit message..."

    local commit_prompt="Generate a git commit message for these changes.

Git diff:
$git_diff

Git status:
$git_status

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
        error "Claude message generation failed"
    }

    # Clean up message (remove quotes if present)
    commit_message=$(echo "$commit_message" | sed 's/^"//;s/"$//' | head -1)

    echo ""
    info "Commit message: $commit_message"
    echo ""

    # Stage all changes and commit
    git add -A
    git commit -m "$commit_message"
    
    success "Committed in submodule: $submodule"

    # Get the new commit hash
    local short_hash
    short_hash=$(git rev-parse --short HEAD)

    # Update parent repo if requested
    if [[ "$update_parent" == "yes" ]]; then
        echo ""
        info "Updating parent repo reference..."
        
        cd "$PROJECT_ROOT"
        
        # Extract short description from commit message (remove emoji and type prefix)
        local short_desc
        short_desc=$(echo "$commit_message" | sed 's/^[^ ]* [a-z]*: //')
        
        git add "$submodule"
        git commit -m "🔗 Update $submodule: $short_desc ($short_hash)"
        
        success "Updated parent repo with new submodule reference"
    fi

    echo ""
    success "Done! Submodule commit: $short_hash"
    
    if [[ "$update_parent" == "yes" ]]; then
        info "Don't forget to push both repos:"
        echo "  cd $submodule && git push"
        echo "  cd $PROJECT_ROOT && git push"
    else
        info "Don't forget to push the submodule:"
        echo "  cd $submodule && git push"
    fi
}

main "$@"
