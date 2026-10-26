# Security Policy

## Supported Versions

Perihelion is under active development. Security updates are applied to the latest `main` branch.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| < 0.1.0 | :x:                |

## Security Scope & Threat Model

Perihelion employs Linux unprivileged user and mount namespaces (`CLONE_NEWUSER`, `CLONE_NEWNS`), OverlayFS compositions, path boundary validators, and P2P BitTorrent streaming.

Key security focus areas include:
- **Filesystem Boundary Isolation:** Ensuring untrusted mod archives (`.peca`) or path mappings cannot escape designated sandbox paths, mutate the base game directory, or perform path traversal (`../`).
- **Unprivileged Namespaces:** Ensuring isolation mechanisms do not rely on elevated privileges (`root`/`sudo`) or expose namespace escape vulnerabilities.
- **Archive Extraction & In-Memory Inspection:** Ensuring corrupted or malicious `.tar.zst` files cannot cause memory corruption, denial-of-service, or arbitrary file writes.
- **P2P Swarm Security:** Ensuring BitTorrent torrent streaming validates SHA-256 piece hashes and does not execute untrusted network payloads.

## Reporting a Vulnerability

If you discover a security vulnerability in Perihelion, please report it responsibly:

1. **Do not disclose the vulnerability in public issues, discussions, or pull requests.**
2. Send an advisory report via GitHub's private vulnerability reporting feature on the repository:
   <https://github.com/dukasn/perihelion/security/advisories/new>
3. Please include:
   - A description of the vulnerability and its potential impact.
   - Minimal reproduction steps, sample manifest, or proof of concept.
   - Your proposed remediation, if available.

We will acknowledge receipt of your report within 48 hours and work with you on a timeline for a coordinated disclosure and patch release.

