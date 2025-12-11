//! RTSP method handlers.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use vibeav_core::server::ServerContext;
use vibeav_core::{SinkReceiver, TrackInfo, TrackType};
use vibeav_sdp::parse as parse_sdp;

use crate::error::RtspError;
use crate::message::{Method, Request, Response, StatusCode};
use crate::transport::{LowerTransport, Transport};

use super::session::{RtspSession, SessionMode, SessionState, TrackSetup};

/// Outbound packet type.
pub type OutboundPacket = (u8, Bytes);

/// Request handler.
pub struct Handler<'a> {
    context: &'a ServerContext,
    sessions: &'a Arc<RwLock<HashMap<String, Arc<RtspSession>>>>,
    current_session: &'a mut Option<Arc<RtspSession>>,
    outbound_tx: mpsc::Sender<OutboundPacket>,
}

impl<'a> Handler<'a> {
    pub fn new(
        context: &'a ServerContext,
        sessions: &'a Arc<RwLock<HashMap<String, Arc<RtspSession>>>>,
        current_session: &'a mut Option<Arc<RtspSession>>,
        outbound_tx: mpsc::Sender<OutboundPacket>,
    ) -> Self {
        Self {
            context,
            sessions,
            current_session,
            outbound_tx,
        }
    }

    /// Handle a request and return response.
    pub async fn handle(&mut self, request: Request) -> Response {
        let cseq = request.headers.cseq().unwrap_or(0);

        let result = match request.method {
            Method::Options => self.handle_options(&request).await,
            Method::Describe => self.handle_describe(&request).await,
            Method::Announce => self.handle_announce(&request).await,
            Method::Setup => self.handle_setup(&request).await,
            Method::Play => self.handle_play(&request).await,
            Method::Record => self.handle_record(&request).await,
            Method::Pause => self.handle_pause(&request).await,
            Method::Teardown => self.handle_teardown(&request).await,
            Method::GetParameter => self.handle_get_parameter(&request).await,
            Method::SetParameter => self.handle_set_parameter(&request).await,
            _ => Err(RtspError::MethodNotAllowed(request.method.to_string())),
        };

        match result {
            Ok(mut response) => {
                response = response.with_cseq(cseq);
                response.headers.insert("Server", &self.context.server_name);
                response
            }
            Err(e) => {
                warn!(error = %e, "Request handler error");
                self.error_response(e, cseq)
            }
        }
    }

    /// Handle OPTIONS.
    async fn handle_options(&self, _request: &Request) -> Result<Response, RtspError> {
        Ok(Response::ok().with_public(&[
            "OPTIONS",
            "DESCRIBE",
            "ANNOUNCE",
            "SETUP",
            "PLAY",
            "RECORD",
            "PAUSE",
            "TEARDOWN",
            "GET_PARAMETER",
            "SET_PARAMETER",
        ]))
    }

    /// Handle DESCRIBE (client wants to receive stream).
    async fn handle_describe(&self, request: &Request) -> Result<Response, RtspError> {
        let stream_path = extract_path(&request.uri)?;

        // Check if stream exists in router
        let _stream = self
            .context
            .router
            .get_stream(&stream_path)
            .await
            .ok_or_else(|| RtspError::NotFound(stream_path.clone()))?;

        // Generate SDP for the stream
        let sdp = self.generate_sdp(&stream_path, &request.uri).await;

        Ok(Response::ok()
            .with_content_type("application/sdp")
            .with_content_base(&request.uri)
            .with_body(sdp.into_bytes()))
    }

    /// Handle ANNOUNCE (client wants to publish stream).
    async fn handle_announce(&mut self, request: &Request) -> Result<Response, RtspError> {
        let stream_path = extract_path(&request.uri)?;

        // Get or create stream in router
        let stream = if let Some(existing) = self.context.router.get_stream(&stream_path).await {
            existing
        } else {
            self.context
                .router
                .create_stream(&stream_path)
                .await
                .map_err(|e| RtspError::Protocol(format!("Failed to create stream: {}", e)))?
        };

        // Parse SDP from body if present
        if let Some(body) = &request.body {
            let sdp_str = std::str::from_utf8(body)
                .map_err(|_| RtspError::Protocol("Invalid UTF-8 in SDP".into()))?;

            let sdp = parse_sdp(sdp_str)
                .map_err(|e| RtspError::Protocol(format!("Failed to parse SDP: {}", e)))?;

            // Store track info on the stream
            for media in &sdp.media {
                if let Some(rtpmap) = media.rtpmap.first() {
                    let track_info = TrackInfo {
                        track_type: match media.media_type.as_str() {
                            "video" => TrackType::Video,
                            "audio" => TrackType::Audio,
                            _ => TrackType::Data,
                        },
                        payload_type: rtpmap.payload_type,
                        clock_rate: rtpmap.clock_rate,
                        encoding: rtpmap.encoding.clone(),
                        parameters: media.fmtp.first().map(|f| f.parameters.clone()),
                    };
                    stream.add_track(track_info).await;
                }
            }

            debug!(
                stream = %stream_path,
                tracks = sdp.media.len(),
                "ANNOUNCE: parsed SDP"
            );
        }

        // Create session in record mode
        let session = Arc::new(RtspSession::new());
        session.set_mode(SessionMode::Record).await;
        session.set_stream_path(stream_path.clone()).await;

        // Store session
        self.sessions
            .write()
            .await
            .insert(session.id().to_string(), session.clone());
        *self.current_session = Some(session.clone());

        debug!(stream = %stream_path, session = %session.id(), "ANNOUNCE: stream announced");

        Ok(Response::ok().with_session_timeout(session.id(), 60))
    }

    /// Handle SETUP.
    async fn handle_setup(&mut self, request: &Request) -> Result<Response, RtspError> {
        let transport_header = request
            .headers
            .get("Transport")
            .ok_or_else(|| RtspError::InvalidTransport("Missing Transport header".into()))?;

        let transport = Transport::parse(transport_header)?;

        // Get or create session
        let session = if let Some(session_id) = request.headers.session() {
            self.sessions
                .read()
                .await
                .get(session_id)
                .cloned()
                .ok_or_else(|| RtspError::SessionNotFound(session_id.to_string()))?
        } else if let Some(session) = self.current_session.as_ref() {
            session.clone()
        } else {
            // Create new session for playback
            let session = Arc::new(RtspSession::new());
            let stream_path = extract_path(&request.uri)?;
            session.set_stream_path(stream_path).await;
            session.set_mode(SessionMode::Play).await;
            self.sessions
                .write()
                .await
                .insert(session.id().to_string(), session.clone());
            *self.current_session = Some(session.clone());
            session
        };

        // Extract track control from URI
        let control = extract_control(&request.uri);
        let stream_path = session
            .stream_path()
            .await
            .ok_or_else(|| RtspError::InvalidRequest("No stream path set".into()))?;

        // Build response transport
        let response_transport = self.setup_transport(&transport, &session, &stream_path).await?;

        // Store track setup
        let track_setup = TrackSetup {
            control: control.clone(),
            stream_id: stream_path.clone(),
            transport: response_transport.clone(),
            rtp_channel: response_transport.interleaved.map(|(rtp, _)| rtp),
            rtcp_channel: response_transport.interleaved.map(|(_, rtcp)| rtcp),
        };
        session.add_track(control, track_setup).await;
        session.set_state(SessionState::Ready).await;

        debug!(
            session = %session.id(),
            transport = ?response_transport,
            "SETUP complete"
        );

        // Serialize transport to header string
        let transport_str = format_transport(&response_transport);

        Ok(Response::ok()
            .with_session_timeout(session.id(), 60)
            .with_transport(transport_str))
    }

    /// Handle PLAY.
    async fn handle_play(&mut self, request: &Request) -> Result<Response, RtspError> {
        let session = self.get_session(request).await?;

        let state = session.state().await;
        if state != SessionState::Ready && state != SessionState::Playing {
            return Err(RtspError::InvalidState(
                "PLAY requires Ready or Playing state".into(),
            ));
        }

        let stream_path = session
            .stream_path()
            .await
            .ok_or_else(|| RtspError::InvalidRequest("No stream path".into()))?;

        // Build RTP-Info header parts
        let mut rtp_info_parts = Vec::new();

        // Add sinks for each track and spawn receiver tasks
        for track in session.tracks().await {
            if let Some(rtp_channel) = track.rtp_channel {
                // Create sink for this track
                let sink_id = format!("{}:{}", session.id(), rtp_channel);
                match self
                    .context
                    .router
                    .add_sink(&stream_path, &sink_id, None)
                    .await
                {
                    Ok(receiver) => {
                        debug!(sink = %sink_id, channel = rtp_channel, "Sink added to stream");

                        // Spawn a task to forward packets from this receiver to the outbound channel
                        let outbound_tx = self.outbound_tx.clone();
                        tokio::spawn(async move {
                            Self::receiver_task(rtp_channel, receiver, outbound_tx).await;
                        });

                        // Build RTP-Info entry for this track
                        // RFC 2326: url=<track-url>;seq=<seq>;rtptime=<rtptime>
                        // Use the control URL from the request URI base
                        let track_url = format!("{}/{}", request.uri, track.control);
                        // Use initial values - actual values will be in RTP packets
                        rtp_info_parts.push(format!("url={};seq=0;rtptime=0", track_url));
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to add sink");
                    }
                }
            }
        }

        session.set_state(SessionState::Playing).await;
        debug!(session = %session.id(), "PLAY started");

        // RFC 2326: PLAY response MUST include Range and RTP-Info headers
        let mut response = Response::ok()
            .with_session_timeout(session.id(), 60)
            .with_range("npt=0.000-"); // Live stream

        if !rtp_info_parts.is_empty() {
            response = response.with_rtp_info(rtp_info_parts.join(","));
        }

        Ok(response)
    }

    /// Task to forward packets from a sink receiver to the outbound channel.
    async fn receiver_task(
        channel: u8,
        mut receiver: SinkReceiver,
        outbound_tx: mpsc::Sender<OutboundPacket>,
    ) {
        debug!(channel = channel, "Receiver task started, waiting for packets");
        loop {
            trace!(channel = channel, "Receiver task: calling recv()");
            match receiver.recv().await {
                Some(packet) => {
                    let len = packet.data.len();
                    debug!(channel = channel, len = len, "Receiver task: got packet, sending to outbound_tx");
                    match outbound_tx.send((channel, packet.data)).await {
                        Ok(()) => {
                            debug!(channel = channel, len = len, "Receiver task: send completed successfully");
                        }
                        Err(e) => {
                            debug!(channel = channel, error = %e, "Receiver task: outbound channel closed");
                            break;
                        }
                    }
                }
                None => {
                    // Receiver closed
                    debug!(channel = channel, "Receiver task: receiver returned None");
                    break;
                }
            }
        }
        debug!(channel = channel, "Receiver task ended");
    }

    /// Handle RECORD.
    async fn handle_record(&mut self, request: &Request) -> Result<Response, RtspError> {
        let session = self.get_session(request).await?;

        if session.mode().await != SessionMode::Record {
            return Err(RtspError::InvalidState("Not in record mode".into()));
        }

        let state = session.state().await;
        if state != SessionState::Ready {
            return Err(RtspError::InvalidState("RECORD requires Ready state".into()));
        }

        let stream_path = session
            .stream_path()
            .await
            .ok_or_else(|| RtspError::InvalidRequest("No stream path".into()))?;

        // Set source on the stream
        let source_id = format!("rtsp:{}", session.id());
        self.context
            .router
            .set_source(&stream_path, &source_id)
            .await
            .map_err(|e| RtspError::Protocol(format!("Failed to set source: {}", e)))?;

        session.set_state(SessionState::Recording).await;
        debug!(session = %session.id(), "RECORD started");

        Ok(Response::ok().with_session_timeout(session.id(), 60))
    }

    /// Handle PAUSE.
    async fn handle_pause(&mut self, request: &Request) -> Result<Response, RtspError> {
        let session = self.get_session(request).await?;

        // Stop sending/receiving but keep session
        session.set_state(SessionState::Ready).await;
        debug!(session = %session.id(), "PAUSE");

        Ok(Response::ok().with_session_timeout(session.id(), 60))
    }

    /// Handle TEARDOWN.
    async fn handle_teardown(&mut self, request: &Request) -> Result<Response, RtspError> {
        let session = self.get_session(request).await?;

        session.set_state(SessionState::Ended).await;

        // Remove from sessions registry
        self.sessions.write().await.remove(session.id());
        *self.current_session = None;

        debug!(session = %session.id(), "TEARDOWN");

        Ok(Response::ok())
    }

    /// Handle GET_PARAMETER (keep-alive).
    async fn handle_get_parameter(&self, _request: &Request) -> Result<Response, RtspError> {
        if let Some(session) = &self.current_session {
            session.touch().await;
        }
        Ok(Response::ok())
    }

    /// Handle SET_PARAMETER.
    async fn handle_set_parameter(&self, _request: &Request) -> Result<Response, RtspError> {
        Ok(Response::ok())
    }

    /// Get session from request or current connection.
    async fn get_session(&self, request: &Request) -> Result<Arc<RtspSession>, RtspError> {
        if let Some(session_id) = request.headers.session() {
            self.sessions
                .read()
                .await
                .get(session_id)
                .cloned()
                .ok_or_else(|| RtspError::SessionNotFound(session_id.to_string()))
        } else if let Some(session) = self.current_session.as_ref() {
            Ok(session.clone())
        } else {
            Err(RtspError::SessionNotFound("No session".into()))
        }
    }

    /// Setup transport and return response transport.
    async fn setup_transport(
        &mut self,
        client_transport: &Transport,
        session: &RtspSession,
        _stream_path: &str,
    ) -> Result<Transport, RtspError> {
        let mut response = client_transport.clone();

        // We currently only support TCP interleaved
        if client_transport.lower_transport != LowerTransport::Tcp {
            return Err(RtspError::UnsupportedTransport(
                "Only TCP interleaved is supported".into(),
            ));
        }

        // Ensure interleaved channels are set
        if response.interleaved.is_none() {
            // Assign channels based on track count
            let track_count = session.tracks().await.len() as u8;
            let rtp_channel = track_count * 2;
            let rtcp_channel = rtp_channel + 1;
            response.interleaved = Some((rtp_channel, rtcp_channel));
        }

        Ok(response)
    }

    /// Generate SDP for a stream.
    async fn generate_sdp(&self, stream_path: &str, base_uri: &str) -> String {
        let mut sdp = String::new();

        // Session-level lines
        sdp.push_str("v=0\r\n");
        sdp.push_str("o=- 0 0 IN IP4 0.0.0.0\r\n");
        sdp.push_str(&format!("s={}\r\n", stream_path));
        sdp.push_str("c=IN IP4 0.0.0.0\r\n");
        sdp.push_str("t=0 0\r\n");
        sdp.push_str(&format!("a=control:{}\r\n", base_uri));

        // Get tracks from stream
        if let Some(stream) = self.context.router.get_stream(stream_path).await {
            let tracks = stream.tracks().await;

            if tracks.is_empty() {
                // Fallback to default tracks if no track info
                sdp.push_str("m=video 0 RTP/AVP 96\r\n");
                sdp.push_str("a=rtpmap:96 H264/90000\r\n");
                sdp.push_str("a=control:trackID=0\r\n");
            } else {
                // Generate media lines from actual track info
                for (idx, track) in tracks.iter().enumerate() {
                    let media_type = match track.track_type {
                        TrackType::Video => "video",
                        TrackType::Audio => "audio",
                        TrackType::Data => "application",
                    };

                    sdp.push_str(&format!(
                        "m={} 0 RTP/AVP {}\r\n",
                        media_type, track.payload_type
                    ));
                    sdp.push_str(&format!(
                        "a=rtpmap:{} {}/{}\r\n",
                        track.payload_type, track.encoding, track.clock_rate
                    ));

                    // Add fmtp if we have parameters
                    if let Some(params) = &track.parameters {
                        sdp.push_str(&format!(
                            "a=fmtp:{} {}\r\n",
                            track.payload_type, params
                        ));
                    }

                    sdp.push_str(&format!("a=control:trackID={}\r\n", idx));
                }
            }
        } else {
            // Stream doesn't exist yet, provide default
            sdp.push_str("m=video 0 RTP/AVP 96\r\n");
            sdp.push_str("a=rtpmap:96 H264/90000\r\n");
            sdp.push_str("a=control:trackID=0\r\n");
        }

        sdp
    }

    /// Create error response.
    fn error_response(&self, error: RtspError, cseq: u32) -> Response {
        let status = match &error {
            RtspError::NotFound(_) => StatusCode::NOT_FOUND,
            RtspError::SessionNotFound(_) => StatusCode::SESSION_NOT_FOUND,
            RtspError::InvalidTransport(_) | RtspError::UnsupportedTransport(_) => {
                StatusCode::UNSUPPORTED_TRANSPORT
            }
            RtspError::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            RtspError::InvalidState(_) => StatusCode::METHOD_NOT_VALID,
            _ => StatusCode::BAD_REQUEST,
        };

        Response::new(status).with_cseq(cseq)
    }
}

/// Format transport to header string.
fn format_transport(transport: &Transport) -> String {
    let mut parts = Vec::new();

    // Protocol/profile/lower
    let proto = match (transport.protocol, transport.profile, transport.lower_transport) {
        (_, _, LowerTransport::Tcp) => "RTP/AVP/TCP",
        _ => "RTP/AVP",
    };
    parts.push(proto.to_string());

    // Cast mode
    match transport.cast_mode {
        crate::transport::CastMode::Unicast => parts.push("unicast".to_string()),
        crate::transport::CastMode::Multicast => parts.push("multicast".to_string()),
    }

    // Interleaved
    if let Some((rtp, rtcp)) = transport.interleaved {
        parts.push(format!("interleaved={}-{}", rtp, rtcp));
    }

    // Client port
    if let Some((rtp, rtcp)) = transport.client_port {
        parts.push(format!("client_port={}-{}", rtp, rtcp));
    }

    // Server port
    if let Some((rtp, rtcp)) = transport.server_port {
        parts.push(format!("server_port={}-{}", rtp, rtcp));
    }

    parts.join(";")
}

/// Extract stream path from URI.
fn extract_path(uri: &str) -> Result<String, RtspError> {
    // Handle rtsp://host:port/path and /path formats
    if uri.starts_with("rtsp://") {
        if let Some(path_start) = uri[7..].find('/') {
            let path = &uri[7 + path_start..];
            // Remove query string and track control
            let path = path.split('?').next().unwrap_or(path);
            let path = if let Some(idx) = path.rfind("/trackID=") {
                &path[..idx]
            } else {
                path
            };
            return Ok(path.to_string());
        }
        return Ok("/".to_string());
    }
    // Already a path
    let path = uri.split('?').next().unwrap_or(uri);
    let path = if let Some(idx) = path.rfind("/trackID=") {
        &path[..idx]
    } else {
        path
    };
    Ok(path.to_string())
}

/// Extract track control from URI.
fn extract_control(uri: &str) -> String {
    // Look for trackID=N pattern
    if let Some(idx) = uri.rfind("/trackID=") {
        return uri[idx + 1..].to_string();
    }
    if let Some(idx) = uri.rfind("trackID=") {
        return uri[idx..].to_string();
    }
    "track0".to_string()
}
