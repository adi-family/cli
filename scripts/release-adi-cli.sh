#!/bin/bash
# ADI CLI Release Script
# Usage: ./scripts/release-adi-cli.sh [version]
# Example: ./scripts/release-adi-cli.sh v0.8.4

set -e

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load libraries
source "$SCRIPT_DIR/lib/log.sh"
source "$SCRIPT_DIR/lib/platform.sh"
source "$SCRIPT_DIR/lib/github.sh"
source "$SCRIPT_DIR/lib/common.sh"

# Configuration
REPO="adi-family/adi-cli"
BINARY_NAME="adi"
CRATE_PATH="crates/adi-cli"

# =============================================================================
# Main Release Flow
# =============================================================================

main() {
    # Get current version from Cargo.toml
    local current_version=$(grep '^version' "$CRATE_PATH/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    info "Current version: v$current_version"

    # Get version from argument or prompt
    local version="${1:-}"
    if [ -z "$version" ]; then
        read -p "Enter new version (or press Enter for v$current_version): " version
        version="${version:-$current_version}"
    fi

    # Normalize version
    version=$(normalize_version "$version")
    local display_version=$(ensure_v_prefix "$version")

    # Update Cargo.toml if version changed
    if [ "$version" != "$current_version" ]; then
        info "Updating $CRATE_PATH/Cargo.toml to v$version..."
        sed -i '' "s/^version = \"$current_version\"/version = \"$version\"/" "$CRATE_PATH/Cargo.toml"
        success "Updated Cargo.toml"
    fi

    info "Releasing $BINARY_NAME $display_version"

    # Check prerequisites
    ensure_command "gh" "brew install gh"
    ensure_command "cargo"

    # Create dist directory
    local dist_dir="dist/adi-cli-$display_version"
    rm -rf "$dist_dir"
    ensure_dir "$dist_dir"

    # Targets to build
    local targets=(
        "aarch64-apple-darwin"
        "x86_64-apple-darwin"
    )

    # Check for cross-compilation targets
    if rustup target list --installed | grep -q "x86_64-unknown-linux-gnu"; then
        targets+=("x86_64-unknown-linux-gnu")
    fi
    if rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
        targets+=("aarch64-unknown-linux-gnu")
    fi

    info "Building for targets: ${targets[*]}"

    # Build for each target
    for target in "${targets[@]}"; do
        info "Building for $target..."

        # Build
        if ! cargo build --release --target "$target" -p adi-cli 2>/dev/null; then
            warn "Failed to build for $target (target may not be installed)"
            continue
        fi

        # Create archive
        local archive_name="adi-${display_version}-${target}.tar.gz"
        local binary_path="target/$target/release/$BINARY_NAME"

        if [ ! -f "$binary_path" ]; then
            warn "Binary not found: $binary_path"
            continue
        fi

        # Create tarball with just the binary
        create_tarball "$dist_dir/$archive_name" "target/$target/release" "$BINARY_NAME"
        success "Created $archive_name"
    done

    # Generate SHA256SUMS
    info "Generating checksums..."
    cd "$dist_dir"
    generate_checksums "SHA256SUMS" *.tar.gz
    cd - >/dev/null
    success "Created SHA256SUMS"

    # Show what we built
    echo ""
    info "Release artifacts:"
    ls -lh "$dist_dir"
    echo ""

    # Confirm release
    read -p "Create GitHub release $display_version? [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Aborted. Artifacts are in $dist_dir"
        exit 0
    fi

    # Check if release already exists
    if release_exists "$REPO" "$display_version"; then
        error "Release $display_version already exists"
    fi

    # Create release
    info "Creating GitHub release..."
    gh release create "$display_version" \
        --repo "$REPO" \
        --title "$BINARY_NAME $display_version" \
        --notes "## Installation

\`\`\`sh
curl -fsSL https://raw.githubusercontent.com/adi-family/cli/main/scripts/install.sh | sh
\`\`\`

Or specify version:
\`\`\`sh
ADI_VERSION=$display_version curl -fsSL https://raw.githubusercontent.com/adi-family/cli/main/scripts/install.sh | sh
\`\`\`" \
        "$dist_dir"/*

    success "Released $BINARY_NAME $display_version"
    echo ""
    info "Release URL: https://github.com/$REPO/releases/tag/$display_version"
}

main "$@"
