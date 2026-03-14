#!/usr/bin/env python3
"""Release a web plugin to the ADI web plugin registry."""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))
WEB_REGISTRY_URL = os.environ.get("ADI_WEB_REGISTRY_URL", "https://cli.registry.beta.withadi.dev")
REGISTRY_TOKEN_1PASSWORD_REF = "op://ADI/ADI Beta Registry Token/password"

CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
NC = "\033[0m"


def info(msg: str):
    print(f"{CYAN}info{NC} {msg}", flush=True)


def success(msg: str):
    print(f"{GREEN}done{NC} {msg}", flush=True)


def warn(msg: str):
    print(f"{YELLOW}warn{NC} {msg}", flush=True)


def error(msg: str):
    print(f"{RED}error{NC} {msg}", file=sys.stderr, flush=True)
    sys.exit(1)


def check_command(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def run_cmd(args: list[str], cwd: Path | None = None, stream: bool = False) -> subprocess.CompletedProcess:
    if stream:
        return subprocess.run(args, cwd=cwd, text=True)
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True)


def get_registry_token() -> str:
    token = os.environ.get("REGISTRY_AUTH_TOKEN", "").strip()
    if token:
        return token
    if not check_command("op"):
        error("REGISTRY_AUTH_TOKEN not set and 1Password CLI (op) not found")
    result = subprocess.run(
        ["op", "read", REGISTRY_TOKEN_1PASSWORD_REF],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        error(f"Failed to read registry token from 1Password: {result.stderr.strip()}")
    token = result.stdout.strip()
    if not token:
        error("Registry token from 1Password is empty")
    return token


def parse_toml_field(text: str, section: str, field: str) -> str:
    in_section = False
    for line in text.splitlines():
        if f"[{section}]" in line:
            in_section = True
            continue
        if in_section and line.startswith("["):
            break
        if in_section and line.startswith(f"{field} = "):
            match = re.search(r'"(.*?)"', line)
            return match.group(1) if match else ""
    return ""


def find_plugin_dir(plugin_id: str) -> Path | None:
    """Find plugin directory by scanning Cargo.toml files for matching plugin ID."""
    for search_dir in (PROJECT_ROOT / "crates", PROJECT_ROOT / "plugins"):
        if not search_dir.is_dir():
            continue
        for cargo_toml in search_dir.rglob("Cargo.toml"):
            try:
                text = cargo_toml.read_text()
                if "package.metadata.plugin" not in text:
                    continue
                found_id = parse_toml_field(text, "package.metadata.plugin", "id")
                if found_id == plugin_id:
                    return cargo_toml.parent
            except OSError:
                continue
    return None


def read_plugin_metadata(cargo_toml: Path) -> dict:
    """Read plugin metadata from Cargo.toml."""
    text = cargo_toml.read_text()

    version = ""
    for line in text.splitlines():
        if line.startswith("version = "):
            version = line.split('"')[1]
            break

    return {
        "id": parse_toml_field(text, "package.metadata.plugin", "id"),
        "version": version,
        "name": parse_toml_field(text, "package.metadata.plugin", "name"),
        "description": parse_toml_field(text, "package.metadata.plugin.provides", "description")
            or parse_toml_field(text, "package", "description"),
        "author": "ADI Team",
        "tags": parse_toml_field(text, "package.metadata.plugin.tags", "categories"),
    }


def bump_version(version: str, bump_type: str) -> str:
    parts = version.split(".")
    major, minor, patch = int(parts[0]), int(parts[1]), int(parts[2].split("-")[0])
    if bump_type == "patch":
        patch += 1
    elif bump_type == "minor":
        minor += 1
        patch = 0
    elif bump_type == "major":
        major += 1
        minor = 0
        patch = 0
    return f"{major}.{minor}.{patch}"


def update_version_in_cargo(cargo_toml: Path, old_version: str, new_version: str):
    text = cargo_toml.read_text()
    new_text = text.replace(f'version = "{old_version}"', f'version = "{new_version}"', 1)
    cargo_toml.write_text(new_text)
    success(f"Updated Cargo.toml: {old_version} -> {new_version}")


def find_web_dir(plugin_dir: Path) -> Path | None:
    """Find web/ directory relative to plugin crate dir."""
    for candidate in (plugin_dir.parent / "web", plugin_dir / "web"):
        if (candidate / "package.json").is_file():
            return candidate
    return None


def build_web(web_dir: Path, plugin_id: str) -> tuple[Path | None, Path | None]:
    """Build web UI and return (js_path, css_path)."""
    info(f"Building web UI from {web_dir}...")

    # cargo check the plugin crate to trigger build.rs (generates TS types)
    plugin_dir = web_dir.parent / "plugin"
    if plugin_dir.is_dir():
        cargo_toml = plugin_dir / "Cargo.toml"
        if cargo_toml.is_file():
            text = cargo_toml.read_text()
            pkg_name = ""
            for line in text.splitlines():
                if line.startswith("name = "):
                    pkg_name = line.split('"')[1]
                    break
            if pkg_name:
                info(f"Running cargo check for {pkg_name} (codegen)...")
                run_cmd(["cargo", "check", "-p", pkg_name], cwd=PROJECT_ROOT, stream=True)

    t0 = time.time()
    result = run_cmd(["bun", "run", "build"], cwd=web_dir, stream=True)
    if result.returncode != 0:
        error("Web UI build failed")
    info(f"Web UI build took {time.time() - t0:.1f}s")

    dist_dir = PROJECT_ROOT / "dist" / plugin_id
    js_path = dist_dir / "web.js"
    css_path = dist_dir / "style.css"

    if not js_path.is_file():
        error(f"Build did not produce {js_path}")

    success(f"Web UI built: {js_path.stat().st_size // 1024}K (web.js)")
    if css_path.is_file():
        success(f"Style CSS: {css_path.stat().st_size // 1024}K (style.css)")
    else:
        css_path = None

    return js_path, css_path


def create_archive(plugin_meta: dict, js_path: Path, css_path: Path | None) -> Path:
    """Create tar.gz archive with manifest.json, main.js, and optionally main.css."""
    pkg_dir = Path(tempfile.mkdtemp())

    manifest = {
        "id": plugin_meta["id"],
        "version": plugin_meta["version"],
        "name": plugin_meta["name"],
        "description": plugin_meta["description"],
        "author": plugin_meta["author"],
        "tags": [],
    }
    (pkg_dir / "manifest.json").write_text(json.dumps(manifest, indent=2))
    shutil.copy2(js_path, pkg_dir / "main.js")

    pkg_files = ["manifest.json", "main.js"]
    if css_path and css_path.is_file():
        shutil.copy2(css_path, pkg_dir / "main.css")
        pkg_files.append("main.css")

    dist_dir = PROJECT_ROOT / "dist" / plugin_meta["id"]
    dist_dir.mkdir(parents=True, exist_ok=True)
    archive_name = f"{plugin_meta['id']}-v{plugin_meta['version']}-web.tar.gz"
    archive_path = dist_dir / archive_name

    result = run_cmd(["tar", "-czf", str(archive_path), "-C", str(pkg_dir)] + pkg_files)
    if result.returncode != 0:
        error(f"Failed to create archive")

    shutil.rmtree(pkg_dir, ignore_errors=True)
    success(f"Created: {archive_name}")
    return archive_path


def publish(archive: Path, plugin_id: str, version: str, registry: str, token: str, max_retries: int = 5):
    """Publish archive to web registry."""
    info(f"Publishing {plugin_id} v{version}...")
    info(f"Registry: {registry}")

    url = f"{registry}/v1/publish/{plugin_id}/{version}"

    http_code = "0"
    body = ""
    for attempt in range(1, max_retries + 1):
        result = run_cmd([
            "curl", "-s", "-w", "\n%{http_code}", "--max-time", "300",
            "-X", "POST", url,
            "-H", "Content-Type: application/gzip",
            "-H", f"X-Registry-Token: {token}",
            "--data-binary", f"@{archive}",
        ])

        lines = result.stdout.strip().splitlines()
        http_code = lines[-1] if lines else "0"
        body = "\n".join(lines[:-1])

        if http_code in ("200", "201"):
            break
        if http_code in ("000", "0") and attempt < max_retries:
            warn(f"Connection failed (attempt {attempt}/{max_retries}), retrying in {attempt * 2}s...")
            time.sleep(attempt * 2)
            continue
        break

    if http_code in ("200", "201"):
        success(f"Published {plugin_id} v{version}")
        if body and check_command("jq"):
            jq_result = subprocess.run(["jq", "."], input=body, capture_output=True, text=True)
            print(jq_result.stdout if jq_result.returncode == 0 else body, flush=True)
        elif body:
            print(body, flush=True)
    else:
        error(f"Failed to publish (HTTP {http_code}): {body}")


def find_related_plugins(plugin_id: str) -> list[str]:
    """Find all plugin IDs that share the same parent directory."""
    plugin_dir = find_plugin_dir(plugin_id)
    if not plugin_dir:
        return []

    # Walk up to find the family root (go above plugin/ to the component dir)
    family_dir = plugin_dir.parent
    if family_dir.name == "plugin":
        family_dir = family_dir.parent

    plugin_ids: list[str] = []
    for cargo_toml in family_dir.rglob("Cargo.toml"):
        try:
            text = cargo_toml.read_text()
            if "package.metadata.plugin" not in text:
                continue
            found_id = parse_toml_field(text, "package.metadata.plugin", "id")
            if found_id and found_id != plugin_id:
                plugin_ids.append(found_id)
        except OSError:
            continue

    return sorted(set(plugin_ids))


def release_single(plugin_id: str, registry: str, no_push: bool, bump: str):
    """Release a single web plugin."""
    info(f"Looking up crate for {plugin_id}...")
    plugin_dir = find_plugin_dir(plugin_id)
    if not plugin_dir:
        error(f"Unknown plugin: {plugin_id}")

    cargo_toml = plugin_dir / "Cargo.toml"
    meta = read_plugin_metadata(cargo_toml)

    if bump:
        new_version = bump_version(meta["version"], bump)
        info(f"Bumping version: {meta['version']} -> {new_version} ({bump})")
        update_version_in_cargo(cargo_toml, meta["version"], new_version)
        meta["version"] = new_version

    info(f"Plugin: {meta['id']} v{meta['version']}")

    web_dir = find_web_dir(plugin_dir)
    if not web_dir:
        error(f"No web/ directory found for {plugin_id}")

    js_path, css_path = build_web(web_dir, meta["id"])
    archive = create_archive(meta, js_path, css_path)

    print()
    info(f"Artifact: {archive}")
    size = archive.stat().st_size
    print(f"  {size:,} bytes ({size // 1024}K)")
    print()

    if not no_push:
        token = get_registry_token()
        publish(archive, meta["id"], meta["version"], registry, token)
        print()
        success(f"Published {meta['id']} v{meta['version']} to web registry")
    else:
        info("Build complete. Use without --no-push to publish.")


def main():
    parser = argparse.ArgumentParser(description="Release a web plugin to the ADI web registry.")
    parser.add_argument("plugin_name", help="Plugin ID to release")
    parser.add_argument("--no-push", action="store_true", help="Build only, skip publishing")
    parser.add_argument("--local", action="store_true", help="Push to local web registry")
    parser.add_argument("--registry", default="", help="Override registry URL")
    parser.add_argument("--bump", default="", choices=["", "patch", "minor", "major"], help="Version bump type")
    parser.add_argument("--related", action="store_true", help="Also release all related plugins")
    args = parser.parse_args()

    registry = WEB_REGISTRY_URL
    if args.registry:
        registry = args.registry
    elif args.local:
        registry = "http://adi.test/web-registry"

    if args.related:
        related = find_related_plugins(args.plugin_name)
        all_plugins = [args.plugin_name] + related

        print()
        info(f"Releasing {args.plugin_name} + {len(related)} related plugin(s)")
        for pid in all_plugins:
            info(f"  - {pid}")
        print()

        failed: list[str] = []
        for pid in all_plugins:
            try:
                release_single(pid, registry, args.no_push, args.bump)
            except SystemExit:
                warn(f"Failed to release: {pid}")
                failed.append(pid)
            print()

        if failed:
            error(f"Failed to release {len(failed)} plugin(s): {', '.join(failed)}")
        success(f"Released {len(all_plugins)} plugin(s)")
    else:
        release_single(args.plugin_name, registry, args.no_push, args.bump)


if __name__ == "__main__":
    main()
