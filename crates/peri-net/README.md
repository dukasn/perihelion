# peri-net

Network adapter for Perihelion.

`peri-net` provides dual-transport package fetching across BitTorrent swarms and direct HTTP streams. It verifies package checksums and pipes asynchronous network chunks straight into synchronous extraction jobs.

---

## Transport Architecture

Centralized mod hubs throttle downloads, impose strict rate limits, and suffer outages. `peri-net` decouples distribution from single servers:

```
                  ┌────────────────────────┐
                  │ Manifest Assets Record │
                  └───────────┬────────────┘
                              │
               ┌──────────────┴──────────────┐
               ▼                             ▼
       [P2P Magnet Swarm]           [HTTP Direct / WebSeed]
        librqbit session                reqwest stream
               │                             │
               └──────────────┬──────────────┘
                              │
                              ▼
                     [AsyncRead Stream]
                              │
                              ▼
                     [SyncIoBridge Pipe]
                              │
                              ▼
                [peri-archive Stream Unpack]
```

### 1. BitTorrent Swarm Engine (`torrent.rs`)
Powered by `librqbit` under the optional `bittorrent` cargo feature:
- Downloads `.peca` packages over distributed swarms through magnet links or `.torrent` files.
- Seeds completed archives back to peers straight from the local VFS cache.
- Configurable upload rate limits to avoid saturating host connections.

### 2. HTTP Streaming & WebSeed Fallback (`stream.rs`)
When swarms lack seeders or when grabbing files from direct mirrors:
- Streams byte chunks via `reqwest` without buffering multi-gigabyte files into RAM.
- Verifies package SHA-256 hashes against manifest records.

### 3. Async-to-Sync Stream Bridging (`bridge_to_sync`)
Archive decoders run synchronously. Instead of saving full archives to disk before unpacking, `bridge_to_sync` wraps streams with `tokio_util::io::SyncIoBridge`. Incoming network bytes feed directly into decompression threads spawned inside `spawn_blocking`—zero intermediate disk writes required.

---

## Cargo Features

| Feature | Description | Default |
| :--- | :--- | :--- |
| `bittorrent` | Enables the `librqbit` engine for P2P downloads and seeding. | Disabled |

---

## Example Usage

```rust
use peri_net::{fetch_http_stream, bridge_to_sync};
use peri_archive::extract_peca_stream;
use std::path::Path;

// Stream an archive over HTTP and unpack it directly to disk
let async_stream = fetch_http_stream("https://example.com/mod.peca").await?;
let sync_reader = bridge_to_sync(async_stream);

tokio::task::spawn_blocking(move || {
    let dest = Path::new("./cache/mod");
    extract_peca_stream(sync_reader, dest)
}).await??;
```
