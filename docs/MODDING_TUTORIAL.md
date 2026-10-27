# Game Modding Guide & Tutorial

Mod managers historically treat game directories as mutable scratchpads. They extract loose zip files directly over vanilla data, overwrite shared textures, and leave orphaned files behind when uninstalled. One bad overwrite corrupts an entire installation.

Perihelion ditches in-place mutation for declarative virtualization. Stock game files stay untouched. Every mod lives in an isolated, versioned layer mounted on the fly through kernel OverlayFS and unprivileged user namespaces. No file collisions reach disk. Profiles swap in seconds.

Modding on Linux should not require Wine-wrapped Windows utilities or blind faith in uninstall scripts. The core workflow stays identical across any game engine: define layout rules, lock dependencies in code, and run inside an isolated mount.

---

## 1. Core Architecture & Sandboxing

Every workspace projects a merged virtual filesystem built from three distinct tiers:

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

The stack resolves dependency order deterministically:
- **Base Game (Bottom):** Your untouched vanilla installation. It anchors the lowest position of the `lowerdir` chain.
- **Mod Layers (Middle):** Extracted `.peca` packages. Higher-priority mods sit leftmost in the OverlayFS `lowerdir` string, shadowing conflicting files from lower layers without altering them on disk.
- **Upper Layer (Top):** An isolated read/write scratchpad (`.perihelion/upper/`). Runtime mutations—saves, INI edits, crash logs—divert here instead of muddying mod directories.
- **Namespace Boundary:** The game process executes inside unprivileged mount and user namespaces. When the game shuts down, the virtual stack unmounts cleanly.

---

## 2. The Universal Modding Lifecycle

Engines differ. Packaging conventions differ. The management loop stays constant.

```
1. Profile Discovery ──▶ 2. Workspace Init ──▶ 3. Mod Packaging ──▶ 4. Graph Solve ──▶ 5. Sandboxed Run
   (Engine Mappings)        (peri.toml)           (.peca archives)       (peri.lock)         (Namespaces)
```

---

### Step 1: Define or Discover the Game Profile

Game engines stash assets wherever their designers chose. Unreal Engine looks under `Content/Paks/~mods`. REDengine 4 scans `archive/pc/mod`. Bethesda Creation titles look inside `Data/`.

A **Game Profile** bridges this gap: it translates relative archive paths from mod packages (`from`) into paths the game engine expects (`to`).

#### Option A: Auto-Discovery from Steam

When a game sits in your Steam library, build a profile using its Steam Application ID:

```bash
peri profile new <game_id> --from-steam <STEAM_APP_ID> --global
```

Perihelion checks standard Steam roots—native, Flatpak, and Snap alike—then parses directory markers against built-in signatures. It pulls:
- The game binary path
- Steam Application ID (wiring up Proton compatibility)
- Engine prefix mappings (such as `mods` $\rightarrow$ `archive/pc/mod`)

#### Option B: Auto-Discovery from a Directory

For games installed via GOG, Heroic, or standalone installers, point Perihelion straight at the root directory:

```bash
peri profile new <game_id> --game-path /path/to/game/root --global
```

Perihelion checks root anchors (`*_Data/Managed`, `Binaries/Win64`, `Data/*.esm`). Once it spots a match, it injects the engine's default routing table.

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

Inspect the registered profile at any time:

```bash
peri profile show <game_id>
```

---

### Step 2: Initialize a Modding Workspace

Workspaces isolate your modding state. Keep them anywhere—on a separate NVMe drive, inside your home directory, or alongside shared tools. Your stock game directory remains untouched.

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

OverlayFS anchors this link as the bottom-most lowerdir layer.

---

### Step 3: Mod Packaging & The `.peca` Standard

Perihelion packages mods as `.peca` archives. A valid `.peca` file is a multi-threaded Zstandard-compressed tarball (`.tar.zst`) with a `peri.toml` manifest at its root.

#### Structure of a `.peca` Mod

```text
my-texture-mod/
├── textures/
│   ├── terrain.dds
│   └── sky.dds
└── peri.toml
```

The package manifest (`my-texture-mod/peri.toml`) declares identity, target game, and priority:

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

Turn loose assets into a distributable archive:

```bash
peri package ./my-texture-mod -o ./dist/high_res_terrain-1.2.0.peca
```

Perihelion validates manifest fields and inspects target game tags. Then it indexes file entries and streams the payload through multi-threaded zstd.

---

### Step 4: Caching, Dependencies & Conflict Resolution

Pull packages into local cache from disk paths, direct HTTP mirrors, or BitTorrent swarms:

```bash
# Local package
peri get ./dist/high_res_terrain-1.2.0.peca

# Direct HTTP/HTTPS mirror
peri get https://mods.example.com/texture_framework-1.0.0.peca

# BitTorrent swarm link
peri get "magnet:?xt=urn:btih:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855&dn=texture_framework.peca"
```

Declare dependencies in workspace `peri.toml`:

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

When two mods supply the same relative file path, Perihelion resolves the collision through a deterministic three-tier waterfall:

1. **Tier 1 (Explicit Edges):** Rules in `[load_order.after]` dictate order immediately. Dependents sit higher in the VFS stack.
2. **Tier 2 (Priority Weights):** Without an explicit edge, priority levels break the tie (`lowest` < `low` < `default` < `high` < `highest` < `absolute`). The heavier mod wins and receives a dynamic DAG edge.
3. **Tier 3 (Hard Conflicts):** Equal priority and no explicit edge? Resolution halts with `ResolveError::HardConflict`. The build refuses to mount until you pick a winner in `[overrides]`.

---

### Step 5: Verification and Synchronization

Run a sanity check before mounting:

```bash
peri check
```

This inspects the dependency DAG for cycles, detects missing packages, and flags unresolved file collisions.

Once verified, materialize the layers:

```bash
peri sync
```

PubGrub solves version constraints across the dependency graph. Hashes verify against SHA-256 digests. Missing archives unpack into `.perihelion/extracted/` with zero disk duplication, and `peri.lock` locks the resolved tree into source control.

---

### Step 6: Sandboxed Execution

Dry-run the mount without entering namespaces or spawning processes:

```bash
peri run --dry-run
```

The output shows the assembled `lowerdir` stack:

```text
lowerdir=/path/to/extracted/high_res_terrain:/path/to/extracted/texture_framework:/path/to/base_game
```

Notice the ordering: dependents sit leftmost. Linux OverlayFS searches left-to-right, so winning layers shadow whatever sits underneath.

Launch the sandbox:

```bash
peri run
```

Or override binaries and arguments on the fly:

```bash
peri run --exec bin/custom_launcher.exe -- --skip-intro +exec autoexec.cfg
```

---

## 3. Practical Walkthrough: Skyrim Special Edition

Skyrim Special Edition highlights the exact headaches Perihelion solves.

Bethesda's Creation Engine bundles plugins (`.esp`, `.esm`, `.esl`), asset archives (`.bsa`), and raw meshes into a single `Data/` folder. Script extenders add another wrinkle: loader binaries (`skse64_loader.exe`) live in the game root, while runtime DLL plugins sit under `Data/SKSE/Plugins/`.

Traditional mod managers dump everything straight into Steam's game directory. Overwrite textures in the wrong sequence, and your game crashes on startup. Uninstalling leaves orphaned scripts baked into your save.

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

Sync the workspace and freeze the lockfile:

```bash
peri check
peri sync
```

Launch through the script extender:

```bash
peri run --exec skse64_loader.exe
```

Under the hood, Perihelion:
1. Spawns an unprivileged user and mount namespace.
2. Mounts the OverlayFS stack (`lowerdir=skyui:skse64:base_game`).
3. Auto-detects your active Proton runner—Proton Experimental, GE-Proton, or a custom build.
4. Binds `WINEPREFIX` and `STEAM_COMPAT_DATA_PATH` to Skyrim's compatdata folder (`compatdata/489830`).
5. Launches `skse64_loader.exe` inside the virtualized mount.

The stock game folder stays read-only. Not one byte changes on disk.

---

## 4. Visual Cockpit: Live Curation in Helion

Prefer keys over config files? Run `helion` inside the workspace:

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

### Essential Keybindings
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

## 5. Maintenance & Sharing

### Resetting Runtime State
Runtime writes land in `.perihelion/upper/`. If bad INI tweaks or broken saves poison your session, wipe the scratchpad:

```bash
rm -rf .perihelion/upper/* .perihelion/work/*
```

Your mods remain intact. Stock game files stay clean. The environment resets instantly.

### Sharing Reproducible Modpacks
Share two files to duplicate an entire setup:
- `peri.toml`
- `peri.lock`

The recipient links their base game and runs:

```bash
peri sync
peri run
```

Perihelion fetches archives, checks SHA-256 digests, and mounts layers in identical topological order. Zero drift. No guesswork.
