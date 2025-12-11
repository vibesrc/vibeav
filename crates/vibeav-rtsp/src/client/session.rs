//! RTSP client session state.

use std::collections::HashMap;

use vibeav_sdp::Session as SdpSession;

use crate::transport::Transport;

/// Client session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Initial state, not connected.
    Disconnected,
    /// Connected, waiting for OPTIONS response.
    Connected,
    /// Received OPTIONS, waiting for DESCRIBE.
    Options,
    /// Received DESCRIBE with SDP.
    Described,
    /// Setting up tracks.
    Setup,
    /// All tracks setup, ready to play.
    Ready,
    /// Playing, receiving media.
    Playing,
    /// Paused.
    Paused,
    /// Tearing down.
    Teardown,
}

/// Track setup information.
#[derive(Debug, Clone)]
pub struct TrackSetup {
    /// Control URL for this track.
    pub control_url: String,
    /// Media type (video, audio, etc.).
    pub media_type: String,
    /// Payload type.
    pub payload_type: u8,
    /// Encoding name (H264, AAC, etc.).
    pub encoding: String,
    /// Clock rate.
    pub clock_rate: u32,
    /// Format parameters.
    pub fmtp: Option<String>,
    /// Transport after SETUP.
    pub transport: Option<Transport>,
    /// RTP interleaved channel.
    pub rtp_channel: Option<u8>,
    /// RTCP interleaved channel.
    pub rtcp_channel: Option<u8>,
}

/// Client session tracking.
#[derive(Debug)]
pub struct ClientSession {
    /// Session ID from server.
    session_id: Option<String>,
    /// Current state.
    state: ClientState,
    /// Current CSeq.
    cseq: u32,
    /// Base URL for the session.
    base_url: String,
    /// Content-Base from DESCRIBE response.
    content_base: Option<String>,
    /// SDP from DESCRIBE.
    sdp: Option<SdpSession>,
    /// Track setups by control URL.
    tracks: HashMap<String, TrackSetup>,
    /// Channel to track mapping (for interleaved).
    channel_map: HashMap<u8, String>,
    /// Next interleaved channel to assign.
    next_channel: u8,
    /// Session timeout from server.
    timeout: Option<u32>,
}

impl ClientSession {
    /// Create a new client session.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            session_id: None,
            state: ClientState::Disconnected,
            cseq: 0,
            base_url: base_url.into(),
            content_base: None,
            sdp: None,
            tracks: HashMap::new(),
            channel_map: HashMap::new(),
            next_channel: 0,
            timeout: None,
        }
    }

    /// Get current state.
    pub fn state(&self) -> ClientState {
        self.state
    }

    /// Set state.
    pub fn set_state(&mut self, state: ClientState) {
        self.state = state;
    }

    /// Get next CSeq and increment.
    pub fn next_cseq(&mut self) -> u32 {
        self.cseq += 1;
        self.cseq
    }

    /// Get session ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Set session ID.
    pub fn set_session_id(&mut self, id: impl Into<String>) {
        self.session_id = Some(id.into());
    }

    /// Get base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get content base (from DESCRIBE response).
    pub fn content_base(&self) -> Option<&str> {
        self.content_base.as_deref()
    }

    /// Set content base.
    pub fn set_content_base(&mut self, base: impl Into<String>) {
        self.content_base = Some(base.into());
    }

    /// Get the effective base URL for control URLs.
    pub fn effective_base(&self) -> &str {
        self.content_base.as_deref().unwrap_or(&self.base_url)
    }

    /// Get SDP.
    pub fn sdp(&self) -> Option<&SdpSession> {
        self.sdp.as_ref()
    }

    /// Set SDP from DESCRIBE response.
    pub fn set_sdp(&mut self, sdp: SdpSession) {
        // Extract track info from SDP
        for media in &sdp.media {
            let media_type = media.media_type.to_string();

            // Get control URL
            let control = media
                .control
                .as_ref()
                .map(|c| resolve_control_url(self.effective_base(), c))
                .unwrap_or_else(|| self.effective_base().to_string());

            // Get rtpmap info
            if let Some(rtpmap) = media.rtpmap.first() {
                let track = TrackSetup {
                    control_url: control.clone(),
                    media_type: media_type.clone(),
                    payload_type: rtpmap.payload_type,
                    encoding: rtpmap.encoding.clone(),
                    clock_rate: rtpmap.clock_rate,
                    fmtp: media.fmtp.first().map(|f| f.parameters.clone()),
                    transport: None,
                    rtp_channel: None,
                    rtcp_channel: None,
                };
                self.tracks.insert(control, track);
            }
        }

        self.sdp = Some(sdp);
    }

    /// Get tracks.
    pub fn tracks(&self) -> &HashMap<String, TrackSetup> {
        &self.tracks
    }

    /// Get mutable tracks.
    pub fn tracks_mut(&mut self) -> &mut HashMap<String, TrackSetup> {
        &mut self.tracks
    }

    /// Get track by control URL.
    pub fn track(&self, control: &str) -> Option<&TrackSetup> {
        self.tracks.get(control)
    }

    /// Get mutable track by control URL.
    pub fn track_mut(&mut self, control: &str) -> Option<&mut TrackSetup> {
        self.tracks.get_mut(control)
    }

    /// Get track by RTP channel.
    pub fn track_by_channel(&self, channel: u8) -> Option<&TrackSetup> {
        self.channel_map
            .get(&channel)
            .and_then(|control| self.tracks.get(control))
    }

    /// Allocate interleaved channels for a track.
    pub fn allocate_channels(&mut self, control: &str) -> (u8, u8) {
        let rtp = self.next_channel;
        let rtcp = self.next_channel + 1;
        self.next_channel += 2;

        // Map channels to track
        self.channel_map.insert(rtp, control.to_string());
        self.channel_map.insert(rtcp, control.to_string());

        // Update track
        if let Some(track) = self.tracks.get_mut(control) {
            track.rtp_channel = Some(rtp);
            track.rtcp_channel = Some(rtcp);
        }

        (rtp, rtcp)
    }

    /// Set session timeout.
    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = Some(timeout);
    }

    /// Get session timeout.
    pub fn timeout(&self) -> Option<u32> {
        self.timeout
    }

    /// Check if all tracks are setup.
    pub fn all_tracks_setup(&self) -> bool {
        !self.tracks.is_empty() && self.tracks.values().all(|t| t.transport.is_some())
    }

    /// Get list of track control URLs.
    pub fn track_controls(&self) -> Vec<String> {
        self.tracks.keys().cloned().collect()
    }
}

/// Resolve a control URL relative to a base URL.
fn resolve_control_url(base: &str, control: &str) -> String {
    // Absolute URL
    if control.starts_with("rtsp://") {
        return control.to_string();
    }

    // Asterisk means use base URL
    if control == "*" {
        return base.to_string();
    }

    // Relative URL - append to base
    let base = base.trim_end_matches('/');
    if control.starts_with('/') {
        // Absolute path - find host part and append
        if let Some(idx) = base.find("://") {
            if let Some(path_start) = base[idx + 3..].find('/') {
                let host_part = &base[..idx + 3 + path_start];
                return format!("{}{}", host_part, control);
            }
        }
        format!("{}{}", base, control)
    } else {
        format!("{}/{}", base, control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_control_url_absolute() {
        assert_eq!(
            resolve_control_url(
                "rtsp://192.168.1.69/stream",
                "rtsp://192.168.1.69/stream/trackID=0"
            ),
            "rtsp://192.168.1.69/stream/trackID=0"
        );
    }

    #[test]
    fn test_resolve_control_url_relative() {
        assert_eq!(
            resolve_control_url("rtsp://192.168.1.69/stream", "trackID=0"),
            "rtsp://192.168.1.69/stream/trackID=0"
        );
    }

    #[test]
    fn test_resolve_control_url_asterisk() {
        assert_eq!(
            resolve_control_url("rtsp://192.168.1.69/stream", "*"),
            "rtsp://192.168.1.69/stream"
        );
    }

    #[test]
    fn test_client_session_new() {
        let session = ClientSession::new("rtsp://192.168.1.69/stream");
        assert_eq!(session.state(), ClientState::Disconnected);
        assert_eq!(session.base_url(), "rtsp://192.168.1.69/stream");
        assert!(session.session_id().is_none());
    }

    #[test]
    fn test_client_session_cseq() {
        let mut session = ClientSession::new("rtsp://192.168.1.69/stream");
        assert_eq!(session.next_cseq(), 1);
        assert_eq!(session.next_cseq(), 2);
        assert_eq!(session.next_cseq(), 3);
    }

    #[test]
    fn test_allocate_channels() {
        let mut session = ClientSession::new("rtsp://192.168.1.69/stream");

        // Add a track manually for testing
        session.tracks.insert(
            "track0".to_string(),
            TrackSetup {
                control_url: "track0".to_string(),
                media_type: "video".to_string(),
                payload_type: 96,
                encoding: "H264".to_string(),
                clock_rate: 90000,
                fmtp: None,
                transport: None,
                rtp_channel: None,
                rtcp_channel: None,
            },
        );

        let (rtp, rtcp) = session.allocate_channels("track0");
        assert_eq!(rtp, 0);
        assert_eq!(rtcp, 1);

        let track = session.track("track0").unwrap();
        assert_eq!(track.rtp_channel, Some(0));
        assert_eq!(track.rtcp_channel, Some(1));
    }
}
