//! Error types for RTP/RTCP parsing.

use thiserror::Error;

/// Errors that can occur when parsing RTP packets.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RtpError {
    /// Packet is too short to contain a valid header.
    #[error("packet too short: need at least {expected} bytes, got {actual}")]
    PacketTooShort { expected: usize, actual: usize },

    /// RTP version field is not 2.
    #[error("invalid RTP version: expected 2, got {0}")]
    InvalidVersion(u8),

    /// CSRC count would extend beyond packet boundary.
    #[error("CSRC list extends beyond packet: need {expected} bytes, got {actual}")]
    CsrcOverflow { expected: usize, actual: usize },

    /// Header extension extends beyond packet boundary.
    #[error("header extension extends beyond packet")]
    ExtensionOverflow,

    /// Padding count is invalid (0 or larger than payload).
    #[error("invalid padding count: {0}")]
    InvalidPadding(u8),
}

/// Errors that can occur when parsing RTCP packets.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RtcpError {
    /// Packet is too short to contain a valid header.
    #[error("packet too short: need at least {expected} bytes, got {actual}")]
    PacketTooShort { expected: usize, actual: usize },

    /// RTCP version field is not 2.
    #[error("invalid RTCP version: expected 2, got {0}")]
    InvalidVersion(u8),

    /// Unknown or unsupported packet type.
    #[error("unknown RTCP packet type: {0}")]
    UnknownPacketType(u8),

    /// Length field doesn't match actual packet size.
    #[error("length mismatch: header says {expected} bytes, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    /// First packet in compound must be SR or RR.
    #[error("compound packet must start with SR or RR, got packet type {0}")]
    InvalidCompoundStart(u8),

    /// Invalid padding in RTCP packet.
    #[error("invalid padding count: {0}")]
    InvalidPadding(u8),

    /// SDES item extends beyond chunk boundary.
    #[error("SDES item overflow")]
    SdesItemOverflow,
}

/// Errors that can occur when parsing interleaved RTP frames.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InterleavedError {
    /// Not enough data for header.
    #[error("incomplete frame header: need 4 bytes, got {0}")]
    IncompleteHeader(usize),

    /// Not enough data for frame payload.
    #[error("incomplete frame: need {expected} bytes, got {actual}")]
    IncompleteFrame { expected: usize, actual: usize },

    /// Frame doesn't start with '$'.
    #[error("invalid frame magic: expected '$' (0x24), got 0x{0:02x}")]
    InvalidMagic(u8),
}
