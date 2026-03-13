#!/usr/bin/env python3
"""Release a single plugin to the ADI plugin registry."""

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.parse
from pathlib import Path

# Force unbuffered stdout so output appears immediately in piped contexts
sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))
WORKFLOWS_DIR = Path(os.environ.get("WORKFLOWS_DIR", SCRIPT_DIR))
REGISTRY_URL = os.environ.get("ADI_REGISTRY_URL", "https://cli.registry.beta.withadi.dev")
REGISTRY_TOKEN_1PASSWORD_REF = "op://ADI/ADI Beta Registry Token/password"

# ANSI colors
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


def run_cmd(args: list[str], cwd: Path | None = None, capture: bool = True, stream: bool = False) -> subprocess.CompletedProcess:
    if stream:
        return subprocess.run(args, cwd=cwd, text=True)
    return subprocess.run(args, cwd=cwd, capture_output=capture, text=True)


def get_platform() -> str:
    os_name = platform.system().lower()
    arch = platform.machine()
    if os_name == "darwin":
        os_name = "darwin"
    elif os_name.startswith(("mingw", "msys", "cygwin")):
        os_name = "windows"

    arch_map = {"x86_64": "x86_64", "amd64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}
    arch = arch_map.get(arch, arch)

    return f"{os_name}-{arch}"


def get_registry_token() -> str:
    """Resolve registry auth token from env or 1Password CLI."""
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


def get_lib_extension(plat: str) -> str:
    if plat.startswith("darwin"):
        return "dylib"
    if plat.startswith("windows"):
        return "dll"
    return "so"


# Legacy short-name -> crate directory fallback
LEGACY_PLUGIN_MAP = {
    "cocoon": "plugins/adi.cocoon",
    "hive": "crates/hive/plugin",
    "agent-loop": "crates/agent-loop/plugin",
    "indexer": "crates/indexer/plugin",
    "knowledgebase": "plugins/adi/knowledgebase/plugin",
    "tasks": "crates/tasks/plugin",
    "workflow": "crates/workflow/plugin",
    "coolify": "crates/coolify/plugin",
    "linter": "crates/linter/plugin",
    "llm-proxy": "crates/llm-proxy/plugin",
    "llm-extract": "crates/llm-extract-plugin",
    "llm-uzu": "crates/llm-uzu-plugin",
    "tsp-gen": "crates/_lib/lib-typespec-api/plugin",
    "typespec": "crates/_lib/lib-typespec-api/plugin",
    "embed": "crates/embed-plugin",
    "audio": "../audio",
    "cli-lang-en": "crates/cli-lang-en",
    "hive-plugin-abi": "crates/hive/plugins/abi",
    "hive-runner-docker": "crates/hive/plugins/runner-docker",
    "hive-runner-podman": "crates/hive/plugins/runner-podman",
    "hive-obs-stdout": "crates/hive/plugins/obs-stdout",
    "hive-obs-file": "crates/hive/plugins/obs-file",
    "hive-obs-loki": "crates/hive/plugins/obs-loki",
    "hive-obs-prometheus": "crates/hive/plugins/obs-prometheus",
    "hive-proxy-cors": "crates/hive/plugins/proxy-cors",
    "hive-proxy-rate-limit": "crates/hive/plugins/proxy-rate-limit",
    "hive-proxy-ip-filter": "crates/hive/plugins/proxy-ip-filter",
    "hive-proxy-headers": "crates/hive/plugins/proxy-headers",
    "hive-proxy-compress": "crates/hive/plugins/proxy-compress",
    "hive-proxy-cache": "crates/hive/plugins/proxy-cache",
    "hive-proxy-rewrite": "crates/hive/plugins/proxy-rewrite",
    "hive-proxy-auth-jwt": "crates/hive/plugins/proxy-auth-jwt",
    "hive-proxy-auth-basic": "crates/hive/plugins/proxy-auth-basic",
    "hive-proxy-auth-api-key": "crates/hive/plugins/proxy-auth-api-key",
    "hive-proxy-auth-oidc": "crates/hive/plugins/proxy-auth-oidc",
    "hive-health-http": "crates/hive/plugins/health-http",
    "hive-health-tcp": "crates/hive/plugins/health-tcp",
    "hive-health-cmd": "crates/hive/plugins/health-cmd",
    "hive-health-grpc": "crates/hive/plugins/health-grpc",
    "hive-health-postgres": "crates/hive/plugins/health-postgres",
    "hive-health-redis": "crates/hive/plugins/health-redis",
    "hive-health-mysql": "crates/hive/plugins/health-mysql",
    "hive-env-dotenv": "crates/hive/plugins/env-dotenv",
    "hive-env-vault": "crates/hive/plugins/env-vault",
    "hive-env-1password": "crates/hive/plugins/env-1password",
    "hive-env-aws-secrets": "crates/hive/plugins/env-aws-secrets",
    "hive-rollout-recreate": "crates/hive/plugins/rollout-recreate",
    "hive-rollout-blue-green": "crates/hive/plugins/rollout-blue-green",
}

# Language plugins
for lang in ("cpp", "csharp", "go", "java", "lua", "php", "python", "ruby", "rust", "swift", "typescript"):
    LEGACY_PLUGIN_MAP[f"lang-{lang}"] = f"crates/indexer/lang/{lang}/plugin"


def get_plugin_crate(name: str) -> str | None:
    """Find crate directory for a plugin by ID."""
    search_dirs = [PROJECT_ROOT / "crates", PROJECT_ROOT / "plugins"]

    # Search by plugin ID in Cargo.toml [package.metadata.plugin]
    for search_dir in search_dirs:
        if not search_dir.is_dir():
            continue
        for cargo_toml in search_dir.rglob("Cargo.toml"):
            try:
                text = cargo_toml.read_text()
                if "package.metadata.plugin" not in text:
                    continue
                # Extract plugin id after [package.metadata.plugin] section
                in_section = False
                for line in text.splitlines():
                    if "package.metadata.plugin" in line:
                        in_section = True
                        continue
                    if in_section and line.startswith("["):
                        break
                    if in_section and line.startswith("id = "):
                        plugin_id = line.split('"')[1]
                        if plugin_id == name:
                            return str(cargo_toml.parent.relative_to(PROJECT_ROOT))
            except (IndexError, OSError):
                continue

    # Fallback to legacy map
    short = name.removesuffix("-plugin")
    return LEGACY_PLUGIN_MAP.get(short)


def bump_version(version: str, bump_type: str) -> str:
    """Bump semantic version."""
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
    else:
        error(f"Unknown bump type: {bump_type}. Use patch, minor, or major.")

    return f"{major}.{minor}.{patch}"


def update_plugin_version(cargo_toml: Path, old_version: str, new_version: str) -> bool:
    """Update version in Cargo.toml."""
    text = cargo_toml.read_text()

    if "version.workspace" in text.replace(" ", "") or "version = { workspace = true }" in text:
        warn("Cargo.toml uses workspace version, cannot bump individually")
        return False

    new_text = text.replace(f'version = "{old_version}"', f'version = "{new_version}"', 1)
    cargo_toml.write_text(new_text)
    success(f"Updated Cargo.toml: {old_version} -> {new_version}")
    return True


def ensure_manifest_gen() -> Path:
    """Ensure manifest-gen binary is available."""
    for profile in ("release", "debug"):
        path = PROJECT_ROOT / "target" / profile / "manifest-gen"
        if path.is_file():
            return path

    info("Building manifest-gen...")
    result = run_cmd(
        ["cargo", "build", "-p", "lib-plugin-manifest", "--features", "generate", "--release"],
        cwd=PROJECT_ROOT,
    )
    if result.returncode != 0:
        run_cmd(
            ["cargo", "build", "-p", "lib-plugin-manifest", "--features", "generate"],
            cwd=PROJECT_ROOT,
        )

    for profile in ("release", "debug"):
        path = PROJECT_ROOT / "target" / profile / "manifest-gen"
        if path.is_file():
            return path

    error("Failed to build manifest-gen")
    return Path()  # unreachable


def parse_toml_field(text: str, section: str, field: str) -> str:
    """Parse a field from a TOML section (simple parser)."""
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


class PluginBuild:
    """Holds plugin build metadata."""
    def __init__(self):
        self.id = ""
        self.version = ""
        self.platform = ""
        self.archive = Path()
        self.name = ""
        self.desc = ""
        self.author = ""
        self.type = ""
        self.web_js: Path | None = None
        self.style_css: Path | None = None


def build_plugin(plugin_name: str, crate_dir: str, dist_dir: Path, release: bool = True) -> PluginBuild:
    """Build a plugin and return metadata."""
    build = PluginBuild()

    cargo_toml = PROJECT_ROOT / crate_dir / "Cargo.toml"
    if not cargo_toml.is_file():
        error(f"Cargo.toml not found in {crate_dir}")

    # Generate plugin.toml from Cargo.toml metadata
    info("Generating plugin manifest...")
    manifest_gen = ensure_manifest_gen()
    generated_toml = Path(tempfile.mktemp(suffix=".toml", prefix="plugin-"))
    result = run_cmd([str(manifest_gen), "--cargo-toml", str(cargo_toml), "--output", str(generated_toml)])
    if result.returncode != 0:
        error(f"Failed to generate manifest from {cargo_toml}")

    # Parse plugin metadata
    toml_text = generated_toml.read_text()
    build.id = parse_toml_field(toml_text, "plugin", "id")
    build.version = parse_toml_field(toml_text, "plugin", "version")
    build.name = parse_toml_field(toml_text, "plugin", "name")
    build.desc = parse_toml_field(toml_text, "plugin", "description")
    build.author = parse_toml_field(toml_text, "plugin", "author")
    build.type = parse_toml_field(toml_text, "plugin", "type")

    if not build.id:
        error("Could not read plugin ID from Cargo.toml metadata")
    if not build.version:
        error("Could not read plugin version from Cargo.toml")

    info(f"Plugin: {build.id} v{build.version}")
    info("Building library (no standalone binary)...")

    # Get package name (may differ from plugin ID)
    actual_cargo = cargo_toml
    text = actual_cargo.read_text()
    if "[workspace]" in text:
        plugin_cargo = PROJECT_ROOT / crate_dir / "plugin" / "Cargo.toml"
        if plugin_cargo.is_file():
            actual_cargo = plugin_cargo

    package_name = ""
    for line in actual_cargo.read_text().splitlines():
        if line.startswith("name = "):
            package_name = line.split('"')[1]
            break

    # Build the library
    build_dir = PROJECT_ROOT
    target_dir = PROJECT_ROOT / "target"
    workspace_toml = PROJECT_ROOT / crate_dir / "Cargo.toml"
    if "[workspace]" in workspace_toml.read_text():
        build_dir = PROJECT_ROOT / crate_dir
        target_dir = build_dir / "target"

    cargo_args = ["cargo", "build", "-p", package_name, "--lib"]
    if release:
        cargo_args.insert(2, "--release")
    stream = not release
    info(f"Running: {' '.join(cargo_args)}")
    t0 = time.time()
    result = run_cmd(cargo_args, cwd=build_dir, stream=stream)
    elapsed = time.time() - t0
    if result.returncode != 0:
        error(f"Build failed" + (f":\n{result.stderr}" if result.stderr else ""))
    info(f"Cargo build took {elapsed:.1f}s")

    # Find the built library
    build.platform = get_platform()
    lib_ext = get_lib_extension(build.platform)
    lib_name = f"lib{package_name.replace('-', '_')}"
    profile_dir = "release" if release else "debug"
    lib_path = target_dir / profile_dir / f"{lib_name}.{lib_ext}"

    if not lib_path.is_file():
        error(f"Library not found: {lib_path}")

    info(f"Built: {lib_path}")

    # Build web UI if present
    parent_dir = (PROJECT_ROOT / crate_dir).parent
    web_dir = None
    for candidate in (parent_dir / "web", PROJECT_ROOT / crate_dir / "web"):
        if (candidate / "package.json").is_file():
            web_dir = candidate
            break

    if web_dir:
        info(f"Building web UI from {web_dir}...")
        if not check_command("npm"):
            error("npm not found. Install Node.js: https://nodejs.org")
        t0 = time.time()
        run_cmd(["npm", "install", "--silent"], cwd=web_dir, stream=stream)
        run_cmd(["npm", "run", "build"], cwd=web_dir, stream=stream)
        info(f"Web UI build took {time.time() - t0:.1f}s")
        dist_dir = PROJECT_ROOT / "dist" / build.id
        web_js = dist_dir / "web.js"
        if web_js.is_file():
            build.web_js = web_js
            size = web_js.stat().st_size
            success(f"Web UI built: {size // 1024}K ({web_js.name})")
        else:
            warn(f"Web UI build did not produce {web_js}, skipping")
        style_css = dist_dir / "style.css"
        if style_css.is_file():
            build.style_css = style_css
            size = style_css.stat().st_size
            success(f"Style CSS built: {size // 1024}K ({style_css.name})")

    # Create package
    pkg_dir = Path(tempfile.mkdtemp())
    shutil.copy2(lib_path, pkg_dir / f"plugin.{lib_ext}")
    shutil.copy2(generated_toml, pkg_dir / "plugin.toml")

    pkg_files = [f"plugin.{lib_ext}", "plugin.toml"]
    if build.web_js:
        shutil.copy2(build.web_js, pkg_dir / "web.js")
        pkg_files.append("web.js")
    if build.style_css:
        shutil.copy2(build.style_css, pkg_dir / "style.css")
        pkg_files.append("style.css")

    archive_name = f"{build.id}-v{build.version}-{build.platform}.tar.gz"
    build.archive = dist_dir / archive_name

    result = run_cmd(
        ["tar", "-czf", str(build.archive), "-C", str(pkg_dir)] + pkg_files,
    )
    if result.returncode != 0:
        error(f"Failed to create archive: {result.stderr}")

    shutil.rmtree(pkg_dir, ignore_errors=True)
    generated_toml.unlink(missing_ok=True)

    success(f"Created: {archive_name}")
    return build


def publish_plugin(build: PluginBuild, registry: str, token: str, max_retries: int = 5):
    """Publish plugin archive to registry."""
    info(f"Publishing {build.id} v{build.version} for {build.platform}...")
    info(f"Registry: {registry}")

    params = urllib.parse.urlencode({
        "name": build.name,
        "description": build.desc,
        "pluginType": build.type,
        "author": build.author,
    })
    url = f"{registry}/v1/publish/{build.id}/{build.version}/{build.platform}?{params}"

    http_code = "0"
    body = ""
    for attempt in range(1, max_retries + 1):
        result = run_cmd([
            "curl", "-s", "-w", "\n%{http_code}", "--max-time", "300",
            "-X", "POST", url,
            "-H", "Content-Type: application/gzip",
            "-H", f"X-Registry-Token: {token}",
            "--data-binary", f"@{build.archive}",
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
        success(f"Published {build.id} v{build.version}")
        if body and check_command("jq"):
            jq_result = subprocess.run(["jq", "."], input=body, capture_output=True, text=True)
            print(jq_result.stdout if jq_result.returncode == 0 else body, flush=True)
        elif body:
            print(body, flush=True)
    else:
        error(f"Failed to publish (HTTP {http_code}): {body}")

    # Upload style.css separately if present
    if build.style_css and build.style_css.is_file():
        info("Uploading style.css...")
        css_url = f"{registry}/v1/publish/{build.id}/{build.version}/style"
        css_code = "0"
        for attempt in range(1, max_retries + 1):
            css_result = run_cmd([
                "curl", "-s", "-w", "\n%{http_code}", "--max-time", "60",
                "-X", "POST", css_url,
                "-H", "Content-Type: text/css",
                "-H", f"X-Registry-Token: {token}",
                "--data-binary", f"@{build.style_css}",
            ])
            css_lines = css_result.stdout.strip().splitlines()
            css_code = css_lines[-1] if css_lines else "0"
            if css_code in ("200", "201"):
                break
            if css_code in ("000", "0") and attempt < max_retries:
                time.sleep(attempt * 2)
                continue
            break
        if css_code in ("200", "201"):
            success("Uploaded style.css")
        else:
            warn(f"Failed to upload style.css (HTTP {css_code}) — server may not support it yet")


def find_related_plugins(plugin_id: str) -> list[str]:
    """Find all plugin IDs that share the same crate family directory."""
    search_dirs = [PROJECT_ROOT / "crates", PROJECT_ROOT / "plugins"]
    target_crate_dir: Path | None = None
    containing_search_dir: Path | None = None

    for search_dir in search_dirs:
        if not search_dir.is_dir():
            continue
        for cargo_toml in search_dir.rglob("Cargo.toml"):
            try:
                text = cargo_toml.read_text()
                if "package.metadata.plugin" not in text:
                    continue
                in_section = False
                for line in text.splitlines():
                    if "package.metadata.plugin" in line:
                        in_section = True
                        continue
                    if in_section and line.startswith("["):
                        break
                    if in_section and line.startswith("id = "):
                        if line.split('"')[1] == plugin_id:
                            target_crate_dir = cargo_toml.parent
                            containing_search_dir = search_dir
                            break
            except (IndexError, OSError):
                continue

    if not target_crate_dir or not containing_search_dir:
        return []

    # Walk up to the top-level crate family (first dir inside search_dir)
    family_dir = target_crate_dir
    while family_dir.parent != containing_search_dir and family_dir.parent != family_dir:
        family_dir = family_dir.parent

    # Find all plugins under the family directory
    plugin_ids: list[str] = []
    for cargo_toml in family_dir.rglob("Cargo.toml"):
        try:
            text = cargo_toml.read_text()
            if "package.metadata.plugin" not in text:
                continue
            in_section = False
            for line in text.splitlines():
                if "package.metadata.plugin" in line:
                    in_section = True
                    continue
                if in_section and line.startswith("["):
                    break
                if in_section and line.startswith("id = "):
                    found_id = line.split('"')[1]
                    if found_id != plugin_id:
                        plugin_ids.append(found_id)
                    break
        except (IndexError, OSError):
            continue

    return sorted(set(plugin_ids))


def release_single_plugin(plugin_name: str, registry: str, no_push: bool, bump: str, local: bool = False):
    """Release a single plugin: bump, lint, build, publish."""
    info(f"Looking up crate for {plugin_name}...")
    t0 = time.time()
    crate_dir = get_plugin_crate(plugin_name)
    info(f"Crate lookup took {time.time() - t0:.1f}s")
    if not crate_dir:
        error(f"Unknown plugin: {plugin_name}. Check plugin ID.")

    crate_path = PROJECT_ROOT / crate_dir
    if not crate_path.is_dir():
        error(f"Plugin crate not found: {crate_dir}")

    # Handle version bump
    if bump:
        cargo_toml = crate_path / "Cargo.toml"
        if not cargo_toml.is_file():
            error(f"Cargo.toml not found in {crate_dir}")

        current_version = ""
        for line in cargo_toml.read_text().splitlines():
            if line.startswith("version = "):
                current_version = line.split('"')[1]
                break

        new_version = bump_version(current_version, bump)
        info(f"Bumping version: {current_version} -> {new_version} ({bump})")
        update_plugin_version(cargo_toml, current_version, new_version)

    # Lint plugin before building (skip for local releases)
    if not local:
        info("Linting plugin...")
        lint_script = WORKFLOWS_DIR / "lint-plugin.sh"
        if lint_script.is_file():
            result = run_cmd([str(lint_script), str(crate_path)])
            if result.returncode != 0:
                error("Plugin lint failed. Fix errors before publishing.")
        success("Lint passed")

    # Check prerequisites
    for cmd in ("cargo", "curl", "jq"):
        if not check_command(cmd):
            error(f"{cmd} not found")

    # Create dist directory
    dist_dir = PROJECT_ROOT / "dist" / "plugins"
    dist_dir.mkdir(parents=True, exist_ok=True)

    print()
    info(f"Building plugin: {plugin_name}")
    print()

    build = build_plugin(plugin_name, crate_dir, dist_dir, release=not local)

    print()
    info(f"Artifact: {build.archive}")
    size = build.archive.stat().st_size
    print(f"  {size:,} bytes ({size // 1024}K)")
    print()

    if not no_push:
        token = get_registry_token()
        publish_plugin(build, registry, token)
        print()
        success(f"Install with: adi plugin install {build.id}")
    else:
        info("Build complete. Use without --no-push to publish.")


def main():
    parser = argparse.ArgumentParser(description="Release a single plugin to the ADI plugin registry.")
    parser.add_argument("plugin_name", help="Plugin name/ID to release")
    parser.add_argument("--no-push", action="store_true", help="Build only, skip publishing")
    parser.add_argument("--local", action="store_true", help="Push to local registry")
    parser.add_argument("--registry", default="", help="Override registry URL")
    parser.add_argument("--bump", default="", choices=["", "patch", "minor", "major"], help="Version bump type")
    parser.add_argument("--related", action="store_true", help="Also release all related plugins in the same crate family")
    args = parser.parse_args()

    registry = REGISTRY_URL
    if args.registry:
        registry = args.registry
    elif args.local:
        registry = "http://adi.test/registry"

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
                release_single_plugin(pid, registry, args.no_push, args.bump, local=args.local)
            except SystemExit:
                warn(f"Failed to release: {pid}")
                failed.append(pid)
            print()

        if failed:
            error(f"Failed to release {len(failed)} plugin(s): {', '.join(failed)}")

        success(f"Released {len(all_plugins)} plugin(s)")
    else:
        release_single_plugin(args.plugin_name, registry, args.no_push, args.bump, local=args.local)


if __name__ == "__main__":
    main()
