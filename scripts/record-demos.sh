#!/usr/bin/env bash
# Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
# Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
if [ -d "$REPO_ROOT/assets/vhs" ]; then
    VHS_DIR="$REPO_ROOT/assets/vhs"
elif [ -d "$REPO_ROOT/docs/vhs" ]; then
    VHS_DIR="$REPO_ROOT/docs/vhs"
else
    VHS_DIR="$REPO_ROOT/vhs"
fi
OUTPUT_DIR="$REPO_ROOT/assets/demos"

show_help() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [TAPE_FILE...]

Automated terminal recorder using VHS for Perihelion demos and README visual previews.

OPTIONS:
    -h, --help      Display this help dialog and exit
    --no-build      Skip building release binaries (assumes cargo build was already run)
    --debug         Use target/debug binaries instead of target/release

EXAMPLES:
    ./scripts/record-demos.sh
    ./scripts/record-demos.sh --no-build
    ./scripts/record-demos.sh assets/vhs/peri-quickstart.tape
EOF
}

SKIP_BUILD=false
PROFILE="release"
CUSTOM_TAPES=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --debug)
            PROFILE="debug"
            shift
            ;;
        *.tape)
            CUSTOM_TAPES+=("$1")
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            show_help
            exit 1
            ;;
    esac
done

# 1. Verify VHS installation
if ! command -v vhs >/dev/null 2>&1; then
    echo "❌ Error: 'vhs' is not installed or not in PATH." >&2
    echo "   Install VHS using one of the following methods:" >&2
    echo "     - Arch Linux: pacman -S vhs" >&2
    echo "     - macOS:      brew install vhs" >&2
    echo "     - Go:         go install github.com/charmbracelet/vhs@latest" >&2
    echo "     - Shell:      curl -sS https://charm.sh/install.sh | bash" >&2
    exit 1
fi

# 2. Build binaries if requested
cd "$REPO_ROOT"
if [ "$SKIP_BUILD" = false ]; then
    echo "📦 Building Perihelion binaries ($PROFILE mode)..."
    if [ "$PROFILE" = "release" ]; then
        cargo build --release --bin peri --bin helion
    else
        cargo build --bin peri --bin helion
    fi
fi

# Ensure built binaries are preferred in PATH
export PATH="$REPO_ROOT/target/$PROFILE:$PATH"

if ! command -v peri >/dev/null 2>&1 || ! command -v helion >/dev/null 2>&1; then
    echo "❌ Error: 'peri' or 'helion' binary not found in target/$PROFILE" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

# 3. Discover target tapes
TAPES=()
if [ ${#CUSTOM_TAPES[@]} -gt 0 ]; then
    TAPES=("${CUSTOM_TAPES[@]}")
else
    while IFS= read -r -d '' tape_path; do
        TAPES+=("$tape_path")
    done < <(find "$VHS_DIR" -maxdepth 1 -name "*.tape" -print0 | sort -z)
fi

if [ ${#TAPES[@]} -eq 0 ]; then
    echo "⚠️  No .tape files found in $VHS_DIR"
    exit 0
fi

echo "🎬 Recording ${#TAPES[@]} VHS tape(s)..."
for tape in "${TAPES[@]}"; do
    echo "  ▶ Recording $tape..."
    vhs "$tape"
done

echo "✨ All terminal recordings generated successfully in $OUTPUT_DIR"

