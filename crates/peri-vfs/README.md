# peri-vfs

Linux Virtual File System (VFS) and sandboxing adapter for Perihelion.

`peri-vfs` manages unprivileged user and mount namespaces, builds kernel OverlayFS stacks, and confines running game processes. The base game install remains completely untouched.

---

## Architectural Invariants & Mechanics

### 1. Inverted OverlayFS Lowerdir Ordering
Linux OverlayFS parses `lowerdir` from left to right:
```text
lowerdir=topmost_layer:...:lowest_layer:base_game
```
Leftmost directories win whenever file paths clash.

In Perihelion's DAG, dependent mods (or mods with higher priority weights) must override base files. When `VfsPlan::format_mount_options()` builds the mount argument, it reverses the topological sort vector. The mod with the highest priority sits leftmost in the string; the base game folder anchors the far right.

### 2. Unprivileged User & Mount Namespaces
`peri-vfs` runs without `sudo` or root privileges.
- Unshares namespaces using `nix::sched::unshare(CloneFlags::CLONE_NEWUSER | CloneFlags::CLONE_NEWNS)`.
- Writes UID/GID mappings (`/proc/self/uid_map` and `/proc/self/gid_map`) so the calling user becomes root inside their private namespace.
- Requires Linux kernel >= 5.11 (for unprivileged OverlayFS mounts) alongside `kernel.unprivileged_userns_clone = 1`.

### 3. Execution Sandboxing (`SandboxPolicy`)
Confining game binaries and community script injectors requires hard boundaries:
- Marks all namespace mounts as private (`MS_REC | MS_PRIVATE`).
- Remounts the host root filesystem (`/`) as read-only.
- Whitelists specific scratch paths—like `/tmp`, `/dev/shm`, and the game's upper OverlayFS directory.

---

## Steam & Proton Integration

`peri-vfs::proton` probes Steam environments and configures Windows games running under Wine/Proton on Linux:

- **Library Probing:** Discovers Steam libraries across native paths (`~/.local/share/Steam`, `~/.steam/steam`) and Flatpak runtimes (`~/.var/app/com.valvesoftware.Steam/.local/share/Steam`).
- **Compatibility Tools:** Detects installed Proton versions, from Proton Experimental to GE-Proton and legacy 8/9 builds.
- **Prefix Isolation:** Configures `STEAM_COMPAT_CLIENT_INSTALL_PATH`, sets `STEAM_COMPAT_DATA_PATH`, and runs game executables through Proton's headless runner script.

---

## Example Usage

```rust
use std::path::PathBuf;
use peri_vfs::{VfsPlan, SandboxPolicy, run_in_namespace};

let plan = VfsPlan::new(
    PathBuf::from("/games/cyberpunk"),
    PathBuf::from("/tmp/peri_vfs_mount"),
)
.with_layers(vec![
    PathBuf::from("/cache/mod_a_extracted"), // Base dependency
    PathBuf::from("/cache/mod_b_extracted"), // Higher priority override
]);

let policy = SandboxPolicy::new()
    .with_readonly_root(true)
    .with_writable_path(PathBuf::from("/tmp"));

// Spawns user + mount namespace, mounts OverlayFS, executes child command
run_in_namespace(&plan, &policy, &["/tmp/peri_vfs_mount/bin/x64/Cyberpunk2077.exe".to_string()])?;
```
