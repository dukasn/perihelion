# Automated Terminal Recordings (VHS)

This directory contains [VHS](https://github.com/charmbracelet/vhs) recording tapes (`.tape` files) used to generate animated terminal preview GIFs embedded across Perihelion's documentation and READMEs.

---

## Tape Inventory

| Tape File | Target Output | Description |
| :--- | :--- | :--- |
| [`peri-quickstart.tape`](./peri-quickstart.tape) | `assets/demos/peri-quickstart.gif` | Demonstrates the headless CLI: `peri --help`, workspace initialization, and DAG validation. |
| [`helion-dashboard.tape`](./helion-dashboard.tape) | `assets/demos/helion-dashboard.gif` | Demonstrates the interactive TUI: multi-tab navigation, mod inspection, and 3-tier conflict waterfall view. |

---

## Running Locally

### 1. Install VHS
Ensure `vhs` and its dependencies (`ttyd`, `ffmpeg`) are installed:

```bash
# macOS (Homebrew)
brew install vhs

# Arch Linux
pacman -S vhs

# Go
go install github.com/charmbracelet/vhs@latest

# Direct Shell Script
curl -sS https://charm.sh/install.sh | bash
```

### 2. Build Binaries & Record
Use the automated runner script in `scripts/`:

```bash
./scripts/record-demos.sh
```

Or run an individual tape manually:
```bash
cargo build --release --bin peri --bin helion
export PATH="$PWD/target/release:$PATH"
vhs assets/vhs/peri-quickstart.tape
```

---

## CI/CD Automation

Terminal recordings are automatically verified and uploaded through GitHub Actions:

1. **Pull Request Validation (`.github/workflows/vhs.yml`):**
   Runs whenever `.tape` files or recording scripts are modified in a PR. Builds the binaries, compiles the recordings, and attaches the resulting GIFs as workflow artifacts (`vhs-terminal-previews`) for visual inspection before merging.
2. **On-Demand Dispatch (`workflow_dispatch`):**
   Can be triggered manually from the GitHub Actions tab. With `commit_changes: true`, the workflow automatically commits the freshly rendered GIFs back to the target branch.
3. **Local Generation via mise:**
   Run `mise run record` or `./scripts/record-demos.sh` locally to generate and commit updated GIFs directly in your feature branch before opening a PR.

