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

Terminal recordings are automatically kept in sync through two GitHub Actions workflows:

1. **Release PR Integration (`.github/workflows/release-plz.yml`):**
   When `release-plz` creates or updates a release pull request, it compiles the release binaries, runs `scripts/record-demos.sh`, and commits freshly generated GIFs to the release branch. Reviewing the release PR allows maintainers to visually inspect the recorded behavior before merging to `main`.
2. **On-Demand / Tape Validation (`.github/workflows/vhs.yml`):**
   Runs whenever `.tape` files or recording scripts are modified, or on manual trigger via `workflow_dispatch`.

