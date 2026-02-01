#!/bin/bash
# Generate API documentation for Rust crates with LLM enrichment and translations
# Usage: adi workflow autodoc
# Example: adi workflow autodoc

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
    # Utils
    ensure_dir() { mkdir -p "$1"; }
    check_command() { command -v "$1" >/dev/null 2>&1; }
    ensure_command() { check_command "$1" || error "$1 not found${2:+. Install: $2}"; }
    require_file() { [[ -f "$1" ]] || error "${2:-File not found: $1}"; }
    require_dir() { [[ -d "$1" ]] || error "${2:-Directory not found: $1}"; }
    require_value() { [[ -n "$1" ]] || error "${2:-Value required}"; echo "$1"; }
fi

# Constants
DOCS_DIR="$PROJECT_ROOT/.adi/docs"
TEMP_DIR="$PROJECT_ROOT/.adi/tmp/autodoc"

# Language name lookup (bash 3.2 compatible)
get_lang_name() {
    local code="$1"
    case "$code" in
        en) echo "English" ;;
        uk) echo "Ukrainian" ;;
        ru) echo "Russian" ;;
        zh) echo "Chinese" ;;
        ja) echo "Japanese" ;;
        ko) echo "Korean" ;;
        es) echo "Spanish" ;;
        de) echo "German" ;;
        fr) echo "French" ;;
        *) echo "" ;;
    esac
}

usage() {
    cat <<EOF
Usage: $0 <crate-name> [OPTIONS]

Generate API documentation for a Rust crate with optional LLM enrichment.

OPTIONS:
    --lang <code>       Language code (en, uk, ru, zh, ja, ko, es, de, fr)
                        Default: en
    --enrich            Enrich documentation with LLM (examples, descriptions)
    --force             Overwrite existing documentation
    -h, --help          Show this help

EXAMPLES:
    $0 lib-embed                          # Generate English docs
    $0 lib-embed --lang uk                # Generate Ukrainian docs
    $0 lib-embed --enrich                 # Generate with LLM enrichment
    $0 lib-embed --lang zh --enrich       # Chinese docs with enrichment

OUTPUT:
    Documentation is saved to:
    .adi/docs/<crate-name>/<lang>/api.md
EOF
    exit 0
}

# Find crate directory by name
find_crate_dir() {
    local name="$1"
    local found=""
    
    while IFS= read -r f; do
        crate_name=$(grep -m1 '^name = ' "$f" 2>/dev/null | sed 's/name = "//;s/"//')
        if [[ "$crate_name" == "$name" ]]; then
            found=$(dirname "$f")
            break
        fi
    done < <(find "$PROJECT_ROOT/crates" -name 'Cargo.toml' -type f 2>/dev/null)
    
    echo "$found"
}

# Extract public API using cargo-public-api
extract_public_api() {
    local crate_dir="$1"
    local crate_name="$2"
    local output_file="$3"
    
    info "Extracting public API for $crate_name..."
    
    # Run cargo-public-api
    cd "$crate_dir"
    
    # Use cargo public-api to get the API
    if ! cargo public-api --simplified > "$output_file" 2>/dev/null; then
        # Fallback: try without simplified flag
        if ! cargo public-api > "$output_file" 2>/dev/null; then
            warn "cargo-public-api failed, using cargo doc extraction"
            # Fallback: extract from rustdoc JSON
            extract_from_rustdoc "$crate_dir" "$crate_name" "$output_file"
        fi
    fi
    
    cd "$PROJECT_ROOT"
}

# Fallback: extract API from rustdoc
extract_from_rustdoc() {
    local crate_dir="$1"
    local crate_name="$2"
    local output_file="$3"
    
    cd "$crate_dir"
    
    # Generate rustdoc JSON
    RUSTDOCFLAGS="-Z unstable-options --output-format json" cargo +nightly doc --no-deps 2>/dev/null || true
    
    # Find the JSON file
    local json_file=$(find "$crate_dir/target/doc" -name "*.json" -type f 2>/dev/null | head -1)
    
    if [[ -f "$json_file" ]]; then
        # Extract basic info from JSON
        cat > "$output_file" <<EOF
# Public API (extracted from rustdoc)

Note: This is a simplified extraction. For full API details, run \`cargo doc --open\`.

$(jq -r '.index | to_entries[] | select(.value.visibility == "public") | "- \(.value.kind): \(.value.name)"' "$json_file" 2>/dev/null || echo "Unable to parse rustdoc JSON")
EOF
    else
        # Last resort: just list public items from source
        cat > "$output_file" <<EOF
# Public API (source extraction)

Note: Automatic API extraction failed. Manual review recommended.

## Public Items
$(grep -rh "^pub " "$crate_dir/src" 2>/dev/null | head -50 || echo "No public items found")
EOF
    fi
    
    cd "$PROJECT_ROOT"
}

# Generate markdown documentation from API
generate_markdown() {
    local api_file="$1"
    local output_file="$2"
    local crate_name="$3"
    local lang="$4"
    
    local lang_name
    lang_name=$(get_lang_name "$lang")
    lang_name="${lang_name:-English}"
    
    cat > "$output_file" <<EOF
<!-- 
  Auto-generated documentation for $crate_name
  Language: $lang_name
  Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
  
  This file was generated by: adi workflow autodoc
  To regenerate: adi workflow autodoc --force
-->

# $crate_name

## Public API

\`\`\`rust
$(cat "$api_file")
\`\`\`

## Overview

<!-- TODO: Add overview description -->

## Usage

<!-- TODO: Add usage examples -->

## API Reference

<!-- TODO: Add detailed API reference -->

EOF
}

# Enrich documentation with LLM
enrich_with_llm() {
    local doc_file="$1"
    local crate_name="$2"
    local lang="$3"
    local crate_dir="$4"
    
    local lang_name
    lang_name=$(get_lang_name "$lang")
    lang_name="${lang_name:-English}"
    
    info "Enriching documentation with LLM (language: $lang_name)..."
    
    # Check if claude CLI is available
    if ! check_command claude; then
        warn "claude CLI not found, skipping LLM enrichment"
        return 0
    fi
    
    # Read the current doc content
    local current_doc=$(cat "$doc_file")
    
    # Read some source files for context (limit to avoid token overflow)
    local source_context=""
    if [[ -d "$crate_dir/src" ]]; then
        source_context=$(find "$crate_dir/src" -name "*.rs" -type f -exec head -100 {} \; 2>/dev/null | head -500)
    fi
    
    # Read Cargo.toml for metadata
    local cargo_toml=""
    if [[ -f "$crate_dir/Cargo.toml" ]]; then
        cargo_toml=$(cat "$crate_dir/Cargo.toml")
    fi
    
    # Read README if exists
    local readme=""
    if [[ -f "$crate_dir/README.md" ]]; then
        readme=$(cat "$crate_dir/README.md")
    fi
    
    # Create the prompt
    local prompt=$(cat <<PROMPT
You are a technical documentation writer. Your task is to enrich the API documentation for the Rust crate "$crate_name".

IMPORTANT: Write all documentation in $lang_name language.

Current documentation (with API extracted):
---
$current_doc
---

Cargo.toml:
---
$cargo_toml
---

README (if available):
---
$readme
---

Source code snippets for context:
---
$source_context
---

Please generate enriched documentation that includes:

1. **Overview** - A clear, concise description of what this crate does and its main purpose
2. **Installation** - How to add this crate as a dependency
3. **Quick Start** - A minimal working example showing basic usage
4. **API Reference** - For each public item in the API:
   - Brief description of what it does
   - Parameters and return types explained
   - Example usage code where helpful
5. **Common Patterns** - Show 2-3 common usage patterns with code examples
6. **Error Handling** - Document any error types and how to handle them
7. **See Also** - Related crates or documentation links

Format the output as valid Markdown. Use proper Rust code blocks with syntax highlighting.
Keep the header comment from the original document.

Output ONLY the enriched documentation, no explanations or meta-commentary.
PROMPT
)
    
    # Create temp file for prompt
    local prompt_file="$TEMP_DIR/prompt_${crate_name}_${lang}.txt"
    echo "$prompt" > "$prompt_file"
    
    # Call claude CLI with the prompt
    local enriched_doc
    if enriched_doc=$(claude -p "$(cat "$prompt_file")" --model claude-sonnet-4-20250514 2>/dev/null); then
        # Write enriched doc
        echo "$enriched_doc" > "$doc_file"
        success "Documentation enriched with LLM"
    else
        warn "LLM enrichment failed, keeping original documentation"
    fi
    
    # Cleanup
    rm -f "$prompt_file"
}

main() {
    local crate_name=""
    local lang="en"
    local enrich=false
    local force=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                ;;
            --lang)
                lang="$2"
                shift 2
                ;;
            --enrich)
                enrich=true
                shift
                ;;
            --force)
                force=true
                shift
                ;;
            *)
                if [ -z "$crate_name" ]; then
                    crate_name="$1"
                else
                    error "Unknown argument: $1"
                fi
                shift
                ;;
        esac
    done

    if [ -z "$crate_name" ]; then
        error "Crate name required. Run with --help for usage."
    fi

    # Validate language
    local lang_name
    lang_name=$(get_lang_name "$lang")
    if [[ -z "$lang_name" ]]; then
        error "Unsupported language: $lang. Supported: en, uk, ru, zh, ja, ko, es, de, fr"
    fi

    # Find crate directory
    local crate_dir
    crate_dir=$(find_crate_dir "$crate_name")

    if [ -z "$crate_dir" ]; then
        error "Crate not found: $crate_name"
    fi

    require_dir "$crate_dir" "Crate directory not found: $crate_dir"

    # Check prerequisites
    ensure_command "cargo"
    
    if ! check_command cargo-public-api; then
        warn "cargo-public-api not found, installing..."
        cargo install cargo-public-api || error "Failed to install cargo-public-api"
    fi

    # Setup directories
    local output_dir="$DOCS_DIR/$crate_name/$lang"
    local output_file="$output_dir/api.md"
    
    # Check if docs already exist
    if [[ -f "$output_file" ]] && [[ "$force" != "true" ]]; then
        error "Documentation already exists: $output_file\nUse --force to overwrite."
    fi

    ensure_dir "$output_dir"
    ensure_dir "$TEMP_DIR"

    echo ""
    info "Generating documentation for: $crate_name"
    info "Language: $lang_name"
    info "Output: $output_file"
    echo ""

    # Step 1: Extract public API
    local api_file="$TEMP_DIR/${crate_name}_api.txt"
    extract_public_api "$crate_dir" "$crate_name" "$api_file"

    if [[ ! -s "$api_file" ]]; then
        warn "No public API extracted (crate may be a binary or have no public items)"
        echo "# $crate_name - No Public API" > "$api_file"
    fi

    success "Public API extracted"

    # Step 2: Generate base markdown
    generate_markdown "$api_file" "$output_file" "$crate_name" "$lang"
    success "Base documentation generated"

    # Step 3: Enrich with LLM (if requested)
    if [ "$enrich" = true ]; then
        enrich_with_llm "$output_file" "$crate_name" "$lang" "$crate_dir"
    fi

    # Cleanup
    rm -f "$api_file"

    echo ""
    success "Documentation generated: $output_file"
    echo ""
    
    # Show preview
    info "Preview (first 30 lines):"
    echo "---"
    head -30 "$output_file"
    echo "---"
    echo ""
    info "Full documentation: $output_file"
}

main "$@"
