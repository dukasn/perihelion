# helion (TUI Engine)

Interactive Terminal User Interface (TUI) engine for Perihelion.

`helion` gives users a fast, keyboard-driven interface to curate mod lists, inspect file conflicts in real time, and switch game profiles.

Built on [`ratatui`](https://ratatui.rs/) and [`crossterm`](https://crates.io/crates/crossterm), it combines responsive 60 FPS terminal rendering with non-blocking background worker actors.

![Helion Interactive TUI Dashboard](../../assets/demos/helion-dashboard.gif)

---

## Architecture: Actor-Driven TUI

```
┌────────────────────────────────────────────────────────┐
│                   Helion Terminal                      │
│                                                        │
│  [Tab Navigation: Overview | Mods | Conflicts | ...]   │
│  ┌─────────────────────────┐ ┌──────────────────────┐  │
│  │ Active Workspace Views  │ │ Detail Inspector     │  │
│  │ (Mod Table / Conflicts) │ │ (Metadata / Diff)    │  │
│  └───────────┬─────────────┘ └──────────────────────┘  │
│              │ Action Stream                           │
└──────────────┼─────────────────────────────────────────┘
               │ (Non-blocking mpsc channel)
               ▼
┌────────────────────────────────────────────────────────┐
│                  Worker Supervisor                     │
│  ┌──────────────────┐  ┌────────────────────────────┐  │
│  │ Solver Worker    │  │ VFI Collision Worker       │  │
│  │ (PubGrub / DAG)  │  │ (Multi-mod file indexing)  │  │
│  └──────────────────┘  └────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────┐  │
│  │ Session Recorder & Crash Recovery                │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

Terminal UIs freeze when disk I/O, network streams, or SAT solvers hog the event loop. `helion` avoids frame drops by splitting user interaction from heavy computation:

- **State Container (`App`):** Tracks navigation tabs, cursor positions, mod activation toggles, in-memory drafts, and diagnostic banners.
- **Worker Pipeline (`worker/`):** Runs PubGrub solving, VFI collision checks, and session telemetry on background Tokio actors over async channels.
- **Session Recovery:** Dumps dirty state to `.perihelion/draft.json`—unexpected terminal drops never lose uncommitted mod list edits.

---

## Core Views

| View | Purpose |
| :--- | :--- |
| [`Overview`](./src/views/overview.rs) | Workspace summary displaying detected game profile, active mod count, VFS mount status, and recent actions. |
| [`Mods`](./src/views/mods.rs) | Mod table supporting multi-select toggles, priority reordering, search filtering, and inspector panels. |
| [`Conflicts`](./src/views/conflicts.rs) | Interactive 3-Tier Resolution waterfall inspector. Shows file-level clashes and lets users override winning layers. |
| [`Downloads`](./src/views/downloads.rs) | Download queue tracking HTTP and BitTorrent package transfer speeds, peer counts, and extraction progress. |
| [`Profiles`](./src/views/profiles.rs) | Switches and creates workspace profiles for different target games or mod configurations. |
| [`Help`](./src/views/help.rs) | Quick modal overlay listing context-sensitive keyboard shortcuts. |

---

## Common Keybindings

| Key | Context | Action |
| :--- | :--- | :--- |
| `Tab` / `Shift+Tab` | Global | Cycle forward / backward through tabs. |
| `1` – `5` | Global | Jump directly to a tab (Overview, Mods, Conflicts, Downloads, Profiles). |
| `q` / `Ctrl+c` | Global | Exit `helion`. |
| `?` | Global | Open shortcut help popup. |
| `j` / `k` or `Down` / `Up` | Lists & Tables | Move selection cursor up / down. |
| `Space` | Mods Table | Toggle mod enabled / disabled state. |
| `p` / `P` | Mods Table | Cycle mod priority up / down (`lowest` to `absolute`). |
| `K` / `J` | Mods Table | Move selected mod up / down in load order. |
| `/` | Mods Table | Focus search filter input. |
| `i` | Mods Table | Toggle inspector drawer. |
| `Enter` | Conflicts | Pick selected resolution override. |

---

## Integration

The main `perihelion` binary crate runs this engine directly (`apps/perihelion/src/bin/helion/main.rs`). It also works as an embeddable library for custom frontends needing terminal mod orchestration.
