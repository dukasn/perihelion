//! Perihelion Archive Adapter (`peri-archive`)
//!
//! Provides creation, extraction, and inspection of `.peca` archives
//! (Zstandard-compressed tarballs containing a mandatory root `peri.toml` manifest).

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read};
use std::path::Path;

use peri_core::{Manifest, MANIFEST_FILENAME};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

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
}
