# Game Modding Guide & Tutorial

Mod managers historically treat game directories as mutable scratchpads. They extract loose zip files directly over vanilla data, overwrite shared textures, and leave orphaned files behind when uninstalled. One bad overwrite corrupts an entire installation.

Perihelion replaces destructive file mutation with declarative virtualization. Vanilla game files remain strictly read-only. Mods exist as isolated, versioned packages composed on the fly inside Linux kernel OverlayFS mounts and unprivileged user namespaces. You get reproducible setups, instant profile switching, and zero risk to your base game files.

This guide details the universal modding lifecycle for any game on Linux, followed by a concrete walkthrough using *The Elder Scrolls V: Skyrim Special Edition*.

---

## 1. Core Architecture & Sandboxing Model

Every Perihelion workspace mounts an ephemeral virtual filesystem constructed from three discrete layers:

```text
                  ┌────────────────────────────────────────┐
                  │          Running Game Process          │
                  │        (Native Binary or Proton)       │
                  └───────────────────┬────────────────────┘
                                      │
               Reads/Writes inside Mount Namespace (/mnt)
                                      ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Virtual File System                             │
├────────────────────────────────────────────────────────────────────────┤
│ Upper Layer (Read/Write)   │ .perihelion/upper/ (saves, INIs, logs)    │
├────────────────────────────┼───────────────────────────────────────────┤
│ Mod Layer N (Read-Only)    │ .perihelion/extracted/<mod_n>/            │
├────────────────────────────┼───────────────────────────────────────────┤
│ Mod Layer 1 (Read-Only)    │ .perihelion/extracted/<mod_1>/            │
├────────────────────────────┼───────────────────────────────────────────┤
│ Base Game (Read-Only)      │ Symlink to clean vanilla install          │
└────────────────────────────────────────────────────────────────────────┘
```

The layers stack according to resolved dependency order:
- **Base Game (Bottom):** Your untouched vanilla installation. It sits at the lowest layer of `lowerdir`.
- **Mod Layers (Middle):** Extracted `.peca` packages. Higher-priority mods sit to the left in the OverlayFS `lowerdir` string, overriding conflicting files from lower mods without touching them on disk.
- **Upper Layer (Top):** An isolated read/write directory (`.perihelion/upper/`). Runtime writes—configuration tweaks, generated caches, crash dumps—land here.
- **Namespace Boundary:** The game process runs inside an unprivileged mount and user namespace. Once the game exits, the virtual stack unmounts cleanly.

---

## 2. The Universal Modding Lifecycle

Regardless of game engine or genre, modding any title with Perihelion follows five distinct stages.

```
1. Profile Discovery ──▶ 2. Workspace Init ──▶ 3. Mod Packaging ──▶ 4. Graph Solve ──▶ 5. Sandboxed Run
   (Engine Mappings)        (peri.toml)           (.peca archives)       (peri.lock)         (Namespaces)
```

---

### Step 1: Define or Discover the Game Profile

Different game engines expect assets in different folder hierarchies. Unreal Engine packs mods into `Content/Paks/~mods`, REDengine 4 places them in `archive/pc/mod`, and Bethesda titles route files into `Data/`.

A **Game Profile** tells Perihelion how to translate a mod's archive paths into the target engine layout.

#### Option A: Auto-Discovery from Steam

If the game exists in your local Steam library, generate a profile using its Steam Application ID:

```bash
peri profile new <game_id> --from-steam <STEAM_APP_ID> --global
```

Perihelion scans standard Steam roots (native, Flatpak, Snap), identifies the install directory, inspects the files against known engine signatures, and automatically fills in:
- Executable binary paths
- Steam Application ID for Proton compatibility
- Prefix translation mappings (e.g., `mods` $\rightarrow$ `archive/pc/mod`)

#### Option B: Auto-Discovery from a Directory

If the game comes from GOG, Epic Games via Heroic, or a standalone installer, point Perihelion directly to its directory:

```bash
peri profile new <game_id> --game-path /path/to/game/root --global
```

Perihelion's heuristic engine detector scans directory anchors and file markers (`*_Data/Managed`, `Binaries/Win64`, `Data/*.esm`) to determine the underlying engine and apply appropriate mappings.

#### Option C: Manual Profile Creation

For bespoke engines or non-standard structures, create the profile explicitly:

```bash
peri profile new my_game \
  --name "My Custom Game" \
  --exec "bin/game.exe" \
  --map "textures:game/assets/textures" \
  --map "scripts:game/scripts" \
  --global
```

Inspect the configured profile at any time:

```bash
peri profile show <game_id>
```

---

### Step 2: Initialize a Modding Workspace

A workspace is an isolated directory holding your mod list, configuration overrides, and cached layers. It lives entirely separate from your game install.

```bash
mkdir -p ~/Games/Modding/<game_id>
cd ~/Games/Modding/<game_id>

peri init <game_id> --name "My Modded Build"
```

This generates:
- `peri.toml`: The declarative manifest tracking required packages, version bounds, priority rules, and explicit load orders.
- `.perihelion/`: Workspace state cache (`cache/mods`, `extracted`, `upper`, `work`, `mnt`).

Link your pristine vanilla game directory as `base_game`:

```bash
ln -s "/path/to/clean/game/installation" base_game
```

OverlayFS uses `base_game` as its bottom-most read-only lower layer.

---

### Step 3: Mod Packaging & The `.peca` Standard

Perihelion distributes mods as `.peca` archives. A valid `.peca` file is a multi-threaded Zstandard-compressed tarball (`.tar.zst`) with a valid `peri.toml` manifest situated at its root.

#### Structure of a `.peca` Mod

```text
my-texture-mod/
├── textures/
│   ├── terrain.dds
│   └── sky.dds
└── peri.toml
```

The mod's manifest (`my-texture-mod/peri.toml`) declares its identity, target game, and priority:

```toml
[package]
name = "high_res_terrain"
version = "1.2.0"
author = "Modder"
priority = "high"
description = "Overhauled 4K ground textures"

[target]
game_id = "my_game"

[dependencies]
texture_framework = ">=1.0.0"
```

#### Compiling with `peri package`

Pack the folder into a distributable archive:

```bash
peri package ./my-texture-mod -o ./dist/high_res_terrain-1.2.0.peca
```

Perihelion validates manifest syntax, checks target compatibility, indexes file paths, and compresses the archive using multi-threaded zstd.

---

### Step 4: Caching, Dependencies & Conflict Resolution

Import packages into the workspace cache via local paths, HTTP mirrors, or BitTorrent magnet links:

```bash
# Local package
peri get ./dist/high_res_terrain-1.2.0.peca

# Direct HTTP/HTTPS mirror
peri get https://mods.example.com/texture_framework-1.0.0.peca

# BitTorrent swarm link
peri get "magnet:?xt=urn:btih:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855&dn=texture_framework.peca"
```

Declare active mods in your workspace `peri.toml`:

```toml
[package]
name = "My Modded Build"
version = "0.1.0"
author = "Player"
priority = "default"

[target]
game_id = "my_game"

[dependencies]
texture_framework = "^1.0.0"
high_res_terrain = "^1.2.0"

[overrides]
high_res_terrain = { priority = "highest" }

[load_order]
after = [
    ["texture_framework", "high_res_terrain"]
]
```

#### The 3-Tier Resolution Waterfall

When multiple mods supply identical relative file paths, Perihelion resolves the collision through a strict three-tier waterfall:

1. **Tier 1 (Explicit Edges):** Rules specified in `[load_order]` take immediate precedence. Dependent mods sit higher in the stack.
2. **Tier 2 (Priority Weights):** If no edge exists, priorities compare: `lowest` < `low` < `default` < `high` < `highest` < `absolute`. The higher priority package wins.
3. **Tier 3 (Hard Conflicts):** Identical priorities with no explicit edge fail deterministically with `ResolveError::HardConflict`. The build refuses to mount until you specify a rule in `[overrides]`.

---

### Step 5: Verification and Synchronization

Verify workspace integrity:

```bash
peri check
```

`peri check` scans the dependency graph for cycles, missing packages, and unhandled file collisions.

Once verified, sync the virtual filesystem:

```bash
peri sync
```

`peri sync`:
1. Solves the dependency DAG with PubGrub.
2. Verifies SHA-256 checksums across all cached archives.
3. Unpacks missing layers into `.perihelion/extracted/`.
4. Writes a deterministic `peri.lock` file locking down exact versions and topological order.

---

### Step 6: Sandboxed Execution

Simulate the mount payload without touching namespaces or processes:

```bash
peri run --dry-run
```

The output details the exact OverlayFS `lowerdir` ordering, active layers, and Proton/Wine compatibility variables. Notice that higher-priority mods appear on the left of `lowerdir`:

```text
lowerdir=/path/to/extracted/high_res_terrain:/path/to/extracted/texture_framework:/path/to/base_game
```

Launch the game inside the isolated sandbox:

```bash
peri run
```

Or pass custom binary targets and command-line arguments:

```bash
peri run --exec bin/custom_launcher.exe -- --skip-intro +exec autoexec.cfg
```

---

## 3. Concrete Example: Modding Skyrim Special Edition

To see these principles in action, we walk through modding *The Elder Scrolls V: Skyrim Special Edition* using the Skyrim Script Extender (SKSE64) and SkyUI.

### The Skyrim SE Layout Problem
- Bethesda's Creation Engine expects all game assets (`.esp` plugins, `.bsa` archives, loose scripts, meshes) inside the `Data/` folder.
- Script extenders require native executable loaders (`skse64_loader.exe`) in the game root, alongside custom DLL plugins and scripts under `Data/SKSE/` and `Data/scripts/`.
- Traditional mod managers copy files directly into Steam's game folder. Updating or removing mods risks leaving stale scripts behind.

---

### Step 1: Generate the Skyrim Profile

Steam App ID for Skyrim Special Edition is `489830`. Generate the profile globally:

```bash
peri profile new skyrim_se --from-steam 489830 --global
```

Inspect the generated configuration:

```bash
peri profile show skyrim_se
```

Perihelion matches the `bethesda_creation` engine signature and outputs:

```toml
id = "skyrim_se"
name = "The Elder Scrolls V: Skyrim Special Edition"
steam_app_id = 489830
default_executable = "SkyrimSE.exe"

[[mappings]]
from = "Data"
to = "Data"

[[mappings]]
from = "SKSE"
to = "Data/SKSE"
```

---

### Step 2: Set Up the Skyrim Workspace

Create a dedicated workspace directory:

```bash
mkdir -p ~/Games/SkyrimWorkspace
cd ~/Games/SkyrimWorkspace

peri init skyrim_se --name "Nordic Overhaul"
```

Link your vanilla Skyrim install as `base_game`:

```bash
# Native Steam path:
ln -s "$HOME/.local/share/Steam/steamapps/common/Skyrim Special Edition" base_game

# Flatpak Steam path (if applicable):
# ln -s "$HOME/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/Skyrim Special Edition" base_game
```

---

### Step 3: Package SKSE64 and SkyUI

#### Packaging SKSE64

Unpack the official SKSE64 archive into a temporary folder:

```text
/tmp/skse_pkg/
├── skse64_loader.exe
├── skse64_1_6_1170.dll
├── skse64_steam_loader.dll
├── Data/
│   └── scripts/
│       ├── Actor.pex
│       └── ...
└── peri.toml
```

Write `/tmp/skse_pkg/peri.toml`:

```toml
[package]
name = "skse64"
version = "2.2.6"
author = "SKSE Team"
priority = "highest"
description = "Skyrim Script Extender 64-bit"

[target]
game_id = "skyrim_se"
```

Package the archive:

```bash
peri package /tmp/skse_pkg -o ~/Games/SkyrimWorkspace/skse64-2.2.6.peca
```

#### Packaging SkyUI

Unpack SkyUI:

```text
/tmp/skyui_pkg/
├── Data/
│   ├── SkyUI_SE.esp
│   └── SkyUI_SE.bsa
└── peri.toml
```

Write `/tmp/skyui_pkg/peri.toml`:

```toml
[package]
name = "skyui"
version = "5.2.0"
author = "SkyUI Team"
priority = "high"
description = "Interface overhaul for Skyrim SE"

[target]
game_id = "skyrim_se"

[dependencies]
skse64 = "^2.2.6"
```

Package the archive:

```bash
peri package /tmp/skyui_pkg -o ~/Games/SkyrimWorkspace/skyui-5.2.0.peca
```

---

### Step 4: Import and Configure Workspace

Cache the packages:

```bash
cd ~/Games/SkyrimWorkspace
peri get ./skse64-2.2.6.peca
peri get ./skyui-5.2.0.peca
```

Update `~/Games/SkyrimWorkspace/peri.toml`:

```toml
[package]
name = "Nordic Overhaul"
version = "0.1.0"
author = "Dovahkiin"
priority = "default"

[target]
game_id = "skyrim_se"

[dependencies]
skse64 = "^2.2.6"
skyui = "^5.2.0"

[overrides]
skse64 = { priority = "highest" }
skyui = { priority = "high" }

[load_order]
after = [
    ["skse64", "skyui"]
]
```

---

### Step 5: Sync and Launch via SKSE64

Verify and sync the workspace:

```bash
peri check
peri sync
```

Perihelion extracts `skse64` and `skyui` into `.perihelion/extracted/` and writes `peri.lock`.

Execute the game through the script extender loader:

```bash
peri run --exec skse64_loader.exe
```

Perihelion:
1. Spawns an unprivileged user namespace.
2. Composes the OverlayFS stack (`lowerdir=skyui:skse64:base_game`).
3. Auto-discovers your Steam Proton runner (Proton Experimental or GE-Proton).
4. Directs `WINEPREFIX` and `STEAM_COMPAT_DATA_PATH` to Skyrim's compatdata folder (`steamapps/compatdata/489830`).
5. Executes `skse64_loader.exe` inside the isolated mount.

The base game directory remains untouched throughout gameplay.

---

## 4. Visual Workflow with Helion (TUI Cockpit)

For live curation without editing TOML files by hand, launch `helion` in any workspace:

```bash
helion
```

```text
┌─ Helion: Nordic Overhaul [skyrim_se] ─────────────────────────────────────────────────┐
│ [1] Overview    [2] Mods (2)    [3] Profiles    [4] Swarm    [?] Help                 │
├───────────────────────────────────────────────────────────────────────────────────────┤
│ Active Layers: 2    Conflicts: 0    VFS: Ready    Status: Synced                      │
│                                                                                       │
│  [x] skse64        v2.2.6   [highest]   Skyrim Script Extender 64-bit                 │
│  [x] skyui         v5.2.0   [high]      Interface overhaul for Skyrim SE              │
│                                                                                       │
│ Load Order:                                                                           │
│   1. skse64 (highest)                                                                 │
│   2. skyui  (high)                                                                    │
├───────────────────────────────────────────────────────────────────────────────────────┤
│ [Space] Toggle   [+/-] Priority   [Shift+J/K] Reorder   [s] Sync   [l] Launch   [q] Quit│
└───────────────────────────────────────────────────────────────────────────────────────┘
```

### Essential Navigation & Shortcuts
- `1` / `2` / `3` / `4`: Switch tabs (**Overview**, **Mods**, **Profiles**, **Swarm**).
- `j` / `k` or `Up` / `Down`: Move cursor through mod lists or profile entries.
- `Space`: Toggle a mod active or inactive (stages a transactional draft).
- `+` / `-`: Increase or decrease mod priority weight.
- `Shift+J` / `Shift+K`: Reorder load order positions.
- `s` (in Profiles tab): Open the interactive Steam library importer.
- `g` (in Profiles tab): Open the custom Game Profile wizard.
- `Ctrl+S`: Commit staged changes directly to `peri.toml`.
- `l`: Launch the sandboxed game directly from the cockpit.

---

## 5. Maintenance, State Resets & Sharing

### Resetting Runtime State
Mod mutations, runtime logs, and modified configuration files write into `.perihelion/upper/`. To restore your modded setup to a completely fresh state without reinstalling mods or verifying base game files:

```bash
rm -rf .perihelion/upper/* .perihelion/work/*
```

### Reproducible Modpack Sharing
Distribute your complete mod list to another machine or player by sharing two files:
- `peri.toml`
- `peri.lock`

The recipient initializes their baseline and runs:

```bash
peri sync
peri run
```

Perihelion pulls packages, verifies SHA-256 integrity, unpacks virtual layers, and launches with identical load order and zero configuration drift.
