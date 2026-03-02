#!/usr/bin/env python3
"""Reset ADI installation - remove local data for clean reinstall."""

import argparse
import os
import platform as plat
import shutil
import signal
import subprocess
import sys
from pathlib import Path

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
DIM = "\033[2m"
NC = "\033[0m"

VALID_SCOPES = ("all", "plugins", "cache", "config", "hive")


def info(msg: str):
    print(f"{CYAN}info{NC} {msg}")


def success(msg: str):
    print(f"{GREEN}done{NC} {msg}")


def warn(msg: str):
    print(f"{YELLOW}warn{NC} {msg}")


def error(msg: str):
    print(f"{RED}error{NC} {msg}", file=sys.stderr)
    sys.exit(1)


# Platform-specific paths
def get_data_dir() -> Path:
    if plat.system() == "Darwin":
        return Path.home() / "Library" / "Application Support"
    return Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))


def get_cache_dir() -> Path:
    if plat.system() == "Darwin":
        return Path.home() / "Library" / "Caches"
    return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))


def get_config_dir() -> Path:
    default = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")) / "adi"
    return Path(os.environ.get("ADI_CONFIG_DIR", default))


DATA_DIR = get_data_dir()
CACHE_DIR = get_cache_dir()
CONFIG_DIR = get_config_dir()
HOME = Path.home()
IS_DARWIN = plat.system() == "Darwin"
IS_LINUX = plat.system() == "Linux"

removed: list[str] = []
skipped: list[str] = []


def safe_rm(path: Path, label: str):
    """Remove a path, tracking what was removed vs skipped."""
    if path.exists() or path.is_symlink():
        shutil.rmtree(path, ignore_errors=True) if path.is_dir() else path.unlink(missing_ok=True)
        removed.append(f"{label} ({path})")
    else:
        skipped.append(label)


def stop_services():
    """Stop running ADI services before cleanup."""
    info("Stopping running services...")

    # Stop hive daemon
    pid_file = HOME / ".adi" / "hive" / "hive.pid"
    if pid_file.is_file():
        try:
            pid = int(pid_file.read_text().strip())
            os.kill(pid, signal.SIGTERM)
            success(f"Stopped hive daemon (PID {pid})")
        except (ValueError, OSError):
            pass

    # Unload cocoon launchd agent (macOS)
    if IS_DARWIN:
        plist = HOME / "Library" / "LaunchAgents" / "com.adi.cocoon.plist"
        if plist.is_file():
            subprocess.run(["launchctl", "unload", str(plist)], capture_output=True)
            success("Unloaded cocoon launchd agent")

    # Stop cocoon systemd service (Linux)
    if IS_LINUX:
        result = subprocess.run(
            ["systemctl", "--user", "is-active", "cocoon.service"],
            capture_output=True, text=True,
        )
        if result.returncode == 0:
            subprocess.run(["systemctl", "--user", "stop", "cocoon.service"], capture_output=True)
            subprocess.run(["systemctl", "--user", "disable", "cocoon.service"], capture_output=True)
            success("Stopped cocoon systemd service")


def reset_binaries():
    info("Removing binaries...")
    safe_rm(HOME / ".local" / "bin" / "adi", "adi binary")
    safe_rm(HOME / ".local" / "bin" / "cocoon", "cocoon binary")


def reset_plugins():
    info("Removing plugins and plugin data...")

    adi_data = DATA_DIR / "adi"
    safe_rm(adi_data / "plugins", "plugins directory")
    safe_rm(adi_data / "tools", "tools directory")
    safe_rm(adi_data / "tools.db", "tools database")
    safe_rm(adi_data / "tasks", "tasks directory")
    safe_rm(adi_data / "knowledgebase", "knowledgebase directory")

    # Plugin data directories
    top_level = {"plugins", "tools", "tasks", "knowledgebase"}
    if adi_data.is_dir():
        for d in adi_data.iterdir():
            if d.is_dir() and d.name not in top_level:
                safe_rm(d, f"plugin data: {d.name}")

    # com.adi.adi data (models, embeddings)
    if IS_DARWIN:
        safe_rm(DATA_DIR / "com.adi.adi", "models + embeddings (com.adi.adi)")

    # Legacy location
    safe_rm(HOME / ".adi" / "plugins", "~/.adi/plugins")


def reset_cache():
    info("Removing caches...")
    safe_rm(CACHE_DIR / "adi", "adi cache")
    if IS_DARWIN:
        safe_rm(CACHE_DIR / "com.adi.adi", "com.adi.adi cache")
    safe_rm(HOME / ".adi" / "cache", "~/.adi/cache")

    # Temp files
    import glob as g
    for p in g.glob("/tmp/adi-hive-*.conf"):
        Path(p).unlink(missing_ok=True)
    tmpdir = Path(os.environ.get("TMPDIR", "/tmp"))
    shutil.rmtree(tmpdir / "adi-update", ignore_errors=True)


def reset_config():
    info("Removing configuration...")
    safe_rm(CONFIG_DIR, "adi config")
    safe_rm(HOME / ".config" / "cocoon", "cocoon config")
    safe_rm(HOME / ".adi" / "daemon.config.json", "daemon config")
    safe_rm(HOME / ".adi" / "daemon.pid", "daemon pid")
    safe_rm(HOME / ".adi" / "config.toml", "global config")


def reset_hive():
    info("Removing hive state...")
    safe_rm(HOME / ".adi" / "hive", "hive directory")
    safe_rm(HOME / ".adi" / "workflows", "global workflows")
    safe_rm(HOME / ".adi" / "tree", "tree index")


def reset_completions():
    info("Removing shell completions...")
    safe_rm(HOME / ".zfunc" / "_adi", "zsh completions")
    safe_rm(HOME / ".local" / "share" / "bash-completion" / "completions" / "adi.bash", "bash completions (XDG)")
    safe_rm(HOME / ".bash_completion.d" / "adi.bash", "bash completions (fallback)")
    safe_rm(HOME / ".config" / "fish" / "completions" / "adi.fish", "fish completions")
    safe_rm(HOME / ".elvish" / "lib" / "adi.elv", "elvish completions")


def reset_service_files():
    info("Removing service files...")
    if IS_DARWIN:
        safe_rm(HOME / "Library" / "LaunchAgents" / "com.adi.cocoon.plist", "cocoon launchd plist")
    else:
        safe_rm(HOME / ".config" / "systemd" / "user" / "cocoon.service", "cocoon systemd service")


def cleanup_empty_dirs():
    adi_data = DATA_DIR / "adi"
    if adi_data.is_dir():
        try:
            adi_data.rmdir()
        except OSError:
            pass

    dot_adi = HOME / ".adi"
    if dot_adi.is_dir():
        ds_store = dot_adi / ".DS_Store"
        ds_store.unlink(missing_ok=True)
        try:
            dot_adi.rmdir()
        except OSError:
            pass


def print_summary():
    print()
    if removed:
        success(f"Removed {len(removed)} item(s):")
        for item in removed:
            print(f"  {GREEN}✓{NC} {item}")

    if skipped:
        print()
        info(f"Already clean ({len(skipped)} item(s) not found):")
        for item in skipped:
            print(f"  {DIM}· {item}{NC}")

    print()
    if removed:
        success("ADI reset complete. Ready for fresh install.")
    else:
        info("Nothing to remove — ADI is already clean.")


def main():
    parser = argparse.ArgumentParser(description="Reset ADI installation by removing local data.")
    parser.add_argument("--scope", default="all", choices=VALID_SCOPES, help="What to reset (default: all)")
    parser.add_argument("--yes", "-y", action="store_true", help="Skip confirmation prompt")
    args = parser.parse_args()

    print()
    info(f"ADI Reset (scope: {args.scope})")
    print()

    if not args.yes and not os.environ.get("_ADI_PRELUDE_LOADED"):
        answer = input(f"{YELLOW}warn{NC} This will permanently delete ADI data (scope: {args.scope}). Continue? [y/N] ")
        if answer.lower() != "y":
            warn("Reset cancelled")
            sys.exit(0)

    stop_services()

    scope_actions = {
        "all": [reset_binaries, reset_plugins, reset_cache, reset_config, reset_hive, reset_completions, reset_service_files],
        "plugins": [reset_plugins],
        "cache": [reset_cache],
        "config": [reset_config],
        "hive": [reset_hive],
    }

    for action in scope_actions[args.scope]:
        action()

    cleanup_empty_dirs()
    print_summary()


if __name__ == "__main__":
    main()
