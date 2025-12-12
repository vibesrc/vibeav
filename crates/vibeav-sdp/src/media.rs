//! SDP media-level types.
//!
//! Parses media descriptions (m=) and media-level attributes.

use crate::session::{Connection, Direction};

/// Parsed rtpmap attribute (a=rtpmap:).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpMap {
    /// Payload type (0-127).
    pub payload_type: u8,
    /// Encoding name (e.g., "H264", "mpeg4-generic").
    pub encoding: String,
    /// Clock rate in Hz.
    pub clock_rate: u32,
    /// Encoding parameters (channels for audio).
    pub encoding_params: Option<String>,
}

impl RtpMap {
    /// Parse from "payload encoding/clock-rate[/params]".
    pub fn parse(value: &str) -> Option<Self> {
        let (pt_str, rest) = value.split_once(' ')?;
        let pt: u8 = pt_str.parse().ok()?;

        let parts: Vec<&str> = rest.split('/').collect();
        if parts.is_empty() {
            return None;
        }

        let encoding = parts[0].to_string();
        let clock_rate = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(90000);
        let encoding_params = parts.get(2).map(|s| s.to_string());

        Some(Self {
            payload_type: pt,
            encoding,
            clock_rate,
            encoding_params,
        })
    }

    /// Get audio channel count (if encoding_params is channels).
    pub fn channels(&self) -> Option<u8> {
        self.encoding_params.as_ref().and_then(|s| s.parse().ok())
    }
}

/// Parsed fmtp attribute (a=fmtp:).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fmtp {
    /// Payload type this applies to.
    pub payload_type: u8,
    /// Raw parameter string.
    pub parameters: String,
}

impl Fmtp {
    /// Parse from "payload param1; param2; ...".
    pub fn parse(value: &str) -> Option<Self> {
        let (pt_str, params) = value.split_once(' ')?;
        let pt: u8 = pt_str.parse().ok()?;

        Some(Self {
            payload_type: pt,
            parameters: params.to_string(),
        })
    }

    /// Get a specific parameter value.
    pub fn get_param(&self, name: &str) -> Option<&str> {
        for part in self.parameters.split(&[';', ' '][..]) {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                if key.trim().eq_ignore_ascii_case(name) {
                    return Some(value.trim());
                }
            }
        }
        None
    }

    /// Get all parameters as key-value pairs.
    pub fn params(&self) -> impl Iterator<Item = (&str, &str)> {
        self.parameters.split(';').filter_map(|part| {
            let part = part.trim();
            part.split_once('=').map(|(k, v)| (k.trim(), v.trim()))
        })
    }

    /// Get H.264 profile-level-id.
    pub fn profile_level_id(&self) -> Option<&str> {
        self.get_param("profile-level-id")
    }

    /// Get H.264 sprop-parameter-sets.
    pub fn sprop_parameter_sets(&self) -> Option<&str> {
        self.get_param("sprop-parameter-sets")
    }

    /// Get H.264 packetization-mode.
    pub fn packetization_mode(&self) -> Option<u8> {
        self.get_param("packetization-mode")
            .and_then(|s| s.parse().ok())
    }
}

/// Media description (m= block).
#[derive(Debug, Clone, Default)]
pub struct MediaDescription {
    /// Media type (audio, video, text, application).
    pub media_type: String,
    /// Port (0 for RTSP unicast).
    pub port: u16,
    /// Number of ports (for multicast).
    pub num_ports: Option<u16>,
    /// Protocol (e.g., "RTP/AVP", "RTP/SAVP").
    pub protocol: String,
    /// Format list (payload types).
    pub formats: Vec<String>,
    /// Media title (i=).
    pub info: Option<String>,
    /// Media-level connection (c=).
    pub connection: Option<Connection>,
    /// Control URL (a=control:).
    pub control: Option<String>,
    /// rtpmap entries by payload type.
    pub rtpmap: Vec<RtpMap>,
    /// fmtp entries by payload type.
    pub fmtp: Vec<Fmtp>,
    /// Direction (a=sendrecv, etc.).
    pub direction: Option<Direction>,
    /// Frame rate (a=framerate:).
    pub framerate: Option<f32>,
    /// Unknown media-level attributes.
    pub attributes: Vec<(String, Option<String>)>,
}

impl MediaDescription {
    /// Parse media line "media port[/count] proto fmt-list".
    pub fn parse_media_line(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let media_type = parts[0].to_string();

        // Parse port/count
        let (port, num_ports) = if let Some((p, c)) = parts[1].split_once('/') {
            (p.parse().ok()?, Some(c.parse().ok()?))
        } else {
            (parts[1].parse().ok()?, None)
        };

        let protocol = parts[2].to_string();
        let formats: Vec<String> = parts[3..].iter().map(|s| s.to_string()).collect();

        Some(Self {
            media_type,
            port,
            num_ports,
            protocol,
            formats,
            ..Default::default()
        })
    }

    /// Get rtpmap for a payload type.
    pub fn get_rtpmap(&self, pt: u8) -> Option<&RtpMap> {
        self.rtpmap.iter().find(|r| r.payload_type == pt)
    }

    /// Get fmtp for a payload type.
    pub fn get_fmtp(&self, pt: u8) -> Option<&Fmtp> {
        self.fmtp.iter().find(|f| f.payload_type == pt)
    }

    /// Get the primary payload type (first in format list).
    pub fn primary_payload_type(&self) -> Option<u8> {
        self.formats.first().and_then(|s| s.parse().ok())
    }

    /// Get encoding name for primary payload type.
    pub fn encoding(&self) -> Option<&str> {
        let pt = self.primary_payload_type()?;
        self.get_rtpmap(pt).map(|r| r.encoding.as_str())
    }

    /// Get clock rate for primary payload type.
    pub fn clock_rate(&self) -> Option<u32> {
        let pt = self.primary_payload_type()?;
        self.get_rtpmap(pt).map(|r| r.clock_rate)
    }

    /// Check if this is a video stream.
    pub fn is_video(&self) -> bool {
        self.media_type == "video"
    }

    /// Check if this is an audio stream.
    pub fn is_audio(&self) -> bool {
        self.media_type == "audio"
    }

    /// Check if this is using RTP.
    pub fn is_rtp(&self) -> bool {
        self.protocol.starts_with("RTP/")
    }
}
