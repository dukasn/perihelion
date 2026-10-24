use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::stream::Stream;
use futures_util::StreamExt;
use reqwest::Response;
use tokio::io::AsyncRead;
use tokio::sync::mpsc;
use tokio_util::io::{StreamReader, SyncIoBridge};

use crate::NetError;

pub type BoxedAsyncRead = Box<dyn AsyncRead + Unpin + Send>;

/// Converts a `reqwest::Response` byte stream into an `AsyncRead`.
pub fn response_to_async_read(response: Response) -> impl AsyncRead + Unpin + Send {
    let byte_stream = response
        .bytes_stream()
        .map(|result| result.map_err(|err| io::Error::other(err.to_string())));
    StreamReader::new(byte_stream)
}

/// Initiates an HTTP GET request and returns the response body as a boxed `AsyncRead` stream.
pub async fn fetch_http_stream(url: &str) -> Result<BoxedAsyncRead, NetError> {
    let client = reqwest::Client::builder().build()?;
    let response = client.get(url).send().await?.error_for_status()?;
    Ok(Box::new(response_to_async_read(response)))
}

/// Bridges any `AsyncRead` stream into a synchronous `std::io::Read` instance.
///
/// This allows feeding network streams directly into synchronous decoders
/// (such as `zstd::stream::Decoder` and `tar::Archive`) inside `tokio::task::spawn_blocking`.
pub fn bridge_to_sync<R: AsyncRead + Unpin + Send + 'static>(async_reader: R) -> SyncIoBridge<R> {
    SyncIoBridge::new(async_reader)
}

/// A stream receiver for incoming P2P pieces/chunks delivered across an async channel.
pub struct PieceStreamReceiver {
    rx: mpsc::Receiver<io::Result<Bytes>>,
}

impl PieceStreamReceiver {
    pub fn new(rx: mpsc::Receiver<io::Result<Bytes>>) -> Self {
        Self { rx }
    }

    /// Converts the piece channel into an `AsyncRead` stream.
    pub fn into_async_read(self) -> impl AsyncRead + Unpin + Send {
        StreamReader::new(self)
    }
}

impl Stream for PieceStreamReceiver {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[tokio::test]
    async fn test_piece_channel_to_sync_io_bridge() {
        let (tx, rx) = mpsc::channel(4);
        let receiver = PieceStreamReceiver::new(rx);
        let async_reader = receiver.into_async_read();
        let mut sync_reader = bridge_to_sync(async_reader);

        // Spawn sender
        tokio::spawn(async move {
            tx.send(Ok(Bytes::from_static(b"Hello "))).await.unwrap();
            tx.send(Ok(Bytes::from_static(b"Streaming ")))
                .await
                .unwrap();
            tx.send(Ok(Bytes::from_static(b"World!"))).await.unwrap();
        });

        // Read synchronously using spawn_blocking
        let result = tokio::task::spawn_blocking(move || {
            let mut buf = String::new();
            sync_reader.read_to_string(&mut buf).unwrap();
            buf
        })
        .await
        .unwrap();

        assert_eq!(result, "Hello Streaming World!");
    }
}
