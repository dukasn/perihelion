# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/dukasn/perihelion/releases/tag/peri-core-v0.1.0) - 2026-10-26

### Added

- *(helion)* implement reactive presentation loop, WAL session persistence, and supervisor isolation
- implement PubGrub CDCL SAT solver and peri.lock deterministic lockfile pipeline
- implement global XDG configuration and zero-trust sandbox boundary enforcement
- implement compiler-style miette diagnostics with serde_spanned (Gap C)
- implement persistent VFI cache (Gap D) and declarative external game profiles (Gap B)
- implement v1.2 spec with Priority enum, Virtual File Index (VFI), and 3-Tier Resolution Waterfall
- initialize Perihelion workspace DAG according to architecture specification

### Other

- configure cargo-dist, release-plz, cargo-deny, and GitHub Actions workflows
- *(licensing)* establish GPLv3/CC-BY-4.0 split licensing, naming policy, and contributor guidelines
