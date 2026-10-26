#!/usr/bin/env bash
# Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
# Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
#
# This script is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This script is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
# SPDX-License-Identifier: GPL-3.0-or-later

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Ensure standard local user paths are prioritized
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

CHECK_ONLY=false
SKIP_BROWSER=false

show_help() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Automated toolchain bootstrapper for Perihelion development.
Installs, pins, and verifies all required toolchain dependencies:
  - mise (declarative multi-language toolchain runner)
  - Rust stable + clippy + rustfmt
  - sccache (compiler caching) + cargo wrapper config
  - lefthook (git pre-commit hook enforcement)
  - cargo-deny (dependency license and vulnerability checker)
  - cargo-dist & release-plz (release automation)
  - vhs & ttyd (terminal recording suite)
  - Headless browser sandbox patching (Flatpak / Linux namespace container support)

OPTIONS:
    -h, --help          Show this help message and exit
    --check             Check current toolchain health without making changes
    --no-browser        Skip Chromium / Rod headless sandbox checks

EXAMPLES:
    ./scripts/bootstrap.sh
    ./scripts/bootstrap.sh --check
    mise run bootstrap
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        --check)
            CHECK_ONLY=true
            shift
            ;;
        --no-browser)
            SKIP_BROWSER=true
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            show_help
            exit 1
            ;;
    esac
done

# Text formatting
BOLD="\033[1m"
GREEN="\033[32m"
YELLOW="\033[33m"
CYAN="\033[36m"
RED="\033[31m"
RESET="\033[0m"

step() {
    echo -e "${CYAN}${BOLD}==>${RESET} ${BOLD}$1${RESET}"
}

success() {
    echo -e "${GREEN}✓${RESET} $1"
}

warn() {
    echo -e "${YELLOW}▲${RESET} $1"
}

fail() {
    echo -e "${RED}✗${RESET} $1" >&2
}

mkdir -p "$HOME/.local/bin" "$HOME/.cargo"

echo -e "${BOLD}Perihelion Developer Toolchain Bootstrap${RESET}"
echo "Repository: $REPO_ROOT"
echo ""

if [ "$CHECK_ONLY" = true ]; then
    step "Running toolchain diagnostics (--check mode)..."
else
    # --------------------------------------------------------------------------
    # 1. Ensure mise is installed
    # --------------------------------------------------------------------------
    step "1/6 Checking mise toolchain manager..."
    if ! command -v mise >/dev/null 2>&1; then
        if [ -x "$HOME/.local/bin/mise" ]; then
            export PATH="$HOME/.local/bin:$PATH"
        else
            echo "mise is not installed. Installing via https://mise.run..."
            curl -fsSL https://mise.run | sh
            export PATH="$HOME/.local/bin:$PATH"
        fi
    fi
    success "mise $(mise --version | head -n1)"

    # --------------------------------------------------------------------------
    # 2. Trust repository configuration and install declared tools
    # --------------------------------------------------------------------------
    step "2/6 Synchronizing toolchain via mise.toml..."
    cd "$REPO_ROOT"
    mise trust -q "$REPO_ROOT/mise.toml" 2>/dev/null || true
    mise install -y

    # Link mise-managed binaries to ~/.local/bin for non-interactive subshell resilience
    MANAGED_TOOLS=(
        "sccache"
        "lefthook"
        "cargo-deny"
        "cargo-dist"
        "release-plz"
        "vhs"
        "ttyd"
        "ffmpeg"
    )

    for tool in "${MANAGED_TOOLS[@]}"; do
        BIN_PATH="$(mise which "$tool" 2>/dev/null || true)"
        if [ -z "$BIN_PATH" ] && [ "$tool" = "cargo-dist" ]; then
            BIN_PATH="$(mise which dist 2>/dev/null || which dist 2>/dev/null || true)"
        fi
        if [ -n "$BIN_PATH" ] && [ -x "$BIN_PATH" ]; then
            ln -sf "$BIN_PATH" "$HOME/.local/bin/$tool"
            if [ "$tool" = "cargo-dist" ]; then
                ln -sf "$BIN_PATH" "$HOME/.local/bin/dist"
            fi
        fi
    done
    success "Synchronized tool versions and symlinks in ~/.local/bin"

    # --------------------------------------------------------------------------
    # 3. Rust toolchain components
    # --------------------------------------------------------------------------
    step "3/6 Verifying Rust toolchain components (clippy, rustfmt)..."
    if command -v rustup >/dev/null 2>&1; then
        rustup component add clippy rustfmt >/dev/null 2>&1 || true
        success "Rustup components 'clippy' and 'rustfmt' active"
    else
        warn "rustup not detected; assuming system-managed toolchain components"
    fi

    # --------------------------------------------------------------------------
    # 4. Sccache compiler cache configuration
    # --------------------------------------------------------------------------
    step "4/6 Configuring sccache acceleration in ~/.cargo/config.toml..."
    CARGO_CONFIG="$HOME/.cargo/config.toml"
    if [ ! -f "$CARGO_CONFIG" ]; then
        cat << 'EOF' > "$CARGO_CONFIG"
[build]
rustc-wrapper = "sccache"
EOF
        success "Created ~/.cargo/config.toml with sccache rustc-wrapper"
    elif ! grep -q 'rustc-wrapper.*sccache' "$CARGO_CONFIG"; then
        if grep -q '\[build\]' "$CARGO_CONFIG"; then
            sed -i '/\[build\]/a rustc-wrapper = "sccache"' "$CARGO_CONFIG"
        else
            echo -e '\n[build]\nrustc-wrapper = "sccache"' >> "$CARGO_CONFIG"
        fi
        success "Injected sccache rustc-wrapper into ~/.cargo/config.toml"
    else
        success "sccache rustc-wrapper already configured in ~/.cargo/config.toml"
    fi

    # --------------------------------------------------------------------------
    # 5. Git pre-commit hooks via lefthook
    # --------------------------------------------------------------------------
    step "5/6 Installing git hooks via lefthook..."
    if [ -d "$REPO_ROOT/.git" ]; then
        if command -v lefthook >/dev/null 2>&1; then
            lefthook install >/dev/null 2>&1 || true
            success "Lefthook pre-commit hooks installed"
        else
            warn "Lefthook binary not found in PATH; skipping hook installation"
        fi
    else
        warn "Not a git repository root; skipping hook installation"
    fi

    # --------------------------------------------------------------------------
    # 6. Chromium / Rod headless sandbox wrapper
    # --------------------------------------------------------------------------
    if [ "$SKIP_BROWSER" = false ]; then
        step "6/6 Checking VHS & headless browser sandbox compatibility..."
        # Rod automatically downloads Chromium into ~/.cache/rod/browser/chromium-<rev>/chrome
        # In Flatpak / unprivileged user namespaces, Chrome crashes unless wrapped with --no-sandbox
        while IFS= read -r -d '' chrome_exec; do
            if [ -f "$chrome_exec" ] && [ -x "$chrome_exec" ]; then
                # Check if it's already a wrapper script
                if file "$chrome_exec" | grep -q "ELF"; then
                    mv "$chrome_exec" "$chrome_exec.bin"
                    cat << 'EOF' > "$chrome_exec"
#!/bin/sh
exec "$0.bin" --no-sandbox --disable-gpu "$@"
EOF
                    chmod +x "$chrome_exec"
                    success "Patched $(basename "$(dirname "$chrome_exec")")/chrome with --no-sandbox wrapper"
                fi
            fi
        done < <(find "$HOME/.cache/rod/browser" -maxdepth 3 -type f -name "chrome" -print0 2>/dev/null || true)
        success "Headless browser sandbox configured"
    fi
    echo ""
fi

# ------------------------------------------------------------------------------
# Diagnostics Summary Table
# ------------------------------------------------------------------------------
echo -e "${BOLD}Current Toolchain Diagnostics Summary:${RESET}"
printf "%-16s %-10s %-26s %s\n" "TOOL" "STATUS" "VERSION" "NOTES"
echo "--------------------------------------------------------------------------------"

check_tool() {
    local name="$1"
    local bin="$2"
    local ver_cmd="$3"
    local notes="$4"

    if command -v "$bin" >/dev/null 2>&1; then
        local ver
        ver=$( (eval "$ver_cmd" 2>/dev/null || true) | head -n1)
        [ -z "$ver" ] && ver="installed"
        printf "%-16s ${GREEN}%-10s${RESET} %-26s %s\n" "$name" "READY" "${ver:0:25}" "$notes"
    else
        printf "%-16s ${RED}%-10s${RESET} %-26s %s\n" "$name" "MISSING" "-" "$notes"
    fi
}

check_tool "mise" "mise" "mise --version | awk '{print \$1}'" "Tool manager"
check_tool "rustc" "rustc" "rustc --version | awk '{print \$2}'" "Rust compiler"
check_tool "cargo" "cargo" "cargo --version | awk '{print \$2}'" "Package manager"
check_tool "clippy" "cargo-clippy" "cargo clippy --version | awk '{print \$2}'" "Strict linter"
check_tool "rustfmt" "rustfmt" "rustfmt --version | awk '{print \$2}'" "Code formatter"
check_tool "sccache" "sccache" "sccache --version | awk '{print \$2}'" "Compiler cache"
check_tool "lefthook" "lefthook" "lefthook version" "Git pre-commit hooks"
check_tool "cargo-deny" "cargo-deny" "cargo-deny --version | awk '{print \$2}'" "Security & licenses"
check_tool "cargo-dist" "dist" "dist --version | awk '{print \$2}'" "Distribution packager"
check_tool "release-plz" "release-plz" "release-plz --version | awk '{print \$2}'" "Release PR automation"
check_tool "vhs" "vhs" "vhs --version | awk '{print \$3}'" "Terminal GIF recorder"
check_tool "ttyd" "ttyd" "ttyd --version | awk '{print \$3}'" "Terminal server for VHS"
check_tool "ffmpeg" "ffmpeg" "ffmpeg -version | head -n1 | awk '{print \$3}'" "Media encoder for VHS"

echo "--------------------------------------------------------------------------------"
echo -e "${GREEN}${BOLD}✨ Bootstrap complete! Your environment is ready for Perihelion development.${RESET}"
echo -e "Helpful commands:"
echo -e "  - ${CYAN}mise run check${RESET}     Run tests, clippy (-D warnings), and format check"
echo -e "  - ${CYAN}mise run record${RESET}    Re-record README preview GIFs via VHS"
echo -e "  - ${CYAN}mise run test${RESET}      Run all workspace tests"
