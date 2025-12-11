//! RTP packet parsing (RFC 3550 Section 5.1).
//!
//! Provides zero-copy parsing of RTP packets.

use crate::error::RtpError;

/// RTP packet version. MUST be 2 for RFC 3550.
pub const RTP_VERSION: u8 = 2;

/// Minimum RTP header size (without CSRC or extensions).
pub const RTP_HEADER_SIZE: usize = 12;

/// Maximum number of CSRC entries (4-bit CC field).
pub const MAX_CSRC_COUNT: usize = 15;

/// Profile ID for one-byte header extensions (RFC 5285).
pub const EXTENSION_PROFILE_ONE_BYTE: u16 = 0xBEDE;

/// Profile ID mask for two-byte header extensions (RFC 5285).
/// Two-byte extensions use 0x100X where X is appbits.
pub const EXTENSION_PROFILE_TWO_BYTE_MASK: u16 = 0xFFF0;
pub const EXTENSION_PROFILE_TWO_BYTE: u16 = 0x1000;

/// Parsed RTP header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpHeader {
    /// RTP version (MUST be 2).
    pub version: u8,
    /// Padding flag - if set, last byte of payload is padding count.
    pub padding: bool,
    /// Extension flag - if set, header extension follows CSRC list.
    pub extension: bool,
    /// Marker bit - profile-defined (e.g., end of frame for video).
    pub marker: bool,
    /// Payload type (0-127).
    pub payload_type: u8,
    /// Sequence number (increments by 1 per packet).
    pub sequence: u16,
    /// Timestamp (sampling instant of first octet).
    pub timestamp: u32,
    /// Synchronization source identifier.
    pub ssrc: u32,
}

/// Zero-copy view of an RTP packet.
#[derive(Debug, Clone)]
pub struct RtpPacket<'a> {
    /// Parsed header fields.
    pub header: RtpHeader,
    /// CSRC list (0-15 entries, from mixer).
    pub csrc: &'a [u8],
    /// Header extension data (if present).
    pub extension: Option<RtpExtension<'a>>,
    /// Payload data (zero-copy slice).
    pub payload: &'a [u8],
    /// Number of padding bytes (0 if no padding).
    pub padding_len: u8,
}

/// RTP header extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpExtension<'a> {
    /// Profile-defined identifier.
    pub profile: u16,
    /// Extension data.
    pub data: &'a [u8],
}

impl RtpHeader {
    /// Parse RTP header from bytes.
    ///
    /// Returns the header and the number of bytes consumed.
    pub fn parse(data: &[u8]) -> Result<(Self, usize), RtpError> {
        if data.len() < RTP_HEADER_SIZE {
            return Err(RtpError::PacketTooShort {
                expected: RTP_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let byte0 = data[0];
        let byte1 = data[1];

        let version = (byte0 >> 6) & 0x03;
        if version != RTP_VERSION {
            return Err(RtpError::InvalidVersion(version));
        }

        let padding = (byte0 >> 5) & 0x01 != 0;
        let extension = (byte0 >> 4) & 0x01 != 0;
        let csrc_count = (byte0 & 0x0F) as usize;

        let marker = (byte1 >> 7) & 0x01 != 0;
        let payload_type = byte1 & 0x7F;

        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        // Calculate header size including CSRC list
        let header_size = RTP_HEADER_SIZE + (csrc_count * 4);
        if data.len() < header_size {
            return Err(RtpError::CsrcOverflow {
                expected: header_size,
                actual: data.len(),
            });
        }

        Ok((
            Self {
                version,
                padding,
                extension,
                marker,
                payload_type,
                sequence,
                timestamp,
                ssrc,
            },
            header_size,
        ))
    }

    /// Serialize header to bytes.
    pub fn write(&self, csrc_count: u8, buf: &mut [u8]) -> usize {
        let byte0 = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (csrc_count & 0x0F);
        let byte1 = ((self.marker as u8) << 7) | (self.payload_type & 0x7F);

        buf[0] = byte0;
        buf[1] = byte1;
        buf[2..4].copy_from_slice(&self.sequence.to_be_bytes());
        buf[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        buf[8..12].copy_from_slice(&self.ssrc.to_be_bytes());

        RTP_HEADER_SIZE
    }
}

impl<'a> RtpPacket<'a> {
    /// Parse an RTP packet from bytes.
    ///
    /// Returns a zero-copy view of the packet.
    pub fn parse(data: &'a [u8]) -> Result<Self, RtpError> {
        let (header, _) = RtpHeader::parse(data)?;

        // Extract CSRC count from original byte
        let csrc_count = (data[0] & 0x0F) as usize;
        let csrc_end = RTP_HEADER_SIZE + (csrc_count * 4);
        let csrc = &data[RTP_HEADER_SIZE..csrc_end];
        let mut offset = csrc_end;

        // Parse extension if present
        let extension = if header.extension {
            if data.len() < offset + 4 {
                return Err(RtpError::ExtensionOverflow);
            }

            let profile = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let ext_words = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            let ext_len = ext_words * 4;

            if data.len() < offset + 4 + ext_len {
                return Err(RtpError::ExtensionOverflow);
            }

            let ext_data = &data[offset + 4..offset + 4 + ext_len];
            offset += 4 + ext_len;

            Some(RtpExtension {
                profile,
                data: ext_data,
            })
        } else {
            None
        };

        // Handle padding
        let padding_len = if header.padding {
            if offset >= data.len() {
                return Err(RtpError::InvalidPadding(0));
            }
            let pad = data[data.len() - 1];
            if pad == 0 || pad as usize > data.len() - offset {
                return Err(RtpError::InvalidPadding(pad));
            }
            pad
        } else {
            0
        };

        // Payload is everything between header/extension and padding
        let payload_end = data.len() - padding_len as usize;
        let payload = &data[offset..payload_end];

        Ok(Self {
            header,
            csrc,
            extension,
            payload,
            padding_len,
        })
    }

    /// Get the CSRC list as u32 values.
    pub fn csrc_list(&self) -> impl Iterator<Item = u32> + '_ {
        self.csrc
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    /// Get the number of CSRC entries.
    pub fn csrc_count(&self) -> usize {
        self.csrc.len() / 4
    }

    /// Check if this is a one-byte header extension (RFC 5285).
    pub fn is_one_byte_extension(&self) -> bool {
        self.extension
            .as_ref()
            .map(|e| e.profile == EXTENSION_PROFILE_ONE_BYTE)
            .unwrap_or(false)
    }

    /// Check if this is a two-byte header extension (RFC 5285).
    pub fn is_two_byte_extension(&self) -> bool {
        self.extension
            .as_ref()
            .map(|e| (e.profile & EXTENSION_PROFILE_TWO_BYTE_MASK) == EXTENSION_PROFILE_TWO_BYTE)
            .unwrap_or(false)
    }
}

/// Iterator over one-byte header extension elements (RFC 5285).
pub struct OneByteExtensionIter<'a> {
    data: &'a [u8],
    offset: usize,
}

/// A single one-byte extension element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneByteExtensionElement<'a> {
    /// Extension ID (1-14).
    pub id: u8,
    /// Extension data.
    pub data: &'a [u8],
}

impl<'a> OneByteExtensionIter<'a> {
    /// Create iterator from extension data.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for OneByteExtensionIter<'a> {
    type Item = OneByteExtensionElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.offset < self.data.len() {
            let byte = self.data[self.offset];

            // Skip padding bytes (0x00)
            if byte == 0 {
                self.offset += 1;
                continue;
            }

            let id = (byte >> 4) & 0x0F;
            let len = (byte & 0x0F) as usize + 1;

            // ID 15 is terminator
            if id == 15 {
                return None;
            }

            // ID 0 is padding (should have been handled above)
            if id == 0 {
                self.offset += 1;
                continue;
            }

            let data_start = self.offset + 1;
            let data_end = data_start + len;

            if data_end > self.data.len() {
                return None;
            }

            let element = OneByteExtensionElement {
                id,
                data: &self.data[data_start..data_end],
            };

            self.offset = data_end;
            return Some(element);
        }

        None
    }
}

impl<'a> RtpExtension<'a> {
    /// Iterate over one-byte extension elements.
    ///
    /// Returns None if this is not a one-byte extension.
    pub fn one_byte_elements(&self) -> Option<OneByteExtensionIter<'a>> {
        if self.profile == EXTENSION_PROFILE_ONE_BYTE {
            Some(OneByteExtensionIter::new(self.data))
        } else {
            None
        }
    }
}
