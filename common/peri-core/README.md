# peri-core

Pure functional domain for Perihelion.

`peri-core` holds the deterministic domain logic of Perihelion. It parses mod manifests, builds dependency graphs, and runs constraint solvers over mod version trees.

No system calls live here. No async runtime runs here. No network requests exist here. Functions take immutable input and return pure data or structured domain errors.

---

## Architectural Invariants

- **Zero Side Effects:** Never import `tokio`, `reqwest`, `nix`, `tar`, or direct filesystem mutation modules into this crate.
- **Deterministic Outcomes:** Given identical manifests and game profiles, the solver and collision index produce identical dependency trees and load orders every run.
- **In-Memory Testing:** Because the domain isolates all side effects, tests execute in memory without filesystem setup or teardown.

---

## Core Modules

| Module | Responsibility |
| :--- | :--- |
| [`manifest`](./src/manifest.rs) | Parses and validates `peri.toml` files, covering package metadata, target game specifications, asset pointers, and dependency requirements. |
| [`solver`](./src/solver.rs) | Solves version requirements using the PubGrub constraint-satisfaction algorithm with SemVer ranges. |
| [`graph`](./src/graph.rs) | Constructs Directed Acyclic Graphs (`petgraph`) for mod dependency trees and load orders. Checks for cycles and extracts topological orders. |
| [`vfi`](./src/vfi.rs) | Virtual File Index. Discovers file-level path collisions across mod asset trees and runs the 3-Tier Resolution Waterfall. |
| [`profile`](./src/profile.rs) | Maps game-specific directory layouts (e.g. `bin/x64/`, `archive/pc/mod/`) to virtual mount points. |
| [`lockfile`](./src/lockfile.rs) | Serializes and parses `peri.lock` files to pin exact package versions and archive checksums. |
| [`draft`](./src/draft.rs) | In-memory session draft state tracking staged user edits before writing to disk. |
| [`security`](./src/security.rs) | Enforces path safety rules, blocking directory traversal attacks (`..` escapes) and absolute path injections. |
| [`config`](./src/config.rs) | Data models for global Perihelion runtime configuration (`~/.config/perihelion/config.toml`). |
| [`error`](./src/error.rs) | Domain error types (`ManifestError`, `ResolveError`, `ProfileError`, `ConfigError`, `SecurityError`). |

---

## The 3-Tier Resolution Waterfall

When two or more mods package files that map to the identical relative path in a game directory, `peri-core` resolves the clash using a strict three-tier cascade:

```
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

```rust
use std::collections::HashMap;
use peri_core::{Manifest, DependencyGraph, VirtualFileIndex};

// Parse manifest data
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

// Build a graph from resolved packages
let mut graph = DependencyGraph::new();
graph.add_mod(&manifest);
let load_order = graph.topological_sort()?;
```
