#!/usr/bin/env python3
"""Generate API documentation for Rust crates with LLM enrichment and translations."""

import argparse
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

# When run via `adi workflow`, these env vars are injected.
# When run directly, derive from script location.
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))
DOCS_DIR = PROJECT_ROOT / ".adi" / "docs"
TEMP_DIR = PROJECT_ROOT / ".adi" / "tmp" / "autodoc"

# ANSI colors
CYAN = "\033[0;36m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
NC = "\033[0m"

LANG_NAMES = {
    "en": "English",
    "uk": "Ukrainian",
    "ru": "Russian",
    "zh": "Chinese",
    "ja": "Japanese",
    "ko": "Korean",
    "es": "Spanish",
    "de": "German",
    "fr": "French",
}


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
    """Check if a command is available on PATH."""
    from shutil import which
    return which(cmd) is not None


def find_crate_dir(name: str) -> Path | None:
    """Find crate directory by package name in Cargo.toml."""
    crates_dir = PROJECT_ROOT / "crates"
    if not crates_dir.is_dir():
        return None

    for cargo_toml in crates_dir.rglob("Cargo.toml"):
        try:
            text = cargo_toml.read_text()
            for line in text.splitlines():
                if line.startswith("name = "):
                    crate_name = line.split('"')[1]
                    if crate_name == name:
                        return cargo_toml.parent
                    break
        except (IndexError, OSError):
            continue
    return None


def run_cmd(args: list[str], cwd: Path | None = None, capture: bool = True) -> subprocess.CompletedProcess:
    """Run a command, returning CompletedProcess."""
    return subprocess.run(args, cwd=cwd, capture_output=capture, text=True)


def extract_public_api(crate_dir: Path, crate_name: str, output_file: Path):
    """Extract public API using cargo-public-api with fallbacks."""
    info(f"Extracting public API for {crate_name}...")

    # Try cargo public-api --simplified
    result = run_cmd(["cargo", "public-api", "--simplified"], cwd=crate_dir)
    if result.returncode == 0 and result.stdout.strip():
        output_file.write_text(result.stdout)
        return

    # Fallback: try without --simplified
    result = run_cmd(["cargo", "public-api"], cwd=crate_dir)
    if result.returncode == 0 and result.stdout.strip():
        output_file.write_text(result.stdout)
        return

    warn("cargo-public-api failed, using rustdoc extraction")
    extract_from_rustdoc(crate_dir, crate_name, output_file)


def extract_from_rustdoc(crate_dir: Path, crate_name: str, output_file: Path):
    """Fallback: extract API from rustdoc JSON."""
    env = os.environ.copy()
    env["RUSTDOCFLAGS"] = "-Z unstable-options --output-format json"
    subprocess.run(
        ["cargo", "+nightly", "doc", "--no-deps"],
        cwd=crate_dir, capture_output=True, text=True, env=env,
    )

    # Find JSON file
    doc_dir = crate_dir / "target" / "doc"
    json_files = list(doc_dir.glob("*.json")) if doc_dir.is_dir() else []

    if json_files:
        json_file = json_files[0]
        result = run_cmd([
            "jq", "-r",
            '.index | to_entries[] | select(.value.visibility == "public") | "- \\(.value.kind): \\(.value.name)"',
            str(json_file),
        ])
        items = result.stdout.strip() if result.returncode == 0 else "Unable to parse rustdoc JSON"
        output_file.write_text(
            f"# Public API (extracted from rustdoc)\n\n"
            f"Note: This is a simplified extraction. For full API details, run `cargo doc --open`.\n\n"
            f"{items}\n"
        )
    else:
        # Last resort: grep public items from source
        src_dir = crate_dir / "src"
        items = []
        if src_dir.is_dir():
            for rs_file in src_dir.rglob("*.rs"):
                try:
                    for line in rs_file.read_text().splitlines():
                        if line.startswith("pub "):
                            items.append(line)
                            if len(items) >= 50:
                                break
                except OSError:
                    continue
                if len(items) >= 50:
                    break

        items_text = "\n".join(items) if items else "No public items found"
        output_file.write_text(
            f"# Public API (source extraction)\n\n"
            f"Note: Automatic API extraction failed. Manual review recommended.\n\n"
            f"## Public Items\n{items_text}\n"
        )


def generate_markdown(api_file: Path, output_file: Path, crate_name: str, lang: str):
    """Generate base markdown documentation from API."""
    lang_name = LANG_NAMES.get(lang, "English")
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    api_content = api_file.read_text()

    output_file.write_text(
        f"<!-- \n"
        f"  Auto-generated documentation for {crate_name}\n"
        f"  Language: {lang_name}\n"
        f"  Generated: {timestamp}\n"
        f"  \n"
        f"  This file was generated by: adi workflow autodoc\n"
        f"  To regenerate: adi workflow autodoc --force\n"
        f"-->\n\n"
        f"# {crate_name}\n\n"
        f"## Public API\n\n"
        f"```rust\n{api_content}```\n\n"
        f"## Overview\n\n"
        f"<!-- TODO: Add overview description -->\n\n"
        f"## Usage\n\n"
        f"<!-- TODO: Add usage examples -->\n\n"
        f"## API Reference\n\n"
        f"<!-- TODO: Add detailed API reference -->\n"
    )


def enrich_with_llm(doc_file: Path, crate_name: str, lang: str, crate_dir: Path):
    """Enrich documentation with LLM via claude CLI."""
    lang_name = LANG_NAMES.get(lang, "English")
    info(f"Enriching documentation with LLM (language: {lang_name})...")

    if not check_command("claude"):
        warn("claude CLI not found, skipping LLM enrichment")
        return

    current_doc = doc_file.read_text()

    # Read source files for context (limited)
    source_lines = []
    src_dir = crate_dir / "src"
    if src_dir.is_dir():
        for rs_file in src_dir.rglob("*.rs"):
            try:
                lines = rs_file.read_text().splitlines()[:100]
                source_lines.extend(lines)
                if len(source_lines) >= 500:
                    break
            except OSError:
                continue
    source_context = "\n".join(source_lines[:500])

    # Read Cargo.toml
    cargo_toml_path = crate_dir / "Cargo.toml"
    cargo_toml = cargo_toml_path.read_text() if cargo_toml_path.is_file() else ""

    # Read README
    readme_path = crate_dir / "README.md"
    readme = readme_path.read_text() if readme_path.is_file() else ""

    prompt = (
        f'You are a technical documentation writer. Your task is to enrich the API documentation '
        f'for the Rust crate "{crate_name}".\n\n'
        f"IMPORTANT: Write all documentation in {lang_name} language.\n\n"
        f"Current documentation (with API extracted):\n---\n{current_doc}\n---\n\n"
        f"Cargo.toml:\n---\n{cargo_toml}\n---\n\n"
        f"README (if available):\n---\n{readme}\n---\n\n"
        f"Source code snippets for context:\n---\n{source_context}\n---\n\n"
        f"Please generate enriched documentation that includes:\n\n"
        f"1. **Overview** - A clear, concise description of what this crate does and its main purpose\n"
        f"2. **Installation** - How to add this crate as a dependency\n"
        f"3. **Quick Start** - A minimal working example showing basic usage\n"
        f"4. **API Reference** - For each public item in the API:\n"
        f"   - Brief description of what it does\n"
        f"   - Parameters and return types explained\n"
        f"   - Example usage code where helpful\n"
        f"5. **Common Patterns** - Show 2-3 common usage patterns with code examples\n"
        f"6. **Error Handling** - Document any error types and how to handle them\n"
        f"7. **See Also** - Related crates or documentation links\n\n"
        f"Format the output as valid Markdown. Use proper Rust code blocks with syntax highlighting.\n"
        f"Keep the header comment from the original document.\n\n"
        f"Output ONLY the enriched documentation, no explanations or meta-commentary."
    )

    # Write prompt to temp file to avoid shell escaping issues
    TEMP_DIR.mkdir(parents=True, exist_ok=True)
    prompt_file = TEMP_DIR / f"prompt_{crate_name}_{lang}.txt"
    prompt_file.write_text(prompt)

    try:
        result = run_cmd(
            ["claude", "-p", prompt, "--model", "claude-sonnet-4-20250514"],
        )
        if result.returncode == 0 and result.stdout.strip():
            doc_file.write_text(result.stdout)
            success("Documentation enriched with LLM")
        else:
            warn("LLM enrichment failed, keeping original documentation")
    finally:
        prompt_file.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(
        description="Generate API documentation for a Rust crate with optional LLM enrichment.",
    )
    parser.add_argument("crate_name", help="Name of the crate to document")
    parser.add_argument("--lang", default="en", help="Language code (en, uk, ru, zh, ja, ko, es, de, fr)")
    parser.add_argument("--enrich", action="store_true", help="Enrich documentation with LLM")
    parser.add_argument("--force", action="store_true", help="Overwrite existing documentation")
    args = parser.parse_args()

    lang_name = LANG_NAMES.get(args.lang)
    if not lang_name:
        error(f"Unsupported language: {args.lang}. Supported: {', '.join(LANG_NAMES)}")

    # Find crate directory
    crate_dir = find_crate_dir(args.crate_name)
    if not crate_dir:
        error(f"Crate not found: {args.crate_name}")

    if not crate_dir.is_dir():
        error(f"Crate directory not found: {crate_dir}")

    # Check prerequisites
    if not check_command("cargo"):
        error("cargo not found")

    if not check_command("cargo-public-api"):
        warn("cargo-public-api not found, installing...")
        result = run_cmd(["cargo", "install", "cargo-public-api"])
        if result.returncode != 0:
            error("Failed to install cargo-public-api")

    # Setup directories
    output_dir = DOCS_DIR / args.crate_name / args.lang
    output_file = output_dir / "api.md"

    if output_file.is_file() and not args.force:
        error(f"Documentation already exists: {output_file}\nUse --force to overwrite.")

    output_dir.mkdir(parents=True, exist_ok=True)
    TEMP_DIR.mkdir(parents=True, exist_ok=True)

    print()
    info(f"Generating documentation for: {args.crate_name}")
    info(f"Language: {lang_name}")
    info(f"Output: {output_file}")
    print()

    # Step 1: Extract public API
    api_file = TEMP_DIR / f"{args.crate_name}_api.txt"
    extract_public_api(crate_dir, args.crate_name, api_file)

    if not api_file.is_file() or api_file.stat().st_size == 0:
        warn("No public API extracted (crate may be a binary or have no public items)")
        api_file.write_text(f"# {args.crate_name} - No Public API\n")

    success("Public API extracted")

    # Step 2: Generate base markdown
    generate_markdown(api_file, output_file, args.crate_name, args.lang)
    success("Base documentation generated")

    # Step 3: Enrich with LLM (if requested)
    if args.enrich:
        enrich_with_llm(output_file, args.crate_name, args.lang, crate_dir)

    # Cleanup
    api_file.unlink(missing_ok=True)

    print()
    success(f"Documentation generated: {output_file}")
    print()

    # Show preview
    info("Preview (first 30 lines):")
    print("---")
    lines = output_file.read_text().splitlines()[:30]
    print("\n".join(lines))
    print("---")
    print()
    info(f"Full documentation: {output_file}")


if __name__ == "__main__":
    main()
