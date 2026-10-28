# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/dukasn/perihelion/compare/v0.1.2...v0.1.3) - 2026-10-28

### Added

- *(vfs,cli)* implement pluggable runner pipeline for wine, native, and wrapper chains
- *(core,cli)* implement headless automation primitives, JSON output, and lock verification
- *(vfs,cli)* implement dynamic base game resolution ladder and arbitrary lower roots
- *(vfs,cli,helion)* support cascading signature and provider discovery
- *(helion)* add game profile wizard and steam game importer to tui
- *(cli)* modularize profile management and add engine layout auto-detection

### Fixed

- *(archive,cli)* apply profile layout mappings and drop peri.toml when unpacking layers

### Other

- *(cli)* use --source in peri package examples
- restructure tutorial into universal modding guide with Skyrim SE example
- add end-to-end Skyrim SE modding tutorial and guide links

## [0.1.2](https://github.com/dukasn/perihelion/compare/v0.1.1...v0.1.2) - 2026-10-26

### Added

- *(helion)* implement customizable themes, real-time preview, and responsive footer
- *(ci)* automate terminal recordings with VHS and release-plz integration
- *(cli)* enhance help messages and styling across peri, perid, and helion

### Other

- *(logging)* add structured tracing spans and env filter across core crates and CLI binaries
- *(cargo)* prepare crates for public release with metadata and production profiles
- overhaul project and crate documentation

## [0.1.1](https://github.com/dukasn/perihelion/compare/v0.1.0...v0.1.1) - 2026-10-26

### Changed

- bundle `peri`, `perid` and `helion` into one package, released as a single archive
- limit changelog to the `perihelion` package and release only on release-PR merge

## [0.1.0](https://github.com/dukasn/perihelion/releases/tag/v0.1.0) - 2026-10-26

### Added

- initial release
