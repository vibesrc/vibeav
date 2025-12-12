//! RTSP connection handler.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, trace, warn};

use vibeav_core::server::ServerContext;

use crate::error::{Result, RtspError};
use crate::message::{Request, Response};

use super::handler::Handler;
use super::session::RtspSession;

/// Buffer size for reading.
const READ_BUFFER_SIZE: usize = 8192;

/// Maximum request size.
const MAX_REQUEST_SIZE: usize = 64 * 1024;

/// Outbound packet from sink (channel + data).
type OutboundPacket = (u8, Bytes);

/// An RTSP connection.
pub struct Connection {
    /// TCP socket.
    socket: TcpStream,
    /// Peer address.
    peer: SocketAddr,
    /// Server context.
    context: ServerContext,
    /// Sessions registry.
    sessions: Arc<RwLock<HashMap<String, Arc<RtspSession>>>>,
    /// Current session for this connection.
    current_session: Option<Arc<RtspSession>>,
    /// Multiplexed outbound channel from sink receiver tasks.
    outbound_tx: mpsc::Sender<OutboundPacket>,
    /// Receiver for multiplexed outbound packets.
    outbound_rx: mpsc::Receiver<OutboundPacket>,
    /// Shutdown signal.
    shutdown: broadcast::Receiver<()>,
    /// Read buffer.
    buffer: BytesMut,
}

impl Connection {
    /// Create a new connection.
    pub fn new(
        socket: TcpStream,
        peer: SocketAddr,
        context: ServerContext,
        sessions: Arc<RwLock<HashMap<String, Arc<RtspSession>>>>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        // Create the outbound channel for multiplexing sink receiver packets
        let (outbound_tx, outbound_rx) = mpsc::channel(256);

        Self {
            socket,
            peer,
            context,
            sessions,
            current_session: None,
            outbound_tx,
            outbound_rx,
            shutdown,
            buffer: BytesMut::with_capacity(READ_BUFFER_SIZE),
        }
    }

    /// Run the connection handler.
    pub async fn run(mut self) -> Result<()> {
        debug!(peer = %self.peer, "Connection started");

        let mut read_buf = [0u8; READ_BUFFER_SIZE];

        loop {
            tokio::select! {
                biased;

                // Check for shutdown
                _ = self.shutdown.recv() => {
                    debug!(peer = %self.peer, "Connection shutdown requested");
                    break;
                }

                // Receive outbound packets from sinks (for playback)
                Some((channel, data)) = self.outbound_rx.recv() => {
                    debug!(peer = %self.peer, channel = channel, len = data.len(), "Connection: sending interleaved data");
                    if let Err(e) = self.send_interleaved(channel, &data).await {
                        warn!(peer = %self.peer, error = %e, "Failed to send interleaved data");
                        break;
                    }
                }

                // Read from socket
                result = self.socket.read(&mut read_buf) => {
                    match result {
                        Ok(0) => {
                            debug!(peer = %self.peer, "Connection closed by peer");
                            break;
                        }
                        Ok(n) => {
                            self.buffer.extend_from_slice(&read_buf[..n]);
                            // Try to parse and handle messages
                            while let Some(action) = self.try_parse_message()? {
                                match action {
                                    MessageAction::Request(request) => {
                                        self.handle_request(request).await?;
                                    }
                                    MessageAction::Interleaved { channel, data } => {
                                        self.handle_interleaved(channel, data).await?;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(peer = %self.peer, error = %e, "Read error");
                            break;
                        }
                    }
                }
            }
        }

        // Cleanup
        self.cleanup().await;
        debug!(peer = %self.peer, "Connection ended");
        Ok(())
    }

    /// Try to parse a message from the buffer.
    fn try_parse_message(&mut self) -> Result<Option<MessageAction>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        // Check for interleaved RTP/RTCP (starts with '$')
        if self.buffer[0] == b'$' {
            if self.buffer.len() < 4 {
                return Ok(None); // Need more data
            }
            let channel = self.buffer[1];
            let length = u16::from_be_bytes([self.buffer[2], self.buffer[3]]) as usize;

            if self.buffer.len() < 4 + length {
                return Ok(None); // Need more data
            }

            let data = self.buffer.split_to(4 + length).split_off(4).freeze();
            return Ok(Some(MessageAction::Interleaved { channel, data }));
        }

        // Try to parse RTSP request
        // Find end of headers (double CRLF)
        let header_end = self
            .buffer
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4);

        let Some(header_end) = header_end else {
            // Check if buffer is too large
            if self.buffer.len() > MAX_REQUEST_SIZE {
                return Err(RtspError::InvalidRequest("Request too large".into()));
            }
            return Ok(None);
        };

        // Parse headers to get Content-Length
        let header_bytes = &self.buffer[..header_end];
        let header_str = std::str::from_utf8(header_bytes)
            .map_err(|_| RtspError::InvalidRequest("Invalid UTF-8".into()))?;

        // Extract Content-Length if present
        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let total_length = header_end + content_length;
        if self.buffer.len() < total_length {
            return Ok(None); // Need more data
        }

        // Parse the full request
        let request_bytes = self.buffer.split_to(total_length).freeze();
        let (request, _) = Request::parse(&request_bytes)?;

        Ok(Some(MessageAction::Request(request)))
    }

    /// Handle an RTSP request.
    async fn handle_request(&mut self, request: Request) -> Result<()> {
        trace!(peer = %self.peer, method = %request.method, uri = %request.uri, "Request received");

        let mut handler = Handler::new(
            &self.context,
            &self.sessions,
            &mut self.current_session,
            self.outbound_tx.clone(),
        );

        let response = handler.handle(request).await;

        self.send_response(&response).await
    }

    /// Handle interleaved RTP/RTCP data.
    async fn handle_interleaved(&mut self, channel: u8, data: Bytes) -> Result<()> {
        trace!(peer = %self.peer, channel = channel, len = data.len(), "Interleaved data received");

        // Find which track this channel belongs to
        if let Some(session) = &self.current_session {
            if let Some(track) = session.track_by_channel(channel).await {
                // Forward to router
                if track.rtp_channel == Some(channel) {
                    // RTP packet
                    self.context
                        .router
                        .on_rtp(&track.stream_id, data)
                        .await
                        .ok();
                }
                // RTCP packets are currently ignored
            }
        }

        Ok(())
    }

    /// Send an RTSP response.
    async fn send_response(&mut self, response: &Response) -> Result<()> {
        let bytes = response.to_bytes();
        trace!(peer = %self.peer, status = %response.status, "Sending response");
        self.socket
            .write_all(&bytes)
            .await
            .map_err(|e| RtspError::Io(e.to_string()))?;
        Ok(())
    }

    /// Send interleaved data.
    async fn send_interleaved(&mut self, channel: u8, data: &[u8]) -> Result<()> {
        let mut frame = Vec::with_capacity(4 + data.len());
        frame.push(b'$');
        frame.push(channel);
        frame.extend_from_slice(&(data.len() as u16).to_be_bytes());
        frame.extend_from_slice(data);

        self.socket
            .write_all(&frame)
            .await
            .map_err(|e| RtspError::Io(e.to_string()))?;
        // Flush to ensure data is sent immediately
        self.socket
            .flush()
            .await
            .map_err(|e| RtspError::Io(e.to_string()))?;
        Ok(())
    }

    /// Cleanup on connection close.
    async fn cleanup(&mut self) {
        if let Some(session) = self.current_session.take() {
            // Remove session from registry
            self.sessions.write().await.remove(session.id());

            // Cleanup core resources
            if let Some(core_id) = session.core_session_id().await {
                let _ = self.context.router.remove_session(&core_id).await;
            }
        }
    }
}

/// Action from parsing a message.
enum MessageAction {
    Request(Request),
    Interleaved { channel: u8, data: Bytes },
}
