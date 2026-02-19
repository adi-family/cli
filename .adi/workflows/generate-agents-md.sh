#!/bin/bash
# Generate AGENTS.md with crate structure documentation
set -e

OUTPUT="${1:-AGENTS.md}"

# Find crates with plugin/ = user-facing
USER_FACING=$(find crates/*/plugin -name Cargo.toml 2>/dev/null | cut -d/ -f2 | sort -u)

# Find crates with http/ but no plugin/ = backend
HAS_HTTP=$(find crates/*/http -name Cargo.toml 2>/dev/null | cut -d/ -f2 | sort -u)
HAS_PLUGIN=$(find crates/*/plugin -name Cargo.toml 2>/dev/null | cut -d/ -f2 | sort -u)
BACKEND=$(comm -23 <(echo "$HAS_HTTP") <(echo "$HAS_PLUGIN"))

# Libraries
LIBRARIES=$(ls -1 crates/lib/ 2>/dev/null | sort)

# Plugins (*-plugin)
PLUGINS=$(find crates -maxdepth 2 -name Cargo.toml -path "crates/*-plugin/*" 2>/dev/null | cut -d/ -f2 | sort -u)

# Tools (tool-* and standalone)
TOOLS=$(find crates -maxdepth 2 -name Cargo.toml 2>/dev/null | grep -E "crates/(tool-|cocoon|webrtc)" | cut -d/ -f2 | sort -u)

get_subdirs() {
    find "crates/$1" -maxdepth 2 -type d \( -name "core" -o -name "http" -o -name "plugin" -o -name "cli" -o -name "mcp" \) 2>/dev/null | \
        sed "s|crates/$1/||" | sort | tr '\n' ', ' | sed 's/,$//'
}

get_desc() {
    local toml="$1"
    local desc=$(grep -m1 "^description" "$toml" 2>/dev/null | sed 's/description = "//;s/"$//')
    echo "${desc:--}"
}

get_crate_desc() {
    local crate="$1"
    # Try core/, then plugin/, then http/, then mcp/
    for sub in core plugin http mcp; do
        local toml="crates/$crate/$sub/Cargo.toml"
        if [ -f "$toml" ]; then
            local desc=$(grep -m1 "^description" "$toml" 2>/dev/null | sed 's/description = "//;s/"$//')
            if [ -n "$desc" ]; then
                echo "$desc"
                return
            fi
        fi
    done
    echo ""
}

{
cat << 'HEADER'
# ADI Crate Structure

> Auto-generate with: `adi wf generate-agents-md`

## User-Facing Components
Components with plugin for `adi` CLI integration.

| Crate | Structure | Description |
|-------|-----------|-------------|
HEADER

for c in $USER_FACING; do
    subs=$(get_subdirs "$c")
    desc=$(get_crate_desc "$c")
    printf '| `%s` | %s | %s |\n' "$c" "$subs" "$desc"
done

cat << 'BACKEND_HEADER'

## Backend Services
HTTP services without CLI plugin.

| Crate | Structure | Description |
|-------|-----------|-------------|
BACKEND_HEADER

for c in $BACKEND; do
    subs=$(get_subdirs "$c")
    desc=$(get_crate_desc "$c")
    printf '| `%s` | %s | %s |\n' "$c" "$subs" "$desc"
done

cat << 'LIB_HEADER'

## Libraries
Shared libraries in `crates/lib/`.

| Library | Purpose |
|---------|---------|
LIB_HEADER

for lib in $LIBRARIES; do
    desc=$(get_desc "crates/lib/$lib/Cargo.toml")
    printf '| `%s` | %s |\n' "$lib" "$desc"
done

cat << 'PLUGIN_HEADER'

## Standalone Plugins

| Plugin | Description |
|--------|-------------|
PLUGIN_HEADER

for p in $PLUGINS; do
    desc=$(get_desc "crates/$p/Cargo.toml")
    printf '| `%s` | %s |\n' "$p" "$desc"
done

cat << 'TOOLS_HEADER'

## Tools

| Tool | Description |
|------|-------------|
TOOLS_HEADER

for t in $TOOLS; do
    desc=$(get_desc "crates/$t/Cargo.toml")
    printf '| `%s` | %s |\n' "$t" "$desc"
done

cat << 'WORKFLOWS_HEADER'

## Workflows
Available workflows in `.adi/workflows/`. Run with `adi wf <name>` or directly via `.adi/workflows/<name>.sh`.

| Workflow | Description |
|----------|-------------|
WORKFLOWS_HEADER

for wf in .adi/workflows/*.toml; do
    name=$(grep -m1 '^name = ' "$wf" 2>/dev/null | sed 's/name = "//;s/"$//')
    desc=$(grep -m1 '^description = ' "$wf" 2>/dev/null | sed 's/description = "//;s/"$//')
    [ -n "$name" ] && printf '| `%s` | %s |\n' "$name" "$desc"
done

cat << 'CODE_STYLE_HEADER'

## Code Style Guidelines

CODE_STYLE_HEADER

# Inline *.inline.md files
for doc in docs/code-style/*.inline.md; do
    [ -f "$doc" ] || continue
    echo ""
    cat "$doc"
    echo ""
done

# List non-inline files as references (skip if .inline.md version exists)
has_non_inline=false
for doc in docs/code-style/*.md; do
    [ -f "$doc" ] || continue
    [[ "$doc" == *.inline.md ]] && continue
    name=$(basename "$doc" .md)
    # Skip if inline version exists
    [ -f "docs/code-style/${name}.inline.md" ] && continue
    has_non_inline=true
done

if $has_non_inline; then
    echo ""
    echo "**Additional guidelines:**"
    for doc in docs/code-style/*.md; do
        [ -f "$doc" ] || continue
        [[ "$doc" == *.inline.md ]] && continue
        name=$(basename "$doc" .md)
        [ -f "docs/code-style/${name}.inline.md" ] && continue
        summary=$(grep -m1 "^\*\*" "$doc" 2>/dev/null | sed 's/\*\*//g' | cut -c1-60 || echo "-")
        echo "- [\`$name\`]($doc): $summary"
    done
fi

echo ""
} > "$OUTPUT"

echo "Generated $OUTPUT"
