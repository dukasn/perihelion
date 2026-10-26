# Contributing to Perihelion

Thank you for your interest in contributing to **Perihelion**! 

Perihelion is a next-generation, game-agnostic native Linux mod manager and Virtual File System (VFS) orchestrator designed to make game modding declarative, non-destructive, and lightning fast.

This document provides guidelines, architectural principles, licensing requirements, and development workflows to help you make successful contributions.

---

## 1. Project Organization & Architecture

Perihelion follows a strict **"Functional Core, Imperative Shell"** pattern:

```text
perihelion/
├── apps/
│   ├── peri/             # Headless CLI shell (clap, miette, tracing, tokio)
│   ├── perid/            # Background orchestration daemon
│   └── helion/           # Terminal-native interactive TUI dashboard (ratatui, crossterm)
├── crates/
│   ├── peri-vfs/         # Adapter: Linux OverlayFS & unprivileged namespaces (nix)
│   ├── peri-archive/     # Adapter: .peca archives (tar, multi-threaded zstd)
│   └── peri-net/         # Adapter: P2P BitTorrent (librqbit) & HTTP streaming (reqwest)
└── common/
    └── peri-core/        # Pure Domain: manifests, DAG solver, VFI, profiles (NO I/O)
```

### Component Roles & Naming
- **Perihelion**: The overarching open-source project and virtual filesystem ecosystem.
- **`peri`**: The headless CLI orchestration utility (`apps/perihelion/src/bin/peri`).
- **`perid`**: The official daemon background service (`apps/perihelion/src/bin/perid`).
- **`helion`**: The keyboard-first interactive TUI dashboard (`apps/helion`).

> [!IMPORTANT]
> Please review our [Project Naming & Authenticity Policy](TRADEMARK.md). While we encourage community participation and forks, we require that third-party distributions and forks clearly identify themselves and do not claim to be the official upstream release.

---

## 2. Immutable Architectural Invariants

When working on the codebase, you must preserve these four core invariants:

### 🚨 Invariant A: Pure Functional Domain (`common/peri-core`)
- **NO I/O, NO Async, NO Syscalls:** `peri-core` must **NEVER** depend on `tokio`, `reqwest`, `nix`, `tar`, or direct filesystem operations.
- It contains only deterministic, reproducible domain logic: manifest parsing (`serde`, `toml`), graph resolution (`petgraph`, `pubgrub`), Virtual File Index conflict analysis (`vfi.rs`), security path validation, and declarative layout profiles.
- All network, disk, or kernel namespace operations belong strictly in `crates/peri-*` or `apps/*`.

### 🚨 Invariant B: OverlayFS Ordering Mechanics
- Linux OverlayFS parses `lowerdir` from left-to-right: `lowerdir=topmost_layer:...:lowest_layer:base_game`.
- If Mod B depends on Mod A (or Mod B overrides Mod A), Mod B must appear higher in the VFS stack.
- Therefore, in `VfsPlan::format_mount_options()`, the topological order is reversed so that dependents and highest-priority mods appear **first** in the `lowerdir` mount string.

### 🚨 Invariant C: The 3-Tier Resolution Waterfall
When file collisions are detected across mods via the Virtual File Index (VFI), resolve them in this exact order:
1. **Tier 1 (Explicit Edge):** If an explicit `requires` or `loads_after` rule exists between the mods in the DAG, the dependent mod wins.
2. **Tier 2 (Priority Weight):** If no explicit edge exists, compare effective priorities (`lowest` < `low` < `default` < `high` < `highest` < `absolute`). The higher priority mod wins and dynamically receives an edge (`loser -> winner`) in the DAG. Workspace `[overrides]` take precedence.
3. **Tier 3 (Hard Conflict):** If both mods have the same priority and no explicit edge, fail deterministically with `ResolveError::HardConflict`, prompting the user to declare a rule in `[overrides]`.

### 🚨 Invariant D: `.peca` Archive Standard
- A valid `.peca` file is strictly a multi-threaded Zstandard-compressed tarball (`.tar.zst`).
- A `.peca` archive **must** contain a valid `peri.toml` manifest at its root directory.
- `peri-archive::read_manifest_from_peca` must inspect the manifest without decompressing the entire archive to disk.

---

## 3. Licensing of Contributions

Perihelion uses a split-licensing model depending on the contribution type:

1. **Source Code & Tooling:**
   - All code contributions (Rust, shell scripts, build configurations, manifests) are licensed under the **GNU General Public License, Version 3 (GPL-3.0-or-later)**.
   - See [LICENSE](LICENSE).

2. **Visual Assets & Media:**
   - All contributions to the `assets/` directory (logos, banners, vector illustrations, promotional graphics) are licensed under the **Creative Commons Attribution 4.0 International License (CC BY 4.0)**.
   - See [LICENSE-CC-BY-4.0](LICENSE-CC-BY-4.0).

### Developer Certificate of Origin (DCO)
By submitting a Pull Request or patch to this repository, you certify that:
- The contribution was created in whole or in part by you, and you have the right to submit it under the project's respective open-source licenses (GPLv3 for code, CC BY 4.0 for assets); **or**
- The contribution is based on previous work that is covered by an appropriate open-source license compatible with GPLv3 or CC BY 4.0, and you have preserved all required attribution notices; **and**
- You understand that your contribution will be publicly recorded and distributed under these open-source licenses.

---

## 4. Development Environment & Setup

### Quick Start (Automated Bootstrap)
You can configure and verify the complete toolchain in one command:
```bash
./scripts/bootstrap.sh
```
Or via [mise](https://mise.jdx.dev/):
```bash
mise run bootstrap
```
This automatically manages and pins:
- Stable Rust with `clippy` and `rustfmt`
- `sccache` compiler caching acceleration
- `lefthook` Git pre-commit hooks
- `cargo-deny` license & advisory checker
- `cargo-dist` & `release-plz` release packaging
- `vhs`, `ttyd` & `ffmpeg` terminal demo recording suite
- Headless browser sandbox compatibility wrappers for containerized/Flatpak environments

To verify toolchain health at any time:
```bash
./scripts/bootstrap.sh --check
```

### Environment Notes & Sandboxing
- If developing inside containerized or Flatpak development environments (e.g., VSCode Flatpak on Silverblue/Bazzite), **do not** use `flatpak-spawn --host` for compiler wrappers or `sccache`. `flatpak-spawn` strips cargo environment variables (`CARGO_PKG_VERSION`, `CARGO_MANIFEST_DIR`), which breaks proc-macro compilation.
- Ensure standard user paths are in your shell `PATH`:
  ```bash
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
  ```

---

## 5. Verification & Testing Playbook

Before submitting any Pull Request, ensure that all verification checks pass locally:

```bash
# 1. Run all workspace unit and integration tests
cargo test --workspace

# 2. Enforce strict clippy linter (zero warnings policy)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Check code formatting
cargo fmt --all --check

# 4. Check license headers on source files and SVG assets
python3 scripts/check_license_headers.py --check

# 5. End-to-end CLI sanity check
cargo run --bin peri -- --help

# 6. Background daemon sanity check
cargo run --bin perid -- --help

# 7. Helion TUI sanity check
cargo run -p helion
```

> [!TIP]
> We maintain a **zero-warning policy** for `cargo clippy`. Pull requests that introduce new clippy warnings will not pass CI.

---

## 6. Commit Message & PR Guidelines

### Conventional Commits
We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```text
<type>(<scope>): <short description>

[optional body]
```

#### Types:
- `feat`: A new feature or capability.
- `fix`: A bug fix.
- `refactor`: Code change that neither fixes a bug nor adds a feature.
- `perf`: A code change that improves performance.
- `test`: Adding missing tests or correcting existing tests.
- `docs`: Documentation only changes.
- `chore`: Maintenance tasks, dependencies, tooling updates.

#### Scopes:
- `(core)`: Changes to `common/peri-core`
- `(vfs)`: Changes to `crates/peri-vfs`
- `(archive)`: Changes to `crates/peri-archive`
- `(net)`: Changes to `crates/peri-net`
- `(peri)`: Changes to `apps/perihelion/src/bin/peri` CLI
- `(perid)`: Changes to `apps/perihelion/src/bin/perid` daemon
- `(helion)`: Changes to `apps/helion` TUI
- `(assets)`: Changes to visual assets or banners

#### Example:
```text
feat(helion): implement async download progress streaming via channels
fix(vfs): resolve mount order precedence in multi-layer overlay
docs: update layout mapping guide in README
```

---

## 7. Code of Conduct

We are committed to providing a welcoming, inclusive, and harassment-free community. All contributors and participants are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

Thank you for contributing to Perihelion!

