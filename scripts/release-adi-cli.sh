#!/bin/bash
# ADI CLI Release Script
# Usage: ./scripts/release-adi-cli.sh [version]
# Example: ./scripts/release-adi-cli.sh v0.8.4

set -e

REPO="adi-family/adi-cli"
BINARY_NAME="adi"
CRATE_PATH="crates/adi-cli"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info() { printf "${CYAN}[INFO]${NC} %s\n" "$1"; }
success() { printf "${GREEN}[DONE]${NC} %s\n" "$1"; }
warn() { printf "${YELLOW}[WARN]${NC} %s\n" "$1"; }
error() { printf "${RED}[ERROR]${NC} %s\n" "$1" >&2; exit 1; }

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version' "$CRATE_PATH/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
info "Current version: v$CURRENT_VERSION"

# Get version from argument or prompt
VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    read -p "Enter new version (or press Enter for v$CURRENT_VERSION): " VERSION
    VERSION="${VERSION:-$CURRENT_VERSION}"
fi

# Remove 'v' prefix for comparison
VERSION="${VERSION#v}"

# Ensure version starts with 'v' for display
DISPLAY_VERSION="v$VERSION"

# Update Cargo.toml if version changed
if [ "$VERSION" != "$CURRENT_VERSION" ]; then
    info "Updating $CRATE_PATH/Cargo.toml to v$VERSION..."
    sed -i '' "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION\"/" "$CRATE_PATH/Cargo.toml"
    success "Updated Cargo.toml"
fi

info "Releasing $BINARY_NAME $DISPLAY_VERSION"

# Check prerequisites
command -v gh >/dev/null 2>&1 || error "gh CLI not found. Install: brew install gh"
command -v cargo >/dev/null 2>&1 || error "cargo not found"

# Create dist directory
DIST_DIR="dist/adi-cli-$DISPLAY_VERSION"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Targets to build
TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
)

# Check for cross-compilation targets
if rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
    TARGETS+=("x86_64-unknown-linux-gnu")
fi
if rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
    TARGETS+=("aarch64-unknown-linux-gnu")
fi

info "Building for targets: ${TARGETS[*]}"

# Build for each target
for TARGET in "${TARGETS[@]}"; do
    info "Building for $TARGET..."

    # Build
    cargo build --release --target "$TARGET" -p adi-cli 2>/dev/null || {
        warn "Failed to build for $TARGET (target may not be installed)"
        continue
    }

    # Create archive
    ARCHIVE_NAME="adi-${DISPLAY_VERSION}-${TARGET}.tar.gz"
    BINARY_PATH="target/$TARGET/release/$BINARY_NAME"

    if [ ! -f "$BINARY_PATH" ]; then
        warn "Binary not found: $BINARY_PATH"
        continue
    fi

    # Create tarball with just the binary
    tar -czf "$DIST_DIR/$ARCHIVE_NAME" -C "target/$TARGET/release" "$BINARY_NAME"
    success "Created $ARCHIVE_NAME"
done

# Generate SHA256SUMS
info "Generating checksums..."
cd "$DIST_DIR"
shasum -a 256 *.tar.gz > SHA256SUMS
cd - >/dev/null
success "Created SHA256SUMS"

# Show what we built
echo ""
info "Release artifacts:"
ls -lh "$DIST_DIR"
echo ""

# Confirm release
read -p "Create GitHub release $DISPLAY_VERSION? [y/N] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    info "Aborted. Artifacts are in $DIST_DIR"
    exit 0
fi

# Check if release already exists
if gh release view "$DISPLAY_VERSION" -R "$REPO" >/dev/null 2>&1; then
    error "Release $DISPLAY_VERSION already exists"
fi

# Create release
info "Creating GitHub release..."
gh release create "$DISPLAY_VERSION" \
    --repo "$REPO" \
    --title "$BINARY_NAME $DISPLAY_VERSION" \
    --notes "## Installation

\`\`\`sh
curl -fsSL https://raw.githubusercontent.com/adi-family/cli/main/scripts/install.sh | sh
\`\`\`

Or specify version:
\`\`\`sh
ADI_VERSION=$DISPLAY_VERSION curl -fsSL https://raw.githubusercontent.com/adi-family/cli/main/scripts/install.sh | sh
\`\`\`" \
    "$DIST_DIR"/*

success "Released $BINARY_NAME $DISPLAY_VERSION"
echo ""
info "Release URL: https://github.com/$REPO/releases/tag/$DISPLAY_VERSION"
