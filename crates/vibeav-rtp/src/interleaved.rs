//! Interleaved RTP/RTCP framing for TCP transport (RTSP Section 10.12).
//!
//! When RTP/RTCP is transported over TCP (e.g., RTSP interleaved mode),
//! each packet is prefixed with a 4-byte header:
//!
//! ```text
//! | '$' | channel | length (16-bit BE) | data... |
//! ```

use crate::error::InterleavedError;

/// Magic byte indicating start of interleaved frame.
pub const INTERLEAVED_MAGIC: u8 = b'$';

/// Interleaved frame header size.
pub const INTERLEAVED_HEADER_SIZE: usize = 4;

/// Interleaved RTP/RTCP frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterleavedFrame<'a> {
    /// Channel number (typically even=RTP, odd=RTCP).
    pub channel: u8,
    /// Frame payload data.
    pub data: &'a [u8],
}

impl<'a> InterleavedFrame<'a> {
    /// Parse an interleaved frame from bytes.
    ///
    /// Returns the frame and total bytes consumed.
    pub fn parse(data: &'a [u8]) -> Result<(Self, usize), InterleavedError> {
        if data.len() < INTERLEAVED_HEADER_SIZE {
            return Err(InterleavedError::IncompleteHeader(data.len()));
        }

        if data[0] != INTERLEAVED_MAGIC {
            return Err(InterleavedError::InvalidMagic(data[0]));
        }

        let channel = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]) as usize;
        let total_size = INTERLEAVED_HEADER_SIZE + length;

        if data.len() < total_size {
            return Err(InterleavedError::IncompleteFrame {
                expected: total_size,
                actual: data.len(),
            });
        }

        let payload = &data[INTERLEAVED_HEADER_SIZE..total_size];

        Ok((
            Self {
                channel,
                data: payload,
            },
            total_size,
        ))
    }

    /// Check if this frame contains RTP data (even channel).
    pub fn is_rtp(&self) -> bool {
        self.channel % 2 == 0
    }

    /// Check if this frame contains RTCP data (odd channel).
    pub fn is_rtcp(&self) -> bool {
        self.channel % 2 == 1
    }

    /// Get the corresponding RTP channel for this RTCP channel.
    ///
    /// Returns None if this is already an RTP channel.
    pub fn rtp_channel(&self) -> Option<u8> {
        if self.is_rtcp() {
            Some(self.channel - 1)
        } else {
            None
        }
    }

    /// Get the corresponding RTCP channel for this RTP channel.
    ///
    /// Returns None if this is already an RTCP channel.
    pub fn rtcp_channel(&self) -> Option<u8> {
        if self.is_rtp() {
            Some(self.channel + 1)
        } else {
            None
        }
    }
}

/// Write an interleaved frame header to a buffer.
///
/// Returns the header size (4 bytes).
pub fn write_interleaved_header(buf: &mut [u8], channel: u8, length: u16) -> usize {
    buf[0] = INTERLEAVED_MAGIC;
    buf[1] = channel;
    buf[2..4].copy_from_slice(&length.to_be_bytes());
    INTERLEAVED_HEADER_SIZE
}

/// Builder for interleaved frames.
pub struct InterleavedFrameBuilder {
    channel: u8,
}

impl InterleavedFrameBuilder {
    /// Create a new builder for the given channel.
    pub fn new(channel: u8) -> Self {
        Self { channel }
    }

    /// Build the frame header + data into a buffer.
    ///
    /// Returns the total bytes written.
    pub fn build(&self, buf: &mut [u8], data: &[u8]) -> usize {
        let length = data.len().min(u16::MAX as usize) as u16;
        write_interleaved_header(buf, self.channel, length);
        buf[INTERLEAVED_HEADER_SIZE..INTERLEAVED_HEADER_SIZE + data.len()]
            .copy_from_slice(data);
        INTERLEAVED_HEADER_SIZE + data.len()
    }

    /// Calculate required buffer size for given payload.
    pub fn required_size(payload_len: usize) -> usize {
        INTERLEAVED_HEADER_SIZE + payload_len
    }
}

/// Iterator over interleaved frames in a buffer.
pub struct InterleavedFrameIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> InterleavedFrameIter<'a> {
    /// Create a new iterator over interleaved frames.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Get the remaining unprocessed data.
    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.offset..]
    }
}

impl<'a> Iterator for InterleavedFrameIter<'a> {
    type Item = Result<InterleavedFrame<'a>, InterleavedError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        // Skip any non-'$' bytes (could be RTSP text response mixed in)
        while self.offset < self.data.len() && self.data[self.offset] != INTERLEAVED_MAGIC {
            self.offset += 1;
        }

        if self.offset >= self.data.len() {
            return None;
        }

        match InterleavedFrame::parse(&self.data[self.offset..]) {
            Ok((frame, size)) => {
                self.offset += size;
                Some(Ok(frame))
            }
            Err(InterleavedError::IncompleteHeader(_) | InterleavedError::IncompleteFrame { .. }) => {
                // Not enough data yet, stop iteration but don't error
                None
            }
            Err(e) => {
                // Invalid magic or other error
                self.offset = self.data.len();
                Some(Err(e))
            }
        }
    }
}

/// Check if a buffer starts with an interleaved frame.
pub fn is_interleaved(data: &[u8]) -> bool {
    !data.is_empty() && data[0] == INTERLEAVED_MAGIC
}

/// Get the expected frame size from header (if complete).
pub fn peek_frame_size(data: &[u8]) -> Option<usize> {
    if data.len() < INTERLEAVED_HEADER_SIZE {
        return None;
    }

    if data[0] != INTERLEAVED_MAGIC {
        return None;
    }

    let length = u16::from_be_bytes([data[2], data[3]]) as usize;
    Some(INTERLEAVED_HEADER_SIZE + length)
}
