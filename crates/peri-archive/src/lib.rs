//! Perihelion Archive Adapter (`peri-archive`)
//!
//! Provides creation, extraction, and inspection of `.peca` archives
//! (Zstandard-compressed tarballs containing a mandatory root `peri.toml` manifest).

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};

use peri_core::{Manifest, PackageFileIndex, MANIFEST_FILENAME};
use sha2::{Digest, Sha256};
use thiserror::Error;

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

    #[error("Checksum mismatch: expected {expected}, calculated {calculated}")]
    ChecksumMismatch {
        expected: String,
        calculated: String,
    },
}

/// Computes the SHA-256 hash of a file as a lowercase hexadecimal string.
pub fn compute_file_sha256(path: &Path) -> Result<String, ArchiveError> {
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

    Ok(manifest)
}

/// Extracts a `.peca` archive to `destination_dir`.
/// Validates that a root `peri.toml` exists upon extraction and returns the parsed manifest.
pub fn extract_peca(peca_path: &Path, destination_dir: &Path) -> Result<Manifest, ArchiveError> {
    let file = File::open(peca_path)?;
    let buf_reader = BufReader::new(file);
    let decoder = zstd::stream::Decoder::new(buf_reader)
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    let mut archive = tar::Archive::new(decoder);

    fs::create_dir_all(destination_dir)?;
    archive.unpack(destination_dir)?;

    let manifest_path = destination_dir.join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(ArchiveError::MissingManifest);
    }

    let manifest_content = fs::read_to_string(manifest_path)?;
    let manifest = Manifest::from_toml_str(&manifest_content)?;

    Ok(manifest)
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
}
