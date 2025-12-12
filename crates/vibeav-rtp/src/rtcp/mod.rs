//! RTCP packet parsing (RFC 3550 Section 6).
//!
//! Provides zero-copy parsing of RTCP packets:
//! - SR (Sender Report)
//! - RR (Receiver Report)
//! - SDES (Source Description)
//! - BYE (Goodbye)
//! - APP (Application-defined)

mod bye;
mod rr;
mod sdes;
mod sr;

pub use bye::{RtcpBye, RtcpByeBuilder};
pub use rr::{ReceptionReport, RtcpRr};
pub use sdes::{RtcpSdes, SdesChunk, SdesItem, SdesItemType};
pub use sr::{RtcpSr, SenderInfo};

use crate::error::RtcpError;

/// RTCP packet version. MUST be 2 for RFC 3550.
pub const RTCP_VERSION: u8 = 2;

/// Minimum RTCP header size.
pub const RTCP_HEADER_SIZE: usize = 4;

/// RTCP packet types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RtcpPacketType {
    /// Sender Report (PT=200).
    Sr = 200,
    /// Receiver Report (PT=201).
    Rr = 201,
    /// Source Description (PT=202).
    Sdes = 202,
    /// Goodbye (PT=203).
    Bye = 203,
    /// Application-defined (PT=204).
    App = 204,
}

impl RtcpPacketType {
    /// Try to convert from u8.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            200 => Some(Self::Sr),
            201 => Some(Self::Rr),
            202 => Some(Self::Sdes),
            203 => Some(Self::Bye),
            204 => Some(Self::App),
            _ => None,
        }
    }
}

/// Common RTCP packet header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpHeader {
    /// RTP version (MUST be 2).
    pub version: u8,
    /// Padding flag.
    pub padding: bool,
    /// Report count or subtype (5 bits).
    pub count: u8,
    /// Packet type (200-204).
    pub packet_type: u8,
    /// Packet length in 32-bit words minus 1.
    pub length: u16,
}

impl RtcpHeader {
    /// Parse RTCP header from bytes.
    ///
    /// Returns the header and total packet length in bytes (including header).
    pub fn parse(data: &[u8]) -> Result<(Self, usize), RtcpError> {
        if data.len() < RTCP_HEADER_SIZE {
            return Err(RtcpError::PacketTooShort {
                expected: RTCP_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let byte0 = data[0];
        let version = (byte0 >> 6) & 0x03;

        if version != RTCP_VERSION {
            return Err(RtcpError::InvalidVersion(version));
        }

        let padding = (byte0 >> 5) & 0x01 != 0;
        let count = byte0 & 0x1F;
        let packet_type = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]);

        // Total packet size = (length + 1) * 4 bytes
        let packet_size = (length as usize + 1) * 4;

        if data.len() < packet_size {
            return Err(RtcpError::LengthMismatch {
                expected: packet_size,
                actual: data.len(),
            });
        }

        Ok((
            Self {
                version,
                padding,
                count,
                packet_type,
                length,
            },
            packet_size,
        ))
    }

    /// Write header to buffer.
    pub fn write(&self, buf: &mut [u8]) -> usize {
        let byte0 =
            (self.version << 6) | ((self.padding as u8) << 5) | (self.count & 0x1F);

        buf[0] = byte0;
        buf[1] = self.packet_type;
        buf[2..4].copy_from_slice(&self.length.to_be_bytes());

        RTCP_HEADER_SIZE
    }
}

/// Parsed RTCP packet (any type).
#[derive(Debug, Clone)]
pub enum RtcpPacket<'a> {
    /// Sender Report.
    Sr(RtcpSr<'a>),
    /// Receiver Report.
    Rr(RtcpRr<'a>),
    /// Source Description.
    Sdes(RtcpSdes<'a>),
    /// Goodbye.
    Bye(RtcpBye<'a>),
    /// Application-defined (raw data).
    App(RtcpApp<'a>),
    /// Unknown packet type (raw data).
    Unknown { packet_type: u8, data: &'a [u8] },
}

/// Application-defined RTCP packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcpApp<'a> {
    /// Subtype (5 bits).
    pub subtype: u8,
    /// SSRC/CSRC.
    pub ssrc: u32,
    /// Application name (4 ASCII characters).
    pub name: [u8; 4],
    /// Application-dependent data.
    pub data: &'a [u8],
}

impl<'a> RtcpApp<'a> {
    /// Minimum APP packet size (header + SSRC + name).
    pub const MIN_SIZE: usize = RTCP_HEADER_SIZE + 8;

    /// Parse APP packet from data (after header).
    pub fn parse(header: &RtcpHeader, data: &'a [u8]) -> Result<Self, RtcpError> {
        if data.len() < 8 {
            return Err(RtcpError::PacketTooShort {
                expected: 8,
                actual: data.len(),
            });
        }

        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let name = [data[4], data[5], data[6], data[7]];
        let app_data = &data[8..];

        Ok(Self {
            subtype: header.count,
            ssrc,
            name,
            data: app_data,
        })
    }

    /// Get the name as a string slice.
    pub fn name_str(&self) -> &str {
        std::str::from_utf8(&self.name).unwrap_or("")
    }
}

/// Parse a single RTCP packet.
pub fn parse_rtcp_packet(data: &[u8]) -> Result<(RtcpPacket<'_>, usize), RtcpError> {
    let (header, packet_size) = RtcpHeader::parse(data)?;
    let payload = &data[RTCP_HEADER_SIZE..packet_size];

    let packet = match header.packet_type {
        200 => RtcpPacket::Sr(RtcpSr::parse(&header, payload)?),
        201 => RtcpPacket::Rr(RtcpRr::parse(&header, payload)?),
        202 => RtcpPacket::Sdes(RtcpSdes::parse(&header, payload)?),
        203 => RtcpPacket::Bye(RtcpBye::parse(&header, payload)?),
        204 => RtcpPacket::App(RtcpApp::parse(&header, payload)?),
        pt => RtcpPacket::Unknown {
            packet_type: pt,
            data: payload,
        },
    };

    Ok((packet, packet_size))
}

/// Iterator over RTCP packets in a compound packet.
pub struct RtcpCompoundIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> RtcpCompoundIter<'a> {
    /// Create iterator over compound RTCP packet.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for RtcpCompoundIter<'a> {
    type Item = Result<RtcpPacket<'a>, RtcpError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        match parse_rtcp_packet(&self.data[self.offset..]) {
            Ok((packet, size)) => {
                self.offset += size;
                Some(Ok(packet))
            }
            Err(e) => {
                // Stop iteration on error
                self.offset = self.data.len();
                Some(Err(e))
            }
        }
    }
}

/// Parse compound RTCP packet, returning iterator over individual packets.
pub fn parse_compound_rtcp(data: &[u8]) -> RtcpCompoundIter<'_> {
    RtcpCompoundIter::new(data)
}

/// Validate that compound packet starts with SR or RR.
pub fn validate_compound_rtcp(data: &[u8]) -> Result<(), RtcpError> {
    let (header, _) = RtcpHeader::parse(data)?;

    if header.packet_type != 200 && header.packet_type != 201 {
        return Err(RtcpError::InvalidCompoundStart(header.packet_type));
    }

    Ok(())
}
