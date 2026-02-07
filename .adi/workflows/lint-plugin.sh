#!/usr/bin/env bash
# Plugin Linter - validates plugin structure before publishing
# Usage: adi workflow lint-plugin <plugin-name|plugin-path>
set -uo pipefail

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    WORKFLOWS_DIR="$_SCRIPT_DIR"
fi

# Colors (override prelude for custom linter output)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

# Counters
ERRORS=0
WARNINGS=0

# Override logging for linter-specific output
error() { echo -e "${RED}ERROR:${NC} $*"; ((ERRORS++)); }
warn() { echo -e "${YELLOW}WARN:${NC} $*"; ((WARNINGS++)); }
info() { echo -e "${BLUE}INFO:${NC} $*"; }
success() { echo -e "${GREEN}OK:${NC} $*"; }
section() { echo -e "\n${BOLD}${CYAN}==> $*${NC}"; }

usage() {
    cat <<EOF
Usage: $0 [OPTIONS] <plugin>

Lint a plugin before publishing. Validates manifest, binary, and structure.

ARGUMENTS:
    plugin              Plugin name (e.g., cocoon, tasks) or path to plugin directory

OPTIONS:
    --fix               Attempt to fix common issues automatically
    --strict            Treat warnings as errors
    -h, --help          Show this help

EXAMPLES:
    $0 cocoon                    # Lint cocoon plugin
    $0 crates/cocoon             # Lint by path
    $0 --fix cocoon              # Lint and fix issues
    $0 --strict cocoon           # Fail on warnings
    $0 --all                     # Lint all plugins in crates/

CHECKS PERFORMED:
    - plugin.toml exists and is valid TOML
    - Required fields: id, name, version, type
    - Binary section matches actual library file
    - Version format (semver)
    - Service declarations in [[provides]]
    - CLI configuration in [cli] (if present):
      - command: required, lowercase alphanumeric with hyphens
      - description: required, min 10 chars
      - aliases: optional, must be array format
    - Cross-check: [cli] should have matching .cli service
    - Platform compatibility
    - Min host version compatibility
EOF
    exit 0
}

# Parse TOML value (simple parser for basic values)
get_toml_value() {
    local file="$1"
    local key="$2"
    local line value
    line=$(grep -E "^${key}\s*=" "$file" 2>/dev/null | head -1 || true)
    if [[ -n "$line" ]]; then
        # Extract value after = remove quotes and trim whitespace
        value=$(echo "$line" | sed 's/^[^=]*=//' | xargs | sed 's/^"\(.*\)"$/\1/' | sed "s/^'\(.*\)'$/\1/")
        echo "$value"
    fi
}

# Check if TOML section exists
has_toml_section() {
    local file="$1"
    local section="$2"
    grep -qE "^\[${section}\]" "$file" 2>/dev/null || return 1
}

# Check if TOML array section exists
has_toml_array_section() {
    local file="$1"
    local section="$2"
    grep -qE "^\[\[${section}\]\]" "$file" 2>/dev/null || return 1
}

# Get binary name from manifest or default
get_binary_name() {
    local manifest="$1"
    local name=""

    if has_toml_section "$manifest" "binary"; then
        # Extract name from [binary] section using sed
        local name_line
        name_line=$(sed -n '/^\[binary\]/,/^\[/p' "$manifest" | grep "^name" || true)
        if [[ -n "$name_line" ]]; then
            name=$(echo "$name_line" | sed 's/^[^=]*=//' | xargs | sed 's/^"\(.*\)"$/\1/')
        fi
    fi

    if [[ -z "$name" ]]; then
        name="plugin"
    fi

    echo "$name"
}

# Validate semver format
is_valid_semver() {
    local version="$1"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$ ]]
}

# Get library extension for current platform
get_lib_extension() {
    case "$(uname -s)" in
        Darwin) echo "dylib" ;;
        Linux) echo "so" ;;
        MINGW*|CYGWIN*|MSYS*) echo "dll" ;;
        *) echo "so" ;;
    esac
}

# Main lint function
lint_plugin() {
    local plugin_dir="$1"
    local fix_mode="${2:-false}"

    section "Linting plugin: $plugin_dir"

    # Check for Cargo.toml with [package.metadata.plugin]
    local cargo_toml="$plugin_dir/Cargo.toml"
    if [[ ! -f "$cargo_toml" ]]; then
        error "Cargo.toml not found at $cargo_toml"
        return 1
    fi

    if ! grep -q 'package\.metadata\.plugin' "$cargo_toml" 2>/dev/null; then
        # Backward compat: check for legacy plugin.toml
        local manifest="$plugin_dir/plugin.toml"
        if [[ -f "$manifest" ]]; then
            warn "Found legacy plugin.toml - migrate to [package.metadata.plugin] in Cargo.toml"
        else
            error "No [package.metadata.plugin] section in Cargo.toml"
        fi
        return 1
    fi
    success "Cargo.toml has [package.metadata.plugin]"

    # Generate plugin.toml from metadata for validation
    local manifest_gen="$PROJECT_ROOT/target/release/manifest-gen"
    if [[ ! -f "$manifest_gen" ]]; then
        manifest_gen="$PROJECT_ROOT/target/debug/manifest-gen"
    fi
    local manifest
    manifest=$(mktemp "${TMPDIR:-/tmp}/plugin-lint.XXXXXX.toml")
    if [[ -f "$manifest_gen" ]]; then
        "$manifest_gen" --cargo-toml "$cargo_toml" --output "$manifest" 2>/dev/null || {
            error "Failed to generate manifest from Cargo.toml metadata"
            return 1
        }
        success "Manifest generated successfully from Cargo.toml"
    else
        warn "manifest-gen not found - skipping manifest generation validation"
        warn "Build with: cargo build -p lib-plugin-manifest --features generate"
        return 0
    fi

    # Validate TOML syntax
    section "Validating TOML syntax"
    if command -v taplo &>/dev/null; then
        if taplo check "$manifest" 2>/dev/null; then
            success "TOML syntax valid"
        else
            error "Invalid TOML syntax"
        fi
    else
        info "TOML validator (taplo) not found. Skipping syntax check."
    fi

    # Check [plugin] section
    section "Checking [plugin] section"
    if ! has_toml_section "$manifest" "plugin"; then
        error "Missing [plugin] section"
        return 1
    fi
    success "[plugin] section exists"

    # Required fields in [plugin]
    local plugin_id plugin_name plugin_version plugin_type
    plugin_id=$(get_toml_value "$manifest" "id")
    plugin_name=$(get_toml_value "$manifest" "name")
    plugin_version=$(get_toml_value "$manifest" "version")
    plugin_type=$(get_toml_value "$manifest" "type")

    # Check id
    if [[ -z "$plugin_id" ]]; then
        error "Missing required field: id"
    elif [[ ! "$plugin_id" =~ ^[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$ ]]; then
        warn "Plugin ID '$plugin_id' should follow format: vendor.plugin-name (e.g., adi.cocoon)"
    else
        success "id: $plugin_id"
    fi

    # Check name
    if [[ -z "$plugin_name" ]]; then
        error "Missing required field: name"
    else
        success "name: $plugin_name"
    fi

    # Check version
    if [[ -z "$plugin_version" ]]; then
        error "Missing required field: version"
    elif ! is_valid_semver "$plugin_version"; then
        error "Invalid version format: '$plugin_version' (expected semver: X.Y.Z)"
    else
        success "version: $plugin_version"
    fi

    # Check type
    if [[ -z "$plugin_type" ]]; then
        error "Missing required field: type"
        if [[ "$fix_mode" == "true" ]]; then
            # Detect type from plugin structure
            local detected_type="core"
            if has_toml_section "$manifest" "language"; then
                detected_type="lang"
            elif [[ "$plugin_id" == *".lang."* ]]; then
                detected_type="lang"
            fi
            info "FIX: Adding type = \"$detected_type\" to manifest"
            sed -i '' '/^\[plugin\]/a\
type = "'"$detected_type"'"
' "$manifest"
            success "Added type = \"$detected_type\""
            plugin_type="$detected_type"
        fi
    elif [[ ! "$plugin_type" =~ ^(core|extension|lang|theme)$ ]]; then
        warn "Unknown plugin type: '$plugin_type' (expected: core, extension, lang, theme)"
    else
        success "type: $plugin_type"
    fi

    # Check min_host_version
    local min_host_version
    min_host_version=$(get_toml_value "$manifest" "min_host_version")
    if [[ -n "$min_host_version" ]]; then
        if ! is_valid_semver "$min_host_version"; then
            error "Invalid min_host_version format: '$min_host_version'"
        else
            success "min_host_version: $min_host_version"
        fi
    else
        warn "No min_host_version specified (recommended for compatibility)"
    fi

    # Check [binary] section and actual binary
    section "Checking binary configuration"
    local binary_name expected_lib actual_lib lib_ext
    binary_name=$(get_binary_name "$manifest")
    lib_ext=$(get_lib_extension)

    # Expected library files (try multiple variants)
    local found_lib=""
    local variants=(
        "${binary_name}.${lib_ext}"
        "lib${binary_name}.${lib_ext}"
    )

    for variant in "${variants[@]}"; do
        if [[ -f "$plugin_dir/$variant" ]]; then
            found_lib="$variant"
            break
        fi
    done

    # Also check in target/release for source builds
    if [[ -z "$found_lib" ]]; then
        local target_dir="$PROJECT_ROOT/target/release"
        for variant in "${variants[@]}"; do
            if [[ -f "$target_dir/$variant" ]]; then
                info "Binary found in target/release: $variant"
                found_lib="$variant"
                break
            fi
        done
    fi

    if [[ -z "$found_lib" ]]; then
        # Check if this is a source directory (has Cargo.toml) - binary will be built during release
        if [[ -f "$plugin_dir/Cargo.toml" ]]; then
            info "Binary not built yet (will be built during release): $binary_name"
            info "Expected: ${variants[*]}"
        else
            error "No library file found for binary name '$binary_name'"
            error "Expected one of: ${variants[*]}"
        fi

        # Check what libraries actually exist
        local existing_libs
        existing_libs=$(find "$plugin_dir" -maxdepth 1 -name "*.${lib_ext}" 2>/dev/null | head -5)
        if [[ -n "$existing_libs" ]]; then
            info "Found libraries in plugin dir:"
            echo "$existing_libs" | while read -r lib; do
                echo "  - $(basename "$lib")"
            done

            # Suggest fix
            local first_lib
            first_lib=$(echo "$existing_libs" | head -1)
            local suggested_name
            suggested_name=$(basename "$first_lib" ".$lib_ext" | sed 's/^lib//')

            if [[ "$fix_mode" == "true" ]]; then
                info "FIX: Adding [binary] section with name = \"$suggested_name\""
                if has_toml_section "$manifest" "binary"; then
                    sed -i '' "s/^name = .*/name = \"$suggested_name\"/" "$manifest"
                else
                    echo -e "\n[binary]\nname = \"$suggested_name\"" >> "$manifest"
                fi
                success "Added [binary] name = \"$suggested_name\""
            else
                warn "Suggested fix: Add '[binary]' section with name = \"$suggested_name\""
            fi
        fi
    else
        success "Binary found: $found_lib (name: $binary_name)"
    fi

    # Check [[provides]] section
    section "Checking service declarations"

    # Check for wrong [provides] (single table) instead of [[provides]] (array)
    if has_toml_section "$manifest" "provides" && ! has_toml_array_section "$manifest" "provides"; then
        error "Wrong provides format: [provides] should be [[provides]] (array of tables)"
        warn "Lang plugins use different schema - may need migration"
    elif ! has_toml_array_section "$manifest" "provides"; then
        warn "No [[provides]] section - plugin won't register any services"
    else
        # Count provides sections
        local provides_count
        provides_count=$(grep -c '^\[\[provides\]\]' "$manifest" || echo 0)
        success "Found $provides_count service declaration(s)"

        # Check each provides has required fields
        local provides_ids
        provides_ids=$(sed -n '/^\[\[provides\]\]/,/^\[/p' "$manifest" | grep "^id" | sed 's/^[^=]*=//' | xargs | sed 's/^"\(.*\)"$/\1/' || true)

        if [[ -z "$provides_ids" ]]; then
            error "[[provides]] section missing 'id' field"
        else
            echo "$provides_ids" | tr ' ' '\n' | while read -r svc_id; do
                if [[ -n "$svc_id" ]]; then
                    success "  Service: $svc_id"
                fi
            done
        fi
    fi

    # Check [cli] section (optional - for plugins that provide CLI commands)
    section "Checking CLI configuration"

    if has_toml_section "$manifest" "cli"; then
        success "[cli] section present"

        # Extract cli values from [cli] section
        local cli_command cli_description

        cli_command=$(sed -n '/^\[cli\]/,/^\[/p' "$manifest" | grep "^command" | sed 's/^[^=]*=//' | xargs | sed 's/^"\(.*\)"$/\1/' || true)
        cli_description=$(sed -n '/^\[cli\]/,/^\[/p' "$manifest" | grep "^description" | sed 's/^[^=]*=//' | xargs | sed 's/^"\(.*\)"$/\1/' || true)

        # Check command (required if [cli] exists)
        if [[ -z "$cli_command" ]]; then
            error "[cli] section missing required field: command"
        elif [[ ! "$cli_command" =~ ^[a-z][a-z0-9-]*$ ]]; then
            error "Invalid CLI command name: '$cli_command' (must be lowercase alphanumeric with hyphens)"
        else
            success "cli.command: $cli_command"
        fi

        # Check description (required if [cli] exists)
        if [[ -z "$cli_description" ]]; then
            error "[cli] section missing required field: description"
        elif [[ ${#cli_description} -lt 10 ]]; then
            warn "CLI description is very short (${#cli_description} chars)"
        else
            success "cli.description: ${cli_description:0:50}..."
        fi

        # Check aliases format if present
        local cli_aliases_line
        cli_aliases_line=$(sed -n '/^\[cli\]/,/^\[/p' "$manifest" | grep "^aliases" || true)
        if [[ -n "$cli_aliases_line" ]]; then
            # Basic check - should be array format
            if [[ "$cli_aliases_line" =~ \[.*\] ]]; then
                success "cli.aliases: present"
            else
                error "cli.aliases should be an array (e.g., aliases = [\"t\"])"
            fi
        fi

        # Cross-check: if [cli] exists, should have .cli service in [[provides]]
        if has_toml_array_section "$manifest" "provides"; then
            local has_cli_service
            has_cli_service=$(grep -E 'id.*=.*".*\.cli"' "$manifest" || true)
            if [[ -z "$has_cli_service" ]]; then
                warn "[cli] section exists but no .cli service in [[provides]]"
                warn "Add: [[provides]] with id = \"${plugin_id}.cli\""
            fi
        fi
    else
        # No [cli] section - check if plugin provides .cli service (inconsistency)
        if has_toml_array_section "$manifest" "provides"; then
            local has_cli_service
            has_cli_service=$(grep -E 'id.*=.*".*\.cli"' "$manifest" || true)
            if [[ -n "$has_cli_service" ]]; then
                warn "Plugin provides .cli service but has no [cli] section"
                warn "Add [cli] section to register top-level command"
            else
                info "No [cli] section (plugin has no CLI command)"
            fi
        else
            info "No [cli] section (plugin has no CLI command)"
        fi
    fi

    # Check [tags] section
    section "Checking metadata"
    if has_toml_section "$manifest" "tags"; then
        success "[tags] section present"
    else
        warn "No [tags] section - consider adding categories for discoverability"
    fi

    # Check description
    local description
    description=$(get_toml_value "$manifest" "description")
    if [[ -z "$description" ]]; then
        warn "No description provided"
    elif [[ ${#description} -lt 20 ]]; then
        warn "Description is very short (${#description} chars) - consider a more detailed description"
    else
        success "description: ${description:0:50}..."
    fi

    # Check author
    local author
    author=$(get_toml_value "$manifest" "author")
    if [[ -z "$author" ]]; then
        warn "No author specified"
    else
        success "author: $author"
    fi

    # Version consistency - single source of truth in Cargo.toml
    section "Checking version source"
    success "Version from Cargo.toml: $plugin_version (single source of truth)"

    return 0
}

# Parse arguments
FIX_MODE=false
STRICT_MODE=false
LINT_ALL=false
PLUGIN=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            ;;
--fix)
            FIX_MODE=true
            shift
            ;;
        --strict)
            STRICT_MODE=true
            shift
            ;;
        --all)
            LINT_ALL=true
            shift
            ;;
        *)
            PLUGIN="$1"
            shift
            ;;
    esac
done

# Handle --all flag
if [[ "$LINT_ALL" == "true" ]]; then
    echo -e "${BOLD}${CYAN}Linting all plugins...${NC}"
    echo ""

    TOTAL_ERRORS=0
    TOTAL_WARNINGS=0
    PLUGINS_CHECKED=0
    PLUGINS_FAILED=0

    for cargo_file in $(find "$PROJECT_ROOT/crates" -name "Cargo.toml" -type f 2>/dev/null); do
        if ! grep -q 'package\.metadata\.plugin' "$cargo_file" 2>/dev/null; then
            continue
        fi
        plugin_dir=$(dirname "$cargo_file")
        plugin_name=$(basename "$plugin_dir")

        ERRORS=0
        WARNINGS=0

        if lint_plugin "$plugin_dir" "$FIX_MODE" 2>&1; then
            ((PLUGINS_CHECKED++))
        else
            ((PLUGINS_FAILED++))
        fi

        TOTAL_ERRORS=$((TOTAL_ERRORS + ERRORS))
        TOTAL_WARNINGS=$((TOTAL_WARNINGS + WARNINGS))
        ((PLUGINS_CHECKED++))
        echo ""
    done

    echo ""
    section "Overall Summary"
    echo "Plugins checked: $PLUGINS_CHECKED"
    echo "Plugins with errors: $PLUGINS_FAILED"
    echo "Total errors: $TOTAL_ERRORS"
    echo "Total warnings: $TOTAL_WARNINGS"

    if [[ $TOTAL_ERRORS -gt 0 ]]; then
        exit 1
    elif [[ $TOTAL_WARNINGS -gt 0 && "$STRICT_MODE" == "true" ]]; then
        exit 1
    fi
    exit 0
fi

if [[ -z "$PLUGIN" ]]; then
    error "No plugin specified"
    echo ""
    usage
fi

# Resolve plugin path - supports plugin ID (e.g., "adi.workflow") or directory path
resolve_plugin_dir() {
    local plugin="$1"
    
    # If it's already a directory path
    if [[ -d "$plugin" ]]; then
        echo "$plugin"
        return
    fi
    
    # Try to find by plugin ID in Cargo.toml [package.metadata.plugin]
    local found_path=""
    local plugin_id=""
    while IFS= read -r f; do
        if ! grep -q 'package\.metadata\.plugin' "$f" 2>/dev/null; then
            continue
        fi
        plugin_id=$(grep -A1 '\[package\.metadata\.plugin\]' "$f" 2>/dev/null | grep '^id = ' | sed 's/id = "//;s/"//' | tr -d '\n')
        if [[ "$plugin_id" == "$plugin" ]]; then
            found_path=$(dirname "$f")
            break
        fi
    done < <(find "$PROJECT_ROOT/crates" -name 'Cargo.toml' -type f 2>/dev/null)
    
    if [[ -n "$found_path" ]]; then
        echo "$found_path"
        return
    fi
    
    # Fallback to directory-based resolution
    if [[ -d "$PROJECT_ROOT/crates/$plugin" ]]; then
        echo "$PROJECT_ROOT/crates/$plugin"
    fi
}

PLUGIN_DIR=$(resolve_plugin_dir "$PLUGIN")

if [[ -z "$PLUGIN_DIR" ]]; then
    error "Plugin not found: $PLUGIN"
    echo "Tried:"
    echo "  - Direct path: $PLUGIN"
    echo "  - Plugin ID lookup in plugin.toml files"
    echo "  - Directory: $PROJECT_ROOT/crates/$PLUGIN"
    exit 1
fi

# Run linter
lint_plugin "$PLUGIN_DIR" "$FIX_MODE"

# Summary
echo ""
section "Summary"
if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}${BOLD}$ERRORS error(s)${NC}, ${YELLOW}$WARNINGS warning(s)${NC}"
    exit 1
elif [[ $WARNINGS -gt 0 && "$STRICT_MODE" == "true" ]]; then
    echo -e "${YELLOW}${BOLD}$WARNINGS warning(s)${NC} (strict mode)"
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "${GREEN}${BOLD}Passed${NC} with ${YELLOW}$WARNINGS warning(s)${NC}"
    exit 0
else
    echo -e "${GREEN}${BOLD}All checks passed!${NC}"
    exit 0
fi
