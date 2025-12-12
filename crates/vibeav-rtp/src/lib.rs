//! RTP/RTCP packet parsing for vibeav.
//!
//! This crate provides zero-copy parsing for:
//! - RTP packets (RFC 3550)
//! - RTCP packets (SR, RR, SDES, BYE, APP)
//! - Interleaved RTP/RTCP framing (for TCP transport)
//!
//! # Examples
//!
//! ## Parsing an RTP packet
//!
//! ```
//! use vibeav_rtp::rtp::RtpPacket;
//!
//! # fn example(data: &[u8]) -> Result<(), vibeav_rtp::error::RtpError> {
//! let packet = RtpPacket::parse(data)?;
//! println!("SSRC: {:08x}", packet.header.ssrc);
//! println!("Sequence: {}", packet.header.sequence);
//! println!("Payload: {} bytes", packet.payload.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Parsing RTCP compound packets
//!
//! ```
//! use vibeav_rtp::rtcp::{parse_compound_rtcp, RtcpPacket};
//!
//! # fn example(data: &[u8]) {
//! for result in parse_compound_rtcp(data) {
//!     match result {
//!         Ok(RtcpPacket::Sr(sr)) => {
//!             println!("Sender Report from {:08x}", sr.ssrc);
//!         }
//!         Ok(RtcpPacket::Rr(rr)) => {
//!             println!("Receiver Report from {:08x}", rr.ssrc);
//!         }
//!         Ok(RtcpPacket::Sdes(sdes)) => {
//!             for (ssrc, cname) in sdes.cnames() {
//!                 println!("CNAME {:08x}: {}", ssrc, cname);
//!             }
//!         }
//!         Ok(RtcpPacket::Bye(bye)) => {
//!             for ssrc in bye.ssrcs() {
//!                 println!("BYE from {:08x}", ssrc);
//!             }
//!         }
//!         _ => {}
//!     }
//! }
//! # }
//! ```
//!
//! ## Parsing interleaved frames
//!
//! ```
//! use vibeav_rtp::interleaved::InterleavedFrame;
//!
//! # fn example(data: &[u8]) -> Result<(), vibeav_rtp::error::InterleavedError> {
//! let (frame, consumed) = InterleavedFrame::parse(data)?;
//! if frame.is_rtp() {
//!     println!("RTP on channel {}", frame.channel);
//! } else {
//!     println!("RTCP on channel {}", frame.channel);
//! }
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod interleaved;
pub mod rtcp;
pub mod rtp;

// Re-export commonly used types at crate root
pub use error::{InterleavedError, RtcpError, RtpError};
pub use interleaved::{InterleavedFrame, InterleavedFrameIter};
pub use rtcp::{
    parse_compound_rtcp, parse_rtcp_packet, ReceptionReport, RtcpApp, RtcpBye, RtcpHeader,
    RtcpPacket, RtcpPacketType, RtcpRr, RtcpSdes, RtcpSr, SdesChunk, SdesItem, SdesItemType,
    SenderInfo,
};
pub use rtp::{
    OneByteExtensionElement, OneByteExtensionIter, RtpExtension, RtpHeader, RtpPacket,
    RTP_HEADER_SIZE, RTP_VERSION,
};
