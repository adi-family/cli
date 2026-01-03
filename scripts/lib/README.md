# Bash Script Libraries

Reusable bash libraries for ADI project scripts. All libraries include double-loading guards to prevent re-execution.

## Quick Start

```bash
#!/bin/bash
set -e

# Load libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/log.sh"      # Includes colors.sh
source "$SCRIPT_DIR/lib/platform.sh" # Includes log.sh, colors.sh
source "$SCRIPT_DIR/lib/download.sh" # Includes log.sh, colors.sh
source "$SCRIPT_DIR/lib/github.sh"   # Includes download.sh, log.sh, colors.sh
source "$SCRIPT_DIR/lib/common.sh"   # Includes log.sh, colors.sh

# Use library functions
info "Detecting platform..."
OS=$(detect_os)
ARCH=$(detect_arch)
TARGET=$(get_target "$OS" "$ARCH")

success "Platform: $TARGET"
```

## Libraries

### colors.sh
Color codes and TTY detection.

**Functions:**
- `has_tty` - Check if running with a TTY
- `in_multiplexer` - Check if running in tmux/screen
- `supports_color` - Check if terminal supports color
- `setup_colors` - Initialize color codes (auto-called on load)

**Exports:**
- `RED`, `GREEN`, `YELLOW`, `BLUE`, `CYAN`, `BOLD`, `DIM`, `NC`

**Example:**
```bash
source scripts/lib/colors.sh
echo -e "${GREEN}Success!${NC}"
```

### log.sh
Logging functions with color support. Auto-loads `colors.sh`.

**Functions:**
- `log <message>` - Info message (blue prefix)
- `info <message>` - Info message (cyan prefix)
- `success <message>` - Success message (green prefix)
- `warn <message>` - Warning message (yellow prefix)
- `error <message>` - Error message (red prefix), exits with code 1

**Example:**
```bash
source scripts/lib/log.sh
info "Starting process..."
success "Process completed"
error "Something went wrong"  # exits script
```

### platform.sh
Platform detection. Auto-loads `log.sh` and `colors.sh`.

**Functions:**
- `detect_os` - Returns: `darwin`, `linux`, `windows`
- `detect_arch` - Returns: `x86_64`, `aarch64`
- `get_target <os> <arch>` - Returns Rust target triple
- `get_target_musl <os> <arch>` - Returns musl target triple
- `get_platform [os] [arch]` - Returns: `darwin-aarch64`, etc.
- `get_lib_extension <platform>` - Returns: `dylib`, `so`, `dll`
- `get_archive_extension [os]` - Returns: `tar.gz` or `zip`

**Example:**
```bash
source scripts/lib/platform.sh
OS=$(detect_os)
ARCH=$(detect_arch)
TARGET=$(get_target "$OS" "$ARCH")
info "Building for $TARGET"
```

### download.sh
Download utilities. Auto-loads `log.sh` and `colors.sh`.

**Functions:**
- `has_curl` - Check if curl is available
- `has_wget` - Check if wget is available
- `check_downloader` - Ensure curl or wget is available
- `download <url> <output>` - Download file silently
- `download_with_progress <url> <output>` - Download with progress bar
- `fetch <url>` - Fetch content to stdout

**Example:**
```bash
source scripts/lib/download.sh
download "https://example.com/file.tar.gz" "/tmp/file.tar.gz"
fetch "https://api.github.com/repos/owner/repo/releases/latest"
```

### github.sh
GitHub API helpers. Auto-loads `download.sh`, `log.sh`, and `colors.sh`.

**Functions:**
- `github_api_call <endpoint>` - Call GitHub API endpoint
- `fetch_latest_version <repo>` - Get latest release tag
- `fetch_release_info <repo> <tag>` - Get release information
- `release_exists <repo> <tag>` - Check if release exists
- `download_github_asset <repo> <tag> <asset> <output>` - Download asset

**Example:**
```bash
source scripts/lib/github.sh
VERSION=$(fetch_latest_version "adi-family/adi-cli")
info "Latest version: $VERSION"

download_github_asset "adi-family/adi-cli" "$VERSION" "adi-${VERSION}-darwin-aarch64.tar.gz" "/tmp/adi.tar.gz"
```

### common.sh
Common utilities. Auto-loads `log.sh` and `colors.sh`.

**Functions:**

**Requirements Checking:**
- `require_value <value> [msg]` - Exit if value is empty
- `require_env <var_name>` - Exit if env var not set
- `require_file <file> [msg]` - Exit if file doesn't exist
- `require_dir <dir> [msg]` - Exit if directory doesn't exist
- `require_one_of <msg> <val1> <val2> ...` - Exit if all values empty

**Command Checking:**
- `check_command <cmd>` - Returns 0 if exists
- `ensure_command <cmd> [hint]` - Exit if command missing

**Checksums:**
- `verify_checksum <file> <expected>` - Verify SHA256
- `generate_checksums <output> <files...>` - Create SHA256SUMS

**Archives:**
- `extract_archive <archive> <dest>` - Extract tar.gz/zip
- `create_tarball <output> <dir> <files...>` - Create tar.gz

**Path Management:**
- `setup_path <dir>` - Add to PATH in shell RC

**Cryptography:**
- `generate_secret [length]` - Generate strong secret

**Version:**
- `get_cargo_version [toml]` - Extract version from Cargo.toml
- `normalize_version <version>` - Remove 'v' prefix
- `ensure_v_prefix <version>` - Add 'v' prefix

**Directory:**
- `ensure_dir <dir>` - Create if missing
- `create_temp_dir` - Create temp dir with cleanup trap

**Docker:**
- `docker_image_exists <image:tag>` - Check if image exists in registry
- `deploy_docker_image <registry> <name> <version> [dockerfile] [context]` - Build and push image with version and latest tags

**Root Checking:**
- `check_root` - Exit if not root
- `check_not_root` - Exit if root

**Example:**
```bash
source scripts/lib/common.sh

# Require values to be set
REGISTRY_URL=$(require_value "$ADI_REGISTRY_URL" "ADI_REGISTRY_URL not set")
DATABASE_URL=$(require_env "DATABASE_URL")

# Require files/directories
require_file ".env.local"
require_dir "/tmp/build"

# Require at least one value
require_one_of "Either GITHUB_TOKEN or CI_TOKEN must be set" "$GITHUB_TOKEN" "$CI_TOKEN"

# Other utilities
ensure_command "cargo" "brew install rust"
VERSION=$(get_cargo_version)  # Extract from Cargo.toml
SECRET=$(generate_secret)
TEMP=$(create_temp_dir)  # Auto-cleaned on exit
extract_archive "file.tar.gz" "$TEMP"

# Docker deployment
deploy_docker_image "registry.example.com" "my-app" "$VERSION"
# Custom Dockerfile and context:
deploy_docker_image "registry.example.com" "my-app" "$VERSION" "Dockerfile.prod" "./build"
```

## Dependency Tree

```
colors.sh (no deps)
  ├── log.sh
  │   ├── platform.sh
  │   ├── download.sh
  │   │   └── github.sh
  │   └── common.sh
```

## Design Patterns

### Double-Loading Guard
All libraries include a guard to prevent re-execution:

```bash
if [[ -n "${MYLIB_LOADED}" ]]; then
    return 0
fi
MYLIB_LOADED=1
```

### Dependency Loading
Libraries auto-load their dependencies:

```bash
SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/colors.sh
source "$SCRIPT_LIB_DIR/colors.sh"
```

### Error Handling
All error functions exit with code 1:

```bash
error "Something went wrong"  # exits script
```

## Migration Guide

To migrate existing scripts:

1. **Replace color definitions:**
   ```bash
   # Before:
   RED='\033[0;31m'
   GREEN='\033[0;32m'
   # ...

   # After:
   source scripts/lib/colors.sh
   ```

2. **Replace logging functions:**
   ```bash
   # Before:
   info() { printf "${CYAN}[INFO]${NC} %s\n" "$1"; }

   # After:
   source scripts/lib/log.sh
   # Now use: info "message"
   ```

3. **Replace platform detection:**
   ```bash
   # Before:
   detect_os() { ... }

   # After:
   source scripts/lib/platform.sh
   OS=$(detect_os)
   ```

4. **Replace download functions:**
   ```bash
   # Before:
   download() { ... }

   # After:
   source scripts/lib/download.sh
   download "$URL" "$OUTPUT"
   ```

## Best Practices

1. **Load at script start:** Load all needed libraries at the beginning
2. **Use relative paths:** Always use `$SCRIPT_DIR/lib/...` for portability
3. **Check dependencies:** Use `ensure_command` for external tools
4. **Prefer specific libraries:** Load only what you need, but don't worry about double-loading
5. **Use error function:** Always use `error` instead of `exit 1` for consistency

## Example Script

See `scripts/lib/example.sh` for a complete example using all libraries.
