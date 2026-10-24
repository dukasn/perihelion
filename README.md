# Perihelion

**Perihelion** is a next-generation, game-agnostic, native Linux package manager and virtual file system (VFS) orchestrator.

By leveraging kernel-level OverlayFS and isolated user/mount namespaces, decentralized distribution (BitTorrent, HTTP), and standardized compression (`.peca`), Perihelion provides a sandboxed and scriptable CLI (`peri`) for managing modlists across supported games.

---

## Workspace Structure

The project follows the "Functional Core, Imperative Shell" (Ports and Adapters) architecture:

```text
perihelion/
├── Cargo.toml                 # Workspace root & unified dependencies
├── .lefthook.yml              # Pre-commit hooks (fmt, clippy, test)
├── mise.toml                  # Toolchain pinning
├── apps/
│   ├── peri/                  # (Runtime) The CLI executable (`clap`, `miette`, `tokio`)
│   └── (helion/)              # (Future) The GUI executable (`egui`)
├── crates/
│   ├── peri-vfs/              # (Adapter) Linux namespaces, OverlayFS (`nix`)
│   ├── peri-net/              # (Adapter) BitTorrent P2P & HTTP streaming
│   └── peri-archive/          # (Adapter) .peca archives (`tar`, multi-threaded `zstd`)
└── common/
    └── peri-core/             # (Domain) Manifests, DAG solving (`petgraph`, `serde`)
```

---

## Quick Start

### Build & Test

```bash
# Run test suite across workspace
cargo test --workspace

# Run clippy linter
cargo clippy --workspace --all-targets -- -D warnings

# Build the peri CLI
cargo build --release -p peri
```

### CLI Commands (`peri`)

| Command | Action |
| :--- | :--- |
| `peri init <game_id>` | Initializes a Perihelion workspace mapped to a specific game profile |
| `peri get <uri>` | Fetches a mod/modpack, parses its manifest, and adds it to the VFS cache |
| `peri sync` | Downloads, unpacks, and validates the entire dependency tree |
| `peri package` | Compiles a local mod directory into a validated `.peca` archive |
| `peri check` | Runs global dependency DAG checks and validates file integrity |
| `peri run` | Spawns the namespace, constructs the VFS layer, and executes the game |

