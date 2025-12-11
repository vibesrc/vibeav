//! RTSP Transport header parsing.

use crate::error::{RtspError, Result};

/// Transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportProtocol {
    #[default]
    Rtp,
}

/// Transport profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportProfile {
    #[default]
    Avp,
    Savp,
}

/// Lower transport (TCP/UDP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LowerTransport {
    #[default]
    Udp,
    Tcp,
}

/// Cast mode (unicast/multicast).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CastMode {
    #[default]
    Multicast,
    Unicast,
}

/// Transport mode (play/record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportMode {
    #[default]
    Play,
    Record,
}

/// Parsed Transport header.
#[derive(Debug, Clone, Default)]
pub struct Transport {
    /// Transport protocol (RTP).
    pub protocol: TransportProtocol,
    /// Profile (AVP, SAVP).
    pub profile: TransportProfile,
    /// Lower transport (UDP, TCP).
    pub lower_transport: LowerTransport,
    /// Cast mode (unicast, multicast).
    pub cast_mode: CastMode,
    /// Transport mode (play, record).
    pub mode: TransportMode,
    /// Interleaved channels (for TCP).
    pub interleaved: Option<(u8, u8)>,
    /// Client ports (RTP, RTCP).
    pub client_port: Option<(u16, u16)>,
    /// Server ports (RTP, RTCP).
    pub server_port: Option<(u16, u16)>,
    /// Multicast port.
    pub port: Option<(u16, u16)>,
    /// Destination address.
    pub destination: Option<String>,
    /// Source address.
    pub source: Option<String>,
    /// TTL for multicast.
    pub ttl: Option<u8>,
    /// SSRC.
    pub ssrc: Option<u32>,
    /// Append mode.
    pub append: bool,
}

impl Transport {
    /// Create a new transport with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a TCP interleaved transport.
    pub fn tcp_interleaved(rtp_channel: u8, rtcp_channel: u8) -> Self {
        Self {
            protocol: TransportProtocol::Rtp,
            profile: TransportProfile::Avp,
            lower_transport: LowerTransport::Tcp,
            cast_mode: CastMode::Unicast,
            interleaved: Some((rtp_channel, rtcp_channel)),
            ..Default::default()
        }
    }

    /// Create a UDP unicast transport.
    pub fn udp_unicast(client_rtp: u16, client_rtcp: u16) -> Self {
        Self {
            protocol: TransportProtocol::Rtp,
            profile: TransportProfile::Avp,
            lower_transport: LowerTransport::Udp,
            cast_mode: CastMode::Unicast,
            client_port: Some((client_rtp, client_rtcp)),
            ..Default::default()
        }
    }

    /// Set mode to record.
    pub fn with_mode_record(mut self) -> Self {
        self.mode = TransportMode::Record;
        self
    }

    /// Check if TCP interleaved.
    pub fn is_interleaved(&self) -> bool {
        self.lower_transport == LowerTransport::Tcp && self.interleaved.is_some()
    }

    /// Check if UDP.
    pub fn is_udp(&self) -> bool {
        self.lower_transport == LowerTransport::Udp
    }

    /// Parse Transport header value.
    pub fn parse(value: &str) -> Result<Self> {
        let mut transport = Transport::new();

        // Split by semicolons
        let parts: Vec<&str> = value.split(';').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return Err(RtspError::InvalidTransport("empty transport".into()));
        }

        // Parse protocol/profile/lower-transport
        let proto_parts: Vec<&str> = parts[0].split('/').collect();
        if proto_parts.is_empty() {
            return Err(RtspError::InvalidTransport("missing protocol".into()));
        }

        // Protocol
        match proto_parts[0].to_uppercase().as_str() {
            "RTP" => transport.protocol = TransportProtocol::Rtp,
            _ => return Err(RtspError::InvalidTransport(format!("unknown protocol: {}", proto_parts[0]))),
        }

        // Profile
        if proto_parts.len() > 1 {
            match proto_parts[1].to_uppercase().as_str() {
                "AVP" => transport.profile = TransportProfile::Avp,
                "SAVP" => transport.profile = TransportProfile::Savp,
                _ => {} // Ignore unknown profiles
            }
        }

        // Lower transport
        if proto_parts.len() > 2 {
            match proto_parts[2].to_uppercase().as_str() {
                "TCP" => transport.lower_transport = LowerTransport::Tcp,
                "UDP" => transport.lower_transport = LowerTransport::Udp,
                _ => {}
            }
        }

        // Parse parameters
        for part in &parts[1..] {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((key, value)) = part.split_once('=') {
                let key = key.to_lowercase();
                match key.as_str() {
                    "interleaved" => {
                        transport.interleaved = parse_port_range(value)?;
                        transport.lower_transport = LowerTransport::Tcp;
                    }
                    "client_port" => {
                        transport.client_port = parse_port_range(value)?;
                    }
                    "server_port" => {
                        transport.server_port = parse_port_range(value)?;
                    }
                    "port" => {
                        transport.port = parse_port_range(value)?;
                    }
                    "destination" => {
                        transport.destination = Some(value.to_string());
                    }
                    "source" => {
                        transport.source = Some(value.to_string());
                    }
                    "ttl" => {
                        transport.ttl = value.parse().ok();
                    }
                    "ssrc" => {
                        transport.ssrc = u32::from_str_radix(value, 16).ok();
                    }
                    "mode" => {
                        let mode = value.trim_matches('"').to_uppercase();
                        if mode == "RECORD" {
                            transport.mode = TransportMode::Record;
                        }
                    }
                    _ => {} // Ignore unknown parameters
                }
            } else {
                // Flag parameters (no value)
                match part.to_lowercase().as_str() {
                    "unicast" => transport.cast_mode = CastMode::Unicast,
                    "multicast" => transport.cast_mode = CastMode::Multicast,
                    "append" => transport.append = true,
                    _ => {}
                }
            }
        }

        Ok(transport)
    }

    /// Serialize to Transport header value.
    pub fn to_header_value(&self) -> String {
        let mut parts = Vec::new();

        // Protocol/Profile/Lower-Transport
        let proto = match (self.profile, self.lower_transport) {
            (TransportProfile::Avp, LowerTransport::Udp) => "RTP/AVP",
            (TransportProfile::Avp, LowerTransport::Tcp) => "RTP/AVP/TCP",
            (TransportProfile::Savp, LowerTransport::Udp) => "RTP/SAVP",
            (TransportProfile::Savp, LowerTransport::Tcp) => "RTP/SAVP/TCP",
        };
        parts.push(proto.to_string());

        // Cast mode
        match self.cast_mode {
            CastMode::Unicast => parts.push("unicast".to_string()),
            CastMode::Multicast => parts.push("multicast".to_string()),
        }

        // Interleaved
        if let Some((rtp, rtcp)) = self.interleaved {
            parts.push(format!("interleaved={}-{}", rtp, rtcp));
        }

        // Client port
        if let Some((rtp, rtcp)) = self.client_port {
            parts.push(format!("client_port={}-{}", rtp, rtcp));
        }

        // Server port
        if let Some((rtp, rtcp)) = self.server_port {
            parts.push(format!("server_port={}-{}", rtp, rtcp));
        }

        // Mode
        if self.mode == TransportMode::Record {
            parts.push("mode=\"RECORD\"".to_string());
        }

        // SSRC
        if let Some(ssrc) = self.ssrc {
            parts.push(format!("ssrc={:08X}", ssrc));
        }

        // TTL
        if let Some(ttl) = self.ttl {
            parts.push(format!("ttl={}", ttl));
        }

        // Destination
        if let Some(ref dest) = self.destination {
            parts.push(format!("destination={}", dest));
        }

        parts.join(";")
    }
}

/// Parse "port-port" range.
fn parse_port_range<T>(value: &str) -> Result<Option<(T, T)>>
where
    T: std::str::FromStr,
{
    if let Some((a, b)) = value.split_once('-') {
        let a: T = a.parse().map_err(|_| RtspError::InvalidTransport(format!("invalid port: {}", a)))?;
        let b: T = b.parse().map_err(|_| RtspError::InvalidTransport(format!("invalid port: {}", b)))?;
        Ok(Some((a, b)))
    } else {
        // Single value - can't determine second port generically, so return None
        let _: T = value.parse().map_err(|_| RtspError::InvalidTransport(format!("invalid port: {}", value)))?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_udp_unicast() {
        let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589").unwrap();
        assert_eq!(t.protocol, TransportProtocol::Rtp);
        assert_eq!(t.profile, TransportProfile::Avp);
        assert_eq!(t.lower_transport, LowerTransport::Udp);
        assert_eq!(t.cast_mode, CastMode::Unicast);
        assert_eq!(t.client_port, Some((4588, 4589)));
    }

    #[test]
    fn test_parse_tcp_interleaved() {
        let t = Transport::parse("RTP/AVP/TCP;unicast;interleaved=0-1").unwrap();
        assert_eq!(t.lower_transport, LowerTransport::Tcp);
        assert_eq!(t.interleaved, Some((0, 1)));
        assert!(t.is_interleaved());
    }

    #[test]
    fn test_parse_with_server_port() {
        let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257").unwrap();
        assert_eq!(t.client_port, Some((4588, 4589)));
        assert_eq!(t.server_port, Some((6256, 6257)));
    }

    #[test]
    fn test_parse_multicast() {
        let t = Transport::parse("RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16").unwrap();
        assert_eq!(t.cast_mode, CastMode::Multicast);
        assert_eq!(t.destination, Some("224.2.0.1".to_string()));
        assert_eq!(t.port, Some((3456, 3457)));
        assert_eq!(t.ttl, Some(16));
    }

    #[test]
    fn test_parse_with_ssrc() {
        let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;ssrc=0A3C4D5E").unwrap();
        assert_eq!(t.ssrc, Some(0x0A3C4D5E));
    }

    #[test]
    fn test_parse_record_mode() {
        let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;mode=\"RECORD\"").unwrap();
        assert_eq!(t.mode, TransportMode::Record);
    }

    #[test]
    fn test_serialize_tcp_interleaved() {
        let t = Transport::tcp_interleaved(0, 1);
        let value = t.to_header_value();
        assert!(value.contains("RTP/AVP/TCP"));
        assert!(value.contains("unicast"));
        assert!(value.contains("interleaved=0-1"));
    }

    #[test]
    fn test_serialize_udp_unicast() {
        let t = Transport::udp_unicast(4588, 4589);
        let value = t.to_header_value();
        assert!(value.contains("RTP/AVP"));
        assert!(value.contains("unicast"));
        assert!(value.contains("client_port=4588-4589"));
    }

    #[test]
    fn test_serialize_record_mode() {
        let t = Transport::tcp_interleaved(0, 1).with_mode_record();
        let value = t.to_header_value();
        assert!(value.contains("mode=\"RECORD\""));
    }
}
