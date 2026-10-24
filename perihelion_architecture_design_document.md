# Project Perihelion: Architecture & Design Document

**Status:** v1.0 Draft  
**Target Platform:** Linux (Native)  
**Tech Stack:** Rust, Tokio, OverlayFS, BitTorrent (`librqbit`), Zstandard, `petgraph`  

---

## 1. Executive Summary

**Perihelion** is a next-generation, game-agnostic, native Linux package manager and virtual file system (VFS) orchestrator. It is designed to modernize game modding by replacing heavy, GUI-bound, game-specific managers with a fast, universal, headless engine.

By leveraging native Linux features (OverlayFS, Mount Namespaces), decentralized distribution (BitTorrent, generic HTTP), and standardized compression, Perihelion provides a deeply sandboxed and scriptable CLI (`peri`) for managing massive modlists across *N* supported games.

### 1.1 Core Goals
* **Game-Agnostic Engine:** The core engine knows nothing about specific games. Game support is driven by declarative VFS layout profiles.
* **Zero-Overhead Sandboxing:** Use kernel-level OverlayFS to construct target game directories without modifying base game files.
* **The `.peca` Standard:** Establish the Perihelion Compressed Archive (`.peca`)—a blazing-fast `.tar.zst` format with embedded metadata—as the universal distribution standard.
* **Infrastructure as Code (IaC):** Treat modlists as version-controlled code via declarative `peri.toml` manifests, enabling CI/CD automated validation.
* **Decentralized Distribution:** Shift hosting burdens away from centralized APIs using P2P (BitTorrent) with WebSeed fallbacks.

### 1.2 Non-Goals (For v1)
* A Graphical User Interface (GUI). Focus is strictly on the `peri` CLI MVP.
* Native Windows support (relies on Linux-specific namespaces/OverlayFS).

---

## 2. System Architecture (The DAG Workspace)

The project utilizes a strict Cargo Workspace DAG (Directed Acyclic Graph) following the "Functional Core, Imperative Shell" (Ports and Adapters) pattern. This guarantees testability, prevents cyclic dependencies, and ensures a trivial integration with future GUIs.

```text
perihelion/
├── Cargo.toml                 # Workspace root
├── .lefthook.yml              # Pre-commit hooks (fmt, clippy, nextest)
├── mise.toml                  # Toolchain pinning
├── apps/
│   ├── peri/                  # (Runtime) The CLI executable (`clap`, `miette`)
│   └── (helion/)              # (Future) The GUI executable (`egui`)
├── crates/
│   ├── peri-vfs/              # (Adapter) Linux namespaces, OverlayFS (`nix`)
│   ├── peri-net/              # (Adapter) `librqbit` P2P, `reqwest` HTTP
│   └── peri-archive/          # (Adapter) `tar`, `zstd` streaming extraction
└── common/
    └── peri-core/             # (Domain) Manifests, Graph solving (`petgraph`, `serde`)
```

### 2.1 The Domain (`common/peri-core`)
**Rules:** Pure functional logic. No I/O, no async runtimes, no side effects.
* **Responsibilities:**
    * Parsing `peri.toml` manifests into strictly typed `serde` structs.
    * Solving Directed Acyclic Graphs (DAGs) for load orders and dependencies using `petgraph`.
    * Mapping generalized mod files to specific game layouts.

### 2.2 The Adapters (`crates/peri-*`)
**Rules:** Depend only on `common/peri-core`. 
* **`peri-vfs`:** Handles system calls to spawn isolated mount namespaces and mount OverlayFS stacks.
* **`peri-net`:** Handles fetch operations.
* **`peri-archive`:** Handles the fast extraction and creation of `.peca` files.

### 2.3 The Runtimes (`apps/*`)
**Rules:** The thin integration shell.
* **Responsibilities:** Argument parsing, initializing `tokio`, orchestrating the adapters, and rendering rich diagnostic outputs.

---

## 3. The Standard: Perihelion Compressed Archive (`.peca`)

To ensure seamless distribution without alienating Linux power users, Perihelion enforces the `.peca` format (a "Trojan Horse" archive).

* **Under the Hood:** A `.peca` file is strictly a Zstandard-compressed Tarball (`.tar.zst`).
* **The Rule:** A valid `.peca` file **must** contain a `peri.toml` manifest at its root directory.
* **Advantages:**
  1. Blazing fast multi-threaded decompression via `zstd`.
  2. Allows streaming network extraction directly into the VFS cache.
  3. Fully interoperable with standard Linux archive managers (`Ark`, `File Roller`).

---

## 4. Data Models: The Universal `peri.toml` Manifest

The manifest is game-agnostic, defining both the target environment and the distribution network.

```toml
[package]
name = "Cyber Engine Tweaks"
author = "yamashi"
version = "1.30.0"

[target]
# Defines which game profile the VFS should use to mount this mod
game_id = "cyberpunk_2077"
engine_version = ">=2.12"

[assets]
# Primary P2P distribution
magnet = "magnet:?xt=urn:btih:123456789abcdef&dn=cet_1.30.0.peca&ws=https://example.com/cet.peca"
# Fallback / Direct HTTP
url = "https://github.com/maximegmd/CyberEngineTweaks/releases/download/v1.30.0/cet.peca"
sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

[dependencies]
# Global dependency resolution
"github.com/author/some-framework" = ">=2.0"

[overrides]
# Explicit file conflict resolution
"bin/x64/plugins/cyber_engine_tweaks.asi" = "self"
```

---

## 5. Dependency Management & Load Order (The DAG)

Load orders and dependencies are mathematically proven using `petgraph` inside `peri-core`.

1. **Cycle Detection:** Before mounting, the engine runs `is_cyclic_directed()`. If mod authors create a circular dependency (A requires B requires A), `peri` halts execution and pinpoints the loop.
2. **Topological Sorting:** If acyclic, `toposort()` resolves all `Requires`, `LoadsAfter`, and `IncompatibleWith` rules into a flat, deterministic array. This array dictates the exact sequence of OverlayFS layers.

---

## 6. CLI Interface (`peri` v1 MVP)

Built with declarative `clap`.

| Command | Action |
| :--- | :--- |
| `peri init <game_id>` | Initializes a Perihelion workspace mapped to a specific game profile. |
| `peri get <uri>` | Fetches a mod/modpack, parses its manifest, and adds it to the VFS cache. |
| `peri sync` | Downloads, unpacks, and validates the entire dependency tree. |
| `peri package` | Compiles a local mod directory into a validated `.peca` archive. |
| `peri check` | Runs global dependency DAG checks and validates file integrity. |
| `peri run` | Spawns the namespace, constructs the VFS layer, and executes the game. |

---

## 7. Observability & Issue Handling

* **Domain Errors (`thiserror`):** Strict, programmatic error enums defined in `peri-core` (e.g., `ResolveError::CyclicDependencyDetected`).
* **User Diagnostics (`miette`):** Used at the CLI boundary to translate domain errors into actionable, compiler-style terminal outputs (highlighting specific lines in the `peri.toml`).
* **Logging (`tracing`):** Structured, span-based telemetry for debugging concurrent network and filesystem operations. Decouples state from UI entirely.

---

## 8. Future Milestones (v2 and beyond)

* **`helion` GUI:** An `egui`-based native application that provides a TUI-inspired, data-dense interface utilizing the same `peri-core` domain.
* **Perihelion Daemon (`perid`):** A background service enabling users to continuously seed cached mods to the P2P network.
* **Advanced SAT Solving:** Transition from simple graph sorting to a full SAT solver (`pubgrub`) for complex version constraint resolution.