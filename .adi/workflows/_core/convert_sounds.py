#!/usr/bin/env python3
"""Convert raw audio files to web-optimized MP3 and OGG formats."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
PROJECT_ROOT = Path(os.environ.get("PROJECT_ROOT", SCRIPT_DIR.parent.parent))

AUDIO_EXTENSIONS = {".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"}

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


def find_audio_files(source_dir: Path) -> list[Path]:
    return sorted(
        f for f in source_dir.iterdir()
        if f.is_file() and f.suffix.lower() in AUDIO_EXTENSIONS
    )


def convert_file(file: Path, output_dir: Path, preset: str) -> bool:
    name = file.stem
    result = subprocess.run(
        [
            "adi", "audio", "preset",
            "-i", str(file),
            "-o", str(output_dir / f"{name}.ogg"),
            "-o", str(output_dir / f"{name}.mp3"),
            "-p", preset,
        ],
        capture_output=True,
        text=True,
    )

    if result.returncode == 0:
        for line in result.stdout.splitlines():
            if any(kw in line for kw in ("Applied", "Loudness", "Peak", "Gain")):
                print(line)
        return True

    return False


def main():
    default_source = PROJECT_ROOT / "apps" / "web-app" / "public" / "raw_sounds"
    default_output = PROJECT_ROOT / "apps" / "web-app" / "public" / "sounds"

    parser = argparse.ArgumentParser(
        description="Convert raw audio files to web-optimized MP3 and OGG formats.",
        epilog=(
            "PRESETS:\n"
            "  web-sfx           Web UI sound effects (-10 LUFS, punchy)\n"
            "  web-notification  Notifications (-16 LUFS, moderate)\n"
            "  game-sfx          Game sound effects (-12 LUFS)\n"
            "  ringtone          Ringtones (-8 LUFS, loud)\n"
            "  podcast           Podcast/voice (-16 LUFS)\n"
            "  music-master      Music mastering (-14 LUFS)\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("preset", nargs="?", default="web-sfx", help="Audio preset (default: web-sfx)")
    parser.add_argument("source_dir", nargs="?", default=str(default_source), help="Source directory with raw audio files")
    parser.add_argument("output_dir", nargs="?", default=str(default_output), help="Output directory for converted files")
    args = parser.parse_args()

    preset = args.preset
    source_dir = Path(args.source_dir)
    output_dir = Path(args.output_dir)

    info(f"Converting audio files with preset: {preset}")
    info(f"Source: {source_dir}")
    info(f"Output: {output_dir}")
    print()

    output_dir.mkdir(parents=True, exist_ok=True)

    if not source_dir.is_dir():
        error(f"Source directory does not exist: {source_dir}")
        sys.exit(1)

    audio_files = find_audio_files(source_dir)
    if not audio_files:
        warn(f"No audio files found in {source_dir}")
        sys.exit(0)

    info(f"Found {len(audio_files)} audio file(s)")
    print()

    processed = 0
    failed = 0

    for file in audio_files:
        info(f"Processing: {file.name}")

        if convert_file(file, output_dir, preset):
            success(f"  -> {file.stem}.ogg + {file.stem}.mp3")
            processed += 1
        else:
            error(f"  Failed to convert: {file.name}")
            failed += 1

        print()

    print("-" * 40)
    success(f"Conversion complete! Processed: {processed}, Failed: {failed}")
    print()

    info("Output files:")
    output_files = sorted(
        f for f in output_dir.iterdir()
        if f.is_file() and f.suffix.lower() in {".mp3", ".ogg"}
    )
    for f in output_files:
        size = f.stat().st_size
        for unit in ("B", "K", "M", "G"):
            if size < 1024:
                print(f"  {size:>6}{unit}  {f.name}")
                break
            size //= 1024


if __name__ == "__main__":
    main()
