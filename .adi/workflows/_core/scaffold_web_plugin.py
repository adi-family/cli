#!/usr/bin/env python3
"""Scaffold a new web-only plugin with Rust stub + TypeScript/Lit web UI + tsp EventBus."""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", Path(__file__).resolve().parent.parent.parent))

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
BOLD = "\033[1m"
DIM = "\033[2m"
NC = "\033[0m"


def info(msg: str):
    print(f"{CYAN}info{NC} {msg}")


def success(msg: str):
    print(f"{GREEN}done{NC} {msg}")


def warn(msg: str):
    print(f"{YELLOW}warn{NC} {msg}")


def error(msg: str):
    print(f"{RED}error{NC} {msg}", file=sys.stderr)
    sys.exit(1)


# ── Name transforms ──────────────────────────────────────────────────────────

def to_pascal(kebab: str) -> str:
    """my-cool-tool -> MyCoolTool"""
    return "".join(w.capitalize() for w in kebab.split("-"))


def to_snake(kebab: str) -> str:
    """my-cool-tool -> my_cool_tool"""
    return kebab.replace("-", "_")


def to_spaced(kebab: str) -> str:
    """my-cool-tool -> My Cool Tool"""
    return " ".join(w.capitalize() for w in kebab.split("-"))


# ── Templates ─────────────────────────────────────────────────────────────────

def cargo_toml(name: str, description: str) -> str:
    pascal_spaced = to_spaced(name)
    return f'''[package]
name = "{name}-plugin"
version = "0.1.0"
edition = "2021"
license = "BSL-1.0"
description = "{description}"
authors = ["ADI Team"]

[lib]
crate-type = ["cdylib"]

[dependencies]
lib-plugin-prelude = {{ path = "../../_lib/lib-plugin-prelude" }}

[package.metadata.plugin]
id = "adi.{name}"
name = "ADI {pascal_spaced}"
type = "core"

[package.metadata.plugin.compatibility]
min_host_version = "2.0.0"

[[package.metadata.plugin.provides]]
id = "adi.{name}.web"
version = "1.0.0"
description = "{description}"

[package.metadata.plugin.web_ui]
entry = "web.js"
sandbox = false

[package.metadata.plugin.tags]
categories = ["web"]
'''


def lib_rs(name: str, description: str) -> str:
    pascal = to_pascal(name)
    pascal_spaced = to_spaced(name)
    return f'''use lib_plugin_prelude::*;

pub struct {pascal}Plugin;

#[async_trait]
impl Plugin for {pascal}Plugin {{
    fn metadata(&self) -> PluginMetadata {{
        PluginMetadata::new(
            "adi.{name}",
            "{pascal_spaced}",
            env!("CARGO_PKG_VERSION"),
        )
        .with_type(PluginType::Core)
        .with_author("ADI Team")
        .with_description("{description}")
    }}

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {{
        Ok(())
    }}

    async fn shutdown(&self) -> Result<()> {{
        Ok(())
    }}

    fn provides(&self) -> Vec<&'static str> {{
        vec![]
    }}
}}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {{
    Box::new({pascal}Plugin)
}}
'''


def bus_tsp(name: str) -> str:
    pascal = to_pascal(name)
    return f'''// ADI {to_spaced(name)} — EventBus definitions
// Source of truth for code generation via tsp-gen eventbus codegen.
//
// Regenerate: cd web && npm run generate

// ── EventBus: {pascal} ──────────────────────────────────────

@bus("{name}")
interface {pascal}Bus {{
    // @event yourEvent(data: string): void;
}}
'''


def package_json(name: str) -> str:
    return (
        '{\n'
        f'  "name": "@adi/{name}-web-plugin",\n'
        '  "version": "0.1.0",\n'
        '  "private": true,\n'
        '  "type": "module",\n'
        '  "scripts": {\n'
        '    "generate:events": "tsp-gen ../bus.tsp -l typescript -s eventbus --eventbus-module @adi-family/sdk-plugin/types --eventbus-interface EventRegistry --eventbus-rename kebab-case -o src/generated/bus",\n'
        '    "generate": "npm run generate:events",\n'
        '    "build": "npm run generate && vite build",\n'
        '    "typecheck": "tsc --noEmit",\n'
        '    "dev": "vite build --watch"\n'
        '  },\n'
        '  "dependencies": {\n'
        '    "@adi-family/sdk-plugin": "file:../../../packages/plugin-sdk",\n'
        '    "lit": "^3.3.1"\n'
        '  },\n'
        '  "devDependencies": {\n'
        '    "@tailwindcss/vite": "^4.2.0",\n'
        '    "tailwindcss": "^4.2.0",\n'
        '    "typescript": "^5.7.0",\n'
        '    "vite": "^6.1.0"\n'
        '  }\n'
        '}\n'
    )


VITE_CONFIG = '''import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
  build: {
    lib: {
      entry: "src/index.ts",
      formats: ["es"],
      fileName: () => "web.js",
    },
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        assetFileNames: "style[extname]",
      },
    },
    target: "es2022",
    minify: true,
  },
});
'''


TSCONFIG = '''{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "experimentalDecorators": true,
    "useDefineForClassFields": false,
    "declaration": false,
    "sourceMap": true,
    "outDir": "dist",
    "rootDir": "src",
    "skipLibCheck": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src"]
}
'''


def index_ts(name: str) -> str:
    pascal = to_pascal(name)
    return f"""import './styles.css';
import './generated/bus';
export {{ {pascal}Plugin, {pascal}Plugin as PluginShell }} from './plugin.js';
export {{ Adi{pascal}Element }} from './component.js';
"""


def plugin_ts(name: str) -> str:
    pascal = to_pascal(name)
    pascal_spaced = to_spaced(name)
    return f"""import {{ AdiPlugin }} from '@adi-family/sdk-plugin';
import './generated/bus';

export class {pascal}Plugin extends AdiPlugin {{
  readonly id = 'adi.{name}';
  readonly version = '0.1.0';

  async onRegister(): Promise<void> {{
    const {{ Adi{pascal}Element }} = await import('./component.js');
    if (!customElements.get('adi-{name}')) {{
      customElements.define('adi-{name}', Adi{pascal}Element);
    }}

    this.bus.emit('route:register', {{ path: '/{name}', element: 'adi-{name}' }});
    this.bus.emit('nav:add', {{ id: '{name}', label: '{pascal_spaced}', path: '/{name}' }});
  }}
}}
"""


def component_ts(name: str) -> str:
    pascal = to_pascal(name)
    pascal_spaced = to_spaced(name)
    return f"""import {{ LitElement, html }} from 'lit';

export class Adi{pascal}Element extends LitElement {{
  override createRenderRoot() {{
    return this;
  }}

  override render() {{
    return html`<div class="p-6">
      <h2 class="text-xl font-semibold text-text">{pascal_spaced}</h2>
      <p class="text-sm text-text-muted mt-2">Plugin is working.</p>
    </div>`;
  }}
}}
"""


STYLES_CSS = """@import "tailwindcss";
@import "../../../../packages/css/plugin-base.css";
"""


# ── Workspace registration ───────────────────────────────────────────────────

def register_workspace_member(name: str):
    """Add the plugin crate to the root Cargo.toml workspace members."""
    cargo_path = PROJECT_ROOT / "Cargo.toml"
    text = cargo_path.read_text()

    member = f'    "crates/{name}/plugin",'
    if member.strip(', ') in text:
        warn("Plugin already in workspace members, skipping")
        return

    # Insert after monaco-editor/plugin line
    anchor = '"crates/monaco-editor/plugin",'
    idx = text.find(anchor)
    if idx != -1:
        insert_pos = text.index("\n", idx) + 1
        text = text[:insert_pos] + member + "\n" + text[insert_pos:]
    else:
        # Fallback: insert before closing ] of members array
        bracket = text.rfind("]", 0, text.find("[workspace.package]"))
        if bracket != -1:
            text = text[:bracket] + member + "\n" + text[bracket:]
        else:
            warn("Could not find workspace members array, add manually")
            return

    cargo_path.write_text(text)
    success("Added to workspace members in Cargo.toml")


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Scaffold a new web-only plugin.")
    parser.add_argument("--name", required=True, help="Plugin name in kebab-case")
    parser.add_argument("--description", default="ADI web plugin", help="Short description")
    args = parser.parse_args()

    name: str = args.name
    description: str = args.description

    # Validate
    if not re.match(r"^[a-z][a-z0-9-]*$", name):
        error(f"Invalid name '{name}': must be kebab-case (lowercase letters, digits, hyphens)")

    crate_dir = PROJECT_ROOT / "crates" / name
    if crate_dir.exists():
        error(f"Directory already exists: crates/{name}/")

    pascal = to_pascal(name)
    info(f"Scaffolding web plugin: adi.{name} ({pascal}Plugin)")
    print()

    # Create directories
    plugin_src = crate_dir / "plugin" / "src"
    web_src = crate_dir / "web" / "src"
    plugin_src.mkdir(parents=True)
    web_src.mkdir(parents=True)

    # Write Rust stub
    (crate_dir / "plugin" / "Cargo.toml").write_text(cargo_toml(name, description))
    (plugin_src / "lib.rs").write_text(lib_rs(name, description))
    success("Created plugin/ (Rust stub)")

    # Write bus.tsp
    (crate_dir / "bus.tsp").write_text(bus_tsp(name))
    success("Created bus.tsp (EventBus definitions)")

    # Write web files
    (crate_dir / "web" / ".gitignore").write_text("node_modules/\ndist/\n")
    (crate_dir / "web" / "package.json").write_text(package_json(name))
    (crate_dir / "web" / "vite.config.ts").write_text(VITE_CONFIG)
    (crate_dir / "web" / "tsconfig.json").write_text(TSCONFIG)
    (web_src / "index.ts").write_text(index_ts(name))
    (web_src / "plugin.ts").write_text(plugin_ts(name))
    (web_src / "component.ts").write_text(component_ts(name))
    (web_src / "types.ts").write_text("// Types generated from bus.tsp — see src/generated/bus/types.ts\n")
    (web_src / "styles.css").write_text(STYLES_CSS)
    success("Created web/ (TypeScript/Lit)")

    # Register in workspace
    register_workspace_member(name)

    # npm install
    web_dir = crate_dir / "web"
    info("Running npm install...")
    result = subprocess.run(
        ["npm", "install", "--silent"],
        cwd=web_dir,
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        success("npm install complete")
    else:
        warn(f"npm install failed: {result.stderr.strip()}")
        warn(f"Run manually: cd crates/{name}/web && npm install")

    # Summary
    print()
    success(f"Scaffolded web plugin: {BOLD}adi.{name}{NC}")
    print()
    print(f"  {DIM}crates/{name}/{NC}")
    print(f"  {DIM}├── bus.tsp{NC}           EventBus definitions (tsp-gen source)")
    print(f"  {DIM}├── plugin/{NC}          Rust stub (cdylib)")
    print(f"  {DIM}│   ├── Cargo.toml{NC}")
    print(f"  {DIM}│   └── src/lib.rs{NC}")
    print(f"  {DIM}└── web/{NC}             TypeScript/Lit web UI")
    print(f"  {DIM}    ├── package.json{NC}")
    print(f"  {DIM}    ├── vite.config.ts{NC}")
    print(f"  {DIM}    ├── tsconfig.json{NC}")
    print(f"  {DIM}    └── src/{NC}")
    print(f"  {DIM}        ├── index.ts{NC}")
    print(f"  {DIM}        ├── plugin.ts{NC}")
    print(f"  {DIM}        ├── component.ts{NC}")
    print(f"  {DIM}        ├── types.ts{NC}")
    print(f"  {DIM}        ├── styles.css{NC}")
    print(f"  {DIM}        └── generated/bus/{NC}  (auto-generated from bus.tsp)")
    print()
    print(f"  Next steps:")
    print(f"    1. Edit crates/{name}/bus.tsp to define your events")
    print(f"    2. cd crates/{name}/web && npm run generate  {DIM}# regenerate types{NC}")
    print(f"    3. adi wf build-plugin  {DIM}# select adi.{name} to build and install{NC}")
    print()


if __name__ == "__main__":
    main()
