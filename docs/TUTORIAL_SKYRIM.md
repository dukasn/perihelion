# Modding Skyrim Special Edition on Linux with Perihelion

Traditional Skyrim modding on Linux is notorious for brittle hacks. Players run Windows mod managers through Wine prefixes, fight file overwrites in the `Data/` directory, and risk corrupting clean installs whenever a script extender mod breaks. 

Perihelion removes this pain. It treats mod lists as declarative code, verifies file collisions before runtime, and mounts your mods on the fly using Linux kernel OverlayFS and unprivileged user namespaces. Stock game files stay strictly read-only. Your base install never changes.

This guide walks through setting up a crash-resilient Skyrim Special Edition setup from scratch using both the `peri` CLI and the `helion` terminal dashboard.

---

## Architecture at a Glance

Perihelion separates your mod curation into three distinct layers:

```text
                  ┌────────────────────────────────────────┐
                  │          Running Game Process          │
                  │        (skse64_loader.exe / Wine)      │
                  └───────────────────┬────────────────────┘
                                      │
               Reads/Writes inside Mount Namespace (/mnt)
                                      ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Virtual File System                             │
├────────────────────────────────────────────────────────────────────────┤
│ Upper Layer (Read/Write)   │ .perihelion/upper/ (saves, INIs, logs)    │
├────────────────────────────┼───────────────────────────────────────────┤
│ Mod Layer N (Read-Only)    │ .perihelion/extracted/skyui/              │
├────────────────────────────┼───────────────────────────────────────────┤
│ Mod Layer 1 (Read-Only)    │ .perihelion/extracted/skse64/             │
├────────────────────────────┼───────────────────────────────────────────┤
│ Base Game (Read-Only)      │ ~/.local/share/Steam/.../Skyrim SE/       │
└────────────────────────────────────────────────────────────────────────┘
```

When you launch, the kernel presents a merged view:
- Lower layers stack on top of each other. Winning mods override lower mods without mutating files on disk.
- Runtime mutations (saves, plugin logs, generated caches) divert into `.perihelion/upper`.
- Exiting the game tears down the namespace. Your base game install remains completely vanilla.

---

## Prerequisites

Ensure your system has the following in place:

1. **Linux Kernel 5.11+** with unprivileged user namespaces enabled (`kernel.unprivileged_userns_clone = 1`, default on modern distributions).
2. **Skyrim Special Edition** installed via Steam (App ID `489830`). Both native Steam and Flatpak Steam work out of the box.
3. **Perihelion toolchain** (`peri` and `helion` binaries available in `$PATH`):
   ```bash
   export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
   ```

---

## Step 1: Discover & Configure the Game Profile

Perihelion uses declarative profiles to translate mod file paths into game engine directories. Bethesda Creation Engine titles look for plugins, archives, and loose assets inside `Data/` or script extender roots.

Generate the profile automatically by querying Steam:

```bash
peri profile new skyrim_se --from-steam 489830 --global
```

Perihelion inspects your local Steam library folders, identifies the Skyrim install directory, parses engine markers (`Data/*.esm`, `Data/*.esp`, `*_loader.exe`), and writes a global profile to `~/.config/perihelion/profiles/skyrim_se.toml`.

Inspect the generated profile:

```bash
peri profile show skyrim_se
```

The output confirms the engine bindings and layout translations:

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

> [!TIP]
> **Using Helion TUI instead:** Launch `helion`, press `3` to jump to the **Profiles** tab, and press `s` to open the **Steam Importer**. Scroll to Skyrim Special Edition and hit `Enter`. The profile generates instantly.

---

## Step 2: Initialize Your Modding Workspace

A workspace is an isolated directory holding your declarative manifest (`peri.toml`), dependency lockfile (`peri.lock`), and extracted virtual layers.

Create a workspace anywhere outside your game folder:

```bash
mkdir -p ~/Games/SkyrimWorkspace
cd ~/Games/SkyrimWorkspace

peri init skyrim_se --name "Skyrim Nordic Overhaul"
```

This creates:
- `peri.toml`: The root configuration defining desired mods, versions, and overrides.
- `.perihelion/`: Internal staging directories (`cache/mods`, `extracted`, `upper`, `work`, `mnt`).

Next, link your stock Skyrim installation as `base_game`. This serves as the bottom-most read-only layer in OverlayFS:

```bash
# For standard native Steam installations:
ln -s "$HOME/.local/share/Steam/steamapps/common/Skyrim Special Edition" base_game

# For Flatpak Steam installations:
# ln -s "$HOME/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common/Skyrim Special Edition" base_game
```

---

## Step 3: Package Mods into the `.peca` Standard

Perihelion uses `.peca` archives—multi-threaded Zstandard-compressed tarballs (`.tar.zst`) containing a valid `peri.toml` manifest at their root.

Let's package two essential mods: **SKSE64** and **SkyUI**.

### Mod 1: Skyrim Script Extender (SKSE64)

Create a staging directory:

```bash
mkdir -p /tmp/skse64_build
cd /tmp/skse64_build
```

Unpack the official SKSE64 release files here so the structure looks like this:

```text
/tmp/skse64_build/
├── skse64_loader.exe
├── skse64_1_6_1170.dll
├── skse64_steam_loader.dll
├── Data/
│   └── scripts/
│       ├── Actor.pex
│       └── ...
└── peri.toml
```

Write the mod manifest `/tmp/skse64_build/peri.toml`:

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

Package it with `peri package`:

```bash
peri package /tmp/skse64_build -o ~/Games/SkyrimWorkspace/skse64-2.2.6.peca
```

### Mod 2: SkyUI

Create another staging directory:

```bash
mkdir -p /tmp/skyui_build
cd /tmp/skyui_build
```

Place SkyUI's loose files into their engine layout:

```text
/tmp/skyui_build/
├── Data/
│   ├── SkyUI_SE.esp
│   └── SkyUI_SE.bsa
└── peri.toml
```

Write `/tmp/skyui_build/peri.toml`:

```toml
[package]
name = "skyui"
version = "5.2.0"
author = "SkyUI Team"
priority = "high"
description = "Elegant, PC-friendly interface overhaul"

[target]
game_id = "skyrim_se"

[dependencies]
skse64 = "^2.2.6"
```

Package the archive:

```bash
peri package /tmp/skyui_build -o ~/Games/SkyrimWorkspace/skyui-5.2.0.peca
```

---

## Step 4: Cache and Declare Dependencies

Return to your workspace:

```bash
cd ~/Games/SkyrimWorkspace
```

Import the `.peca` packages into Perihelion's local cache:

```bash
peri get ./skse64-2.2.6.peca
peri get ./skyui-5.2.0.peca
```

Now update your workspace manifest (`peri.toml`) to declare your active mod list:

```toml
[package]
name = "Skyrim Nordic Overhaul"
version = "0.1.0"
author = "Dovahkiin"
priority = "default"
description = "Crash-resilient Skyrim SE build"

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

### How Priority and Ordering Work

Perihelion resolves file collisions through a deterministic 3-tier waterfall:

1. **Tier 1 (Explicit Edges):** Rules in `[load_order]` force the dependent mod to sit higher in the stack.
2. **Tier 2 (Priority Weights):** If no edge exists, priorities compare: `lowest` < `low` < `default` < `high` < `highest` < `absolute`. The higher priority mod wins.
3. **Tier 3 (Hard Conflicts):** Same priority with no explicit edge triggers `ResolveError::HardConflict`. The build halts until you declare an override.

---

## Step 5: Verify and Synchronize

Check your workspace for missing files, dependency cycles, or unhandled file collisions:

```bash
peri check
```

Once check passes, synchronize the virtual filesystem:

```bash
peri sync
```

`peri sync` performs the heavy lifting:
- Runs PubGrub dependency resolution across all packages.
- Verifies SHA-256 content hashes.
- Unpacks required layers into `.perihelion/extracted/` without mutating any existing directories.
- Writes a lockfile (`peri.lock`) freezing the exact versions and load order.

---

## Step 6: Test and Launch with Proton

Preview the exact mount options and environment variables without mounting filesystems:

```bash
peri run --dry-run --exec skse64_loader.exe
```

Output shows the OverlayFS construction:

```text
Preparing Perihelion Virtual File System sandbox:
   Game:          The Elder Scrolls V: Skyrim Special Edition (skyrim_se)
   Active Layers: 2
   Mount Point:   /home/user/Games/SkyrimWorkspace/.perihelion/mnt
   Mount Options: lowerdir=/home/user/Games/SkyrimWorkspace/.perihelion/extracted/skyui:/home/user/Games/SkyrimWorkspace/.perihelion/extracted/skse64:/home/user/Games/SkyrimWorkspace/base_game,upperdir=/home/user/Games/SkyrimWorkspace/.perihelion/upper,workdir=/home/user/Games/SkyrimWorkspace/.perihelion/work
Compatibility Layer: Proton / Wine
   Proton Runner:       /home/user/.local/share/Steam/steamapps/common/Proton - Experimental/proton
   Compat Data:         /home/user/.local/share/Steam/steamapps/compatdata/489830
   Wine Prefix:         /home/user/.local/share/Steam/steamapps/compatdata/489830/pfx
   Target Binary:       skse64_loader.exe []
[DRY-RUN] Verification complete. Sandboxed mount and execution ready.
```

Notice the `lowerdir` ordering: dependents (`skyui`) appear **first** on the left, followed by dependencies (`skse64`), and vanilla files (`base_game`) at the end. This follows OverlayFS search precedence.

Launch the game for real:

```bash
peri run --exec skse64_loader.exe
```

Perihelion creates an unprivileged user namespace, mounts the virtual stack, forwards your Steam Proton environment (`WINEPREFIX`, `SteamAppId=489830`, `STEAM_COMPAT_DATA_PATH`), and spawns SKSE64.

---

## Step 7: Managing Mods with Helion (TUI Cockpit)

Prefer an interactive terminal cockpit? Launch `helion` inside your workspace:

```bash
helion
```

```text
┌─ Helion: Skyrim Nordic Overhaul [skyrim_se] ──────────────────────────────────────────┐
│ [1] Overview    [2] Mods (2)    [3] Profiles    [4] Swarm    [?] Help                 │
├───────────────────────────────────────────────────────────────────────────────────────┤
│ Active Layers: 2    Conflicts: 0    VFS: Ready    Status: Synced                      │
│                                                                                       │
│  [x] skse64        v2.2.6   [highest]   Skyrim Script Extender 64-bit                 │
│  [x] skyui         v5.2.0   [high]      Elegant, PC-friendly interface overhaul       │
│                                                                                       │
│ Load Order:                                                                           │
│   1. skse64 (highest)                                                                 │
│   2. skyui  (high)                                                                    │
├───────────────────────────────────────────────────────────────────────────────────────┤
│ [Space] Toggle   [+/-] Priority   [Shift+J/K] Reorder   [s] Sync   [l] Launch   [q] Quit│
└───────────────────────────────────────────────────────────────────────────────────────┘
```

### Essential Helion Shortcuts:
- `1` / `2` / `3` / `4`: Switch tabs (**Overview**, **Mods**, **Profiles**, **Swarm**).
- `j` / `k` or `Up` / `Down`: Navigate mod entries.
- `Space`: Toggle a mod active or disabled (stages a draft transaction).
- `+` / `-`: Raise or lower mod priority weight.
- `Shift+J` / `Shift+K`: Move selected mod down or up in the load order.
- `Tab`: Jump focus between panels.
- `Ctrl+S`: Save and commit staged changes to `peri.toml`.
- `l`: Launch the sandboxed game directly from the cockpit.

---

## Maintenance & Recovery

### Resetting Game State Without Reinstalling
Because all writes land in `.perihelion/upper`, bad INI tweaks or corrupted runtime saves can be cleaned in one command without touching your mods:

```bash
rm -rf .perihelion/upper/* .perihelion/work/*
```

### Sharing Your Modpack
Share your entire curated build with another Linux user by sending two files:
1. `peri.toml`
2. `peri.lock`

The recipient runs:

```bash
peri sync
peri run --exec skse64_loader.exe
```

Every mod downloads, verifies, and mounts with the exact same load order and SHA-256 integrity. Zero drift. Zero broken installs.
