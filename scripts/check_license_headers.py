#!/usr/bin/env python3
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
"""
Perihelion License Header Utility
=================================

Ensures that:
1. All Rust source files (*.rs) contain the full GNU General Public License v3 (GPL-3.0-or-later) header.
2. All visual vector assets (*.svg) contain the Creative Commons Attribution 4.0 (CC BY 4.0) mark.
3. All maintenance scripts (*.py in scripts/) contain an explicit license header or SPDX identifier.

Usage:
    python3 scripts/check_license_headers.py --check   # Validate without modifying (exits with code 1 on missing)
    python3 scripts/check_license_headers.py --apply   # Automatically insert or update headers
"""

import argparse
import os
import re
import sys
from pathlib import Path

# Full GPLv3 header for Rust files
RUST_GPL_HEADER = """// Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
// Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
"""

# Full GPLv3 header for Python scripts
PYTHON_GPL_HEADER = """# Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
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
# SPDX-License-Identifier: GPL-3.0-or-later
"""

# Creative Commons Attribution 4.0 International mark for SVG assets
SVG_CC_HEADER = """<!--
  Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
  Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors

  This asset is licensed under the Creative Commons Attribution 4.0
  International License (CC BY 4.0).
  You may obtain a copy of the License at:
  https://creativecommons.org/licenses/by/4.0/
-->
"""

OLD_COPYRIGHT = "Copyright (C) 2026 Perihelion Contributors"
NEW_COPYRIGHT = "Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors"

# Patterns to detect existing license headers
RUST_LICENSE_PATTERNS = [
    re.compile(r"GNU General Public License", re.IGNORECASE),
    re.compile(r"GPL-3\.0", re.IGNORECASE),
]

SVG_LICENSE_PATTERNS = [
    re.compile(r"Creative Commons Attribution 4\.0", re.IGNORECASE),
    re.compile(r"CC BY 4\.0", re.IGNORECASE),
    re.compile(r"CC-BY-4\.0", re.IGNORECASE),
]

PYTHON_LICENSE_PATTERNS = [
    re.compile(r"SPDX-License-Identifier:", re.IGNORECASE),
    re.compile(r"GNU General Public License", re.IGNORECASE),
    re.compile(r"GPL-3\.0", re.IGNORECASE),
    re.compile(r"MIT License", re.IGNORECASE),
    re.compile(r"Apache License", re.IGNORECASE),
]

# Directories and paths to ignore
IGNORED_DIRS = {
    ".git",
    "target",
    "__pycache__",
    ".idea",
    ".vscode",
    "node_modules",
}


def has_rust_license(content: str) -> bool:
    """Check if Rust source content contains GPL header and Eduardo Marques Santos copyright."""
    head = "\n".join(content.splitlines()[:40])
    has_gpl = any(p.search(head) for p in RUST_LICENSE_PATTERNS)
    has_author = "Eduardo Marques Santos" in head
    return has_gpl and has_author


def has_svg_license(content: str) -> bool:
    """Check if SVG content contains CC-BY-4.0 mark and Eduardo Marques Santos copyright."""
    has_cc = any(p.search(content) for p in SVG_LICENSE_PATTERNS)
    has_author = "Eduardo Marques Santos" in content
    return has_cc and has_author


def has_python_license(content: str) -> bool:
    """Check if Python script content contains license notice and Eduardo Marques Santos copyright."""
    head = "\n".join(content.splitlines()[:35])
    has_lic = any(p.search(head) for p in PYTHON_LICENSE_PATTERNS)
    has_author = "Eduardo Marques Santos" in head or "SPDX-FileCopyrightText:" in head
    return has_lic and has_author


def apply_rust_header(content: str) -> str:
    """Prepend or update GPLv3 header to Rust content, respecting optional shebang."""
    if OLD_COPYRIGHT in content:
        return content.replace(OLD_COPYRIGHT, NEW_COPYRIGHT)

    lines = content.splitlines(keepends=True)
    if not lines:
        return RUST_GPL_HEADER + "\n"

    # If first line is a shebang, insert header after shebang
    if lines[0].startswith("#!") and "//! " not in lines[0] and not lines[0].startswith("#!["):
        shebang = lines[0]
        rest = "".join(lines[1:])
        return f"{shebang}\n{RUST_GPL_HEADER}\n{rest}"

    return f"{RUST_GPL_HEADER}\n{content}"


def apply_python_header(content: str) -> str:
    """Prepend or update GPLv3 header to Python content, respecting optional shebang."""
    if OLD_COPYRIGHT in content:
        return content.replace(OLD_COPYRIGHT, NEW_COPYRIGHT)

    lines = content.splitlines(keepends=True)
    if not lines:
        return PYTHON_GPL_HEADER + "\n"

    if lines[0].startswith("#!"):
        shebang = lines[0]
        rest = "".join(lines[1:])
        return f"{shebang}{PYTHON_GPL_HEADER}\n{rest}"

    return f"{PYTHON_GPL_HEADER}\n{content}"


def apply_svg_header(content: str) -> str:
    """Insert or update CC BY 4.0 comment into SVG content, respecting XML declaration if present."""
    if OLD_COPYRIGHT in content:
        return content.replace(OLD_COPYRIGHT, NEW_COPYRIGHT)

    xml_decl_match = re.match(r"^<\?xml[^>]*\?>\s*", content)
    if xml_decl_match:
        decl = xml_decl_match.group(0)
        rest = content[len(decl):]
        return f"{decl.strip()}\n{SVG_CC_HEADER}\n{rest}"

    return f"{SVG_CC_HEADER}\n{content}"


def find_files(repo_root: Path):
    """Find all target Rust, SVG, and script files in the repository."""
    rust_files = []
    svg_files = []
    python_files = []

    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in IGNORED_DIRS and not d.startswith(".")]

        rel_root = Path(root).relative_to(repo_root)

        if "prototyping" in rel_root.parts:
            continue

        for f in files:
            p = Path(root) / f
            if f.endswith(".rs"):
                rust_files.append(p)
            elif f.endswith(".svg"):
                svg_files.append(p)
            elif f.endswith(".py") and "scripts" in rel_root.parts:
                python_files.append(p)

    return sorted(rust_files), sorted(svg_files), sorted(python_files)


def main():
    parser = argparse.ArgumentParser(
        description="Verify or apply license headers to Perihelion source files, assets, and scripts."
    )
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--check",
        action="store_true",
        help="Check files for valid license headers without modifying them (exit code 1 on failure).",
    )
    group.add_argument(
        "--apply",
        action="store_true",
        help="Apply or update license headers on files (default behavior).",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Repository root directory (defaults to parent of scripts/).",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose output.",
    )

    args = parser.parse_args()
    repo_root = args.root.resolve()
    mode_check = args.check

    rust_files, svg_files, python_files = find_files(repo_root)

    total_scanned = len(rust_files) + len(svg_files) + len(python_files)
    missing_rust = []
    missing_svg = []
    missing_python = []
    updated_files = 0

    print(f"🔍 Perihelion License Verifier scanning: {repo_root}")
    print(f"   Found {len(rust_files)} Rust files (*.rs), {len(svg_files)} SVG assets (*.svg), and {len(python_files)} scripts (*.py)\n")

    # 1. Process Rust files
    for p in rust_files:
        try:
            content = p.read_text(encoding="utf-8")
        except Exception as e:
            print(f"❌ Error reading {p.relative_to(repo_root)}: {e}", file=sys.stderr)
            continue

        if not has_rust_license(content):
            missing_rust.append(p)
            if not mode_check:
                new_content = apply_rust_header(content)
                p.write_text(new_content, encoding="utf-8")
                updated_files += 1
                print(f"  [+] Updated GPLv3 header: {p.relative_to(repo_root)}")
            else:
                print(f"  [OUTDATED / MISSING GPLv3] {p.relative_to(repo_root)}")
        elif args.verbose:
            print(f"  [OK GPLv3]      {p.relative_to(repo_root)}")

    # 2. Process SVG assets
    for p in svg_files:
        try:
            content = p.read_text(encoding="utf-8")
        except Exception as e:
            print(f"❌ Error reading {p.relative_to(repo_root)}: {e}", file=sys.stderr)
            continue

        if not has_svg_license(content):
            missing_svg.append(p)
            if not mode_check:
                new_content = apply_svg_header(content)
                p.write_text(new_content, encoding="utf-8")
                updated_files += 1
                print(f"  [+] Updated CC BY 4.0 mark: {p.relative_to(repo_root)}")
            else:
                print(f"  [OUTDATED / MISSING CC BY 4.0] {p.relative_to(repo_root)}")
        elif args.verbose:
            print(f"  [OK CC BY 4.0]  {p.relative_to(repo_root)}")

    # 3. Process Python scripts
    for p in python_files:
        try:
            content = p.read_text(encoding="utf-8")
        except Exception as e:
            print(f"❌ Error reading {p.relative_to(repo_root)}: {e}", file=sys.stderr)
            continue

        if not has_python_license(content):
            missing_python.append(p)
            if not mode_check:
                new_content = apply_python_header(content)
                p.write_text(new_content, encoding="utf-8")
                updated_files += 1
                print(f"  [+] Updated script header: {p.relative_to(repo_root)}")
            else:
                print(f"  [OUTDATED / MISSING HEADER] {p.relative_to(repo_root)}")
        elif args.verbose:
            print(f"  [OK SCRIPT]     {p.relative_to(repo_root)}")

    print("\n" + "=" * 60)
    if mode_check:
        total_missing = len(missing_rust) + len(missing_svg) + len(missing_python)
        if total_missing > 0:
            print(f"❌ License Check Failed: {total_missing} file(s) missing or outdated headers.")
            if missing_rust:
                print(f"   - Outdated/Missing GPLv3: {len(missing_rust)} Rust files")
            if missing_svg:
                print(f"   - Outdated/Missing CC BY 4.0: {len(missing_svg)} SVG assets")
            if missing_python:
                print(f"   - Outdated/Missing Header: {len(missing_python)} Python scripts")
            print("\nRun `python3 scripts/check_license_headers.py --apply` to automatically update them.")
            sys.exit(1)
        else:
            print(f"✅ License Check Passed: All {total_scanned} files have appropriate license headers.")
            sys.exit(0)
    else:
        print(f"✅ License Processing Complete:")
        print(f"   - Scanned: {total_scanned} files ({len(rust_files)} .rs, {len(svg_files)} .svg, {len(python_files)} .py)")
        print(f"   - Updated: {updated_files} files with headers")
        sys.exit(0)


if __name__ == "__main__":
    main()
