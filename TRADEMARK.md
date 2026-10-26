# Project Naming & Authenticity Policy

This document outlines the naming conventions, authenticity requirements, and trademark/brand guidelines for the **Perihelion** project.

---

## 1. Project Philosophy

Perihelion is an open-source, community-driven free software project. We do not operate as a commercial company, nor do we intend to build a commercialized corporate brand out of Perihelion. We do not claim exclusive ownership over the common dictionary word "perihelion" or general terminology—anyone is free to use the word for unrelated software, scientific purposes, gaming guilds, or creative works.

We fully embrace the software freedoms granted by the GNU General Public License (GPLv3) and Creative Commons Attribution 4.0 (CC BY 4.0), including the freedom to inspect, fork, modify, and redistribute the project.

At the same time, users must be able to trust the authenticity and origin of the software they install and run on their systems. Linux gaming utilities and Virtual File System (VFS) drivers interact deeply with unprivileged user/mount namespaces, file indices, and distributed P2P swarms. A malicious or misconfigured third-party fork pretending to be the official software could cause data loss or security issues while erroneously damaging the reputation of the upstream project.

Therefore, this policy exists solely to:
1. Prevent confusion between official upstream releases and third-party forks.
2. Ensure no third party misrepresents their derivative, website, or distribution as official or maintained by this project.
3. Provide crystal-clear guidance for contributors, fork authors, and Linux distribution package maintainers without overreaching into general word usage.

---

## 2. Project Names and Component Roles

The following names identify distinct components of this project:

| Name | Role | Location | Description |
| :--- | :--- | :--- | :--- |
| **Perihelion** | The Overall Project | Repository & Ecosystem | The umbrella open-source project, architecture, protocol, and virtual filesystem tooling suite. |
| **`peri`** | Headless CLI & Engine | `apps/peri` | The command-line orchestration utility and automation interface. |
| **`perid`** | Official Daemon Service | `apps/perid` | The official background daemon for P2P swarm seeding, cache monitoring, and background tasks. |
| **`helion`** | Interactive TUI Shell | `apps/helion` | The terminal-native interactive dashboard and keyboard-first management interface. |
| **Project Artwork** | Official Identity | `assets/` | The official banners, logos, and vector brand marks. |

---

## 3. Allowed and Encouraged Uses

You are welcome and encouraged to:

1. **State Compatibility & Origins:**
   - You may truthfully state that your software, modpack, profile, or script is compatible with, builds upon, or interfaces with Perihelion, `peri`, `perid`, or `helion`.
   - Examples of acceptable phrasing:
     - *"A tool for generating Perihelion manifests"*
     - *"Skyrim modpack blueprint formatted for peri"*
     - *"Third-party GUI frontend compatible with Perihelion"*
     - *"Custom client interacting with perid"*
     - *"Forked from Perihelion"* or *"Based on Perihelion"*

2. **Package Unmodified Upstream Releases:**
   - Package maintainers for Linux distributions (e.g. Arch Linux AUR, Fedora COPR, Debian, Nixpkgs, Alpine, Void, Flathub/Flatpak, Homebrew) are warmly invited to distribute packages or systemd units named `peri`, `perid`, `helion`, or `perihelion`, provided that:
     - The package builds directly from the official upstream repository (<https://github.com/dukasn/perihelion>) without alterations to project identity.
     - Distribution packaging patches (e.g. FHS path adjustments, systemd user unit definitions, compiler flags) are permitted.
     - The package metadata links directly to the upstream repository.

3. **Personal, Educational, and Commentary Use:**
   - You may freely use the names to discuss, review, write articles, create videos, or teach others about Perihelion.

4. **Use Under CC BY 4.0:**
   - You may share and adapt artwork from `assets/` under the terms of CC BY 4.0, provided you include the required attribution and do not violate the anti-impersonation rules below.

---

## 4. Prohibited Uses (Anti-Impersonation & Brand Confusion)

To protect users from confusion and security risks, the following uses are strictly prohibited:

1. **No Impersonation or False Claims of Maintenance:**
   - You may not represent or imply that your fork, build, service, website, or organization is the "official" version, is maintained by, or is endorsed by the upstream Perihelion project or its authors.

2. **Distinct Naming for Modified Forks:**
   - If you modify the codebase and distribute binaries, background services, or public services, you **must not** release them under the bare, unmodified names **"Perihelion"**, **"peri"**, **"perid"**, or **"helion"** in a way that confuses users about their origin.
   - You must either:
     - Choose a distinct name for your project (e.g. *"Starlight — a fork of Perihelion"*), or
     - Explicitly qualify the name (e.g. *"<YourName>'s fork of Perihelion"* or *"Community Build of peri for <Platform>"*).

3. **No Deceptive Primary Branding with Project Artwork:**
   - You may not use the official project logo (`assets/perihelion-logo.svg`) or banner (`assets/perihelion-banner.webp`) as the primary logo, avatar, or product identity of a separate fork, derivative tool, or commercial product without prior written approval.

4. **No Impersonation via Domains, Repositories, or Accounts:**
   - We do not own or police the word "Perihelion" in the abstract; anyone is free to register domain names, organization names, or accounts using the term for unrelated endeavors, personal use, or general astronomy.
   - However, you **may not** register or operate domain names, package names, code repositories, or social media handles specifically engineered to masquerade as the official presence of this software project (e.g., creating domains like `perihelion-official-download.com`, mimicking official GitHub organizations, or creating channels that falsely claim to be upstream maintainers).
   - Unofficial community forums, mirror sites, or documentation projects are welcome as long as they prominently state that they are an independent, unofficial community effort and are not affiliated with or endorsed by the upstream maintainers.

---

## 5. Summary Guidelines for Fork Authors

If you fork Perihelion:
- ✅ Keep copyright notices and GPLv3 licenses intact.
- ✅ State clearly in your `README.md` that your software is a derivative/fork.
- ✅ Choose a distinguishable name or clear qualifier.
- ❌ Do not publish binary releases named simply `peri`, `perid`, or `helion` if they contain third-party modifications not accepted upstream.
- ❌ Do not use the official Perihelion logos as your project's primary icon.

---

## 6. Questions & Clarifications

If you are unsure whether your planned use of the names "Perihelion", "peri", "perid", or "helion" is compliant with this policy, please open a discussion or contact the maintainers via the repository:
<https://github.com/dukasn/perihelion>

