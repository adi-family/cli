#!/usr/bin/env python3
"""Build and optionally install a plugin locally without publishing to registry."""

import argparse
import os
import platform as plat
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))
WORKFLOWS_DIR = Path(os.environ.get("WORKFLOWS_DIR", SCRIPT_DIR))

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
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


def run_cmd(args: list[str], cwd: Path | None = None, capture: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, cwd=cwd, capture_output=capture, text=True)


def check_command(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def get_platform() -> str:
    os_name = plat.system().lower()
    arch = plat.machine()
    if os_name.startswith(("mingw", "msys", "cygwin")):
        os_name = "windows"
    arch_map = {"x86_64": "x86_64", "amd64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}
    arch = arch_map.get(arch, arch)
    return f"{os_name}-{arch}"


def get_lib_extension(platform_str: str) -> str:
    if platform_str.startswith("darwin"):
        return "dylib"
    if platform_str.startswith("windows"):
        return "dll"
    return "so"


def get_plugins_dir() -> Path:
    env_dir = os.environ.get("ADI_PLUGINS_DIR")
    if env_dir:
        return Path(env_dir)
    if plat.system() == "Darwin":
        return Path.home() / "Library" / "Application Support" / "adi" / "plugins"
    return Path.home() / ".local" / "share" / "adi" / "plugins"


# Legacy short-name -> crate directory fallback (same as release_plugin.py)
LEGACY_PLUGIN_MAP = {
    "cocoon": "plugins/adi.cocoon",
    "hive": "crates/hive/plugin",
    "agent-loop": "crates/agent-loop/plugin",
    "indexer": "crates/indexer/plugin",
    "knowledgebase": "plugins/adi.knowledgebase/plugin",
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
for _lang in ("cpp", "csharp", "go", "java", "lua", "php", "python", "ruby", "rust", "swift", "typescript"):
    LEGACY_PLUGIN_MAP[f"lang-{_lang}"] = f"crates/indexer/lang/{_lang}/plugin"


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


def parse_toml_section_field(text: str, section: str, field: str) -> str:
    """Parse a field from a TOML section."""
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
        self.lib_path = Path()
        self.toml_path = Path()
        self.web_js: Path | None = None
        self.style_css: Path | None = None


def build_plugin(plugin_name: str, crate_dir: str, skip_lint: bool) -> PluginBuild:
    """Build a plugin and return metadata."""
    build = PluginBuild()

    cargo_toml = PROJECT_ROOT / crate_dir / "Cargo.toml"
    if not cargo_toml.is_file():
        error(f"Cargo.toml not found in {crate_dir}")

    # Generate plugin.toml from Cargo.toml metadata
    manifest_gen = ensure_manifest_gen()
    generated_toml = Path(tempfile.mktemp(suffix=".toml", prefix="plugin-"))
    result = run_cmd([str(manifest_gen), "--cargo-toml", str(cargo_toml), "--output", str(generated_toml)])
    if result.returncode != 0:
        error(f"Failed to generate manifest from {cargo_toml}")

    # Parse plugin metadata from generated manifest
    toml_text = generated_toml.read_text()
    build.id = parse_toml_section_field(toml_text, "plugin", "id")
    build.version = parse_toml_section_field(toml_text, "plugin", "version")
    build.toml_path = generated_toml

    if not build.id:
        error("Could not read plugin ID from Cargo.toml metadata")
    if not build.version:
        error("Could not read plugin version from Cargo.toml")

    info(f"Plugin: {build.id} v{build.version}")

    # Lint plugin unless skipped
    if not skip_lint:
        info("Linting plugin...")
        lint_script = WORKFLOWS_DIR / "_core" / "lint_plugin.py"
        lint_result = run_cmd(
            [sys.executable, str(lint_script), str(PROJECT_ROOT / crate_dir)],
            capture=True,
        )
        if lint_result.returncode != 0:
            warn("Lint failed - continuing anyway (use --skip-lint to suppress)")
        else:
            success("Lint passed")
    else:
        info("Skipping lint (--skip-lint)")

    info("Building library...")

    # Get package name from Cargo.toml (may differ from plugin ID)
    actual_cargo = cargo_toml
    cargo_text = actual_cargo.read_text()
    if "[workspace]" in cargo_text:
        plugin_cargo = PROJECT_ROOT / crate_dir / "plugin" / "Cargo.toml"
        if plugin_cargo.is_file():
            actual_cargo = plugin_cargo

    package_name = ""
    for line in actual_cargo.read_text().splitlines():
        if line.startswith("name = "):
            package_name = line.split('"')[1]
            break

    # Build ONLY the library (not the binary)
    build_dir = PROJECT_ROOT
    target_dir = PROJECT_ROOT / "target"
    workspace_toml = PROJECT_ROOT / crate_dir / "Cargo.toml"
    if "[workspace]" in workspace_toml.read_text():
        build_dir = PROJECT_ROOT / crate_dir
        target_dir = build_dir / "target"

    result = run_cmd(["cargo", "build", "--release", "-p", package_name, "--lib"], cwd=build_dir)
    if result.returncode != 0:
        error(f"Build failed:\n{result.stderr}")

    # Find the built library
    build.platform = get_platform()
    lib_ext = get_lib_extension(build.platform)
    lib_name = f"lib{package_name.replace('-', '_')}"
    build.lib_path = target_dir / "release" / f"{lib_name}.{lib_ext}"

    if not build.lib_path.is_file():
        error(f"Library not found: {build.lib_path}")

    success(f"Built: {build.lib_path}")

    # Build web UI if present (sibling web/ directory with package.json)
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
        run_cmd(["npm", "install", "--silent"], cwd=web_dir)
        run_cmd(["npm", "run", "build"], cwd=web_dir)
        web_js = web_dir / "dist" / "web.js"
        if web_js.is_file():
            build.web_js = web_js
            size = web_js.stat().st_size
            success(f"Web UI built: {size // 1024}K ({web_js.name})")
        else:
            warn("Web UI build did not produce dist/web.js, skipping")
        style_css = web_dir / "dist" / "style.css"
        if style_css.is_file():
            build.style_css = style_css
            size = style_css.stat().st_size
            success(f"Style CSS built: {size // 1024}K ({style_css.name})")

    return build


def install_plugin(build: PluginBuild, force: bool, plugins_dir: Path):
    """Install a built plugin to the local plugins directory."""
    install_dir = plugins_dir / build.id / build.version
    version_file = plugins_dir / build.id / ".version"

    # Check if already installed
    if install_dir.is_dir() and not force:
        warn(f"Plugin {build.id} v{build.version} already installed at:")
        warn(f"  {install_dir}")
        warn("Use --force to replace")
        sys.exit(1)

    # Remove existing installation if force
    if install_dir.is_dir():
        info("Removing existing installation...")
        shutil.rmtree(install_dir)

    # Create installation directory
    install_dir.mkdir(parents=True, exist_ok=True)

    # Get library extension
    lib_ext = get_lib_extension(build.platform)

    # Copy files (rename library to plugin.<ext> as expected by adi-cli)
    info(f"Installing to: {install_dir}")
    shutil.copy2(build.lib_path, install_dir / f"plugin.{lib_ext}")
    shutil.copy2(build.toml_path, install_dir / "plugin.toml")
    if build.web_js and build.web_js.is_file():
        shutil.copy2(build.web_js, install_dir / "web.js")
        info("Installed web UI: web.js")
    if build.style_css and build.style_css.is_file():
        shutil.copy2(build.style_css, install_dir / "style.css")
        info("Installed style: style.css")

    # Sign binary on macOS
    if plat.system() == "Darwin":
        subprocess.run(
            ["codesign", "-s", "-", "-f", str(install_dir / f"plugin.{lib_ext}")],
            capture_output=True, text=True,
        )

    # Update version file (tracks current active version)
    version_file.write_text(build.version)

    # Update latest symlink (points to current version directory)
    latest_link = plugins_dir / build.id / "latest"
    if latest_link.is_symlink() or latest_link.exists():
        latest_link.unlink()
    latest_link.symlink_to(build.version)

    # Update command index symlinks
    update_command_index(install_dir / "plugin.toml", build.id, plugins_dir)

    success(f"Installed {build.id} v{build.version}")
    print()
    info(f"Installation directory: {install_dir}")
    result = run_cmd(["ls", "-la", str(install_dir)])
    if result.stdout:
        print(result.stdout)


def update_command_index(plugin_toml: Path, plugin_id: str, plugins_dir: Path):
    """Create/update command index symlinks for fast CLI command discovery."""
    commands_dir = plugins_dir / "commands"
    commands_dir.mkdir(parents=True, exist_ok=True)

    # Parse command from [cli] section
    toml_text = plugin_toml.read_text()
    cli_command = parse_toml_section_field(toml_text, "cli", "command")
    if not cli_command:
        return

    # Remove old symlinks for this plugin
    for link in commands_dir.iterdir():
        if not link.is_symlink():
            continue
        target = os.readlink(str(link))
        if target.startswith(f"../{plugin_id}/"):
            link.unlink()

    # Create main command symlink (points through latest/ for version-agnostic resolution)
    (commands_dir / cli_command).symlink_to(f"../{plugin_id}/latest/plugin.toml")

    # Parse and create alias symlinks
    aliases_raw = parse_toml_section_field(toml_text, "cli", "aliases")
    if not aliases_raw:
        # Try to parse array format directly
        in_cli = False
        for line in toml_text.splitlines():
            if re.match(r"^\[cli\]", line):
                in_cli = True
                continue
            if in_cli and line.startswith("["):
                break
            if in_cli and line.startswith("aliases"):
                # Extract aliases from array like aliases = ["t", "tk"]
                matches = re.findall(r'"([^"]*)"', line)
                for alias in matches:
                    alias = alias.strip()
                    if alias:
                        (commands_dir / alias).symlink_to(f"../{plugin_id}/latest/plugin.toml")
                break
    else:
        # Single alias as string value
        alias = aliases_raw.strip()
        if alias:
            (commands_dir / alias).symlink_to(f"../{plugin_id}/latest/plugin.toml")


def create_archive(build: PluginBuild, dist_dir: Path):
    """Create a distributable tar.gz archive."""
    dist_dir.mkdir(parents=True, exist_ok=True)

    lib_ext = get_lib_extension(build.platform)
    archive_name = f"{build.id}-v{build.version}-{build.platform}.tar.gz"
    archive_path = dist_dir / archive_name

    # Create package in temp dir
    pkg_dir = Path(tempfile.mkdtemp())
    shutil.copy2(build.lib_path, pkg_dir / f"plugin.{lib_ext}")
    shutil.copy2(build.toml_path, pkg_dir / "plugin.toml")

    pkg_files = [f"plugin.{lib_ext}", "plugin.toml"]
    if build.web_js and build.web_js.is_file():
        shutil.copy2(build.web_js, pkg_dir / "web.js")
        pkg_files.append("web.js")
    if build.style_css and build.style_css.is_file():
        shutil.copy2(build.style_css, pkg_dir / "style.css")
        pkg_files.append("style.css")

    result = run_cmd(["tar", "-czf", str(archive_path), "-C", str(pkg_dir)] + pkg_files)
    if result.returncode != 0:
        error(f"Failed to create archive: {result.stderr}")

    shutil.rmtree(pkg_dir, ignore_errors=True)

    success(f"Built: {archive_path}")
    size_result = run_cmd(["ls", "-lh", str(archive_path)])
    if size_result.stdout:
        print(size_result.stdout.strip())
    print()
    info("To install locally, run again with --install")


def main():
    parser = argparse.ArgumentParser(
        description="Build and optionally install a plugin locally without publishing to registry.",
    )
    parser.add_argument("plugin_name", help="Plugin name or ID (e.g., adi.hive, cocoon)")
    parser.add_argument("--install", action="store_true", help="Install to local plugins directory after building")
    parser.add_argument("--force", action="store_true", help="Force replace existing installation (with --install)")
    parser.add_argument("--skip-lint", action="store_true", help="Skip linting step (faster build)")
    args = parser.parse_args()

    # Find crate directory
    crate_dir = get_plugin_crate(args.plugin_name)
    if not crate_dir:
        error(f"Unknown plugin: {args.plugin_name}. Run with --help to see usage.")

    crate_path = PROJECT_ROOT / crate_dir
    if not crate_path.is_dir():
        error(f"Plugin crate not found: {crate_dir}")

    # Check prerequisites
    if not check_command("cargo"):
        error("cargo not found")

    print()
    info(f"Building plugin: {args.plugin_name}")
    print()

    # Build plugin
    build = build_plugin(args.plugin_name, crate_dir, args.skip_lint)
    print()

    if args.install:
        plugins_dir = get_plugins_dir()
        info("Installing plugin locally...")
        install_plugin(build, args.force, plugins_dir)
        print()
        success(f"Plugin ready to use: adi {build.id} <command>")
    else:
        dist_dir = PROJECT_ROOT / "dist" / "plugins"
        create_archive(build, dist_dir)

    # Clean up generated toml
    build.toml_path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
