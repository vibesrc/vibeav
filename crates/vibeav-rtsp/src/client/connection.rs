//! RTSP client connection handler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

use vibeav_core::Router;
use vibeav_sdp::parse;

use crate::error::{Result, RtspError};
use crate::message::{Request, Response};
use crate::transport::Transport;

use super::config::RtspClientConfig;
use super::session::{ClientSession, ClientState};

/// Buffer size for reading.
const READ_BUFFER_SIZE: usize = 8192;

/// Maximum response size.
const MAX_RESPONSE_SIZE: usize = 64 * 1024;

/// RTSP client for pulling streams from remote servers.
pub struct RtspClient {
    /// Client configuration.
    config: RtspClientConfig,
    /// Router for forwarding packets.
    router: Arc<Router>,
    /// Running flag.
    running: AtomicBool,
    /// Shutdown sender.
    shutdown_tx: broadcast::Sender<()>,
}

impl RtspClient {
    /// Create a new RTSP client.
    pub fn new(config: RtspClientConfig, router: Arc<Router>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            router,
            running: AtomicBool::new(false),
            shutdown_tx,
        }
    }

    /// Start the client (connects and begins pulling stream).
    pub async fn start(&self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(RtspError::Protocol("Client already running".into()));
        }

        info!(
            source = %self.config.source_url,
            stream_path = %self.config.stream_path,
            "Starting RTSP client"
        );

        // Create stream in router
        let _ = self.router.create_stream(&self.config.stream_path).await;

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let mut attempt = 0u32;

        loop {
            attempt += 1;

            tokio::select! {
                biased;

                _ = shutdown_rx.recv() => {
                    info!("RTSP client shutdown requested");
                    break;
                }

                result = self.run_session() => {
                    match result {
                        Ok(()) => {
                            debug!("Session ended normally");
                        }
                        Err(e) => {
                            error!(error = %e, attempt = attempt, "Session error");
                        }
                    }

                    if !self.config.reconnect {
                        break;
                    }

                    if self.config.max_reconnect_attempts > 0
                        && attempt >= self.config.max_reconnect_attempts
                    {
                        error!("Max reconnect attempts reached");
                        break;
                    }

                    info!(
                        delay = ?self.config.reconnect_delay,
                        attempt = attempt,
                        "Reconnecting..."
                    );

                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            info!("Shutdown during reconnect delay");
                            break;
                        }
                        _ = tokio::time::sleep(self.config.reconnect_delay) => {}
                    }
                }
            }
        }

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Stop the client.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Check if client is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Run a single session (connect, setup, play, receive).
    async fn run_session(&self) -> Result<()> {
        let (host, port) = self
            .config
            .host_port()
            .ok_or_else(|| RtspError::Protocol("Invalid source URL".into()))?;

        // Connect to server
        let addr = format!("{}:{}", host, port);
        debug!(addr = %addr, "Connecting to RTSP server");

        let stream = timeout(self.config.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| RtspError::Io("Connection timeout".into()))?
            .map_err(|e| RtspError::Io(e.to_string()))?;

        info!(addr = %addr, "Connected to RTSP server");

        let mut conn = ClientConnection::new(
            stream,
            self.config.clone(),
            self.router.clone(),
            self.shutdown_tx.subscribe(),
        );

        conn.run().await
    }

    /// Get the source URL.
    pub fn source_url(&self) -> &str {
        &self.config.source_url
    }

    /// Get the stream path.
    pub fn stream_path(&self) -> &str {
        &self.config.stream_path
    }
}

/// Internal connection handler.
struct ClientConnection {
    /// TCP socket.
    socket: TcpStream,
    /// Configuration.
    config: RtspClientConfig,
    /// Router.
    router: Arc<Router>,
    /// Session state.
    session: ClientSession,
    /// Shutdown receiver.
    shutdown: broadcast::Receiver<()>,
    /// Read buffer.
    buffer: BytesMut,
}

impl ClientConnection {
    fn new(
        socket: TcpStream,
        config: RtspClientConfig,
        router: Arc<Router>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        let session = ClientSession::new(&config.source_url);
        Self {
            socket,
            config,
            router,
            session,
            shutdown,
            buffer: BytesMut::with_capacity(READ_BUFFER_SIZE),
        }
    }

    /// Run the connection: OPTIONS -> DESCRIBE -> SETUP -> PLAY -> receive loop.
    async fn run(&mut self) -> Result<()> {
        self.session.set_state(ClientState::Connected);

        // Set source on the stream
        let source_id = format!("rtsp-client:{}", self.config.source_url);
        self.router
            .set_source(&self.config.stream_path, &source_id)
            .await
            .map_err(|e| RtspError::Protocol(format!("Failed to set source: {}", e)))?;

        // OPTIONS
        self.send_options().await?;
        self.session.set_state(ClientState::Options);

        // DESCRIBE
        self.send_describe().await?;
        self.session.set_state(ClientState::Described);

        // SETUP each track
        let controls: Vec<String> = self.session.track_controls();
        for control in controls {
            self.send_setup(&control).await?;
        }
        self.session.set_state(ClientState::Ready);

        // PLAY
        self.send_play().await?;
        self.session.set_state(ClientState::Playing);

        // Receive RTP loop
        self.receive_loop().await?;

        // TEARDOWN
        self.send_teardown().await.ok();

        // Clear source
        let _ = self.router.clear_source(&self.config.stream_path).await;

        Ok(())
    }

    /// Send OPTIONS request.
    async fn send_options(&mut self) -> Result<()> {
        let cseq = self.session.next_cseq();
        let request = Request::options(&self.config.source_url)
            .with_cseq(cseq)
            .with_user_agent(&self.config.user_agent);

        debug!(cseq = cseq, "Sending OPTIONS");
        self.send_request(&request).await?;

        let response = self.receive_response().await?;
        if !response.status.is_success() {
            return Err(RtspError::Protocol(format!(
                "OPTIONS failed: {} {}",
                response.status, response.reason
            )));
        }

        debug!(public = ?response.headers.get("Public"), "OPTIONS response");
        Ok(())
    }

    /// Send DESCRIBE request.
    async fn send_describe(&mut self) -> Result<()> {
        let cseq = self.session.next_cseq();
        let request = Request::describe(&self.config.source_url)
            .with_cseq(cseq)
            .with_user_agent(&self.config.user_agent);

        debug!(cseq = cseq, "Sending DESCRIBE");
        self.send_request(&request).await?;

        let response = self.receive_response().await?;
        if !response.status.is_success() {
            return Err(RtspError::Protocol(format!(
                "DESCRIBE failed: {} {}",
                response.status, response.reason
            )));
        }

        // Get Content-Base if present
        if let Some(base) = response.headers.get("Content-Base") {
            self.session.set_content_base(base.trim_end_matches('/'));
        }

        // Parse SDP body
        let body = response
            .body
            .as_ref()
            .ok_or_else(|| RtspError::Protocol("No SDP in DESCRIBE response".into()))?;

        let sdp_str = std::str::from_utf8(body)
            .map_err(|_| RtspError::Protocol("Invalid UTF-8 in SDP".into()))?;

        let sdp = parse(sdp_str)
            .map_err(|e| RtspError::Protocol(format!("Failed to parse SDP: {}", e)))?;

        debug!(
            tracks = sdp.media.len(),
            session_name = %sdp.name,
            "Parsed SDP"
        );

        // Store track info on the stream
        for media in &sdp.media {
            if let Some(rtpmap) = media.rtpmap.first() {
                let track_info = vibeav_core::TrackInfo {
                    track_type: match media.media_type.as_str() {
                        "video" => vibeav_core::TrackType::Video,
                        "audio" => vibeav_core::TrackType::Audio,
                        _ => vibeav_core::TrackType::Data,
                    },
                    payload_type: rtpmap.payload_type,
                    clock_rate: rtpmap.clock_rate,
                    encoding: rtpmap.encoding.clone(),
                    parameters: media.fmtp.first().map(|f| f.parameters.clone()),
                };

                if let Some(stream) = self.router.get_stream(&self.config.stream_path).await {
                    stream.add_track(track_info).await;
                }
            }
        }

        self.session.set_sdp(sdp);
        Ok(())
    }

    /// Send SETUP request for a track.
    async fn send_setup(&mut self, control: &str) -> Result<()> {
        let cseq = self.session.next_cseq();

        // Allocate interleaved channels
        let (rtp_ch, rtcp_ch) = self.session.allocate_channels(control);

        // Build transport header
        let transport_str = if self.config.prefer_tcp {
            format!("RTP/AVP/TCP;unicast;interleaved={}-{}", rtp_ch, rtcp_ch)
        } else {
            "RTP/AVP;unicast".to_string()
        };

        let mut request = Request::setup(control)
            .with_cseq(cseq)
            .with_user_agent(&self.config.user_agent)
            .with_header("Transport", &transport_str);

        // Include session ID if we have one
        if let Some(session_id) = self.session.session_id() {
            request = request.with_header("Session", session_id);
        }

        debug!(cseq = cseq, control = %control, transport = %transport_str, "Sending SETUP");
        self.send_request(&request).await?;

        let response = self.receive_response().await?;
        if !response.status.is_success() {
            return Err(RtspError::Protocol(format!(
                "SETUP failed: {} {}",
                response.status, response.reason
            )));
        }

        // Extract session ID
        if let Some(session_header) = response.headers.get("Session") {
            let session_id = session_header.split(';').next().unwrap_or(session_header);
            self.session.set_session_id(session_id.trim());

            // Extract timeout if present
            if let Some(timeout_part) = session_header.split(';').find(|s| s.contains("timeout")) {
                if let Some(timeout_str) = timeout_part.split('=').nth(1) {
                    if let Ok(timeout) = timeout_str.trim().parse::<u32>() {
                        self.session.set_timeout(timeout);
                    }
                }
            }
        }

        // Parse transport response
        if let Some(transport_header) = response.headers.get("Transport") {
            let transport = Transport::parse(transport_header)?;

            // Update track with transport info
            if let Some(track) = self.session.track_mut(control) {
                // Server may have assigned different channels
                if let Some((rtp, rtcp)) = transport.interleaved {
                    track.rtp_channel = Some(rtp);
                    track.rtcp_channel = Some(rtcp);
                }
                track.transport = Some(transport);
            }
        }

        debug!(
            session = ?self.session.session_id(),
            control = %control,
            "SETUP complete"
        );
        Ok(())
    }

    /// Send PLAY request.
    async fn send_play(&mut self) -> Result<()> {
        let cseq = self.session.next_cseq();
        let session_id = self
            .session
            .session_id()
            .ok_or_else(|| RtspError::Protocol("No session ID".into()))?;

        let request = Request::play(self.session.base_url())
            .with_cseq(cseq)
            .with_user_agent(&self.config.user_agent)
            .with_header("Session", session_id)
            .with_header("Range", "npt=0.000-");

        debug!(cseq = cseq, "Sending PLAY");
        self.send_request(&request).await?;

        let response = self.receive_response().await?;
        if !response.status.is_success() {
            return Err(RtspError::Protocol(format!(
                "PLAY failed: {} {}",
                response.status, response.reason
            )));
        }

        info!(
            stream_path = %self.config.stream_path,
            source = %self.config.source_url,
            "PLAY started"
        );
        Ok(())
    }

    /// Send TEARDOWN request.
    async fn send_teardown(&mut self) -> Result<()> {
        let cseq = self.session.next_cseq();

        let mut request = Request::teardown(self.session.base_url())
            .with_cseq(cseq)
            .with_user_agent(&self.config.user_agent);

        if let Some(session_id) = self.session.session_id() {
            request = request.with_header("Session", session_id);
        }

        debug!(cseq = cseq, "Sending TEARDOWN");
        self.send_request(&request).await?;

        // Try to read response but don't fail if we can't
        let _ = timeout(Duration::from_secs(2), self.receive_response()).await;

        self.session.set_state(ClientState::Disconnected);
        Ok(())
    }

    /// Main receive loop for RTP packets.
    async fn receive_loop(&mut self) -> Result<()> {
        let mut read_buf = [0u8; READ_BUFFER_SIZE];

        loop {
            tokio::select! {
                biased;

                _ = self.shutdown.recv() => {
                    debug!("Shutdown signal received");
                    break;
                }

                result = self.socket.read(&mut read_buf) => {
                    match result {
                        Ok(0) => {
                            debug!("Connection closed by server");
                            break;
                        }
                        Ok(n) => {
                            self.buffer.extend_from_slice(&read_buf[..n]);
                            self.process_buffer().await?;
                        }
                        Err(e) => {
                            warn!(error = %e, "Read error");
                            return Err(RtspError::Io(e.to_string()));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process data in buffer.
    async fn process_buffer(&mut self) -> Result<()> {
        while !self.buffer.is_empty() {
            // Check for interleaved RTP/RTCP (starts with '$')
            if self.buffer[0] == b'$' {
                if self.buffer.len() < 4 {
                    break; // Need more data
                }

                let channel = self.buffer[1];
                let length = u16::from_be_bytes([self.buffer[2], self.buffer[3]]) as usize;

                if self.buffer.len() < 4 + length {
                    break; // Need more data
                }

                let data = self.buffer.split_to(4 + length).split_off(4).freeze();
                self.handle_interleaved(channel, data).await?;
            } else if self.buffer.starts_with(b"RTSP/") {
                // RTSP response (shouldn't happen in receive loop normally)
                // Skip it for now - could be async response
                if let Some(end) = self.find_response_end() {
                    let _ = self.buffer.split_to(end);
                } else {
                    break;
                }
            } else {
                // Unknown data - skip one byte and continue
                warn!(byte = self.buffer[0], "Unknown data in buffer");
                let _ = self.buffer.split_to(1);
            }
        }

        Ok(())
    }

    /// Handle interleaved RTP/RTCP data.
    async fn handle_interleaved(&mut self, channel: u8, data: Bytes) -> Result<()> {
        trace!(channel = channel, len = data.len(), "Received interleaved data");

        // Check if this is an RTP channel (even channels are RTP, odd are RTCP)
        if let Some(track) = self.session.track_by_channel(channel) {
            if track.rtp_channel == Some(channel) {
                // RTP packet - forward to router
                self.router
                    .on_rtp(&self.config.stream_path, data)
                    .await
                    .ok();
            }
            // RTCP packets are ignored for now
        }

        Ok(())
    }

    /// Send an RTSP request.
    async fn send_request(&mut self, request: &Request) -> Result<()> {
        let bytes = request.to_bytes();
        trace!(len = bytes.len(), "Sending request");

        timeout(self.config.write_timeout, self.socket.write_all(&bytes))
            .await
            .map_err(|_| RtspError::Io("Write timeout".into()))?
            .map_err(|e| RtspError::Io(e.to_string()))?;

        Ok(())
    }

    /// Receive an RTSP response.
    async fn receive_response(&mut self) -> Result<Response> {
        let mut read_buf = [0u8; READ_BUFFER_SIZE];

        loop {
            // Try to parse response from buffer
            if let Some(response) = self.try_parse_response()? {
                return Ok(response);
            }

            // Need more data
            let n = timeout(self.config.read_timeout, self.socket.read(&mut read_buf))
                .await
                .map_err(|_| RtspError::Io("Read timeout".into()))?
                .map_err(|e| RtspError::Io(e.to_string()))?;

            if n == 0 {
                return Err(RtspError::Io("Connection closed".into()));
            }

            self.buffer.extend_from_slice(&read_buf[..n]);

            if self.buffer.len() > MAX_RESPONSE_SIZE {
                return Err(RtspError::MessageTooLarge {
                    actual: self.buffer.len(),
                    max: MAX_RESPONSE_SIZE,
                });
            }
        }
    }

    /// Try to parse a response from the buffer.
    fn try_parse_response(&mut self) -> Result<Option<Response>> {
        // Find end of headers
        let header_end = self
            .buffer
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4);

        let Some(header_end) = header_end else {
            return Ok(None);
        };

        // Parse headers to get Content-Length
        let header_bytes = &self.buffer[..header_end];
        let header_str = std::str::from_utf8(header_bytes)
            .map_err(|_| RtspError::Protocol("Invalid UTF-8 in response".into()))?;

        // Extract Content-Length
        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let total_length = header_end + content_length;
        if self.buffer.len() < total_length {
            return Ok(None);
        }

        // Parse the response
        let response_bytes = self.buffer.split_to(total_length).freeze();
        let (response, _) = Response::parse(&response_bytes)?;

        Ok(Some(response))
    }

    /// Find the end of a response in the buffer.
    fn find_response_end(&self) -> Option<usize> {
        let header_end = self
            .buffer
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4)?;

        let header_str = std::str::from_utf8(&self.buffer[..header_end]).ok()?;

        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let total = header_end + content_length;
        if self.buffer.len() >= total {
            Some(total)
        } else {
            None
        }
    }
}
