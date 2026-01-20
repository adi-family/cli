#!/bin/bash
# Convert raw audio files to web-optimized formats (MP3 + OGG)
# Usage: adi workflow convert-sounds
#    or: ./convert-sounds.sh [preset] [source_dir] [output_dir]

set -e

# When run via `adi workflow`, prelude is auto-injected.
# When run directly, use minimal fallback.
if [[ -z "${_ADI_PRELUDE_LOADED:-}" ]]; then
    _SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
    # Colors
    RED='\033[0;31m' GREEN='\033[0;32m' YELLOW='\033[1;33m' CYAN='\033[0;36m' NC='\033[0m'
    info() { printf "${CYAN}info${NC} %s\n" "$1"; }
    success() { printf "${GREEN}done${NC} %s\n" "$1"; }
    warn() { printf "${YELLOW}warn${NC} %s\n" "$1"; }
    error() { printf "${RED}error${NC} %s\n" "$1" >&2; }
fi

# Default values
PRESET="${1:-web-sfx}"
SOURCE_DIR="${2:-$PROJECT_ROOT/apps/web-app/public/raw_sounds}"
OUTPUT_DIR="${3:-$PROJECT_ROOT/apps/web-app/public/sounds}"

usage() {
    cat <<EOF
Usage: $0 [preset] [source_dir] [output_dir]

Convert raw audio files to web-optimized MP3 and OGG formats.

PRESETS:
    web-sfx          Web UI sound effects (-10 LUFS, punchy)
    web-notification Notifications (-16 LUFS, moderate)
    game-sfx         Game sound effects (-12 LUFS)
    ringtone         Ringtones (-8 LUFS, loud)
    podcast          Podcast/voice (-16 LUFS)
    music-master     Music mastering (-14 LUFS)

EXAMPLES:
    $0                                    # Use defaults
    $0 web-sfx                            # Specify preset only
    $0 web-sfx ./raw ./output             # Specify all paths

EOF
    exit 0
}

[[ "$1" == "-h" || "$1" == "--help" ]] && usage

info "Converting audio files with preset: $PRESET"
info "Source: $SOURCE_DIR"
info "Output: $OUTPUT_DIR"
echo ""

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Find all audio files in source directory
shopt -s nullglob
AUDIO_FILES=()
for ext in mp3 wav flac ogg m4a aac; do
    AUDIO_FILES+=("$SOURCE_DIR"/*."$ext")
done

if [ ${#AUDIO_FILES[@]} -eq 0 ]; then
    warn "No audio files found in $SOURCE_DIR"
    exit 0
fi

info "Found ${#AUDIO_FILES[@]} audio file(s)"
echo ""

# Process each file
PROCESSED=0
FAILED=0

for file in "${AUDIO_FILES[@]}"; do
    # Get filename without extension
    filename=$(basename "$file")
    name="${filename%.*}"
    
    info "Processing: $filename"
    
    # Convert to both MP3 and OGG
    if adi audio preset -i "$file" -o "$OUTPUT_DIR/$name.ogg" -o "$OUTPUT_DIR/$name.mp3" -p "$PRESET" 2>&1 | grep -E "(Applied|Loudness|Peak|Gain)" | head -5; then
        success "  -> $name.ogg + $name.mp3"
        PROCESSED=$((PROCESSED + 1))
    else
        error "  Failed to convert: $filename"
        FAILED=$((FAILED + 1))
    fi
    echo ""
done

echo "----------------------------------------"
success "Conversion complete! Processed: $PROCESSED, Failed: $FAILED"
echo ""
info "Output files:"
ls -lh "$OUTPUT_DIR"/*.{mp3,ogg} 2>/dev/null || true
