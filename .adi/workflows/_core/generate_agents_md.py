#!/usr/bin/env python3
"""Generate AGENTS.md and CLAUDE.md with crate structure documentation."""

import os
import re
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

CRATES_DIR = PROJECT_ROOT / "crates"
WORKFLOWS_DIR = PROJECT_ROOT / ".adi" / "workflows"
CODE_STYLE_DIR = PROJECT_ROOT / "docs" / "code-style"

KNOWN_SUBDIRS = ("core", "http", "plugin", "cli", "mcp")


def parse_toml_field(path: Path, field: str) -> str:
    """Extract first occurrence of a top-level TOML field value."""
    if not path.is_file():
        return ""
    for line in path.read_text().splitlines():
        match = re.match(rf'^{field}\s*=\s*"(.*)"', line)
        if match:
            return match.group(1)
    return ""


def get_crate_subdirs(crate: str) -> str:
    """Return comma-separated list of known sub-crates (core, http, plugin, cli, mcp)."""
    crate_dir = CRATES_DIR / crate
    found = [d for d in KNOWN_SUBDIRS if (crate_dir / d).is_dir()]
    return ",".join(found)


def get_crate_description(crate: str) -> str:
    """Get description from the first sub-crate Cargo.toml that has one."""
    for sub in ("core", "plugin", "http", "mcp"):
        desc = parse_toml_field(CRATES_DIR / crate / sub / "Cargo.toml", "description")
        if desc:
            return desc
    return ""


def find_user_facing_crates() -> list[str]:
    """Crates with a plugin/ subdir containing Cargo.toml."""
    return sorted(
        d.name
        for d in CRATES_DIR.iterdir()
        if d.is_dir() and (d / "plugin" / "Cargo.toml").is_file()
    )


def find_backend_crates(user_facing: list[str]) -> list[str]:
    """Crates with http/ but no plugin/ subdir."""
    user_facing_set = set(user_facing)
    return sorted(
        d.name
        for d in CRATES_DIR.iterdir()
        if d.is_dir()
        and (d / "http" / "Cargo.toml").is_file()
        and d.name not in user_facing_set
    )


def find_libraries() -> list[str]:
    """List directories in crates/lib/."""
    lib_dir = CRATES_DIR / "lib"
    if not lib_dir.is_dir():
        return []
    return sorted(d.name for d in lib_dir.iterdir() if d.is_dir())


def find_standalone_plugins() -> list[str]:
    """Crates matching *-plugin pattern."""
    return sorted(
        d.name
        for d in CRATES_DIR.iterdir()
        if d.is_dir()
        and d.name.endswith("-plugin")
        and (d / "Cargo.toml").is_file()
    )


def find_tools() -> list[str]:
    """Crates matching tool-*, cocoon, or webrtc* patterns."""
    return sorted(
        d.name
        for d in CRATES_DIR.iterdir()
        if d.is_dir()
        and (d / "Cargo.toml").is_file()
        and (d.name.startswith("tool-") or d.name == "cocoon" or d.name.startswith("webrtc"))
    )


def find_workflows() -> list[tuple[str, str]]:
    """Discover workflows from .adi/workflows/*.toml, returning (name, description) pairs."""
    results = []
    for toml_path in sorted(WORKFLOWS_DIR.glob("*.toml")):
        name = parse_toml_field(toml_path, "name")
        if not name:
            continue
        desc = parse_toml_field(toml_path, "description")
        results.append((name, desc))
    return results


def collect_inline_docs() -> str:
    """Read and concatenate all *.inline.md files from docs/code-style/."""
    parts: list[str] = []
    if not CODE_STYLE_DIR.is_dir():
        return ""
    for doc in sorted(CODE_STYLE_DIR.glob("*.inline.md")):
        parts.append("")
        parts.append(doc.read_text().rstrip())
        parts.append("")
    return "\n".join(parts)


def collect_reference_docs() -> str:
    """List non-inline code-style docs as reference links, skipping those with inline versions."""
    if not CODE_STYLE_DIR.is_dir():
        return ""

    entries: list[str] = []
    for doc in sorted(CODE_STYLE_DIR.glob("*.md")):
        if doc.name.endswith(".inline.md"):
            continue
        stem = doc.stem
        inline_version = CODE_STYLE_DIR / f"{stem}.inline.md"
        if inline_version.is_file():
            continue
        # Extract first bold text as summary
        summary = "-"
        for line in doc.read_text().splitlines():
            match = re.search(r"\*\*(.+?)\*\*", line)
            if match:
                summary = match.group(1)[:60]
                break
        rel_path = doc.relative_to(PROJECT_ROOT)
        entries.append(f"- [`{stem}`]({rel_path}): {summary}")

    if not entries:
        return ""

    return "\n**Additional guidelines:**\n" + "\n".join(entries)


def build_table_rows(crates: list[str], *, with_subdirs: bool) -> str:
    """Build markdown table rows for crate lists."""
    lines: list[str] = []
    for crate in crates:
        desc = get_crate_description(crate) if with_subdirs else parse_toml_field(CRATES_DIR / crate / "Cargo.toml", "description")
        if with_subdirs:
            subs = get_crate_subdirs(crate)
            lines.append(f"| `{crate}` | {subs} | {desc} |")
        else:
            lines.append(f"| `{crate}` | {desc or '-'} |")
    return "\n".join(lines)


def generate() -> str:
    """Generate the full markdown content."""
    user_facing = find_user_facing_crates()
    backend = find_backend_crates(user_facing)
    libraries = find_libraries()
    plugins = find_standalone_plugins()
    tools = find_tools()
    workflows = find_workflows()

    sections: list[str] = []

    # Header
    sections.append(
        "# ADI Crate Structure\n"
        "\n"
        "> Auto-generate with: `adi wf generate-agents-md`"
    )

    # User-facing
    sections.append(
        "## User-Facing Components\n"
        "Components with plugin for `adi` CLI integration.\n"
        "\n"
        "| Crate | Structure | Description |\n"
        "|-------|-----------|-------------|\n"
        + build_table_rows(user_facing, with_subdirs=True)
    )

    # Backend
    sections.append(
        "## Backend Services\n"
        "HTTP services without CLI plugin.\n"
        "\n"
        "| Crate | Structure | Description |\n"
        "|-------|-----------|-------------|\n"
        + build_table_rows(backend, with_subdirs=True)
    )

    # Libraries
    lib_rows = "\n".join(
        f"| `{lib}` | {parse_toml_field(CRATES_DIR / 'lib' / lib / 'Cargo.toml', 'description') or '-'} |"
        for lib in libraries
    )
    sections.append(
        "## Libraries\n"
        "Shared libraries in `crates/lib/`.\n"
        "\n"
        "| Library | Purpose |\n"
        "|---------|---------|\n"
        + lib_rows
    )

    # Standalone plugins
    sections.append(
        "## Standalone Plugins\n"
        "\n"
        "| Plugin | Description |\n"
        "|--------|-------------|\n"
        + build_table_rows(plugins, with_subdirs=False)
    )

    # Tools
    sections.append(
        "## Tools\n"
        "\n"
        "| Tool | Description |\n"
        "|------|-------------|\n"
        + build_table_rows(tools, with_subdirs=False)
    )

    # Workflows
    wf_rows = "\n".join(f"| `{name}` | {desc} |" for name, desc in workflows)
    sections.append(
        "## Workflows\n"
        "Available workflows in `.adi/workflows/`. Run with `adi wf <name>` or directly via `.adi/workflows/<name>.sh`.\n"
        "\n"
        "| Workflow | Description |\n"
        "|----------|-------------|\n"
        + wf_rows
    )

    # Code style
    sections.append("## Code Style Guidelines")

    inline_content = collect_inline_docs()
    if inline_content:
        sections.append(inline_content)

    ref_content = collect_reference_docs()
    if ref_content:
        sections.append(ref_content)

    sections.append("")
    return "\n\n".join(sections)


def main():
    content = generate()

    agents_path = PROJECT_ROOT / "AGENTS.md"
    claude_path = PROJECT_ROOT / "CLAUDE.md"

    agents_path.write_text(content)
    claude_path.write_text(content)

    print(f"Generated {agents_path} and {claude_path}")


if __name__ == "__main__":
    main()
