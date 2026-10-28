// Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
// Copyright (C) 2026 Eduardo Marques Santos and Perihelion Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Perihelion Archive Adapter (`peri-archive`)
//!
//! Provides creation, extraction, and inspection of `.peca` archives
//! (Zstandard-compressed tarballs containing a mandatory root `peri.toml` manifest).

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};

use peri_core::{validate_safe_path, Manifest, PackageFileIndex, SecurityError, MANIFEST_FILENAME};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{debug, info_span, trace};

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Manifest error: {0}")]
    Manifest(#[from] peri_core::ManifestError),

    #[error("Archive validation error: {0}")]
    Validation(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Missing mandatory root '{MANIFEST_FILENAME}' manifest in .peca archive")]
    MissingManifest,

    #[error("Security violation: {0}")]
    Security(#[from] SecurityError),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Checksum mismatch: expected {expected}, calculated {calculated}")]
    ChecksumMismatch {
        expected: String,
        calculated: String,
    },
}

/// Computes the SHA-256 hash of a file as a lowercase hexadecimal string.
pub fn compute_file_sha256(path: &Path) -> Result<String, ArchiveError> {
    trace!(path = %path.display(), "Computing SHA-256 file checksum");
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Reads only the `peri.toml` manifest from a `.peca` archive without full extraction.
pub fn read_manifest_from_peca(peca_path: &Path) -> Result<Manifest, ArchiveError> {
    trace!(path = %peca_path.display(), "Reading manifest from .peca archive");
    let file = File::open(peca_path)?;
    let decoder =
        zstd::stream::Decoder::new(file).map_err(|e| ArchiveError::Compression(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.into_owned();

        if path == Path::new(MANIFEST_FILENAME) {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            return Manifest::from_toml_str(&content).map_err(ArchiveError::Manifest);
        }
    }

    Err(ArchiveError::MissingManifest)
}

/// Lists all file paths contained in a `.peca` archive (excluding root peri.toml).
pub fn list_peca_files(peca_path: &Path) -> Result<Vec<String>, ArchiveError> {
    trace!(path = %peca_path.display(), "Listing files in .peca archive");
    let file = File::open(peca_path)?;
    let decoder =
        zstd::stream::Decoder::new(file).map_err(|e| ArchiveError::Compression(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);
    let mut paths = Vec::new();

    for entry_result in archive.entries()? {
        let entry = entry_result?;
        let path = entry.path()?.into_owned();
        let path_str = path.to_string_lossy().to_string();
        if path_str != MANIFEST_FILENAME && !entry.header().entry_type().is_dir() {
            paths.push(path_str);
        }
    }

    Ok(paths)
}

/// Inspects a `.peca` archive and builds its `PackageFileIndex` in memory.
pub fn generate_package_file_index(peca_path: &Path) -> Result<PackageFileIndex, ArchiveError> {
    let span = info_span!("generate_package_file_index", peca = %peca_path.display());
    let _enter = span.enter();
    debug!("Generating PackageFileIndex from archive");
    let manifest = read_manifest_from_peca(peca_path)?;
    let files = list_peca_files(peca_path)?;
    Ok(PackageFileIndex::new(
        manifest.package.name,
        manifest.package.version,
        manifest.package.priority,
        files,
    ))
}

/// Saves a `PackageFileIndex` to the designated VFI cache directory (e.g. `.perihelion/cache/vfi/<package_name>.vfi.json`).
pub fn save_package_vfi_cache(
    vfi_cache_dir: &Path,
    index: &PackageFileIndex,
) -> Result<PathBuf, ArchiveError> {
    fs::create_dir_all(vfi_cache_dir)?;
    let sanitized_name = index.package_name.replace(['/', '\\', ' '], "_");
    let cache_file = vfi_cache_dir.join(format!("{sanitized_name}.vfi.json"));
    let json = index.to_json()?;
    fs::write(&cache_file, json)?;
    debug!(package = %index.package_name, path = %cache_file.display(), "Persisted package VFI cache");
    Ok(cache_file)
}

/// Loads a cached `PackageFileIndex` if present on disk.
pub fn load_package_vfi_cache(
    vfi_cache_dir: &Path,
    package_name: &str,
) -> Result<Option<PackageFileIndex>, ArchiveError> {
    let sanitized_name = package_name.replace(['/', '\\', ' '], "_");
    let cache_file = vfi_cache_dir.join(format!("{sanitized_name}.vfi.json"));
    if !cache_file.exists() {
        return Ok(None);
    }
    trace!(package = %package_name, path = %cache_file.display(), "Loading package VFI cache");
    let content = fs::read_to_string(cache_file)?;
    let index = PackageFileIndex::from_json(&content)?;
    Ok(Some(index))
}

/// Creates a `.peca` archive from a directory.
///
/// Enforces that `source_dir` contains a valid `peri.toml` at its root.
pub fn create_peca(
    source_dir: &Path,
    output_path: &Path,
    compression_level: i32,
) -> Result<Manifest, ArchiveError> {
    let span =
        info_span!("create_peca", source = %source_dir.display(), output = %output_path.display());
    let _enter = span.enter();
    debug!(
        compression_level = compression_level,
        "Creating .peca archive"
    );

    let manifest_path = source_dir.join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(ArchiveError::MissingManifest);
    }

    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output_file = File::create(output_path)?;
    let buf_writer = BufWriter::new(output_file);
    let mut encoder = zstd::stream::Encoder::new(buf_writer, compression_level)
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    encoder
        .multithread(num_cpus())
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;

    {
        let mut tar_builder = tar::Builder::new(&mut encoder);
        tar_builder.append_dir_all(".", source_dir)?;
        tar_builder.finish()?;
    }

    encoder
        .finish()
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;

    debug!(mod_name = %manifest.package.name, version = %manifest.package.version, "Finished creating .peca archive");
    Ok(manifest)
}

/// Extracts a `.peca` (tar.zst) archive on the fly directly from an arbitrary synchronous `Read` stream.
///
/// Ensures root `peri.toml` is present, validates against path traversal attacks,
/// and returns the parsed `(Manifest, PackageFileIndex)`.
pub fn extract_peca_stream<R: Read>(
    reader: R,
    destination_dir: &Path,
) -> Result<(Manifest, PackageFileIndex), ArchiveError> {
    let span = info_span!("extract_peca_stream", destination = %destination_dir.display());
    let _enter = span.enter();
    trace!("Extracting archive stream into destination directory");

    fs::create_dir_all(destination_dir)?;

    let decoder =
        zstd::stream::Decoder::new(reader).map_err(|e| ArchiveError::Compression(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);

    let mut indexed_files = Vec::new();

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.into_owned();

        // Security check: Zero-trust sandbox boundary and path traversal enforcement
        validate_safe_path(&path, destination_dir)?;

        let path_str = path.to_string_lossy().to_string();
        if path_str != MANIFEST_FILENAME && !entry.header().entry_type().is_dir() {
            indexed_files.push(path_str);
        }

        entry.unpack_in(destination_dir)?;
    }

    let manifest_path = destination_dir.join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(ArchiveError::MissingManifest);
    }

    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    let index = PackageFileIndex::new(
        &manifest.package.name,
        &manifest.package.version,
        manifest.package.priority,
        indexed_files,
    );

    debug!(mod_name = %manifest.package.name, version = %manifest.package.version, files_count = index.files.len(), "Stream extraction complete");
    Ok((manifest, index))
}

/// Extracts a `.peca` archive from disk to `destination_dir`.
/// Validates that a root `peri.toml` exists upon extraction and returns the parsed manifest.
pub fn extract_peca(peca_path: &Path, destination_dir: &Path) -> Result<Manifest, ArchiveError> {
    let span = info_span!("extract_peca", peca = %peca_path.display(), destination = %destination_dir.display());
    let _enter = span.enter();
    debug!("Extracting .peca archive file from disk");
    let file = File::open(peca_path)?;
    let buf_reader = BufReader::new(file);
    let (manifest, _) = extract_peca_stream(buf_reader, destination_dir)?;
    Ok(manifest)
}

/// Extracts a `.peca` archive as a VFS layer: entries are relocated through `map`
/// and the root `peri.toml` is parsed but not written into the layer.
pub fn extract_peca_layer(
    peca_path: &Path,
    destination_dir: &Path,
    map: impl Fn(&Path) -> PathBuf,
) -> Result<Manifest, ArchiveError> {
    let span = info_span!("extract_peca_layer", peca = %peca_path.display(), destination = %destination_dir.display());
    let _enter = span.enter();

    fs::create_dir_all(destination_dir)?;

    let decoder = zstd::stream::Decoder::new(BufReader::new(File::open(peca_path)?))
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);
    let mut manifest = None;

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?.into_owned();
        validate_safe_path(&path, destination_dir)?;

        if path == Path::new(MANIFEST_FILENAME) {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            manifest = Some(Manifest::from_toml_str(&content)?);
            continue;
        }

        let target = map(&path);
        validate_safe_path(&target, destination_dir)?;
        let full = destination_dir.join(&target);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&full)?;
    }

    manifest.ok_or(ArchiveError::MissingManifest)
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_extract_peca() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let extract_dir = tempdir().unwrap();

        let manifest_content = r#"
[package]
name = "Test Mod"
author = "Tester"
version = "1.0.0"

[target]
game_id = "test_game"
"#;
        fs::write(src_dir.path().join(MANIFEST_FILENAME), manifest_content).unwrap();
        fs::write(src_dir.path().join("readme.txt"), "hello world").unwrap();

        let peca_file = out_dir.path().join("test.peca");
        let manifest =
            create_peca(src_dir.path(), &peca_file, 3).expect("create_peca should succeed");
        assert_eq!(manifest.package.name, "Test Mod");

        let read_manifest =
            read_manifest_from_peca(&peca_file).expect("should read manifest without extraction");
        assert_eq!(read_manifest.package.name, "Test Mod");

        let extracted_manifest =
            extract_peca(&peca_file, extract_dir.path()).expect("extract_peca should succeed");
        assert_eq!(extracted_manifest.package.name, "Test Mod");
        assert!(extract_dir.path().join("readme.txt").exists());
    }

    #[test]
    fn test_extract_layer_maps_paths_and_skips_manifest() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let layer_dir = tempdir().unwrap();

        let manifest_content = r#"
[package]
name = "Test Mod"
author = "Tester"
version = "1.0.0"

[target]
game_id = "test_game"
"#;
        fs::write(src_dir.path().join(MANIFEST_FILENAME), manifest_content).unwrap();
        fs::create_dir_all(src_dir.path().join("SKSE/Plugins")).unwrap();
        fs::write(src_dir.path().join("SKSE/Plugins/a.ini"), "x").unwrap();

        let peca_file = out_dir.path().join("test.peca");
        create_peca(src_dir.path(), &peca_file, 3).unwrap();

        let manifest = extract_peca_layer(&peca_file, layer_dir.path(), |p| {
            match p.strip_prefix("SKSE") {
                Ok(rest) => Path::new("Data/SKSE").join(rest),
                Err(_) => p.to_path_buf(),
            }
        })
        .unwrap();

        assert_eq!(manifest.package.name, "Test Mod");
        assert!(layer_dir.path().join("Data/SKSE/Plugins/a.ini").exists());
        assert!(!layer_dir.path().join("SKSE").exists());
        assert!(!layer_dir.path().join(MANIFEST_FILENAME).exists());
    }

    #[test]
    fn test_create_fails_without_manifest() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let peca_file = out_dir.path().join("invalid.peca");

        let err = create_peca(src_dir.path(), &peca_file, 3).unwrap_err();
        match err {
            ArchiveError::MissingManifest => (),
            _ => panic!("Expected MissingManifest error, got {err:?}"),
        }
    }

    #[test]
    fn test_vfi_cache_generation_and_loading() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let vfi_cache_dir = tempdir().unwrap();

        let manifest_content = r#"
[package]
name = "Combat Overhaul"
author = "Modder"
version = "2.1.0"
priority = "high"

[target]
game_id = "skyrim"
"#;
        fs::write(src_dir.path().join(MANIFEST_FILENAME), manifest_content).unwrap();
        fs::create_dir_all(src_dir.path().join("scripts")).unwrap();
        fs::write(src_dir.path().join("scripts/combat.pex"), "binary").unwrap();
        fs::write(src_dir.path().join("CombatOverhaul.esp"), "esp").unwrap();

        let peca_file = out_dir.path().join("combat.peca");
        create_peca(src_dir.path(), &peca_file, 3).unwrap();

        // 1. Generate package index
        let index =
            generate_package_file_index(&peca_file).expect("Index generation should succeed");
        assert_eq!(index.package_name, "Combat Overhaul");
        assert_eq!(index.files.len(), 2);
        assert!(index.files.iter().any(|f| f.ends_with("combat.pex")));

        // 2. Save cache
        let cache_file = save_package_vfi_cache(vfi_cache_dir.path(), &index)
            .expect("Save cache should succeed");
        assert!(cache_file.exists());

        // 3. Load cache
        let loaded = load_package_vfi_cache(vfi_cache_dir.path(), "Combat Overhaul")
            .expect("Load cache should succeed")
            .expect("Cache file should exist");
        assert_eq!(loaded, index);
    }

    #[test]
    fn test_extract_peca_stream_in_memory() {
        let src_dir = tempdir().unwrap();
        let out_dir = tempdir().unwrap();
        let extract_dir = tempdir().unwrap();

        let manifest_content = r#"
[package]
name = "Streamed Mod"
author = "Tester"
version = "3.0.0"

[target]
game_id = "test_game"
"#;
        fs::write(src_dir.path().join(MANIFEST_FILENAME), manifest_content).unwrap();
        fs::write(src_dir.path().join("content.dat"), "streamed bytes").unwrap();

        let peca_file = out_dir.path().join("stream.peca");
        create_peca(src_dir.path(), &peca_file, 3).unwrap();

        // Read all archive bytes into in-memory cursor to test stream extraction
        let bytes = fs::read(&peca_file).unwrap();
        let cursor = std::io::Cursor::new(bytes);

        let (manifest, index) = extract_peca_stream(cursor, extract_dir.path())
            .expect("In-memory streaming extraction should succeed");

        assert_eq!(manifest.package.name, "Streamed Mod");
        assert_eq!(index.files, vec!["content.dat"]);
        assert_eq!(
            fs::read_to_string(extract_dir.path().join("content.dat")).unwrap(),
            "streamed bytes"
        );
    }
}
