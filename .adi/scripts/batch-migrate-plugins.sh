#!/usr/bin/env bash
# Batch migrate multiple plugins to v3

set -e

# Plugin mapping: directory:struct_name:display_name
PLUGINS=(
    # Core CLI plugins
    "crates/audio/plugin:AudioPlugin:ADI Audio"
    "crates/linter/plugin:LinterPlugin:ADI Linter"
    "crates/coolify/plugin:CoolifyPlugin:ADI Coolify"
    "crates/browser-debug/plugin:BrowserDebugPlugin:Browser Debug"

    # Language plugins
    "crates/lang/go/plugin:GoLangPlugin:Go Language Support"
    "crates/lang/python/plugin:PythonLangPlugin:Python Language Support"
    "crates/lang/typescript/plugin:TypeScriptLangPlugin:TypeScript Language Support"
    "crates/lang/rust/plugin:RustLangPlugin:Rust Language Support"
    "crates/lang/java/plugin:JavaLangPlugin:Java Language Support"
    "crates/lang/php/plugin:PhpLangPlugin:PHP Language Support"
    "crates/lang/lua/plugin:LuaLangPlugin:Lua Language Support"
    "crates/lang/cpp/plugin:CppLangPlugin:C++ Language Support"
    "crates/lang/swift/plugin:SwiftLangPlugin:Swift Language Support"
    "crates/lang/csharp/plugin:CSharpLangPlugin:C# Language Support"
    "crates/lang/ruby/plugin:RubyLangPlugin:Ruby Language Support"
)

TEMPLATE="$(dirname "$0")/../templates/v3-cli-only-plugin.rs"
ROOT_DIR="$(git rev-parse --show-toplevel)"

for entry in "${PLUGINS[@]}"; do
    IFS=':' read -r plugin_dir struct_name display_name <<< "$entry"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "🔄 Migrating: $display_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    cd "$ROOT_DIR/$plugin_dir"

    # Get plugin ID from plugin.toml
    if [ -f "plugin.toml" ]; then
        PLUGIN_ID=$(grep '^id =' plugin.toml | head -1 | cut -d'"' -f2 | tr -d '\n')
        PLUGIN_DESC=$(grep '^description =' plugin.toml | head -1 | cut -d'"' -f2 | tr -d '\n')
    else
        echo "❌ plugin.toml not found, skipping..."
        continue
    fi

    echo "  Plugin ID: $PLUGIN_ID"

    # 1. Update Cargo.toml
    echo "  📦 Updating Cargo.toml..."
    if grep -q "lib-plugin-abi-v3" Cargo.toml; then
        echo "  ⏭️  Already migrated to v3"
        continue
    fi

    # Replace lib-plugin-abi with lib-plugin-abi-v3
    sed -i.bak 's|lib-plugin-abi = { path = ".*" }|lib-plugin-abi-v3 = { path = "../../lib/lib-plugin-abi-v3" }|' Cargo.toml

    # Remove abi_stable
    sed -i.bak '/^abi_stable/d' Cargo.toml

    # Add async deps if not present
    if ! grep -q "async-trait" Cargo.toml; then
        # Add after first [dependencies] line
        awk '/^\[dependencies\]/{print; print "async-trait = \"0.1\""; print "tokio = { version = \"1.0\", features = [\"full\"] }"; next}1' Cargo.toml > Cargo.toml.tmp
        mv Cargo.toml.tmp Cargo.toml
    fi

    # Update version to 3.0.0
    sed -i.bak 's/^version = ".*"/version = "3.0.0"/' Cargo.toml

    # 2. Update plugin.toml
    echo "  📋 Updating plugin.toml..."
    sed -i.bak 's/^version = ".*"/version = "3.0.0"/' plugin.toml

    # Add compatibility section
    if ! grep -q "\[compatibility\]" plugin.toml; then
        echo "" >> plugin.toml
        echo "[compatibility]" >> plugin.toml
        echo "api_version = 3" >> plugin.toml
        echo "min_host_version = \"0.9.0\"" >> plugin.toml
    fi

    # 3. Rename cli.rs if exists
    if [ -f "src/cli.rs" ]; then
        echo "  📝 Renaming cli.rs → cli_impl.rs..."
        mv src/cli.rs src/cli_impl.rs
    fi

    # 4. Generate lib.rs from template
    echo "  🎨 Generating src/lib.rs from template..."
    # Escape special characters for sed
    PLUGIN_DESC_ESCAPED=$(echo "$PLUGIN_DESC" | sed 's/[\/&]/\\&/g' | tr '\n' ' ')
    cat "$ROOT_DIR/$TEMPLATE" | \
        sed "s/{{PLUGIN_NAME}}/$display_name/g" | \
        sed "s/{{PLUGIN_ID}}/$PLUGIN_ID/g" | \
        sed "s/{{PLUGIN_STRUCT}}/$struct_name/g" | \
        sed "s/{{PLUGIN_DESCRIPTION}}/$PLUGIN_DESC_ESCAPED/g" > src/lib.rs.new

    # Backup old lib.rs
    mv src/lib.rs src/lib.rs.v2.bak
    mv src/lib.rs.new src/lib.rs

    # 5. Try to build
    echo "  🔨 Building..."
    if cargo build --release 2>/dev/null; then
        echo "  ✅ Build successful!"
        rm -f Cargo.toml.bak src/lib.rs.v2.bak
    else
        echo "  ⚠️  Build failed - manual fixes needed"
        echo "     Old lib.rs backed up to src/lib.rs.v2.bak"
    fi

    echo ""
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Batch migration complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
