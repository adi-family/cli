#!/usr/bin/env python3
"""Cocoon Images - Build and release cocoon Docker image variants."""

import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

COCOON_DIR = PROJECT_ROOT / "crates" / "cocoon"
BUILD_SCRIPT = COCOON_DIR / "scripts" / "build-images.sh"

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


USAGE = f"""\
Usage: cocoon-images [OPTIONS]

Build and release cocoon Docker image variants.

OPTIONS:
    --all               Build all standard variants (default)
    --minimal           Build minimal variants (alpine, debian)
    --dev               Build dev variants (ubuntu, python, node)
    --variant NAME      Build specific variant
    --push              Push images to registry
    --tag TAG           Image tag (default: latest)
    --platform PLAT     Target platform (default: linux/amd64,linux/arm64)
    --no-cache          Build without cache
    --dry-run           Show what would be built
    -h, --help          Show this help

VARIANTS:
    alpine      Minimal (~15MB) - bash, curl, git, jq
    debian      Slim (~100MB) - build-essential, python3, vim
    ubuntu      Standard (~150MB) - nodejs, clang, cmake, sudo
    python      Python-focused (~180MB) - pip, poetry, uv, jupyter
    node        Node.js-focused (~200MB) - npm, yarn, pnpm, bun
    full        Everything (~500MB) - rust, go, docker, kubectl, terraform
    gpu         CUDA-enabled (~2GB) - cuda 12.4, cudnn, pytorch-ready
    custom      User-configurable

EXAMPLES:
    cocoon-images                              # Build all variants locally
    cocoon-images --push                       # Build all + push to registry
    cocoon-images --variant ubuntu --push      # Build only ubuntu + push
    cocoon-images --minimal --tag v0.2.1       # Build alpine+debian with tag
    cocoon-images --dev --platform linux/amd64 # Build dev variants for amd64 only
    cocoon-images --dry-run                    # Show what would be built
"""


def main():
    if not BUILD_SCRIPT.is_file():
        error(f"Build script not found: {BUILD_SCRIPT}")

    if not os.access(BUILD_SCRIPT, os.X_OK):
        error(f"Build script not executable: {BUILD_SCRIPT}")

    if len(sys.argv) < 2:
        print()
        print(f"{BOLD}Cocoon Docker Images{NC}")
        print()
        print("Run with --help for options, or use:")
        print()
        print(f"  {CYAN}adi workflow cocoon-images{NC}  # Interactive mode")
        print(f"  {CYAN}cocoon-images --all --push{NC}   # Build all + push")
        print()
        sys.exit(0)

    sys.exit(subprocess.run([str(BUILD_SCRIPT), *sys.argv[1:]]).returncode)


if __name__ == "__main__":
    main()
