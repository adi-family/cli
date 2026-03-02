#!/usr/bin/env python3
"""Release adi CLI: Build cross-platform binaries and create GitHub release."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

REPO = "adi-family/adi-cli"
DIST_DIR = PROJECT_ROOT / "dist" / "cli"

TARGETS = [
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-musl",
]

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
BOLD = "\033[1m"
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


def run_cmd(args: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if check and result.returncode != 0:
        error(f"Command failed: {' '.join(args)}\n{result.stderr}")
    return result


def human_size(path: Path) -> str:
    size = path.stat().st_size
    for unit in ("B", "K", "M", "G"):
        if size < 1024:
            return f"{size}{unit}"
        size //= 1024
    return f"{size}T"


def build_binaries(version: str):
    """Build CLI binaries for all targets."""
    print(f"{BOLD}Step 1: Building binaries...{NC}")

    for target in TARGETS:
        info(f"Building for {target}...")

        # Ensure target is installed
        result = run_cmd(["rustup", "target", "list", "--installed"], check=False)
        if target not in result.stdout:
            warn(f"Target {target} not installed, installing...")
            run_cmd(["rustup", "target", "add", target])

        # Build
        run_cmd(["cargo", "build", "--release", "--target", target, "-p", "cli"], cwd=PROJECT_ROOT)

        # Determine binary name and archive format
        is_windows = "windows" in target
        binary_name = "adi.exe" if is_windows else "adi"
        archive_name = f"adi-v{version}-{target}.zip" if is_windows else f"adi-v{version}-{target}.tar.gz"

        src_binary = PROJECT_ROOT / "target" / target / "release" / binary_name
        if not src_binary.is_file():
            error(f"Binary not found: {src_binary}")

        archive_path = DIST_DIR / archive_name

        if is_windows:
            run_cmd(["zip", "-j", str(archive_path), str(src_binary)])
        else:
            run_cmd(["tar", "-czf", str(archive_path), "-C", str(src_binary.parent), binary_name])

        success(f"Created {archive_name}")

    print()


def verify_archives(version: str) -> list[Path]:
    """Verify all archives exist and return their paths."""
    print(f"{BOLD}Step 2: Verifying archives...{NC}")

    assets = []
    for target in TARGETS:
        is_windows = "windows" in target
        archive_name = f"adi-v{version}-{target}.zip" if is_windows else f"adi-v{version}-{target}.tar.gz"
        archive_path = DIST_DIR / archive_name

        if not archive_path.is_file():
            error(f"Archive not found: {archive_path}")

        assets.append(archive_path)
        info(f"Found: {archive_name} ({human_size(archive_path)})")

    print()
    return assets


def create_github_release(version: str, title: str, draft: bool, assets: list[Path]):
    """Create GitHub release with assets."""
    print(f"{BOLD}Step 3: Creating GitHub release...{NC}")

    tag = f"v{version}"

    # Check if release already exists
    result = run_cmd(["gh", "release", "view", tag, "-R", REPO], check=False)
    if result.returncode == 0:
        warn(f"Release {tag} already exists, deleting...")
        run_cmd(["gh", "release", "delete", tag, "-R", REPO, "--yes"])

    # Build release command
    cmd = ["gh", "release", "create", tag, "-R", REPO, "--title", title]
    if draft:
        cmd.append("--draft")
    cmd.extend(str(a) for a in assets)

    info(f"Creating release {tag}...")
    run_cmd(cmd)

    success("Release created!")
    print()

    # Summary
    print(f"{BOLD}=== Release Summary ==={NC}")
    print()
    print(f"  {GREEN}✓{NC} Version: {version}")
    print(f"  {GREEN}✓{NC} Tag: {tag}")
    print(f"  {GREEN}✓{NC} Assets:")
    for asset in assets:
        print(f"      - {asset.name}")
    print()
    print(f"  {CYAN}→{NC} https://github.com/{REPO}/releases/tag/{tag}")
    print()
    success(f"adi CLI v{version} released!")


def main():
    parser = argparse.ArgumentParser(description="Build adi CLI for multiple platforms and create a GitHub release.")
    parser.add_argument("--version", required=True, help="Version to release (e.g., 1.0.1)")
    parser.add_argument("--title", default="", help="Release title (default: 'adi v{version}')")
    parser.add_argument("--draft", action="store_true", help="Create as draft release")
    parser.add_argument("--skip-build", action="store_true", help="Skip building binaries")
    args = parser.parse_args()

    title = args.title or f"adi v{args.version}"

    print()
    print(f"{BOLD}=== ADI CLI Release ==={NC}")
    print()
    info(f"Version: {args.version}")
    info(f"Tag: v{args.version}")
    info(f"Title: {title}")
    info(f"Draft: {args.draft}")
    print()

    DIST_DIR.mkdir(parents=True, exist_ok=True)

    if not args.skip_build:
        build_binaries(args.version)
    else:
        warn("Skipping build (--skip-build)")
        print()

    assets = verify_archives(args.version)
    create_github_release(args.version, title, args.draft, assets)


if __name__ == "__main__":
    main()
