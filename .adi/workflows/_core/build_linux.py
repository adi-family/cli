#!/usr/bin/env python3
"""Cross-compile Rust services for Linux (x86_64-unknown-linux-musl)."""

import argparse
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

TARGET = "x86_64-unknown-linux-musl"
MUSL_GCC = "x86_64-linux-musl-gcc"

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


def run_cmd(args: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if check and result.returncode != 0:
        error(f"Command failed: {' '.join(args)}\n{result.stderr}")
    return result


@dataclass
class ServiceConfig:
    crate_path: str
    binaries: list[str]
    features: str | None


# Service configurations: name -> crate-path:binary-names[:features]
SERVICE_CONFIGS: dict[str, ServiceConfig] = {
    "auth": ServiceConfig("crates/auth", ["auth-http", "auth-migrate"], None),
    "platform": ServiceConfig("crates/platform", ["platform-http"], None),
    "analytics": ServiceConfig("crates/analytics", ["analytics-http"], None),
    "analytics-ingestion": ServiceConfig("crates/analytics-ingestion", ["analytics-ingestion"], None),
    "signaling-server": ServiceConfig("crates/signaling-server", ["signaling-server"], None),
    "flowmap-api": ServiceConfig("apps/flowmap-api", ["flowmap-api"], None),
    "cocoon": ServiceConfig("plugins/adi/cocoon", ["cocoon"], "standalone"),
    "llm-proxy": ServiceConfig("plugins/adi/llm-proxy/http", ["llm-proxy", "llm-proxy-migrate"], None),
}

ALL_SERVICES = list(SERVICE_CONFIGS.keys())


def ensure_musl_target():
    """Ensure the musl cross-compilation target is installed."""
    result = run_cmd(["rustup", "target", "list", "--installed"], check=False)
    if TARGET not in result.stdout:
        warn(f"Installing {TARGET} target...")
        run_cmd(["rustup", "target", "add", TARGET])


def ensure_musl_gcc():
    """Ensure the musl-cross toolchain (x86_64-linux-musl-gcc) is available."""
    if not shutil.which(MUSL_GCC):
        error(f"musl-cross toolchain not found.\nInstall with: brew install filosottile/musl-cross/musl-cross")


def is_standalone_workspace(crate_dir: Path) -> bool:
    """Check if a crate is a standalone workspace (has its own [workspace] section)."""
    cargo_toml = crate_dir / "Cargo.toml"
    if cargo_toml.is_file():
        content = cargo_toml.read_text()
        if re.search(r"^\[workspace\]", content, re.MULTILINE):
            return True

    root_cargo = PROJECT_ROOT / "Cargo.toml"
    if root_cargo.is_file():
        root_content = root_cargo.read_text()
        # Check if crate is in the workspace exclude list
        exclude_match = re.search(r"exclude\s*=\s*\[([^\]]*)\]", root_content, re.DOTALL)
        if exclude_match:
            crate_path_str = str(crate_dir.relative_to(PROJECT_ROOT))
            if f'"{crate_path_str}"' in exclude_match.group(1):
                return True

    return False


def read_package_name(crate_dir: Path) -> str:
    """Read the package name from a crate's Cargo.toml."""
    cargo_toml = crate_dir / "Cargo.toml"
    content = cargo_toml.read_text()
    match = re.search(r'^name\s*=\s*"([^"]+)"', content, re.MULTILINE)
    if not match:
        error(f"Could not read package name from {cargo_toml}")
    return match.group(1)


def build_service(service: str):
    """Build a single service for the Linux musl target."""
    config = SERVICE_CONFIGS.get(service)
    if not config:
        error(f"Unknown service: {service}")

    info(f"Building {service} (linux/amd64)")

    crate_dir = PROJECT_ROOT / config.crate_path
    standalone = is_standalone_workspace(crate_dir)

    env = os.environ.copy()
    env["CC_x86_64_unknown_linux_musl"] = MUSL_GCC
    env["CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER"] = MUSL_GCC

    for binary in config.binaries:
        features_note = f" (features: {config.features})" if config.features else ""
        info(f"  - {binary}{features_note}")

        cmd = ["cargo", "build", "--release", "--target", TARGET, "--bin", binary]
        if config.features:
            cmd.append(f"--features={config.features}")

        if standalone:
            subprocess.run(cmd, cwd=crate_dir, env=env, check=True)
        else:
            package_name = read_package_name(crate_dir)
            cmd.extend(["-p", package_name])
            subprocess.run(cmd, cwd=PROJECT_ROOT, env=env, check=True)

    # Copy binaries to release dir
    release_dir = PROJECT_ROOT / "release" / "adi.the-ihor.com" / service
    if release_dir.is_dir():
        for binary in config.binaries:
            if standalone:
                src = crate_dir / "target" / TARGET / "release" / binary
            else:
                src = PROJECT_ROOT / "target" / TARGET / "release" / binary
            shutil.copy2(src, release_dir / binary)
            info(f"  -> Copied {binary} to {release_dir}/")


def main():
    parser = argparse.ArgumentParser(description="Cross-compile Rust services for Linux (x86_64-unknown-linux-musl).")
    parser.add_argument("services", nargs="*", help=f"Services to build (default: all). Available: {', '.join(ALL_SERVICES)}")
    args = parser.parse_args()

    services = args.services or ALL_SERVICES

    # Validate service names upfront
    for service in services:
        if service not in SERVICE_CONFIGS:
            error(f"Unknown service: {service}\nAvailable: {', '.join(ALL_SERVICES)}")

    ensure_musl_target()
    ensure_musl_gcc()

    for service in services:
        build_service(service)

    success("Build complete")


if __name__ == "__main__":
    main()
