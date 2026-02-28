//! WebSocket stream wrapper for actor integration.
//!
//! This module provides a thin wrapper around sockudo-ws's `SplitReader` to
//! implement the `futures::Stream` trait, allowing it to be attached to a
//! Kameo actor for message processing.

use std::task::Poll;

use sockudo_ws::SplitReader;

/// Wrapper around a WebSocket reader that implements `futures::Stream`.
///
/// This allows the reader to be attached to a Kameo actor using `attach_stream()`,
/// enabling automatic message forwarding to the actor's message handler.
pub(crate) struct WebSocketStreamWrapper<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    reader: SplitReader<S>,
}

impl<S> WebSocketStreamWrapper<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    /// Creates a new wrapper around a WebSocket reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - The split WebSocket reader to wrap
    pub(crate) const fn new(reader: SplitReader<S>) -> Self {
        Self { reader }
    }
}

impl<S> futures::Stream for WebSocketStreamWrapper<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    type Item = Result<sockudo_ws::Message, sockudo_ws::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let fut = self.reader.next();
        tokio::pin!(fut);
        fut.poll(cx)
    }
}
