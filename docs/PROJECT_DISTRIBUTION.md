# Project Distribution & Release Architecture

Perihelion uses an automated, dual-engine release and distribution pipeline. It pairs [release-plz](https://release-plz.dev/) for semantic versioning, changelogs, and pull requests with [cargo-dist](https://axodotdev.github.io/cargo-dist/) for multi-target binary builds, checksum generation, and GitHub Releases.

---

## 1. Release Lifecycle Pipeline

The release engine operates as a closed loop. Development happens on feature branches, merges to `main`, and triggers automated release preparation without human intervention.

```
Feature Branch (Conventional Commits)
             │
             ▼
        Merge to `main`
             │
             ├──▶ 1. release-plz (PR Job)
             │    ├── Analyzes commit log since previous tag
             │    ├── Calculates next SemVer version
             │    ├── Generates CHANGELOG.md & bumps Cargo.toml / Cargo.lock
             │    └── Opens/updates `release-YYYY-MM-DD...` PR branch
             │
             ├──▶ 2. Automated Visual Verification (in Release PR)
             │    ├── Compiles release binaries with sccache
             │    ├── Runs `./scripts/record-demos.sh` with VHS
             │    └── Commits refreshed preview GIFs to the PR branch
             │
             ▼
   Review & Merge Release PR
             │
             ▼
        Merge to `main`
             │
             ├──▶ 3. release-plz (Release Job)
             │    ├── Detects release commit on `main`
             │    └── Creates & pushes annotated git tag `vX.Y.Z` via PAT
             │
             ▼
   Tag Push (`v*`)
             │
             └──▶ 4. Publish Release (`.github/workflows/release.yml`)
                  ├── Compiles production binaries (`peri`, `helion`, `perid`)
                  ├── Builds release tarballs, installers, and SHA256 checksums
                  └── Publishes GitHub Release with assets and release notes
```

---

## 2. Semantic Versioning Mechanics (Pre-1.0 vs Post-1.0)

Perihelion adheres to Semantic Versioning 2.0.0. Because the project is currently in initial development (`0.y.z`), Cargo's dependency resolution rules introduce specific versioning behaviors that differ from stable `1.x.y` software.

### The Cargo Pre-1.0 Invariant

Cargo treats any bump from `0.1.x` to `0.2.0` as a **breaking change**. If a downstream project or workspace crate depends on `peri-core = "0.1.1"`, Cargo will permit updates to `0.1.2`, but will strictly reject `0.2.0` without manual user intervention.

To preserve compatibility within the `0.1.x` series, `release-plz` defaults to the following rules:

| Commit Type | Pre-1.0 (`0.y.z`) Version Bump | Post-1.0 (`1.y.z`) Version Bump | Cargo Compatibility Impact |
| :--- | :--- | :--- | :--- |
| `fix: ...` | **Patch** (`0.1.1` $\rightarrow$ `0.1.2`) | **Patch** (`1.0.0` $\rightarrow$ `1.0.1`) | Non-breaking bug fix. Auto-upgrades. |
| `feat: ...` | **Patch** (`0.1.1` $\rightarrow$ `0.1.2`) | **Minor** (`1.0.0` $\rightarrow$ `1.1.0`) | Non-breaking feature addition. |
| `feat!: ...` or `BREAKING CHANGE:` | **Minor** (`0.1.1` $\rightarrow$ `0.2.0`) | **Major** (`1.0.0` $\rightarrow$ `2.0.0`) | Breaking API change. Halts auto-updates. |
| `refactor:`, `perf:`, `docs:` | **Patch** (if user-facing) or ignored | **Patch** or ignored | Internal improvements. |

### Why `feat` Commits Incremented `0.1.1` to `0.1.2`

In `0.x` releases, incrementing the minor version (`0.2.0`) indicates that the public API has broken compatibility. Adding features, internal refactors, or new CLI subcommands without breaking existing configuration syntax or manifest schemas remains inside the `0.1.x` patch series.

### Forcing Minor Bumps on Features

If the project strategy shifts to treating every `feat:` as a minor bump regardless of pre-1.0 semantics, enable this flag in [`release-plz.toml`](file:///var/home/dukasn/Github/perihelion/release-plz.toml):

```toml
[workspace]
publish = false
git_only = true
features_always_increment_minor = true
```

When active, any `feat:` commit on `0.1.1` automatically increments to `0.2.0`.

---

## 3. Workflow Concurrency & Isolation Invariants

Binary assets like preview GIFs (`assets/demos/*.gif`) cannot be auto-merged by git. To prevent race conditions and merge conflicts across automated runners, the CI architecture enforces these rules:

1. **Zero Bot Pushes to `main`:** No GitHub Actions workflow pushes commits directly to `main`. All automated changes flow through pull requests.
2. **PR Demo Ownership:** `.github/workflows/prepare-release.yml` owns demo generation on the release PR branch. It builds the binaries, records the tapes, and commits visual updates to the PR before review.
3. **Validation Isolation:** `.github/workflows/record-previews.yml` runs only on feature pull requests touching `.tape` scripts. It uploads recordings as artifacts for maintainer review without writing back to the branch.
4. **Clean PR Diffs:** If recording tapes produce no visual diff against existing demos, the commit step skips execution, keeping PR history free of empty commits.

---

## 4. Local Tooling & Maintenance

All tools powering this pipeline are pinned in [`mise.toml`](file:///var/home/dukasn/Github/perihelion/mise.toml) and verified via `./scripts/bootstrap.sh`.

```bash
# Verify toolchain health
./scripts/bootstrap.sh --check

# Dry-run release-plz version calculation locally
release-plz release-pr --dry-run

# Preview cargo-dist packaging plan
dist plan

# Record terminal tapes locally before opening a PR
mise run record
# Or directly:
./scripts/record-demos.sh
```

---

## 5. Security & Authentication Model

- **`RELEASE_PLZ_TOKEN`:** A GitHub Personal Access Token (PAT) or GitHub App token with `contents: write` and `pull-requests: write`. Standard `GITHUB_TOKEN` credentials cannot trigger downstream workflows when pushing tags; using a dedicated PAT allows the tag push from `release-plz` to trigger the `cargo-dist` release build.
- **`dist-workspace.toml`:** Tracks `cargo-dist` configuration, installer scripts, and archive targets.
- **Cargo Deny:** Enforces dependency licenses and security advisories before any release tag is cut.
