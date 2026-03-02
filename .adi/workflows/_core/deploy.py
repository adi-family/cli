#!/usr/bin/env python3
"""ADI Coolify Deployment Helper - deploy, monitor, and manage service deployments."""

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
BOLD = "\033[1m"
DIM = "\033[2m"
NC = "\033[0m"

# Service registry: key -> (uuid, display_name)
SERVICES = {
    "auth":                ("ngg488ogoc80c8wogowkckow", "Auth API"),
    "platform":            ("cosw4cw0gscso88w8sskgk8g", "Platform API"),
    "signaling":           ("t0k0owcw00w00s4w4o0c000w", "Signaling Server"),
    "web":                 ("tkg84kg0o0ok8gkcs8wcggck", "Web UI"),
    "analytics-ingestion": ("TODO_COOLIFY_UUID",         "Analytics Ingestion"),
    "analytics":           ("TODO_COOLIFY_UUID",         "Analytics API"),
}

ALL_SERVICE_KEYS = list(SERVICES.keys())


def info(msg: str):
    print(f"{CYAN}info{NC} {msg}")


def success(msg: str):
    print(f"{GREEN}done{NC} {msg}")


def warn(msg: str):
    print(f"{YELLOW}warn{NC} {msg}")


def error(msg: str):
    print(f"{RED}error{NC} {msg}", file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------

def load_env_local():
    """Load .env.local from PROJECT_ROOT if it exists."""
    env_file = PROJECT_ROOT / ".env.local"
    if not env_file.is_file():
        return

    for line in env_file.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue

        key, _, value = line.partition("=")
        if not key.isidentifier():
            continue

        # Strip surrounding quotes
        for q in ('"', "'"):
            if value.startswith(q) and value.endswith(q):
                value = value[1:-1]
                break

        os.environ.setdefault(key, value)


def get_api_key() -> str:
    key = os.environ.get("COOLIFY_API_KEY", "")
    if not key:
        error("Environment variable COOLIFY_API_KEY not set")
    return key


def get_api_base() -> str:
    url = os.environ.get("COOLIFY_URL", "http://in.the-ihor.com")
    return f"{url}/api/v1"


# ---------------------------------------------------------------------------
# Service helpers
# ---------------------------------------------------------------------------

def resolve_service(name: str) -> tuple[str, str, str]:
    """Return (key, uuid, display_name) or exit with error."""
    entry = SERVICES.get(name)
    if entry is None:
        error(f"Unknown service '{name}'. Available: {', '.join(ALL_SERVICE_KEYS)}")
    uuid, display = entry
    return name, uuid, display


def status_color(status: str) -> str:
    if status in ("running:healthy", "running", "finished", "success"):
        return GREEN
    if status in ("running:unhealthy", "running:unknown", "queued", "in_progress", "building"):
        return YELLOW
    if status.startswith("exited") or status in ("failed", "error", "cancelled", "stopped"):
        return RED
    return NC


def status_icon(status: str) -> str:
    if status in ("running:healthy", "running", "finished", "success"):
        return "\u25cf"   # ●
    if status in ("running:unhealthy", "running:unknown", "in_progress", "building"):
        return "\u25d0"   # ◐
    if status == "queued":
        return "\u25cb"   # ○
    if status.startswith("exited") or status in ("failed", "error", "cancelled", "stopped"):
        return "\u2717"   # ✗
    return "?"


def is_terminal_status(status: str) -> bool:
    return status in ("finished", "failed", "error", "cancelled", "success")


# ---------------------------------------------------------------------------
# API
# ---------------------------------------------------------------------------

def api_call(method: str, endpoint: str, data: str | None = None) -> dict | list | str:
    """Make an authenticated API call and return parsed JSON."""
    api_key = get_api_key()
    url = f"{get_api_base()}{endpoint}"

    body = data.encode() if data else None
    req = urllib.request.Request(url, data=body, method=method, headers={
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    })

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        try:
            body_text = exc.read().decode()
            return json.loads(body_text)
        except Exception:
            return {"error": f"HTTP {exc.code}"}
    except urllib.error.URLError as exc:
        return {"error": str(exc.reason)}
    except json.JSONDecodeError:
        return {}


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

def cmd_status(_args: argparse.Namespace):
    """Show deployment status for all services."""
    coolify_url = os.environ.get("COOLIFY_URL", "http://in.the-ihor.com")
    print(f"{BOLD}ADI Deployment Status{NC}")
    print(f"{DIM}Coolify: {coolify_url}{NC}")
    print()

    print(f"{'SERVICE':<12} {'NAME':<20} {'STATUS':<20}")
    print("\u2500" * 56)

    for key in ALL_SERVICE_KEYS:
        uuid, display = SERVICES[key]
        app_info = api_call("GET", f"/applications/{uuid}")
        status = "unknown"
        if isinstance(app_info, dict):
            status = app_info.get("status", "unknown")

        color = status_color(status)
        icon = status_icon(status)
        print(f"{key:<12} {display:<20} {color}{icon} {status}{NC}")


def cmd_deploy(args: argparse.Namespace):
    """Deploy one or all services, then watch progress."""
    service = args.service
    force = args.force

    if service == "all":
        services_to_deploy = list(ALL_SERVICE_KEYS)
    else:
        resolve_service(service)
        services_to_deploy = [service]

    force_param = "&force=true" if force else ""

    print(f"{BOLD}Deploying services...{NC}")
    print()

    deployment_items: list[tuple[str, str]] = []

    for svc in services_to_deploy:
        uuid, display = SERVICES[svc]
        print(f"  {CYAN}{display}{NC}: Triggering deploy... ", end="", flush=True)

        result = api_call("GET", f"/deploy?uuid={uuid}{force_param}")
        deploy_uuid = ""
        if isinstance(result, dict):
            deployments = result.get("deployments", [])
            if deployments:
                deploy_uuid = deployments[0].get("deployment_uuid", "")

        if deploy_uuid:
            print(f"{GREEN}Started{NC} ({deploy_uuid})")
            deployment_items.append((svc, deploy_uuid))
        else:
            msg = "Unknown error"
            if isinstance(result, dict):
                msg = result.get("message", result.get("error", msg))
            print(f"{RED}Failed{NC}: {msg}")

    print()

    if deployment_items:
        print(f"{BOLD}Watching deployment progress...{NC}")
        print(f"{DIM}Press Ctrl+C to stop watching{NC}")
        print()
        watch_deployments(deployment_items)


def watch_deployments(items: list[tuple[str, str]]):
    """Poll deployment status until all reach a terminal state."""
    # Print initial blank lines so cursor-up works on first iteration
    for _ in items:
        print()

    try:
        while True:
            all_done = True
            lines: list[str] = []

            for svc, deploy_uuid in items:
                _, display = SERVICES[svc][:2]
                deploy_info = api_call("GET", f"/deployments/{deploy_uuid}")
                status = "unknown"
                if isinstance(deploy_info, dict):
                    status = deploy_info.get("status", "unknown")

                color = status_color(status)
                icon = status_icon(status)
                lines.append(f"  {display}: {color}{icon} {status}{NC}")

                if not is_terminal_status(status):
                    all_done = False

            # Move cursor up and redraw
            sys.stdout.write(f"\033[{len(items)}A\033[J")
            for ln in lines:
                print(ln)

            if all_done:
                break

            time.sleep(2)
    except KeyboardInterrupt:
        print()

    print(f"{GREEN}All deployments completed!{NC}")


def cmd_watch(args: argparse.Namespace):
    """Watch the latest deployment for a service."""
    _, uuid, display = resolve_service(args.service)

    print(f"{BOLD}Watching {display} deployments...{NC}")
    print(f"{DIM}Press Ctrl+C to stop{NC}")
    print()

    try:
        while True:
            deployments = api_call("GET", f"/applications/{uuid}/deployments?take=1")

            status = "none"
            commit = "none"
            if isinstance(deployments, list) and deployments:
                status = deployments[0].get("status", "none")
                commit = (deployments[0].get("commit", "none") or "none")[:7]

            color = status_color(status)
            icon = status_icon(status)
            sys.stdout.write(f"\r  {color}{icon} {status:<15}{NC} commit: {commit}   ")
            sys.stdout.flush()

            if is_terminal_status(status):
                print()
                break

            time.sleep(2)
    except KeyboardInterrupt:
        print()


def cmd_logs(args: argparse.Namespace):
    """Show logs for the most recent deployment of a service."""
    _, uuid, display = resolve_service(args.service)

    deployments = api_call("GET", f"/applications/{uuid}/deployments?take=1")
    deploy_uuid = ""
    if isinstance(deployments, list) and deployments:
        deploy_uuid = deployments[0].get("deployment_uuid", "")

    if not deploy_uuid:
        error(f"No deployments found for {display}")

    print(f"{BOLD}Deployment logs for {display}{NC}")
    print(f"{DIM}Deployment: {deploy_uuid}{NC}")
    print()

    deploy_info = api_call("GET", f"/deployments/{deploy_uuid}")
    logs = "No logs available"
    if isinstance(deploy_info, dict):
        logs = deploy_info.get("logs", logs) or logs
    print(logs)


def cmd_list(args: argparse.Namespace):
    """List recent deployments for a service."""
    _, uuid, display = resolve_service(args.service)
    take = args.count

    print(f"{BOLD}Recent deployments for {display}{NC}")
    print()

    deployments = api_call("GET", f"/applications/{uuid}/deployments?take={take}")

    print(f"{'STATUS':<12} {'COMMIT':<15} {'CREATED'}")
    print("\u2500" * 48)

    if not isinstance(deployments, list):
        return

    for dep in deployments:
        status = dep.get("status", "unknown")
        commit = (dep.get("commit", "") or "")[:7]
        created = dep.get("created_at", "")

        color = status_color(status)
        icon = status_icon(status)

        # Trim fractional seconds for display
        if created and created != "null":
            created = created.split(".")[0].replace("T", " ")

        print(f"{color}{icon} {status:<10}{NC} {commit:<15} {created}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    load_env_local()

    parser = argparse.ArgumentParser(
        description="ADI Coolify Deployment Helper",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=f"""\
services:
  {', '.join(ALL_SERVICE_KEYS)}

environment:
  COOLIFY_URL       Coolify instance URL (default: http://in.the-ihor.com)
  COOLIFY_API_KEY   API token (required)

examples:
  deploy.py status                  Show all services
  deploy.py deploy web              Deploy web UI
  deploy.py deploy all              Deploy everything
  deploy.py deploy auth --force     Force rebuild auth
  deploy.py list platform           Recent deployments
""",
    )
    sub = parser.add_subparsers(dest="command")

    # status
    sub.add_parser("status", help="Show status of all services")

    # deploy
    p_deploy = sub.add_parser("deploy", help="Deploy a service (or 'all')")
    p_deploy.add_argument("service", help="Service key or 'all'")
    p_deploy.add_argument("--force", "-f", action="store_true", help="Force rebuild (no cache)")

    # watch
    p_watch = sub.add_parser("watch", help="Watch deployment progress")
    p_watch.add_argument("service", help="Service key")

    # logs
    p_logs = sub.add_parser("logs", help="Show deployment logs")
    p_logs.add_argument("service", help="Service key")

    # list
    p_list = sub.add_parser("list", help="List recent deployments")
    p_list.add_argument("service", help="Service key")
    p_list.add_argument("count", nargs="?", type=int, default=5, help="Number of deployments (default: 5)")

    args = parser.parse_args()

    commands = {
        "status": cmd_status,
        "deploy": cmd_deploy,
        "watch":  cmd_watch,
        "logs":   cmd_logs,
        "list":   cmd_list,
    }

    handler = commands.get(args.command)
    if handler is None:
        parser.print_help()
        sys.exit(0)

    handler(args)


if __name__ == "__main__":
    main()
