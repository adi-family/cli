#!/usr/bin/env python3
"""Plugin linter - validates plugin structure before publishing."""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

# ANSI colors
RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
BLUE = "\033[0;34m"
CYAN = "\033[0;36m"
BOLD = "\033[1m"
NC = "\033[0m"

# Counters (module-level for accumulation across lint runs)
ERRORS = 0
WARNINGS = 0


def error(msg: str):
    global ERRORS
    print(f"{RED}ERROR:{NC} {msg}")
    ERRORS += 1


def warn(msg: str):
    global WARNINGS
    print(f"{YELLOW}WARN:{NC} {msg}")
    WARNINGS += 1


def info(msg: str):
    print(f"{BLUE}INFO:{NC} {msg}")


def success(msg: str):
    print(f"{GREEN}OK:{NC} {msg}")


def section(msg: str):
    print(f"\n{BOLD}{CYAN}==> {msg}{NC}")


def get_toml_value(file: Path, key: str) -> str:
    """Parse a simple top-level TOML value."""
    try:
        for line in file.read_text().splitlines():
            m = re.match(rf"^{re.escape(key)}\s*=\s*(.*)", line)
            if m:
                raw = m.group(1).strip()
                # Remove surrounding quotes
                if (raw.startswith('"') and raw.endswith('"')) or (raw.startswith("'") and raw.endswith("'")):
                    return raw[1:-1]
                return raw
    except OSError:
        pass
    return ""


def has_toml_section(file: Path, section_name: str) -> bool:
    """Check if [section] exists."""
    pattern = re.compile(rf"^\[{re.escape(section_name)}\]")
    try:
        return any(pattern.match(line) for line in file.read_text().splitlines())
    except OSError:
        return False


def has_toml_array_section(file: Path, section_name: str) -> bool:
    """Check if [[section]] exists."""
    pattern = re.compile(rf"^\[\[{re.escape(section_name)}\]\]")
    try:
        return any(pattern.match(line) for line in file.read_text().splitlines())
    except OSError:
        return False


def get_section_field(file: Path, section_name: str, field: str) -> str:
    """Extract a field value from within a TOML section."""
    in_section = False
    try:
        for line in file.read_text().splitlines():
            if re.match(rf"^\[{re.escape(section_name)}\]", line):
                in_section = True
                continue
            if in_section and line.startswith("["):
                break
            if in_section:
                m = re.match(rf"^{re.escape(field)}\s*=\s*(.*)", line)
                if m:
                    raw = m.group(1).strip()
                    if (raw.startswith('"') and raw.endswith('"')) or (raw.startswith("'") and raw.endswith("'")):
                        return raw[1:-1]
                    return raw
    except OSError:
        pass
    return ""


def get_binary_name(manifest: Path) -> str:
    """Get binary name from manifest or default."""
    name = get_section_field(manifest, "binary", "name")
    return name if name else "plugin"


def is_valid_semver(version: str) -> bool:
    return bool(re.match(r"^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?(\+[a-zA-Z0-9.]+)?$", version))


def get_lib_extension() -> str:
    import platform as plat
    system = plat.system()
    if system == "Darwin":
        return "dylib"
    if system == "Windows":
        return "dll"
    return "so"


def lint_plugin(plugin_dir: Path, fix_mode: bool) -> bool:
    """Main lint function. Returns True on success, False on fatal error."""
    section(f"Linting plugin: {plugin_dir}")

    # Check for Cargo.toml with [package.metadata.plugin]
    cargo_toml = plugin_dir / "Cargo.toml"
    if not cargo_toml.is_file():
        error(f"Cargo.toml not found at {cargo_toml}")
        return False

    cargo_text = cargo_toml.read_text()
    if "package.metadata.plugin" not in cargo_text:
        legacy_manifest = plugin_dir / "plugin.toml"
        if legacy_manifest.is_file():
            warn("Found legacy plugin.toml - migrate to [package.metadata.plugin] in Cargo.toml")
        else:
            error("No [package.metadata.plugin] section in Cargo.toml")
        return False
    success("Cargo.toml has [package.metadata.plugin]")

    # Generate plugin.toml from metadata for validation
    manifest_gen = PROJECT_ROOT / "target" / "release" / "manifest-gen"
    if not manifest_gen.is_file():
        manifest_gen = PROJECT_ROOT / "target" / "debug" / "manifest-gen"

    manifest = Path(tempfile.mktemp(suffix=".toml", prefix="plugin-lint-"))
    if manifest_gen.is_file():
        result = subprocess.run(
            [str(manifest_gen), "--cargo-toml", str(cargo_toml), "--output", str(manifest)],
            capture_output=True, text=True,
        )
        if result.returncode != 0:
            error("Failed to generate manifest from Cargo.toml metadata")
            return False
        success("Manifest generated successfully from Cargo.toml")
    else:
        warn("manifest-gen not found - skipping manifest generation validation")
        warn("Build with: cargo build -p lib-plugin-manifest --features generate")
        return True

    try:
        return _validate_manifest(manifest, plugin_dir, fix_mode)
    finally:
        manifest.unlink(missing_ok=True)


def _validate_manifest(manifest: Path, plugin_dir: Path, fix_mode: bool) -> bool:
    """Validate the generated manifest file."""
    # Validate TOML syntax
    section("Validating TOML syntax")
    if shutil.which("taplo"):
        result = subprocess.run(["taplo", "check", str(manifest)], capture_output=True, text=True)
        if result.returncode == 0:
            success("TOML syntax valid")
        else:
            error("Invalid TOML syntax")
    else:
        info("TOML validator (taplo) not found. Skipping syntax check.")

    # Check [plugin] section
    section("Checking [plugin] section")
    if not has_toml_section(manifest, "plugin"):
        error("Missing [plugin] section")
        return False
    success("[plugin] section exists")

    # Required fields in [plugin]
    plugin_id = get_toml_value(manifest, "id")
    plugin_name = get_toml_value(manifest, "name")
    plugin_version = get_toml_value(manifest, "version")
    plugin_type = get_toml_value(manifest, "type")

    # Check id
    if not plugin_id:
        error("Missing required field: id")
    elif not re.match(r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$", plugin_id):
        warn(f"Plugin ID '{plugin_id}' should follow format: vendor.plugin-name (e.g., adi.cocoon)")
    else:
        success(f"id: {plugin_id}")

    # Check name
    if not plugin_name:
        error("Missing required field: name")
    else:
        success(f"name: {plugin_name}")

    # Check version
    if not plugin_version:
        error("Missing required field: version")
    elif not is_valid_semver(plugin_version):
        error(f"Invalid version format: '{plugin_version}' (expected semver: X.Y.Z)")
    else:
        success(f"version: {plugin_version}")

    # Check type
    if not plugin_type:
        error("Missing required field: type")
        if fix_mode:
            detected_type = "core"
            if has_toml_section(manifest, "language"):
                detected_type = "lang"
            elif plugin_id and ".lang." in plugin_id:
                detected_type = "lang"
            info(f'FIX: Adding type = "{detected_type}" to manifest')
            text = manifest.read_text()
            text = text.replace("[plugin]\n", f"[plugin]\ntype = \"{detected_type}\"\n", 1)
            manifest.write_text(text)
            success(f'Added type = "{detected_type}"')
            plugin_type = detected_type
    elif plugin_type not in ("core", "extension", "lang", "theme"):
        warn(f"Unknown plugin type: '{plugin_type}' (expected: core, extension, lang, theme)")
    else:
        success(f"type: {plugin_type}")

    # Check min_host_version
    min_host_version = get_toml_value(manifest, "min_host_version")
    if min_host_version:
        if not is_valid_semver(min_host_version):
            error(f"Invalid min_host_version format: '{min_host_version}'")
        else:
            success(f"min_host_version: {min_host_version}")
    else:
        warn("No min_host_version specified (recommended for compatibility)")

    # Check [binary] section and actual binary
    section("Checking binary configuration")
    binary_name = get_binary_name(manifest)
    lib_ext = get_lib_extension()

    variants = [f"{binary_name}.{lib_ext}", f"lib{binary_name}.{lib_ext}"]
    found_lib = ""

    for variant in variants:
        if (plugin_dir / variant).is_file():
            found_lib = variant
            break

    # Also check in target/release for source builds
    if not found_lib:
        target_dir = PROJECT_ROOT / "target" / "release"
        for variant in variants:
            if (target_dir / variant).is_file():
                info(f"Binary found in target/release: {variant}")
                found_lib = variant
                break

    if not found_lib:
        if (plugin_dir / "Cargo.toml").is_file():
            info(f"Binary not built yet (will be built during release): {binary_name}")
            info(f"Expected: {' '.join(variants)}")
        else:
            error(f"No library file found for binary name '{binary_name}'")
            error(f"Expected one of: {' '.join(variants)}")

        # Check what libraries actually exist
        existing_libs = sorted(plugin_dir.glob(f"*.{lib_ext}"))[:5]
        if existing_libs:
            info("Found libraries in plugin dir:")
            for lib in existing_libs:
                print(f"  - {lib.name}")

            first_lib = existing_libs[0]
            suggested_name = first_lib.stem.removeprefix("lib")

            if fix_mode:
                info(f'FIX: Adding [binary] section with name = "{suggested_name}"')
                text = manifest.read_text()
                if has_toml_section(manifest, "binary"):
                    text = re.sub(r'^name = .*$', f'name = "{suggested_name}"', text, flags=re.MULTILINE)
                else:
                    text += f'\n[binary]\nname = "{suggested_name}"\n'
                manifest.write_text(text)
                success(f'Added [binary] name = "{suggested_name}"')
            else:
                warn(f"Suggested fix: Add '[binary]' section with name = \"{suggested_name}\"")
    else:
        success(f"Binary found: {found_lib} (name: {binary_name})")

    # Check [[provides]] section
    section("Checking service declarations")

    if has_toml_section(manifest, "provides") and not has_toml_array_section(manifest, "provides"):
        error("Wrong provides format: [provides] should be [[provides]] (array of tables)")
        warn("Lang plugins use different schema - may need migration")
    elif not has_toml_array_section(manifest, "provides"):
        warn("No [[provides]] section - plugin won't register any services")
    else:
        manifest_text = manifest.read_text()
        provides_count = len(re.findall(r"^\[\[provides\]\]", manifest_text, re.MULTILINE))
        success(f"Found {provides_count} service declaration(s)")

        # Check each provides has required fields
        provides_ids = []
        in_provides = False
        for line in manifest_text.splitlines():
            if re.match(r"^\[\[provides\]\]", line):
                in_provides = True
                continue
            if in_provides and line.startswith("["):
                in_provides = False
            if in_provides:
                m = re.match(r'^id\s*=\s*"(.*?)"', line)
                if m:
                    provides_ids.append(m.group(1))
                    in_provides = False

        if not provides_ids:
            error("[[provides]] section missing 'id' field")
        else:
            for svc_id in provides_ids:
                success(f"  Service: {svc_id}")

    # Check [cli] section
    section("Checking CLI configuration")

    if has_toml_section(manifest, "cli"):
        success("[cli] section present")

        cli_command = get_section_field(manifest, "cli", "command")
        cli_description = get_section_field(manifest, "cli", "description")

        # Check command
        if not cli_command:
            error("[cli] section missing required field: command")
        elif not re.match(r"^[a-z][a-z0-9-]*$", cli_command):
            error(f"Invalid CLI command name: '{cli_command}' (must be lowercase alphanumeric with hyphens)")
        else:
            success(f"cli.command: {cli_command}")

        # Check description
        if not cli_description:
            error("[cli] section missing required field: description")
        elif len(cli_description) < 10:
            warn(f"CLI description is very short ({len(cli_description)} chars)")
        else:
            success(f"cli.description: {cli_description[:50]}...")

        # Check aliases format if present
        manifest_text = manifest.read_text()
        in_cli = False
        for line in manifest_text.splitlines():
            if re.match(r"^\[cli\]", line):
                in_cli = True
                continue
            if in_cli and line.startswith("["):
                break
            if in_cli and line.startswith("aliases"):
                if "[" in line and "]" in line:
                    success("cli.aliases: present")
                else:
                    error('cli.aliases should be an array (e.g., aliases = ["t"])')
                break

        # Cross-check: if [cli] exists, should have .cli service in [[provides]]
        if has_toml_array_section(manifest, "provides"):
            if not re.search(r'id\s*=\s*".*\.cli"', manifest_text):
                warn("[cli] section exists but no .cli service in [[provides]]")
                warn(f'Add: [[provides]] with id = "{plugin_id}.cli"')
    else:
        # No [cli] section - check for inconsistency
        if has_toml_array_section(manifest, "provides"):
            manifest_text = manifest.read_text()
            if re.search(r'id\s*=\s*".*\.cli"', manifest_text):
                warn("Plugin provides .cli service but has no [cli] section")
                warn("Add [cli] section to register top-level command")
            else:
                info("No [cli] section (plugin has no CLI command)")
        else:
            info("No [cli] section (plugin has no CLI command)")

    # Check metadata
    section("Checking metadata")
    if has_toml_section(manifest, "tags"):
        success("[tags] section present")
    else:
        warn("No [tags] section - consider adding categories for discoverability")

    # Check description
    description = get_toml_value(manifest, "description")
    if not description:
        warn("No description provided")
    elif len(description) < 20:
        warn(f"Description is very short ({len(description)} chars) - consider a more detailed description")
    else:
        success(f"description: {description[:50]}...")

    # Check author
    author = get_toml_value(manifest, "author")
    if not author:
        warn("No author specified")
    else:
        success(f"author: {author}")

    # Version consistency
    section("Checking version source")
    success(f"Version from Cargo.toml: {plugin_version} (single source of truth)")

    return True


def resolve_plugin_dir(plugin: str) -> Path | None:
    """Resolve plugin name/ID/path to a directory."""
    # If it's already a directory path
    candidate = Path(plugin)
    if candidate.is_dir():
        return candidate

    # Try to find by plugin ID in Cargo.toml [package.metadata.plugin]
    crates_dir = PROJECT_ROOT / "crates"
    for cargo_toml in crates_dir.rglob("Cargo.toml"):
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
                    if plugin_id == plugin:
                        return cargo_toml.parent
        except (IndexError, OSError):
            continue

    # Fallback to directory-based resolution
    fallback = PROJECT_ROOT / "crates" / plugin
    if fallback.is_dir():
        return fallback

    return None


def main():
    global ERRORS, WARNINGS

    parser = argparse.ArgumentParser(description="Lint a plugin before publishing.")
    parser.add_argument("plugin", nargs="?", default="", help="Plugin name, ID, or path to plugin directory")
    parser.add_argument("--fix", action="store_true", help="Attempt to fix common issues automatically")
    parser.add_argument("--strict", action="store_true", help="Treat warnings as errors")
    parser.add_argument("--all", action="store_true", help="Lint all plugins in crates/")
    args = parser.parse_args()

    # Handle --all flag
    if args.all:
        print(f"{BOLD}{CYAN}Linting all plugins...{NC}")
        print()

        total_errors = 0
        total_warnings = 0
        plugins_checked = 0
        plugins_failed = 0

        crates_dir = PROJECT_ROOT / "crates"
        for cargo_file in sorted(crates_dir.rglob("Cargo.toml")):
            try:
                text = cargo_file.read_text()
            except OSError:
                continue
            if "package.metadata.plugin" not in text:
                continue

            ERRORS = 0
            WARNINGS = 0

            if not lint_plugin(cargo_file.parent, args.fix):
                plugins_failed += 1

            total_errors += ERRORS
            total_warnings += WARNINGS
            plugins_checked += 1
            print()

        print()
        section("Overall Summary")
        print(f"Plugins checked: {plugins_checked}")
        print(f"Plugins with errors: {plugins_failed}")
        print(f"Total errors: {total_errors}")
        print(f"Total warnings: {total_warnings}")

        if total_errors > 0:
            sys.exit(1)
        elif total_warnings > 0 and args.strict:
            sys.exit(1)
        sys.exit(0)

    if not args.plugin:
        print(f"{RED}ERROR:{NC} No plugin specified")
        print()
        parser.print_help()
        sys.exit(1)

    plugin_dir = resolve_plugin_dir(args.plugin)
    if not plugin_dir:
        print(f"{RED}ERROR:{NC} Plugin not found: {args.plugin}")
        print("Tried:")
        print(f"  - Direct path: {args.plugin}")
        print("  - Plugin ID lookup in Cargo.toml files")
        print(f"  - Directory: {PROJECT_ROOT / 'crates' / args.plugin}")
        sys.exit(1)

    # Run linter
    lint_plugin(plugin_dir, args.fix)

    # Summary
    print()
    section("Summary")
    if ERRORS > 0:
        print(f"{RED}{BOLD}{ERRORS} error(s){NC}, {YELLOW}{WARNINGS} warning(s){NC}")
        sys.exit(1)
    elif WARNINGS > 0 and args.strict:
        print(f"{YELLOW}{BOLD}{WARNINGS} warning(s){NC} (strict mode)")
        sys.exit(1)
    elif WARNINGS > 0:
        print(f"{GREEN}{BOLD}Passed{NC} with {YELLOW}{WARNINGS} warning(s){NC}")
        sys.exit(0)
    else:
        print(f"{GREEN}{BOLD}All checks passed!{NC}")
        sys.exit(0)


if __name__ == "__main__":
    main()
