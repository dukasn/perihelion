# Perihelion Maintenance & Automation Scripts

This directory contains developer tooling, maintenance utilities, asset generators, and service templates for the **Perihelion** project.

---

## Licensing Policy for Scripts

To balance consistency with contributor flexibility, scripts in this directory follow this licensing structure:

### 1. Default License (GPL-3.0-or-later)
Unless explicitly stated otherwise within the file itself, **all scripts in this directory are governed by the project's standard license: the GNU General Public License v3 (GPL-3.0-or-later)**.
- See the root [`../LICENSE`](../LICENSE) file for the full license terms.

### 2. Per-File License Attribution
Individual scripts may specify an alternative or dedicated open-source license (such as **MIT**, **Apache-2.0**, or **0BSD / CC0**) by including an explicit license header or [SPDX license identifier](https://spdx.dev/ids/) at the top of the file:
```python
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2026 Eduardo Marques Santos and Perihelion Contributors
```
When an explicit header is present, the terms in that header govern that specific file.

### 3. Asset & Media Generator Outputs
Scripts that generate or process visual artwork, media, or documentation (such as `generate_banner.py`):
- The **generator script source code** is software governed by this policy (GPL-3.0-or-later by default).
- The **resulting artwork, media files, and banners** produced and stored in [`../assets/`](../assets/) are licensed under the **Creative Commons Attribution 4.0 International License (CC BY 4.0)**, as specified in [`../assets/README.md`](../assets/README.md).

### 4. Configuration & Service Templates
Template files intended for downstream distribution packaging (such as `perid.service` for systemd) may be adapted, modified, and packaged freely by downstream maintainers without imposing copyleft obligations on external packaging configurations.

---

## Directory Inventory

| File | Purpose | License |
| :--- | :--- | :--- |
| [`bootstrap.sh`](./bootstrap.sh) | Automated toolchain installer and diagnostics bootstrapper via `mise`. | GPL-3.0-or-later |
| [`check_license_headers.py`](./check_license_headers.py) | Scans and enforces GPLv3 headers on `.rs` files and CC BY 4.0 marks on `.svg` assets. | GPL-3.0-or-later |
| [`generate_banner.py`](./generate_banner.py) | Procedural repository banner generator for WebP and PNG assets. | GPL-3.0-or-later *(outputs CC BY 4.0)* |
| [`record-demos.sh`](./record-demos.sh) | Automated VHS terminal tape recorder for compiling preview GIFs. | GPL-3.0-or-later *(outputs CC BY 4.0)* |
| [`perid.service`](./perid.service) | Systemd user service unit template for the background seeding daemon. | CC0 / Permissive template |

---

## Guidelines for Adding New Scripts

When contributing a new script to this directory:
1. **Include a Shebang & Help Flag:** Provide a standard shebang (`#!/usr/bin/env python3` or `#!/usr/bin/env bash`) and implement a `--help` message.
2. **Document Dependencies:** Detail prerequisites (e.g., standard library only, or `pip install pillow`) in the module docstring.
3. **Explicit License Header (Optional but Encouraged):** If you wish to use a specific license, include an SPDX tag or standard license block. Otherwise, it automatically inherits the project's GPL-3.0-or-later license.
4. **Mark Executable:** Run `chmod +x scripts/<script_name>` before committing.

