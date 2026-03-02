#!/usr/bin/env python3
"""Quick build + replace for CLI binary or plugin (local dev iteration)."""

import argparse
import os
import platform as plat
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))
WORKFLOWS_DIR = Path(os.environ.get("WORKFLOWS_DIR", SCRIPT_DIR))

CLI_BIN = Path.home() / ".local" / "bin" / "adi"

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


def run_cmd(args: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True)


def human_size(path: Path) -> str:
    size = path.stat().st_size
    for unit in ("B", "K", "M", "G"):
        if size < 1024:
            return f"{size}{unit}"
        size //= 1024
    return f"{size}T"


def patch_cli():
    if not shutil.which("cargo"):
        error("cargo not found")

    print()
    info("Building CLI (release)...")
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "cli"],
        cwd=PROJECT_ROOT,
    )
    if result.returncode != 0:
        error("Build failed")

    built_bin = PROJECT_ROOT / "target" / "release" / "adi"
    if not built_bin.is_file():
        error(f"Binary not found: {built_bin}")

    size = human_size(built_bin)
    success(f"Built: {built_bin} ({size})")

    # Replace installed binary
    CLI_BIN.parent.mkdir(parents=True, exist_ok=True)

    if CLI_BIN.is_file():
        info(f"Replacing {CLI_BIN}...")
    else:
        info(f"Installing to {CLI_BIN}...")

    shutil.copy2(built_bin, CLI_BIN)
    CLI_BIN.chmod(0o755)

    # Codesign on macOS
    if plat.system() == "Darwin":
        info("Signing binary for macOS (ad-hoc)...")
        result = run_cmd(["codesign", "-s", "-", "-f", str(CLI_BIN)])
        if result.returncode == 0:
            success(f"Signed: {CLI_BIN}")
        else:
            warn("Codesign failed (non-fatal)")

    print()
    success(f"CLI patched: {CLI_BIN} ({size})")
    run_cmd([str(CLI_BIN), "--version"])


def patch_plugin(plugin_id: str):
    if not plugin_id:
        error("Plugin ID required. Example: patch plugin adi.hive")

    print()
    info(f"Patching plugin: {plugin_id}")

    build_script = WORKFLOWS_DIR / "build-plugin.sh"
    if not build_script.is_file():
        error(f"build-plugin.sh not found at {build_script}")

    result = subprocess.run(
        [str(build_script), plugin_id, "--install", "--force", "--skip-lint"],
        cwd=PROJECT_ROOT,
    )
    if result.returncode != 0:
        error("Plugin patch failed")


def main():
    parser = argparse.ArgumentParser(description="Quick build + replace for local development.")
    parser.add_argument("target", choices=["cli", "plugin"], help="What to patch")
    parser.add_argument("plugin_id", nargs="?", default="", help="Plugin ID (for plugin target)")
    args = parser.parse_args()

    if args.target == "cli":
        patch_cli()
    elif args.target == "plugin":
        patch_plugin(args.plugin_id)


if __name__ == "__main__":
    main()
