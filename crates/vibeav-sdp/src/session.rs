//! SDP session-level types.
//!
//! Parses session-level lines: v=, o=, s=, c=, t=, and session-level attributes.

/// SDP version (always 0 per RFC 4566).
pub const SDP_VERSION: u8 = 0;

/// Parsed origin (o=) line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Username (or "-" if not available).
    pub username: String,
    /// Session ID (NTP timestamp or random).
    pub session_id: String,
    /// Session version.
    pub session_version: String,
    /// Network type (usually "IN").
    pub net_type: String,
    /// Address type (usually "IP4" or "IP6").
    pub addr_type: String,
    /// Unicast address of origin.
    pub address: String,
}

impl Origin {
    /// Parse origin from "username session-id session-version net-type addr-type address".
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 6 {
            return None;
        }
        Some(Self {
            username: parts[0].to_string(),
            session_id: parts[1].to_string(),
            session_version: parts[2].to_string(),
            net_type: parts[3].to_string(),
            addr_type: parts[4].to_string(),
            address: parts[5].to_string(),
        })
    }
}

/// Parsed connection (c=) line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Network type (usually "IN").
    pub net_type: String,
    /// Address type ("IP4" or "IP6").
    pub addr_type: String,
    /// Connection address (may include TTL for multicast).
    pub address: String,
    /// TTL for multicast (if present).
    pub ttl: Option<u8>,
    /// Number of addresses (for multicast).
    pub num_addresses: Option<u8>,
}

impl Connection {
    /// Parse connection from "net-type addr-type address".
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }

        let address = parts[2].to_string();
        let mut ttl = None;
        let mut num_addresses = None;

        // Parse address/TTL/count (e.g., "224.2.0.1/16/3")
        if let Some((addr, rest)) = address.split_once('/') {
            let rest_parts: Vec<&str> = rest.split('/').collect();
            if !rest_parts.is_empty() {
                ttl = rest_parts[0].parse().ok();
            }
            if rest_parts.len() > 1 {
                num_addresses = rest_parts[1].parse().ok();
            }
            return Some(Self {
                net_type: parts[0].to_string(),
                addr_type: parts[1].to_string(),
                address: addr.to_string(),
                ttl,
                num_addresses,
            });
        }

        Some(Self {
            net_type: parts[0].to_string(),
            addr_type: parts[1].to_string(),
            address,
            ttl,
            num_addresses,
        })
    }

    /// Check if this is a multicast address.
    pub fn is_multicast(&self) -> bool {
        // IPv4 multicast: 224.0.0.0 - 239.255.255.255
        if self.addr_type == "IP4" {
            if let Some(first_octet) = self.address.split('.').next() {
                if let Ok(n) = first_octet.parse::<u8>() {
                    return (224..=239).contains(&n);
                }
            }
        }
        // IPv6 multicast: starts with ff
        if self.addr_type == "IP6" {
            return self.address.to_lowercase().starts_with("ff");
        }
        false
    }
}

/// Parsed timing (t=) line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    /// Start time (0 = unbounded).
    pub start: u64,
    /// Stop time (0 = unbounded).
    pub stop: u64,
}

impl Timing {
    /// Parse timing from "start stop".
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }
        Some(Self {
            start: parts[0].parse().ok()?,
            stop: parts[1].parse().ok()?,
        })
    }

    /// Check if always available (0 0).
    pub fn is_permanent(&self) -> bool {
        self.start == 0 && self.stop == 0
    }
}

/// Session direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Send and receive (default).
    #[default]
    SendRecv,
    /// Receive only (playback).
    RecvOnly,
    /// Send only (recording).
    SendOnly,
    /// Inactive.
    Inactive,
}

impl Direction {
    /// Parse from attribute name.
    pub fn from_attr(attr: &str) -> Option<Self> {
        match attr {
            "sendrecv" => Some(Self::SendRecv),
            "recvonly" => Some(Self::RecvOnly),
            "sendonly" => Some(Self::SendOnly),
            "inactive" => Some(Self::Inactive),
            _ => None,
        }
    }
}

/// Range type for presentations.
#[derive(Debug, Clone, PartialEq)]
pub enum Range {
    /// NPT range (Normal Play Time).
    Npt {
        start: f64,
        end: Option<f64>,
    },
    /// SMPTE timecode.
    Smpte {
        start: String,
        end: Option<String>,
    },
    /// Absolute UTC time.
    Clock {
        start: String,
        end: Option<String>,
    },
}

impl Range {
    /// Parse range from "npt=start-end" or similar.
    pub fn parse(value: &str) -> Option<Self> {
        if let Some(npt) = value.strip_prefix("npt=") {
            let (start, end) = npt.split_once('-')?;
            let start_val = start.parse().ok()?;
            let end_val = if end.is_empty() {
                None
            } else {
                Some(end.parse().ok()?)
            };
            return Some(Self::Npt {
                start: start_val,
                end: end_val,
            });
        }

        if let Some(smpte) = value.strip_prefix("smpte=") {
            let (start, end) = smpte.split_once('-')?;
            let end_val = if end.is_empty() {
                None
            } else {
                Some(end.to_string())
            };
            return Some(Self::Smpte {
                start: start.to_string(),
                end: end_val,
            });
        }

        if let Some(clock) = value.strip_prefix("clock=") {
            let (start, end) = clock.split_once('-')?;
            let end_val = if end.is_empty() {
                None
            } else {
                Some(end.to_string())
            };
            return Some(Self::Clock {
                start: start.to_string(),
                end: end_val,
            });
        }

        None
    }

    /// Get duration in seconds (for NPT only).
    pub fn duration(&self) -> Option<f64> {
        match self {
            Self::Npt { start, end } => end.map(|e| e - start),
            _ => None,
        }
    }
}

/// Complete SDP session description.
#[derive(Debug, Clone, Default)]
pub struct Session {
    /// Session version (always 0).
    pub version: u8,
    /// Origin (o=) line.
    pub origin: Option<Origin>,
    /// Session name (s=).
    pub name: String,
    /// Session information (i=).
    pub info: Option<String>,
    /// URI (u=).
    pub uri: Option<String>,
    /// Email (e=).
    pub email: Option<String>,
    /// Phone (p=).
    pub phone: Option<String>,
    /// Session-level connection (c=).
    pub connection: Option<Connection>,
    /// Timing (t=).
    pub timing: Option<Timing>,
    /// Session-level control URL (a=control:).
    pub control: Option<String>,
    /// Range (a=range:).
    pub range: Option<Range>,
    /// Direction (a=sendrecv, etc.).
    pub direction: Direction,
    /// Entity tag (a=etag:).
    pub etag: Option<String>,
    /// Media descriptions.
    pub media: Vec<crate::media::MediaDescription>,
    /// Unknown session-level attributes.
    pub attributes: Vec<(String, Option<String>)>,
}

impl Session {
    /// Create a new empty session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get aggregate control URL if present.
    pub fn aggregate_control(&self) -> Option<&str> {
        self.control.as_deref()
    }

    /// Check if aggregate control is available.
    pub fn has_aggregate_control(&self) -> bool {
        self.control.is_some()
    }

    /// Get all media streams.
    pub fn streams(&self) -> &[crate::media::MediaDescription] {
        &self.media
    }

    /// Get video streams.
    pub fn video_streams(&self) -> impl Iterator<Item = &crate::media::MediaDescription> {
        self.media.iter().filter(|m| m.media_type == "video")
    }

    /// Get audio streams.
    pub fn audio_streams(&self) -> impl Iterator<Item = &crate::media::MediaDescription> {
        self.media.iter().filter(|m| m.media_type == "audio")
    }
}
