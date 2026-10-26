# Perihelion

<div align="center">
  <img src="./assets/perihelion-banner.webp" alt="Perihelion Banner - Mod Fearlessly. Share Instantly." width="auto" height="auto"/>
  <p>
    <em>A next-generation, game-agnostic native Linux mod manager and virtual file system orchestrator.</em>
  </p>
</div>

---

Traditional game modding on Linux is full of friction. Players run heavy Windows tools through Wine wrappers, endure throttled download caps, and watch mod managers unpack loose files directly into game folders. One broken overwrite can corrupt an entire install.

Perihelion is built from the ground up as a next-generation Linux mod manager.

Rather than mutating game directories, it brings modern software principles—declarative lockfiles, constraint solving, and filesystem virtualization—into mod curation. The goal is simple: make modding fast, crash-resilient, and effortless to share across any Linux gaming setup.

Vanilla game files stay read-only. Mods exist as isolated, versioned layers composed on the fly inside Linux kernel namespaces. When you launch, the game sees a fully modded tree; on disk, the base directory remains completely untouched.

| Traditional Modding | Perihelion |
| :--- | :--- |
| Overwrites game files directly on disk | Vanilla game installs stay strictly read-only |
| Fragile, manual 40-step install guides | Declarative lockfiles (`peri.lock`) synced in one command |
| Throttled 2 MB/s downloads behind paywalls | Decentralized P2P swarms streaming at line speed |
| Re-extracting 80 GB setups to switch profiles | Instant profile switching via virtual layers |
| Game-specific Windows GUIs layered over Wine | Game-agnostic engine native to Linux and Steam Deck |

---

## What Perihelion Enables

### Players & Communities
- **Untouched Game Files:** Stock game files stay strictly read-only. Test heavy overhauls or risky mods without risking your base install.
- **Instant Profile Swapping:** Jump between vanilla saves and 400-mod setups in seconds—virtual layers swap with zero disk copying.
- **Native Steam Deck & Proton:** Runs inside unprivileged Linux namespaces. No root rights, no Wine-wrapped mod managers, and no .NET crashes.
- **Any Title Supported:** Add first-class modding to any Linux game through a 15-line declarative layout profile.

### Modpack Authors & Curators
- **Modlists as Code:** Share complete mod setups as lightweight blueprints (`peri.toml` and `peri.lock`).
- **Swarm Distribution:** Distribute overhauls over BitTorrent swarms (`.peca`). No bandwidth costs, and no throttled 2 MB/s caps.
- **Zero Load-Order Drift:** The 3-tier DAG solver catches file collisions and ordering bugs before anything mounts.

### Contributors & Tool Builders
- **Pure Domain Engine:** `peri-core` keeps solver and graph logic free of disk or network I/O, ensuring fast, deterministic tests.
- **Decoupled Public Crates:** Build custom tools using `peri-vfs` for OverlayFS sandboxes, `peri-archive` for streaming `.peca` unpacks, and `peri-net` for swarm transport.
- **Fast Terminal Cockpit:** The `helion` TUI pairs a 60 FPS event loop with non-blocking background worker threads.

---

## The Suite

```text
perihelion/
├── apps/
│   ├── peri/             # Scriptable CLI shell for headless setups and automation
│   ├── perid/            # Background daemon for community P2P swarm seeding
│   └── helion/           # Keyboard-first interactive terminal dashboard
├── crates/
│   ├── peri-vfs/         # Linux namespaces, OverlayFS stacks, and Proton discovery
│   ├── peri-archive/     # Multi-threaded .peca (tar.zst) streaming engine
│   └── peri-net/         # BitTorrent swarm transport and HTTP streaming
└── common/
    └── peri-core/        # Pure domain: manifests, PubGrub solver, and VFI conflict logic
```

### 1. `helion` — The Interactive Terminal Dashboard
A keyboard-driven dashboard for live workspace curation:
- **Fast Navigation:** Full keyboard control with vim keys (`j`/`k`), tab cycling, and context drawers.
- **Live Collision Inspector:** Side-by-side file collision view showing winning layers and instant override toggles.
- **P2P Swarm Monitor:** Live transfer speeds, swarm health indicators, and background download queues.

![Helion Interactive TUI Dashboard](./assets/demos/helion-dashboard.gif)

Launch `helion` in any workspace:
```bash
cargo run -p helion
```

### 2. `peri` — The Headless CLI
High-performance command-line utility for scripting, headless setups, and automated testing:

![Peri CLI Quickstart](./assets/demos/peri-quickstart.gif)

```bash
# Initialize a workspace mapped to Cyberpunk 2077
peri init cyberpunk_2077

# Fetch an overhaul archive via magnet link or HTTP mirror
peri get "magnet:?xt=urn:btih:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855&dn=cyber_overhaul.peca"

# Solve dependency DAG, verify SHA-256 hashes, and sync layers
peri sync

# Spawn unprivileged mount namespace and run the game
peri run
```

### 3. `perid` — The Background Seeder
A lightweight background daemon that seeds cached `.peca` packages back to the community swarm:
- **Bandwidth Friendly:** Configurable upload caps via `--upload-limit` or user configuration files.
- **Headless Service:** Runs as a standard background process or through systemd user units (`scripts/perid.service`).

---

## Workspace Crates & Documentation

Every crate is documented with architecture guides, invariants, and API examples:

| Crate | Path | Responsibility | Documentation |
| :--- | :--- | :--- | :--- |
| **`peri-core`** | [`common/peri-core`](./common/peri-core) | Manifest models, PubGrub solver, Petgraph DAG, Virtual File Index | [README](./common/peri-core/README.md) |
| **`peri-vfs`** | [`crates/peri-vfs`](./crates/peri-vfs) | Unprivileged namespaces, OverlayFS ordering, Proton launch wrappers | [README](./crates/peri-vfs/README.md) |
| **`peri-archive`** | [`crates/peri-archive`](./crates/peri-archive) | `.peca` archive packaging, streaming decompression, path sanitization | [README](./crates/peri-archive/README.md) |
| **`peri-net`** | [`crates/peri-net`](./crates/peri-net) | BitTorrent swarm downloads (`librqbit`), HTTP streams, sync bridges | [README](./crates/peri-net/README.md) |
| **`helion`** | [`apps/helion`](./apps/helion) | Terminal UI engine, actor supervisors, async background pipelines | [README](./apps/helion/README.md) |
| **`perihelion`** | [`apps/perihelion`](./apps/perihelion) | User-facing binaries: `peri` (CLI), `perid` (Daemon), and `helion` (TUI runner) | [README](./apps/perihelion/README.md) |

---

## Community Game Profiles

Adding support for a new game requires no C++ extensions or engine patches. Game profiles are declarative definitions mapping package file patterns to game engine directories:

```toml
[profile]
game_id = "starfield"
name = "Starfield"
executable = "Starfield.exe"

[[mappings]]
pattern = "Data/*"
target = "Data"

[[mappings]]
pattern = "*.esm"
target = "Data"
```

When `peri run` constructs the virtual stack, it evaluates these rules to position mods into their correct virtual paths.

> [!TIP]
> Want to support a new title? Check [`common/peri-core/src/profile.rs`](./common/peri-core/src/profile.rs) to see existing layout patterns or submit a community profile pull request.

---

## Development & Verification

Perihelion enforces a strict zero-warning policy across all workspace targets:

```bash
# 1. Run workspace unit and integration tests
cargo test --workspace

# 2. Strict linter checks (zero warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Formatting check
cargo fmt --all --check

# 4. CLI end-to-end sanity check
cargo run --bin peri -- --help
```

> [!NOTE]
> Review [`CONTRIBUTING.md`](./CONTRIBUTING.md) for architectural invariants, Flatpak environment quirks, and pull request guidelines.

---

## Policies & Governance

- **[Contributing Guidelines](./CONTRIBUTING.md):** Architectural boundaries, development workflows, and PR expectations.
- **[Authenticity & Naming Policy](./TRADEMARK.md):** Guidelines for packaging, distribution, and forks.
- **[Security Policy](./SECURITY.md):** Vulnerability disclosure and sandbox threat model.
- **[Code of Conduct](./CODE_OF_CONDUCT.md):** Contributor standards and pledge.

---

## License

<div align="center">
  <p>
    <a href="./LICENSE">
      <img src="./gplv3.webp" alt="GNU General Public License v3.0" height="72"/>
    </a>
    &nbsp;&nbsp;&nbsp;&nbsp;
    <a href="./LICENSE-CC-BY-4.0">
      <img src="./cc.by.png" alt="Creative Commons Attribution 4.0 International" height="72"/>
    </a>
  </p>
  <p>
    <a href="./LICENSE"><strong>Software licensed under GPL-3.0-or-later</strong></a>
    &nbsp;&bull;&nbsp;
    <a href="./LICENSE-CC-BY-4.0"><strong>Branding &amp; media licensed under CC-BY-4.0</strong></a>
  </p>
</div>
