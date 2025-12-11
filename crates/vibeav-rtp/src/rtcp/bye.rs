//! RTCP BYE (Goodbye) packet parsing (RFC 3550 Section 6.6).

use super::RtcpHeader;
use crate::error::RtcpError;

/// RTCP BYE packet.
#[derive(Debug, Clone)]
pub struct RtcpBye<'a> {
    /// Raw SSRC data.
    ssrcs_data: &'a [u8],
    /// Number of SSRCs.
    ssrc_count: u8,
    /// Optional reason for leaving.
    pub reason: Option<&'a [u8]>,
}

impl<'a> RtcpBye<'a> {
    /// Parse BYE packet from payload (after header).
    pub fn parse(header: &RtcpHeader, data: &'a [u8]) -> Result<Self, RtcpError> {
        let ssrc_count = header.count;
        let ssrcs_size = ssrc_count as usize * 4;

        if data.len() < ssrcs_size {
            return Err(RtcpError::PacketTooShort {
                expected: ssrcs_size,
                actual: data.len(),
            });
        }

        let ssrcs_data = &data[..ssrcs_size];

        // Check for optional reason string
        let reason = if data.len() > ssrcs_size {
            let reason_start = ssrcs_size;
            if reason_start < data.len() {
                let reason_len = data[reason_start] as usize;
                let reason_data_start = reason_start + 1;
                let reason_data_end = reason_data_start + reason_len;

                if reason_data_end <= data.len() {
                    Some(&data[reason_data_start..reason_data_end])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            ssrcs_data,
            ssrc_count,
            reason,
        })
    }

    /// Get the number of SSRCs.
    pub fn ssrc_count(&self) -> u8 {
        self.ssrc_count
    }

    /// Iterate over SSRCs that are leaving.
    pub fn ssrcs(&self) -> impl Iterator<Item = u32> + '_ {
        self.ssrcs_data.chunks_exact(4).map(|chunk| {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        })
    }

    /// Get reason as string.
    pub fn reason_str(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.reason.map(String::from_utf8_lossy)
    }
}

/// Builder for creating BYE packets.
pub struct RtcpByeBuilder {
    ssrcs: Vec<u32>,
    reason: Option<String>,
}

impl RtcpByeBuilder {
    /// Create a new BYE builder.
    pub fn new() -> Self {
        Self {
            ssrcs: Vec::new(),
            reason: None,
        }
    }

    /// Add an SSRC to the BYE packet.
    pub fn add_ssrc(mut self, ssrc: u32) -> Self {
        if self.ssrcs.len() < 31 {
            self.ssrcs.push(ssrc);
        }
        self
    }

    /// Set the reason for leaving.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Build the BYE packet into a buffer.
    ///
    /// Returns the number of bytes written.
    pub fn build(&self, buf: &mut [u8]) -> usize {
        let sc = self.ssrcs.len() as u8;

        // Calculate total size
        let ssrcs_size = self.ssrcs.len() * 4;
        let reason_size = self.reason.as_ref().map_or(0, |r| {
            let len = r.len().min(255);
            // 1 byte length + reason + padding to 32-bit boundary
            let unpadded = 1 + len;
            ((unpadded + 3) / 4) * 4
        });
        let total_size = 4 + ssrcs_size + reason_size;
        let length_words = (total_size / 4) - 1;

        // Header
        buf[0] = 0x80 | sc;
        buf[1] = 203; // PT=BYE
        buf[2] = ((length_words >> 8) & 0xFF) as u8;
        buf[3] = (length_words & 0xFF) as u8;

        // SSRCs
        let mut offset = 4;
        for ssrc in &self.ssrcs {
            buf[offset..offset + 4].copy_from_slice(&ssrc.to_be_bytes());
            offset += 4;
        }

        // Reason (optional)
        if let Some(reason) = &self.reason {
            let len = reason.len().min(255);
            buf[offset] = len as u8;
            offset += 1;
            buf[offset..offset + len].copy_from_slice(&reason.as_bytes()[..len]);
            offset += len;

            // Padding to 32-bit boundary
            while offset % 4 != 0 {
                buf[offset] = 0;
                offset += 1;
            }
        }

        offset
    }
}

impl Default for RtcpByeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
