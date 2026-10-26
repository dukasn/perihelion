# peri-archive

Archive adapter for Perihelion's distribution format: the `.peca` archive.

`peri-archive` packs mod trees into multi-threaded Zstandard tarballs. It inspects embedded manifests without touching gigabyte payloads on disk, then streams unpack jobs straight into cache.

---

## The `.peca` Standard

Mod packaging has long suffered from broken ZIPs and loose file dumps directly into game roots. Perihelion enforces a single standard:

- **Format:** POSIX `tar` archive compressed with multi-threaded Zstandard (`zstdmt`).
- **File Extension:** `.peca` (Perihelion Compressed Archive).
- **Mandatory Root Manifest:** Every valid archive **must** contain a valid `peri.toml` at its root.
- **Interoperability:** Standard tools handle `.peca` out of the box. Run `tar --zstd -xf archive.peca` or open it in GUI archivers (Ark, File Roller) without specialized tooling.

---

## Key Capabilities

### 1. In-Stream Manifest Inspection (`read_manifest_from_peca`)
Downloader pipelines often need package metadata before unpacking full texture packs. `peri-archive` reads tar headers sequentially through a streaming Zstandard decoder. Once it hits `peri.toml`, it deserializes the manifest and halts decompression on the spot.

### 2. Path Traversal & Zip-Slip Defense
Every entry unpacked from a `.peca` passes through `peri_core::validate_safe_path`. Path traversal attacks—whether through `../` parent escapes or injected absolute paths—fail immediately with `SecurityError::PathTraversal`.

### 3. Virtual File Index Generation (`PackageFileIndex`)
During extraction or inspection, `peri-archive` scans relative file paths inside the archive. This in-memory index feeds directly into `peri-core`'s Virtual File Index (VFI), catching collision waterfalls before any files touch OverlayFS layers.

---

## Example Usage

```rust
use std::path::Path;
use peri_archive::{create_peca, read_manifest_from_peca, extract_peca};

// Pack a directory containing peri.toml into a .peca archive
let source = Path::new("./my-mod");
let output = Path::new("./dist/my-mod-1.0.0.peca");
let compression_level = 3;
let manifest = create_peca(source, output, compression_level)?;

// Inspect manifest without unpacking mod assets
let inspected = read_manifest_from_peca(output)?;
println!("Mod: {} v{}", inspected.package.name, inspected.package.version);

// Extract safely into destination cache
let dest = Path::new("./cache/my-mod");
let (manifest, vfi_index) = extract_peca(output, dest)?;
```
