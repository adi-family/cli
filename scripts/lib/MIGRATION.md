# Migration Guide: Refactoring Scripts with Libraries

This document shows how to refactor existing bash scripts to use the new library system.

## Before & After Comparison

### Example: install.sh

#### Before (323 lines, lots of duplication)

```bash
#!/bin/sh
set -e

# Colors (repeated in every script)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Logging functions (repeated in every script)
info() {
    printf "${CYAN}info${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}done${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${NC} %s\n" "$1"
}

error() {
    printf "${RED}error${NC} %s\n" "$1" >&2
    exit 1
}

# Platform detection (repeated in every script)
detect_os() {
    case "$(uname -s)" in
        Darwin) echo "darwin" ;;
        Linux) echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) error "Windows detected..." ;;
        *) error "Unsupported OS..." ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *) error "Unsupported arch..." ;;
    esac
}

get_target() {
    # ... more code
}

# Download function (repeated in every script)
download() {
    local url="$1"
    local output="$2"
    info "Downloading from $url"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found"
    fi
}

# Checksum verification (repeated in every script)
verify_checksum() {
    # ... 20 more lines
}

# Extract function (repeated in every script)
extract() {
    # ... 15 more lines
}

# GitHub API (repeated in every script)
fetch_latest_version() {
    # ... 10 more lines
}

# Path setup (repeated in every script)
setup_path() {
    # ... 30 more lines
}

# Main logic (finally!)
main() {
    # 50 lines of actual installation logic
}
```

#### After (96 lines, clean and focused)

```bash
#!/bin/sh
set -e

# Convert to bash for library support
if [ -z "$BASH_VERSION" ]; then
    exec bash "$0" "$@"
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load libraries (4 lines replaces 200+ lines of boilerplate)
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/github.sh"
source "$SCRIPT_DIR/lib/common.sh"

# Configuration
REPO="adi-family/adi-cli"
BINARY_NAME="adi"

# Main logic (clean and readable)
main() {
    echo ""
    printf "${BLUE}ADI CLI Installer${NC}\n"
    echo ""

    # Platform detection (1 line each, was 40+ lines)
    local os=$(detect_os)
    local arch=$(detect_arch)
    local target=$(get_target "$os" "$arch")

    [ "$os" = "windows" ] && error "Windows detected. Use: winget install adi-cli"

    info "Detected platform: $target"

    # Version handling (2 lines, was 15+ lines)
    local version="${ADI_VERSION:-$(fetch_latest_version "$REPO")}"
    version=$(ensure_v_prefix "$(normalize_version "$version")")

    info "Installing version: $version"

    # Setup (2 lines, was 10+ lines)
    local install_dir="${ADI_INSTALL_DIR:-$HOME/.local/bin}"
    ensure_dir "$install_dir"

    # Download and verify (5 lines, was 40+ lines)
    local temp_dir=$(create_temp_dir)
    local archive_name="adi-${version}-${target}.$(get_archive_extension "$os")"
    download "https://github.com/${REPO}/releases/download/${version}/${archive_name}" "$temp_dir/$archive_name"
    verify_checksum "$temp_dir/$archive_name" "$(fetch_checksum)"

    # Extract and install (3 lines, was 20+ lines)
    extract_archive "$temp_dir/$archive_name" "$temp_dir"
    mv "$temp_dir/$BINARY_NAME" "$install_dir/$BINARY_NAME"

    success "Installed!"
    setup_path "$install_dir"
}

main "$@"
```

## Migration Steps

### 1. Add Library Loading

Replace all duplicated code at the top with library imports:

```bash
# Before:
RED='\033[0;31m'
GREEN='\033[0;32m'
# ... 200 lines of functions

# After:
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/github.sh"
source "$SCRIPT_DIR/lib/common.sh"
```

### 2. Remove Duplicated Functions

Delete these functions from your script (now in libraries):

- ❌ Color variable definitions → ✅ `colors.sh`
- ❌ `info()`, `success()`, `warn()`, `error()` → ✅ `log.sh`
- ❌ `detect_os()`, `detect_arch()`, `get_target()` → ✅ `platform.sh`
- ❌ `download()`, `fetch()` → ✅ `download.sh`
- ❌ `fetch_latest_version()` → ✅ `github.sh`
- ❌ `verify_checksum()`, `extract()` → ✅ `common.sh`
- ❌ `setup_path()` → ✅ `common.sh`

### 3. Simplify Main Logic

Focus your script on business logic, not boilerplate:

```bash
# Before: Manual error checking everywhere
if [ ! -d "$install_dir" ]; then
    mkdir -p "$install_dir" || error "Failed to create directory"
fi

# After: Library handles errors
ensure_dir "$install_dir"
```

```bash
# Before: Complex temp directory setup
temp_dir=$(mktemp -d)
trap "rm -rf '$temp_dir'" EXIT

# After: One line
temp_dir=$(create_temp_dir)
```

## Benefits

### Lines of Code Reduction

| Script | Before | After | Reduction |
|--------|--------|-------|-----------|
| install.sh | 323 | 96 | 70% |
| install-cocoon.sh | 390 | ~120 | 69% |
| deploy.sh | 502 | ~180 | 64% |

### Maintainability

- **Single source of truth**: Fix a bug once in the library, all scripts benefit
- **Consistent behavior**: All scripts use the same error handling, logging format, etc.
- **Easier testing**: Test libraries independently
- **Better documentation**: Libraries are documented in one place

### Code Quality

- **Reduced duplication**: No more copy-paste of common functions
- **Better error handling**: Libraries handle edge cases consistently
- **Cleaner scripts**: Focus on business logic, not boilerplate
- **Type safety**: Libraries validate inputs

## Scripts to Migrate

Priority order:

1. ✅ **install.sh** - Example done (see `install-refactored.sh`)
2. ⏳ **install-cocoon.sh** - High similarity, easy win
3. ⏳ **release-adi-cli.sh** - Uses platform detection and GitHub
4. ⏳ **release-plugins.sh** - Similar to above
5. ⏳ **deploy.sh** - Uses colors and logging
6. ⏳ **dev.sh** - Uses colors, logging, and service management

## Testing Migration

After refactoring a script:

1. **Verify functionality:**
   ```bash
   # Test with default settings
   ./scripts/install-refactored.sh

   # Test with custom settings
   ADI_INSTALL_DIR=/tmp/test ./scripts/install-refactored.sh
   ```

2. **Compare outputs:**
   ```bash
   # Run both versions and compare
   ./scripts/install.sh > /tmp/old.log 2>&1
   ./scripts/install-refactored.sh > /tmp/new.log 2>&1
   diff /tmp/old.log /tmp/new.log
   ```

3. **Test edge cases:**
   - Missing dependencies (curl/wget)
   - Invalid versions
   - Network failures
   - Permission errors

## Common Patterns

### Pattern: Command Checking

```bash
# Before:
command -v curl >/dev/null 2>&1 || error "curl not found"

# After:
ensure_command "curl" "brew install curl"
```

### Pattern: Platform Detection

```bash
# Before:
os=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$os" = "darwin" ]; then
    # mac stuff
fi

# After:
os=$(detect_os)
[ "$os" = "darwin" ] && info "Running on macOS"
```

### Pattern: Version Normalization

```bash
# Before:
version="${1:-}"
version="${version#v}"
if [[ "$version" != v* ]]; then
    version="v${version}"
fi

# After:
version=$(ensure_v_prefix "$(normalize_version "$1")")
```

### Pattern: Error Handling

```bash
# Before:
if [ ! -f "$file" ]; then
    echo "Error: File not found" >&2
    exit 1
fi

# After:
[ ! -f "$file" ] && error "File not found"
```

## Rollout Plan

1. **Week 1: Core libraries** ✅
   - Create all 6 libraries
   - Write tests and documentation
   - Create example script

2. **Week 2: Low-risk migrations**
   - Refactor installer scripts (install.sh, install-cocoon.sh)
   - Test in CI/CD
   - Get user feedback

3. **Week 3: Complex scripts**
   - Refactor release scripts
   - Refactor dev.sh
   - Update deployment scripts

4. **Week 4: Cleanup**
   - Remove old scripts after verification
   - Update documentation
   - Add library tests to CI/CD

## Questions?

See `scripts/lib/README.md` for full documentation.
