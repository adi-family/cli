#!/usr/bin/env python3
"""Seal - Commit and push all changes including submodules."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

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


def check_command(cmd: str) -> bool:
    from shutil import which
    return which(cmd) is not None


def run_git(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)


def has_changes(cwd: Path) -> bool:
    """Check if directory has uncommitted changes or untracked files."""
    staged = run_git("diff", "--quiet", cwd=cwd)
    unstaged = run_git("diff", "--cached", "--quiet", cwd=cwd)
    untracked = run_git("ls-files", "--others", "--exclude-standard", cwd=cwd)
    return staged.returncode != 0 or unstaged.returncode != 0 or bool(untracked.stdout.strip())


def commits_ahead(cwd: Path) -> int:
    """Count commits ahead of upstream."""
    result = run_git("rev-list", "--count", "@{u}..HEAD", cwd=cwd)
    if result.returncode != 0:
        return 0
    return int(result.stdout.strip())


def generate_commit_message(git_status: str, git_diff: str, context: str) -> str:
    """Generate commit message using Claude CLI."""
    prompt = f"""Generate a git commit message for these changes.

Context: {context}

Git status:
{git_status}

Git diff (truncated):
{git_diff[:8000]}

RULES:
- Use conventional commit format: <type>: <description>
- Types: feat, fix, refactor, docs, chore, perf, test, style
- Start with emoji matching type (feat=✨, fix=🐛, refactor=♻️, docs=📚, chore=🔧, perf=⚡, test=🧪, style=💄)
- Keep under 72 chars
- Be specific about what changed
- Use imperative mood ("Add" not "Added")

Respond with ONLY the commit message, nothing else. Single line."""

    try:
        result = subprocess.run(
            ["claude", "-p", prompt],
            capture_output=True, text=True, timeout=60,
        )
        if result.returncode == 0 and result.stdout.strip():
            msg = result.stdout.strip().strip('"')
            return msg.splitlines()[0]
    except (subprocess.TimeoutExpired, OSError):
        pass

    return "chore: update files"


def commit_submodule(submodule_path: str) -> bool:
    """Commit changes in a submodule. Returns True if committed."""
    full_path = PROJECT_ROOT / submodule_path

    if not has_changes(full_path):
        return False

    info(f"Committing changes in {submodule_path}...")

    git_status = run_git("status", "--short", cwd=full_path).stdout
    git_diff = run_git("diff", "HEAD", cwd=full_path).stdout or run_git("diff", cwd=full_path).stdout

    commit_message = generate_commit_message(git_status, git_diff, f"Submodule: {submodule_path}")

    run_git("add", "-A", cwd=full_path)
    run_git("commit", "-m", commit_message, cwd=full_path)

    short_hash = run_git("rev-parse", "--short", "HEAD", cwd=full_path).stdout.strip()
    success(f"Committed {submodule_path}: {commit_message} ({short_hash})")
    return True


def get_submodules() -> list[str]:
    """Get list of submodule paths."""
    result = run_git("submodule", "status", cwd=PROJECT_ROOT)
    if result.returncode != 0:
        return []

    paths = []
    for line in result.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 2:
            paths.append(parts[1])
    return paths


def main():
    parser = argparse.ArgumentParser(description="Seal - Commit and push all changes including submodules.")
    parser.add_argument("--message", default="", help="Custom commit message for parent repo (AI-generated if empty)")
    parser.add_argument("--push", default="yes", choices=["yes", "no"], help="Push after commit (default: yes)")
    args = parser.parse_args()

    if not check_command("claude"):
        error("claude CLI not found. Install Claude CLI.")

    print()
    info("🔒 Sealing all changes...")
    print()

    # Step 1: Find and commit submodules with changes
    info("Step 1: Checking submodules for uncommitted changes...")

    submodules_committed = []
    submodules_to_push = []

    for submodule_path in get_submodules():
        full_path = PROJECT_ROOT / submodule_path
        git_dir = full_path / ".git"

        if not (git_dir.is_dir() or git_dir.is_file()):
            continue

        if has_changes(full_path):
            if commit_submodule(submodule_path):
                submodules_committed.append(submodule_path)
                submodules_to_push.append(submodule_path)
        elif commits_ahead(full_path) > 0:
            submodules_to_push.append(submodule_path)

    if not submodules_committed:
        info("No submodules with uncommitted changes")
    else:
        success(f"Committed {len(submodules_committed)} submodule(s)")
    print()

    # Step 2: Commit parent repo changes
    info("Step 2: Committing parent repo changes...")

    if has_changes(PROJECT_ROOT):
        git_status = run_git("status", "--short", cwd=PROJECT_ROOT).stdout
        git_diff = run_git("diff", "HEAD", cwd=PROJECT_ROOT).stdout or run_git("diff", cwd=PROJECT_ROOT).stdout

        commit_message = args.message or generate_commit_message(
            git_status, git_diff, "Parent repo with submodule updates",
        )

        run_git("add", "-A", cwd=PROJECT_ROOT)
        run_git("commit", "-m", commit_message, cwd=PROJECT_ROOT)

        short_hash = run_git("rev-parse", "--short", "HEAD", cwd=PROJECT_ROOT).stdout.strip()
        success(f"Committed parent repo: {commit_message} ({short_hash})")
    else:
        info("No changes in parent repo")
    print()

    # Step 3: Push everything
    if args.push == "yes":
        info("Step 3: Pushing all changes...")

        for submodule in submodules_to_push:
            info(f"Pushing {submodule}...")
            result = run_git("push", cwd=PROJECT_ROOT / submodule)
            if result.returncode == 0:
                success(f"Pushed {submodule}")
            else:
                warn(f"Failed to push {submodule} (may need upstream set)")

        ahead = commits_ahead(PROJECT_ROOT)
        if ahead > 0:
            info("Pushing parent repo...")
            result = run_git("push", cwd=PROJECT_ROOT)
            if result.returncode == 0:
                success("Pushed parent repo")
            else:
                warn("Failed to push parent repo")
        else:
            info("Parent repo already up to date with remote")

        print()
        success("🔒 Sealed and pushed!")
    else:
        print()
        success("🔒 Sealed! (push skipped)")
        print()
        info("To push manually:")
        for submodule in submodules_to_push:
            print(f"  cd {submodule} && git push")
        print("  git push")


if __name__ == "__main__":
    main()
