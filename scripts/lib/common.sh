#!/bin/bash
# =============================================================================
# Common Utilities Library
# =============================================================================
# Provides common utilities for bash scripts
#
# Usage:
#   source scripts/lib/common.sh
#
# Functions:
#   Requirements:
#     require_value <value> [msg]        - Exit if value is empty
#     require_env <var_name>             - Exit if env var not set
#     require_file <file> [msg]          - Exit if file doesn't exist
#     require_dir <dir> [msg]            - Exit if directory doesn't exist
#     require_one_of <msg> <vals...>     - Exit if all values empty
#
#   Commands:
#     check_command <cmd>                - Check if command exists
#     ensure_command <cmd> [hint]        - Ensure command exists or exit
#
#   Checksums:
#     verify_checksum <file> <expected>  - Verify SHA256 checksum
#     generate_checksums <out> <files>   - Generate SHA256SUMS file
#
#   Archives:
#     extract_archive <archive> <dest>   - Extract tar.gz or zip archive
#     create_tarball <out> <dir> <files> - Create tar.gz archive
#
#   Docker:
#     docker_image_exists <image:tag>    - Check if image exists in registry
#     deploy_docker_image <reg> <name> <ver> [dockerfile] [context]
#                                            - Build and push Docker image
#
#   Other:
#     setup_path <install_dir>           - Add directory to PATH
#     generate_secret [length]           - Generate cryptographic secret
#     get_cargo_version [toml]           - Extract version from Cargo.toml
#     normalize_version <version>        - Remove 'v' prefix
#     ensure_v_prefix <version>          - Add 'v' prefix
#     ensure_dir <dir>                   - Create directory if missing
#     create_temp_dir                    - Create temp dir with cleanup trap
#     check_root                         - Exit if not root
#     check_not_root                     - Exit if root
# =============================================================================

# Prevent double-loading
if [[ -n "${COMMON_LIB_LOADED}" ]]; then
    return 0
fi
COMMON_LIB_LOADED=1

# Load logging library
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/log.sh
source "$SCRIPT_LIB_DIR/log.sh"

# -----------------------------------------------------------------------------
# Requirements Checking
# -----------------------------------------------------------------------------

# Require a non-empty value
# Usage: require_value <value> [error_message]
# Example: REGISTRY_URL=$(require_value "$ADI_REGISTRY_URL" "ADI_REGISTRY_URL not set")
require_value() {
    local value="$1"
    local message="${2:-Value is required}"

    if [ -z "$value" ]; then
        error "$message"
    fi
    echo "$value"
}

# Require environment variable to be set
# Usage: require_env <var_name>
# Example: require_env "DATABASE_URL"
require_env() {
    local var_name="$1"
    local value="${!var_name}"

    if [ -z "$value" ]; then
        error "Environment variable $var_name is not set"
    fi
    echo "$value"
}

# Require file to exist
# Usage: require_file <file> [error_message]
require_file() {
    local file="$1"
    local message="${2:-File not found: $file}"

    if [ ! -f "$file" ]; then
        error "$message"
    fi
}

# Require directory to exist
# Usage: require_dir <dir> [error_message]
require_dir() {
    local dir="$1"
    local message="${2:-Directory not found: $dir}"

    if [ ! -d "$dir" ]; then
        error "$message"
    fi
}

# Require one of multiple values to be set
# Usage: require_one_of <error_message> <var1> <var2> ...
# Example: require_one_of "Either GITHUB_TOKEN or CI_TOKEN must be set" "$GITHUB_TOKEN" "$CI_TOKEN"
require_one_of() {
    local message="$1"
    shift

    for value in "$@"; do
        if [ -n "$value" ]; then
            return 0
        fi
    done

    error "$message"
}

# -----------------------------------------------------------------------------
# Command Checking
# -----------------------------------------------------------------------------

# Check if command exists
# Usage: check_command <cmd>
# Returns: 0 if exists, 1 if not
check_command() {
    command -v "$1" >/dev/null 2>&1
}

# Ensure command exists or exit
# Usage: ensure_command <cmd> [install_hint]
ensure_command() {
    local cmd="$1"
    local hint="${2:-}"

    if ! check_command "$cmd"; then
        if [ -n "$hint" ]; then
            error "$cmd not found. Install: $hint"
        else
            error "$cmd not found"
        fi
    fi
}

# -----------------------------------------------------------------------------
# Checksum Verification
# -----------------------------------------------------------------------------

# Verify SHA256 checksum
# Usage: verify_checksum <file> <expected>
verify_checksum() {
    local file="$1"
    local expected="$2"

    if [ -z "$expected" ]; then
        warn "Skipping checksum verification (checksum not available)"
        return 0
    fi

    local actual=""
    if check_command sha256sum; then
        actual=$(sha256sum "$file" | cut -d' ' -f1)
    elif check_command shasum; then
        actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        warn "Skipping checksum verification (sha256sum/shasum not found)"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed!\nExpected: $expected\nActual: $actual"
    fi

    success "Checksum verified"
}

# Generate SHA256 checksums for files
# Usage: generate_checksums <output_file> <files...>
generate_checksums() {
    local output="$1"
    shift

    if check_command sha256sum; then
        sha256sum "$@" > "$output"
    elif check_command shasum; then
        shasum -a 256 "$@" > "$output"
    else
        error "sha256sum or shasum not found"
    fi
}

# -----------------------------------------------------------------------------
# Archive Extraction
# -----------------------------------------------------------------------------

# Extract archive (tar.gz or zip)
# Usage: extract_archive <archive> <dest>
extract_archive() {
    local archive="$1"
    local dest="$2"

    if [ -z "$archive" ] || [ -z "$dest" ]; then
        error "Usage: extract_archive <archive> <dest>"
    fi

    info "Extracting archive"

    case "$archive" in
        *.tar.gz|*.tgz)
            tar -xzf "$archive" -C "$dest"
            ;;
        *.tar.bz2|*.tbz2)
            tar -xjf "$archive" -C "$dest"
            ;;
        *.tar.xz|*.txz)
            tar -xJf "$archive" -C "$dest"
            ;;
        *.zip)
            if check_command unzip; then
                unzip -q "$archive" -d "$dest"
            else
                error "unzip not found, required to extract .zip archives"
            fi
            ;;
        *)
            error "Unknown archive format: $archive"
            ;;
    esac
}

# Create tar.gz archive
# Usage: create_tarball <output> <source_dir> <files...>
create_tarball() {
    local output="$1"
    local source_dir="$2"
    shift 2

    tar -czf "$output" -C "$source_dir" "$@"
}

# -----------------------------------------------------------------------------
# Path Management
# -----------------------------------------------------------------------------

# Add directory to PATH in shell RC file
# Usage: setup_path <install_dir>
setup_path() {
    local install_dir="$1"
    local shell_name=""
    local rc_file=""

    # Detect shell
    if [ -n "$SHELL" ]; then
        shell_name=$(basename "$SHELL")
    fi

    case "$shell_name" in
        zsh)
            rc_file="$HOME/.zshrc"
            ;;
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                rc_file="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                rc_file="$HOME/.bash_profile"
            fi
            ;;
        fish)
            rc_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            rc_file="$HOME/.profile"
            ;;
    esac

    # Check if already in PATH
    case ":$PATH:" in
        *":$install_dir:"*)
            return 0
            ;;
    esac

    echo ""
    warn "$install_dir is not in your PATH"
    echo ""
    echo "Add it by running:"
    echo ""

    case "$shell_name" in
        fish)
            printf "  ${CYAN}fish_add_path %s${NC}\n" "$install_dir"
            ;;
        *)
            printf "  ${CYAN}echo 'export PATH=\"%s:\$PATH\"' >> %s${NC}\n" "$install_dir" "$rc_file"
            ;;
    esac

    echo ""
    echo "Then restart your shell or run:"
    printf "  ${CYAN}source %s${NC}\n" "$rc_file"
}

# -----------------------------------------------------------------------------
# Cryptography
# -----------------------------------------------------------------------------

# Generate strong cryptographic secret
# Usage: generate_secret [length]
generate_secret() {
    local length="${1:-36}"

    if check_command openssl; then
        openssl rand -base64 "$length"
    elif [ -r /dev/urandom ]; then
        head -c "$length" /dev/urandom | base64 | tr -d '\n'
    else
        error "Cannot generate secret: openssl or /dev/urandom required"
    fi
}

# -----------------------------------------------------------------------------
# Version Management
# -----------------------------------------------------------------------------

# Extract version from Cargo.toml
# Usage: get_cargo_version [cargo_toml_path]
# Returns: version string without quotes
get_cargo_version() {
    local cargo_toml="${1:-Cargo.toml}"
    local version

    require_file "$cargo_toml"

    version=$(grep '^version = ' "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
    require_value "$version" "Could not extract version from $cargo_toml" >/dev/null

    echo "$version"
}

# Normalize version (remove 'v' prefix)
# Usage: normalize_version <version>
normalize_version() {
    local version="$1"
    echo "${version#v}"
}

# Add 'v' prefix to version if not present
# Usage: ensure_v_prefix <version>
ensure_v_prefix() {
    local version="$1"
    if [[ "$version" != v* ]]; then
        echo "v${version}"
    else
        echo "$version"
    fi
}

# -----------------------------------------------------------------------------
# Directory Management
# -----------------------------------------------------------------------------

# Create directory if it doesn't exist
# Usage: ensure_dir <dir>
ensure_dir() {
    local dir="$1"
    mkdir -p "$dir"
}

# Create temporary directory with cleanup trap
# Usage: create_temp_dir
# Returns: path to temp directory
create_temp_dir() {
    local temp_dir
    temp_dir=$(mktemp -d)
    trap "rm -rf '$temp_dir'" EXIT
    echo "$temp_dir"
}

# -----------------------------------------------------------------------------
# Docker Utilities
# -----------------------------------------------------------------------------

# Check if Docker image exists in registry
# Usage: docker_image_exists <image:tag>
# Returns: 0 if exists, 1 if not
docker_image_exists() {
    local image="$1"
    docker manifest inspect "$image" >/dev/null 2>&1
}

# Build and push Docker image with version and latest tags
# Usage: deploy_docker_image <registry> <image_name> <version> [dockerfile] [build_context]
# Example: deploy_docker_image "registry.example.com" "my-app" "1.2.3"
# Example: deploy_docker_image "registry.example.com" "my-app" "1.2.3" "Dockerfile" "."
deploy_docker_image() {
    local registry="$1"
    local image_name="$2"
    local version="$3"
    local dockerfile="${4:-Dockerfile}"
    local build_context="${5:-.}"

    require_value "$registry" "Registry is required"
    require_value "$image_name" "Image name is required"
    require_value "$version" "Version is required"
    ensure_command "docker" "Install Docker Desktop"

    local image_tag="$registry/$image_name:$version"
    local latest_tag="$registry/$image_name:latest"

    # Check if image already exists
    if docker_image_exists "$image_tag"; then
        success "Image already exists in registry: $image_tag"
        warn "Skipping build"
        return 0
    fi

    info "Image not found in registry, building..."

    # Build image with both tags
    info "Building Docker image..."
    docker build -f "$dockerfile" \
        -t "$image_tag" \
        -t "$latest_tag" \
        "$build_context"

    success "Build complete"

    # Push both tags
    echo ""
    info "Pushing to registry: $registry"
    docker push "$image_tag"
    docker push "$latest_tag"

    success "Push complete"

    echo ""
    echo -e "${BOLD}Image deployed:${NC}"
    echo -e "  ${CYAN}$image_tag${NC}"
    echo -e "  ${CYAN}$latest_tag${NC}"
}

# -----------------------------------------------------------------------------
# Root/Sudo Checking
# -----------------------------------------------------------------------------

# Check if running as root
# Usage: check_root
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        error "This script must be run as root. Try: sudo $0"
    fi
}

# Check if NOT running as root
# Usage: check_not_root
check_not_root() {
    if [ "$(id -u)" -eq 0 ]; then
        error "This script should not be run as root"
    fi
}
