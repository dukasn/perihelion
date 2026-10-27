# peri-core

Pure functional domain for Perihelion.

`peri-core` houses the deterministic domain logic of Perihelion. It parses mod manifests, solves version constraint trees, analyzes Virtual File Index (VFI) collisions, routes directory paths with pre-compiled radix tables, and matches declarative game engine signatures.

No system calls live here. No async runtime runs here. No network requests exist here. Functions take immutable inputs and return pure data structures or typed domain errors.

---

## Architectural Invariants

- **Zero Side Effects:** Never import `tokio`, `reqwest`, `nix`, `tar`, or direct filesystem mutation modules into this crate.
- **Deterministic Outcomes:** Given identical manifests, profiles, and signatures, the solvers and collision engines produce identical dependency graphs, VFS mount paths, and load orders on every run.
- **Zero-Allocation Routing:** Path transformations and collision hashing operate over borrowed slices and case-folded byte streams, avoiding heap allocations in high-volume mod asset trees.
- **In-Memory Verification:** Side effects stay strictly quarantined outside this crate. Every test executes purely in memory without spinning up scratch directories or teardown hooks.

---

## Core Modules

| Module | Responsibility |
| :--- | :--- |
| [`manifest`](./src/manifest.rs) | Parses and validates `peri.toml` files, covering package metadata, target game specifications, asset pointers, and dependency requirements. |
| [`solver`](./src/solver.rs) | Solves version requirements using the PubGrub constraint-satisfaction algorithm with SemVer ranges. |
| [`graph`](./src/graph.rs) | Constructs Directed Acyclic Graphs (`petgraph`) for mod dependency trees and load orders. Checks for cycles and extracts topological orders. |
| [`path`](./src/path.rs) | Zero-copy path normalization (`NormalizedPath`), pre-compiled radix layout prefix routing (`FastPathRouter`), and 64-bit case-folded hashing (`hash_normalized_path`). |
| [`signature`](./src/signature.rs) | Declarative engine layout signatures (`EngineSignature`) and registry (`SignatureRegistry`) for engine detection via structural anchors and markers. |
| [`vfi`](./src/vfi.rs) | Virtual File Index. Detects file-level collisions across mod asset trees and executes the 3-Tier Resolution Waterfall. |
| [`profile`](./src/profile.rs) | Game layout profiles (`GameProfile`) bridging mod directory trees to virtual mount targets with `FastPathRouter`. |
| [`lockfile`](./src/lockfile.rs) | Serializes and parses `peri.lock` files to pin exact package versions and archive checksums. |
| [`draft`](./src/draft.rs) | In-memory session draft state tracking staged user edits before committing to disk. |
| [`security`](./src/security.rs) | Enforces path safety rules, blocking directory traversal attacks (`..` escapes) and absolute path injections. |
| [`config`](./src/config.rs) | Data models for global configuration (`config.toml`), platform providers, and engine signatures. |
| [`error`](./src/error.rs) | Typed domain errors (`ManifestError`, `ResolveError`, `ProfileError`, `ConfigError`, `SecurityError`). |

---

## Declarative Engine Signatures & Path Routing

Mod packages structure their files differently across game engines. Bethesda titles bundle assets inside `Data/` alongside plugins (`.esm`, `.esp`), while REDengine games separate raw scripts into `r6/scripts/` and archives into `archive/pc/mod/`. Unreal Engine packages mod files into `Content/Paks/~mods/`.

Instead of hardcoding game paths into Rust binaries, `peri-core` drives engine discovery and path transformation through pure declarative structures:

```text
┌────────────────────────────────────────────────────────┐
│             SignatureRegistry (Pure Domain)            │
│  • Structural anchors (all must match)                 │
│  • Disambiguation markers (at least one must match)    │
│  • VFS prefix mappings (from -> to)                    │
└───────────────────────────┬────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────┐
│             FastPathRouter (Radix Table)               │
│  • Rules sorted descending by from_prefix length       │
│  • O(k) longest-prefix evaluation (k = path depth)     │
│  • Direct mapping without intermediate allocations     │
└────────────────────────────────────────────────────────┘
```

1. **`EngineSignature` Matching:** Evaluates abstract probes against directory structural anchors and file markers. When an engine matches, it supplies binary candidates, environment passthroughs, and layout mappings.
2. **`NormalizedPath`:** Single-pass path sanitizer. Converts Windows backslashes (`\`) to POSIX slashes (`/`), trims redundant markers (`./`), and rejects parent traversal (`..`) attempts on sight.
3. **`FastPathRouter`:** Compiles prefix mapping rules and sorts them descending by prefix length. Evaluates the longest matching directory prefix first in $O(k)$ time, replacing linear string comparisons across large mod file collections.

---

## The 3-Tier Resolution Waterfall

When two or more mods package files that map to the identical relative path in a game directory, `peri-core` resolves the clash using a strict three-tier cascade:

```text
[File Collision Detected in VFI]
               │
               ▼
   Tier 1: Explicit Edge?
   Does mod A require or load after mod B in the DAG?
         ├── Yes ──► Dependent mod wins.
         └── No
               │
               ▼
   Tier 2: Priority Weight?
   Compare effective priority weights (lowest < ... < absolute)
         ├── Different ──► Higher priority wins. Loser -> Winner edge added.
         └── Equal
               │
               ▼
   Tier 3: Hard Conflict!
   Fail deterministically with ResolveError::HardConflict.
   User must declare an override rule in peri.toml.
```

1. **Tier 1 (Explicit Edge):** If mod A explicitly declares `requires` or `loads_after` targeting mod B, the dependent mod takes precedence.
2. **Tier 2 (Priority Weight):** When no edge exists, the solver compares priority levels: `lowest` < `low` < `default` < `high` < `highest` < `absolute`. The higher-weight mod wins, and an implicit edge links the loser to the winner in the DAG. Workspace `[overrides]` supersede package-level defaults.
3. **Tier 3 (Hard Conflict):** When two mods share the same priority and lack explicit ordering constraints, the solver stops. It emits `ResolveError::HardConflict`, detailing colliding file paths and conflicting package IDs.

---

## Example Usage

### Manifest Parsing & DAG Sorting

```rust
use peri_core::{DependencyGraph, Manifest};

let manifest_text = r#"
[package]
name = "cyber-tweaks"
version = "1.0.0"
priority = "high"

[target]
game_id = "cyberpunk_2077"
"#;

let manifest = Manifest::from_toml_str(manifest_text)?;
assert_eq!(manifest.package.name, "cyber-tweaks");

let mut graph = DependencyGraph::new();
graph.add_manifest(manifest);
let load_order = graph.topological_sort()?;
```

### Path Normalization & Prefix Routing

```rust
use peri_core::path::{FastPathRouter, NormalizedPath};
use peri_core::profile::LayoutMapping;
use std::path::PathBuf;

let mappings = vec![
    LayoutMapping::new("mods", "archive/pc/mod"),
    LayoutMapping::new("redscript", "r6/scripts"),
];

let router = FastPathRouter::compile(&mappings);

let input = NormalizedPath::from_str(r"mods\visuals\hud.archive").unwrap();
let routed = router.route(&input);
assert_eq!(routed, PathBuf::from("archive/pc/mod/visuals/hud.archive"));
```
