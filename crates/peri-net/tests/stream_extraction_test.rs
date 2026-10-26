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

use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use peri_archive::{create_peca, extract_peca_stream};
use peri_core::MANIFEST_FILENAME;
use peri_net::{bridge_to_sync, fetch_http_stream};

#[tokio::test]
async fn test_http_streaming_direct_extraction() {
    let src_dir = tempdir().unwrap();
    let archive_dir = tempdir().unwrap();
    let dest_dir = tempdir().unwrap();

    let manifest_content = r#"
[package]
name = "StreamingPack"
author = "Dev"
version = "1.0.0"

[target]
game_id = "generic"
"#;
    fs::write(src_dir.path().join(MANIFEST_FILENAME), manifest_content).unwrap();
    fs::create_dir_all(src_dir.path().join("textures")).unwrap();
    fs::write(
        src_dir.path().join("textures/sky.dds"),
        "RAW_TEXTURE_PAYLOAD_BYTES",
    )
    .unwrap();

    let peca_path = archive_dir.path().join("streaming_pack.peca");
    create_peca(src_dir.path(), &peca_path, 3).unwrap();
    let archive_bytes = Arc::new(fs::read(&peca_path).unwrap());

    // Bind local TCP listener for HTTP serving
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let payload = Arc::clone(&archive_bytes);

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;

            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                payload.len()
            );

            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&payload).await;
            let _ = socket.shutdown().await;
        }
    });

    let url = format!("http://127.0.0.1:{}/mod.peca", addr.port());

    // 1. Fetch HTTP stream as AsyncRead
    let async_stream = fetch_http_stream(&url)
        .await
        .expect("HTTP stream connection should succeed");

    // 2. Bridge AsyncRead into SyncIoBridge
    let sync_reader = bridge_to_sync(async_stream);

    // 3. Extract stream directly inside spawn_blocking thread
    let dest = dest_dir.path().to_path_buf();
    let (manifest, index) =
        tokio::task::spawn_blocking(move || extract_peca_stream(sync_reader, &dest))
            .await
            .expect("Task join should succeed")
            .expect("Streaming extraction should succeed");

    assert_eq!(manifest.package.name, "StreamingPack");
    assert!(index.files.iter().any(|f| f == "textures/sky.dds"));

    let extracted_file = dest_dir.path().join("textures/sky.dds");
    assert!(extracted_file.exists());
    assert_eq!(
        fs::read_to_string(extracted_file).unwrap(),
        "RAW_TEXTURE_PAYLOAD_BYTES"
    );
}
