#!/usr/bin/env bash
# Automated v3 migration script for simple CLI plugins

set -e

PLUGIN_DIR="$1"

if [ -z "$PLUGIN_DIR" ]; then
    echo "Usage: $0 <plugin-directory>"
    exit 1
fi

if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Error: Directory $PLUGIN_DIR not found"
    exit 1
fi

cd "$PLUGIN_DIR"

echo "🔄 Migrating $(basename $PLUGIN_DIR) to v3..."

# 1. Update Cargo.toml
echo "  📦 Updating Cargo.toml..."
sed -i.bak 's/lib-plugin-abi = { path = ".*" }/lib-plugin-abi-v3 = { path = "..\/..\/lib\/lib-plugin-abi-v3" }/' Cargo.toml
sed -i.bak '/abi_stable/d' Cargo.toml

# Add async deps if not present
if ! grep -q "async-trait" Cargo.toml; then
    # Insert after [dependencies]
    sed -i.bak '/^\[dependencies\]/a\
async-trait = "0.1"\
tokio = { version = "1.0", features = ["full"] }
' Cargo.toml
fi

# Update version to 3.0.0
sed -i.bak 's/^version = ".*"/version = "3.0.0"/' Cargo.toml

# 2. Update plugin.toml
echo "  📋 Updating plugin.toml..."
if [ -f "plugin.toml" ]; then
    # Update version
    sed -i.bak 's/^version = ".*"/version = "3.0.0"/' plugin.toml

    # Add compatibility section if not present
    if ! grep -q "\[compatibility\]" plugin.toml; then
        # Insert after [plugin] section
        sed -i.bak '/^\[plugin\]/,/^$/{ /^$/i\
\
[compatibility]\
api_version = 3\
min_host_version = "0.9.0"
}' plugin.toml
    fi
fi

# 3. Rename cli.rs to cli_impl.rs to avoid naming conflicts
if [ -f "src/cli.rs" ]; then
    echo "  📝 Renaming cli.rs to cli_impl.rs..."
    mv src/cli.rs src/cli_impl.rs
fi

echo "✅ Migration complete!"
echo "  Next steps:"
echo "  1. Update src/lib.rs manually (implement Plugin + CliCommands traits)"
echo "  2. cargo build --release"
echo "  3. Test the plugin"
